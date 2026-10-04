//! The open world as the player perceives it: positions, sight lines,
//! landmarks, followable edges, weather, and the cost of walking.
//!
//! One rule drives all of it: *you can only be surprised by what you
//! couldn't see.* Everything the player is told about the land comes from
//! what is in sight from where they stand.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use serde::{Deserialize, Serialize};

use scraped_world::history::Settlement;
use scraped_world::structures::StructureKind;
use scraped_world::terrain::{Biome, Grid, CELL_METRES, SIZE};
use scraped_world::World;

/// Metres per cell.
pub const CELL: i32 = CELL_METRES as i32;
/// Eye height of a standing person, in metres.
pub const EYE: f64 = 1.7;
/// Things outdoors within this many metres are "here".
pub const LOCAL: i32 = 300;
/// Within this many metres of a settlement's centre the player is in it.
pub const TOWN: i32 = 1000;
/// How far one travel step goes, in metres.
pub const STEP: f64 = 300.0;
/// Close enough to a destination to have arrived, in metres.
pub const ARRIVE: f64 = 200.0;

/// A point on the land, in metres from the north-west corner (y grows
/// southwards). Whole metres keep the state exact and comparable.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32) -> Self {
        Pos { x, y }
    }

    /// The centre of a cell.
    pub fn of_cell(x: usize, y: usize) -> Self {
        Pos::new(x as i32 * CELL + CELL / 2, y as i32 * CELL + CELL / 2)
    }

    pub fn cell(self) -> (usize, usize) {
        let c = |v: i32| (v.div_euclid(CELL)).clamp(0, SIZE as i32 - 1) as usize;
        (c(self.x), c(self.y))
    }

    pub fn dist(self, o: Pos) -> f64 {
        (self.dist2(o) as f64).sqrt()
    }

    pub fn dist2(self, o: Pos) -> i64 {
        let (dx, dy) = (i64::from(o.x - self.x), i64::from(o.y - self.y));
        dx * dx + dy * dy
    }

    /// Moves by a vector in metres, rounding to whole metres and staying on
    /// the map.
    pub fn moved(self, dx: f64, dy: f64) -> Self {
        let max = (SIZE as i32) * CELL - 1;
        let r = |v: f64| {
            let f = v.floor();
            (if v - f >= 0.5 { f + 1.0 } else { f }) as i32
        };
        Pos::new(
            (self.x + r(dx)).clamp(0, max),
            (self.y + r(dy)).clamp(0, max),
        )
    }
}

/// The serde id of a value (its `rename_all` spelling), for content
/// variables.
pub fn label<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_value(x)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Time of day from minutes since midnight of the first day.
pub fn time_of_day(minutes: u32) -> &'static str {
    match (minutes / 60) % 24 {
        5..=7 => "dawn",
        8..=11 => "morning",
        12..=16 => "afternoon",
        17..=20 => "evening",
        _ => "night",
    }
}

// ---------- bearings and rough measures ----------

/// The eight compass points, clockwise from north. Ids for content.
pub const BEARINGS: [&str; 8] = [
    "north",
    "northeast",
    "east",
    "southeast",
    "south",
    "southwest",
    "west",
    "northwest",
];

/// The compass point from one position to another, or `None` if they are
/// the same spot. Exact integer comparison against tan(22.5°).
pub fn bearing(from: Pos, to: Pos) -> Option<usize> {
    let dx = i64::from(to.x - from.x);
    let dn = i64::from(from.y - to.y);
    if dx == 0 && dn == 0 {
        return None;
    }
    let (ax, an) = (dx.abs(), dn.abs());
    // tan(22.5°) = 0.41421356…
    Some(if ax * 100_000 <= an * 41_421 {
        if dn > 0 {
            0
        } else {
            4
        }
    } else if an * 100_000 <= ax * 41_421 {
        if dx > 0 {
            2
        } else {
            6
        }
    } else {
        match (dx > 0, dn > 0) {
            (true, true) => 1,
            (true, false) => 3,
            (false, false) => 5,
            (false, true) => 7,
        }
    })
}

/// A compass point parsed from a word ("ne", "north-east", "northeast").
pub fn parse_bearing(word: &str) -> Option<usize> {
    let w: String = word.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    const SHORT: [&str; 8] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];
    BEARINGS
        .iter()
        .position(|b| *b == w)
        .or_else(|| SHORT.iter().position(|b| *b == w))
}

/// The unit vector of a compass point (x east, y south).
pub fn unit(b: usize) -> (f64, f64) {
    const R: f64 = std::f64::consts::FRAC_1_SQRT_2;
    [
        (0.0, -1.0),
        (R, -R),
        (1.0, 0.0),
        (R, R),
        (0.0, 1.0),
        (-R, R),
        (-1.0, 0.0),
        (-R, -R),
    ][b % 8]
}

/// Rough distance bands, as the eye judges them. Ids for content.
pub const DISTANCES: [&str; 5] = ["near", "short", "middle", "far", "horizon"];

pub fn distance_band(m: f64) -> &'static str {
    match m {
        m if m < 400.0 => "near",
        m if m < 1500.0 => "short",
        m if m < 4000.0 => "middle",
        m if m < 10000.0 => "far",
        _ => "horizon",
    }
}

/// Rough durations of a walk. Ids for content.
pub const DURATIONS: [&str; 7] = [
    "moments", "quarter", "half", "hour", "hours", "half_day", "day",
];

pub fn duration_band(minutes: u32) -> &'static str {
    match minutes {
        0..=9 => "moments",
        10..=24 => "quarter",
        25..=44 => "half",
        45..=79 => "hour",
        80..=239 => "hours",
        240..=479 => "half_day",
        _ => "day",
    }
}

/// A distance as a traveller would put it: to 100 m under a kilometre,
/// to half a kilometre above.
// DESIGN-Q: reports give rounded metres so maps can be drawn; content
// decides whether to say "about two kilometres" or "a long hour's walk".
pub fn rough_metres(m: f64) -> i64 {
    let step = if m < 1000.0 { 100.0 } else { 500.0 };
    let n = (m / step + 0.5).floor();
    (n * step) as i64
}

/// Minutes rounded to five under a quarter hour, then to quarter hours.
pub fn rough_minutes(m: u32) -> i64 {
    let step = if m < 15 { 5 } else { 15 };
    i64::from((m + step / 2) / step * step)
}

// ---------- weather and light ----------

pub const WEATHERS: [&str; 5] = ["clear", "rain", "fog", "storm", "snow"];

fn mix(mut h: u64) -> u64 {
    h ^= h >> 31;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 29;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 32)
}

/// A deterministic hash of a few numbers, for weather and drift.
pub fn hash(parts: &[u64]) -> u64 {
    parts.iter().fold(0x6a09_e667_f3bc_c909, |h, &p| {
        mix(h ^ p.wrapping_mul(0x9e37_79b9_7f4a_7c15))
    })
}

/// Light outdoors at a time of day.
pub fn outdoor_light(minutes: u32) -> &'static str {
    match time_of_day(minutes) {
        "dawn" | "evening" => "dim",
        "night" => "dark",
        _ => "daylight",
    }
}

/// Weather at a place and time: moving fronts (D06) over the place's own
/// dawn mists.
pub fn weather(w: &World, pos: Pos, minutes: u32) -> &'static str {
    crate::weather::weather(w, pos, minutes)
}

/// How far the eye reaches, in metres.
// DESIGN-Q: clear daylight 20 km, rain 3 km, storm and snow 1 km, fog
// 200 m; dawn and dusk
// cut range to 40%, night to 500 m at most (starlight on skylines).
pub fn sight_range(weather: &str, light: &str) -> f64 {
    let base = match weather {
        "fog" => 200.0,
        "rain" => 3000.0,
        "storm" | "snow" => 1000.0,
        _ => 20000.0,
    };
    match light {
        "daylight" => base,
        "dim" => base * 0.4,
        _ => f64::min(base, 500.0),
    }
}

// ---------- the land ----------

/// Edges a traveller can follow. Ids for content.
pub const EDGES: [&str; 7] = [
    "river",
    "stream",
    "road",
    "coast",
    "lakeshore",
    "treeline",
    "cliff",
];

/// Something visible from afar.
#[derive(Debug, Clone)]
pub struct Landmark {
    /// "town", "ruins", "hill", "mountain", or a structure kind.
    pub kind: &'static str,
    pub pos: Pos,
    /// How far it stands above the ground, in metres (0 for a summit).
    pub height: f64,
    pub settlement: Option<usize>,
    pub structure: Option<usize>,
    /// A natural feature or old mark seen from afar (D03), by index into
    /// `World::features`.
    pub feature: Option<usize>,
    /// Settlement size (1–4), or prominence in hundreds of metres for hills.
    pub size: i64,
    /// How much it draws the eye at close range.
    pub weight: f64,
    /// What sets it apart from others of its kind (D01).
    pub traits: crate::traits::Traits,
}

/// The land prepared for perception and travel.
pub struct Land {
    /// Height of the visible surface: ground, or water.
    pub surface: Grid<f64>,
    pub landmarks: Vec<Landmark>,
    /// Bit `i` set when edge `EDGES[i]` runs through the cell.
    pub edges: Grid<u8>,
    /// Cells a bridge makes crossable.
    pub bridges: Grid<bool>,
    /// Where each structure stands.
    pub structure_pos: Vec<Pos>,
}

fn structure_height(k: StructureKind) -> (f64, f64) {
    // (height above ground in metres, weight)
    (k.info().height, k.info().weight)
}

impl Land {
    pub fn new(w: &World) -> Self {
        let s = SIZE;
        let mut surface = w.terrain.height.clone();
        for y in 0..s {
            for x in 0..s {
                match w.terrain.biome.get(x, y) {
                    Biome::Sea => surface.set(x, y, 0.0),
                    Biome::Lake => surface.set(x, y, *w.water.filled.get(x, y)),
                    _ => {}
                }
            }
        }
        let structure_pos: Vec<Pos> = w
            .structures
            .iter()
            .map(|st| {
                let h = hash(&[w.seed, 0x5354, st.id as u64]);
                let j = |v: u64| (v % 241) as i32 - 120;
                let c = Pos::of_cell(st.cell.ux(), st.cell.uy());
                Pos::new(c.x + j(h), c.y + j(h >> 20))
            })
            .collect();
        let mut bridges = Grid::new(s, false);
        for st in &w.structures {
            if st.kind == StructureKind::Bridge {
                bridges.set(st.cell.ux(), st.cell.uy(), true);
            }
        }

        let mut landmarks = Vec::new();
        for st in &w.history.settlements {
            landmarks.push(town_landmark(st));
        }
        for st in &w.structures {
            if st.settlement.is_some() {
                continue;
            }
            let (height, weight) = structure_height(st.kind);
            // Caves are found by their mouths, not seen from afar.
            if height <= 0.0 {
                continue;
            }
            landmarks.push(Landmark {
                kind: kind_id(st.kind),
                pos: structure_pos[st.id],
                height,
                settlement: None,
                structure: Some(st.id),
                feature: None,
                size: 1,
                weight,
                traits: Default::default(),
            });
        }
        // Features that stand up from the land (D03); the rest are only
        // found close by.
        for f in &w.features {
            let k = scraped_world::features::kind(f.kind);
            if k.height <= 0.0 {
                continue;
            }
            // One standing in water (a waterfall, a sea stack) is reached
            // from the nearest dry bank.
            let (mut fx, mut fy) = (f.cell.ux(), f.cell.uy());
            let dry = |x: usize, y: usize| {
                *bridges.get(x, y)
                    || (!w.terrain.biome.get(x, y).is_water()
                        && !w.water.needs_crossing(&w.terrain, x, y))
            };
            if !dry(fx, fy) {
                let mut best: Option<(i64, usize, usize)> = None;
                for dy in -3i64..=3 {
                    for dx in -3i64..=3 {
                        let (x, y) = (fx as i64 + dx, fy as i64 + dy);
                        if x < 0 || y < 0 || x >= s as i64 || y >= s as i64 {
                            continue;
                        }
                        let d = dx * dx + dy * dy;
                        if dry(x as usize, y as usize) && best.is_none_or(|(b, ..)| d < b) {
                            best = Some((d, x as usize, y as usize));
                        }
                    }
                }
                let Some((_, x, y)) = best else { continue };
                (fx, fy) = (x, y);
            }
            landmarks.push(Landmark {
                kind: f.kind,
                pos: Pos::of_cell(fx, fy),
                height: k.height,
                settlement: None,
                structure: None,
                feature: Some(f.id),
                size: 1,
                weight: k.weight,
                traits: Default::default(),
            });
        }
        landmarks.extend(peaks(w, &surface));
        crate::traits::assign(w, &surface, &mut landmarks, &structure_pos);

        let mut edges = Grid::new(s, 0u8);
        let set = |g: &mut Grid<u8>, x: usize, y: usize, e: usize| {
            let v = *g.get(x, y) | (1 << e);
            g.set(x, y, v);
        };
        for r in &w.history.roads {
            for c in &r.path {
                set(&mut edges, c.ux(), c.uy(), 2);
            }
        }
        for y in 0..s {
            for x in 0..s {
                let b = *w.terrain.biome.get(x, y);
                if b.is_water() {
                    continue;
                }
                if w.water.is_river(&w.terrain, x, y) {
                    set(&mut edges, x, y, 0);
                } else if *w.water.flow.get(x, y) >= scraped_world::water::RIVER_FLOW / 4
                    && !*w.water.pooled.get(x, y)
                {
                    set(&mut edges, x, y, 1);
                }
                let h = *w.terrain.height.get(x, y);
                let wooded = matches!(b, Biome::Forest | Biome::Pine);
                for (nx, ny) in w.terrain.height.neighbours(x, y) {
                    let nb = *w.terrain.biome.get(nx, ny);
                    if nb == Biome::Sea {
                        set(&mut edges, x, y, 3);
                    }
                    if nb == Biome::Lake {
                        set(&mut edges, x, y, 4);
                    }
                    if wooded && !nb.is_water() && !matches!(nb, Biome::Forest | Biome::Pine) {
                        set(&mut edges, x, y, 5);
                    }
                    if !nb.is_water() && (h - *w.terrain.height.get(nx, ny)).abs() > 90.0 {
                        set(&mut edges, x, y, 6);
                    }
                }
            }
        }
        Land {
            surface,
            landmarks,
            edges,
            bridges,
            structure_pos,
        }
    }

    /// Height of the surface at a point, between cell centres.
    pub fn ground(&self, p: Pos) -> f64 {
        let half = f64::from(CELL) / 2.0;
        let max = (SIZE - 1) as f64;
        let fx = ((f64::from(p.x) - half) / f64::from(CELL)).clamp(0.0, max);
        let fy = ((f64::from(p.y) - half) / f64::from(CELL)).clamp(0.0, max);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(SIZE - 1), (y0 + 1).min(SIZE - 1));
        let (tx, ty) = (fx - x0 as f64, fy - y0 as f64);
        let g = |x, y| *self.surface.get(x, y);
        let top = g(x0, y0) + (g(x1, y0) - g(x0, y0)) * tx;
        let bottom = g(x0, y1) + (g(x1, y1) - g(x0, y1)) * tx;
        top + (bottom - top) * ty
    }

    pub fn has_edge(&self, x: usize, y: usize, e: usize) -> bool {
        *self.edges.get(x, y) & (1 << e) != 0
    }

    /// Whether a cell can be walked into.
    pub fn passable(&self, w: &World, x: usize, y: usize) -> bool {
        if *self.bridges.get(x, y) {
            return true;
        }
        !w.terrain.biome.get(x, y).is_water() && !w.water.needs_crossing(&w.terrain, x, y)
    }

    /// What stops a walker at a cell: "sea", "lake", "river", or `None`.
    pub fn obstacle(&self, w: &World, x: usize, y: usize) -> Option<&'static str> {
        if self.passable(w, x, y) {
            return None;
        }
        Some(match w.terrain.biome.get(x, y) {
            Biome::Sea => "sea",
            Biome::Lake => "lake",
            _ => "river",
        })
    }

    /// Whether `b` (standing `hb` metres tall) can be seen from `a` (eyes
    /// `ha` metres up) within `range`. Trees beyond the nearest 100 m of
    /// either end block the view. Symmetric by construction.
    pub fn sight(&self, w: &World, a: Pos, ha: f64, b: Pos, hb: f64, range: f64) -> bool {
        let ((a, ha), (b, hb)) = if a <= b {
            ((a, ha), (b, hb))
        } else {
            ((b, hb), (a, ha))
        };
        let d = a.dist(b);
        if d > range {
            return false;
        }
        let za = self.ground(a) + ha;
        let zb = self.ground(b) + hb;
        let n = (d / 100.0).floor() as i64;
        for k in 1..n {
            let t = k as f64 / n as f64;
            let p = Pos::new(
                a.x + ((f64::from(b.x - a.x)) * t).floor() as i32,
                a.y + ((f64::from(b.y - a.y)) * t).floor() as i32,
            );
            let along = d * t;
            let mut h = self.ground(p);
            if along > 100.0 && d - along > 100.0 {
                let (x, y) = p.cell();
                h += match w.terrain.biome.get(x, y) {
                    Biome::Forest => 15.0,
                    Biome::Pine => 20.0,
                    _ => 0.0,
                };
            }
            if h > za + (zb - za) * t {
                return false;
            }
        }
        true
    }

    /// The settlement whose ground a position is in.
    pub fn town(&self, w: &World, p: Pos) -> Option<usize> {
        w.history
            .settlements
            .iter()
            .map(|s| (Pos::of_cell(s.cell.ux(), s.cell.uy()).dist2(p), s.id))
            .filter(|(d, _)| *d <= i64::from(TOWN) * i64::from(TOWN))
            .min()
            .map(|(_, id)| id)
    }

    /// Minutes to walk from `a` to the neighbouring point `b`.
    pub fn walk_minutes(&self, w: &World, a: Pos, b: Pos) -> f64 {
        let (x, y) = b.cell();
        let mut speed = match w.terrain.biome.get(x, y) {
            Biome::Grassland => 75.0,
            Biome::Shore | Biome::Scrub => 65.0,
            Biome::Desert | Biome::Tundra => 60.0,
            Biome::Forest | Biome::Pine => 50.0,
            Biome::Rock => 45.0,
            Biome::Marsh | Biome::Snow => 30.0,
            Biome::Sea | Biome::Lake => 40.0,
        };
        let (ax, ay) = a.cell();
        if self.has_edge(x, y, 2) {
            speed = f64::max(speed, 85.0);
        } else if w.water.is_river(&w.terrain, x, y) && !w.water.is_river(&w.terrain, ax, ay) {
            // Wading in; walking along the bank costs nothing extra.
            speed = f64::min(speed, 30.0);
        }
        let rise = self.ground(b) - self.ground(a);
        let climb = if rise > 0.0 {
            rise / 10.0
        } else {
            -rise / 30.0
        };
        a.dist(b) / speed + climb
    }

    /// A walkable route between cells, cheapest by walking time, or `None`.
    pub fn route(&self, w: &World, from: Pos, to: Pos) -> Option<Vec<Pos>> {
        self.route_by(w, from, to, &|x, y| self.passable(w, x, y))
    }

    /// A route over cells `pass` allows (the land as the player has changed
    /// it: sluices, bridges).
    pub fn route_by(
        &self,
        w: &World,
        from: Pos,
        to: Pos,
        pass: &dyn Fn(usize, usize) -> bool,
    ) -> Option<Vec<Pos>> {
        let s = SIZE;
        let (fx, fy) = from.cell();
        let (tx, ty) = to.cell();
        let idx = |x: usize, y: usize| y * s + x;
        let goal = idx(tx, ty);
        let mut best = vec![u64::MAX; s * s];
        let mut prev = vec![usize::MAX; s * s];
        let mut heap = BinaryHeap::new();
        // Costs in hundredths of a minute, so the heap orders integers.
        let h = |x: usize, y: usize| {
            (Pos::of_cell(x, y).dist(Pos::of_cell(tx, ty)) / 85.0 * 100.0) as u64
        };
        best[idx(fx, fy)] = 0;
        heap.push(Reverse((h(fx, fy), 0u64, idx(fx, fy))));
        while let Some(Reverse((_, g, i))) = heap.pop() {
            if g > best[i] {
                continue;
            }
            if i == goal {
                break;
            }
            let (x, y) = (i % s, i / s);
            for (nx, ny) in w.terrain.height.neighbours(x, y) {
                let j = idx(nx, ny);
                if j != goal && !pass(nx, ny) {
                    continue;
                }
                // No squeezing between two closed cells at a corner.
                if nx != x && ny != y && !pass(nx, y) && !pass(x, ny) {
                    continue;
                }
                let step = self.walk_minutes(w, Pos::of_cell(x, y), Pos::of_cell(nx, ny));
                let ng = g + (step * 100.0) as u64 + 1;
                if ng < best[j] {
                    best[j] = ng;
                    prev[j] = i;
                    heap.push(Reverse((ng + h(nx, ny), ng, j)));
                }
            }
        }
        if best[goal] == u64::MAX {
            return None;
        }
        let mut cells = vec![goal];
        while *cells.last().expect("non-empty") != idx(fx, fy) {
            cells.push(prev[*cells.last().expect("non-empty")]);
        }
        cells.reverse();
        // A diagonal step with one closed cell beside it goes round by the
        // open one, so walking it never clips the closed cell's corner.
        let mut walked = vec![cells[0]];
        for pair in cells.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let (ax, ay, bx, by) = (a % s, a / s, b % s, b / s);
            if ax != bx && ay != by && (!pass(bx, ay) || !pass(ax, by)) {
                walked.push(if pass(bx, ay) {
                    idx(bx, ay)
                } else {
                    idx(ax, by)
                });
            }
            walked.push(b);
        }
        let cells = walked;
        let mut path: Vec<Pos> = cells[1..]
            .iter()
            .map(|&i| Pos::of_cell(i % s, i / s))
            .collect();
        path.pop();
        path.push(to);
        Some(path)
    }

    /// A walk along an edge from the cell nearest `from`: towards a compass
    /// point, or upstream/downstream for water. At most `max` cells.
    pub fn follow(
        &self,
        w: &World,
        edge: usize,
        from: Pos,
        way: Way,
        max: usize,
        obstacle: &dyn Fn(usize, usize) -> Option<&'static str>,
    ) -> (Vec<Pos>, Option<&'static str>) {
        let s = SIZE;
        let start = match self.nearest_edge(from, edge, 1) {
            Some(c) => c,
            None => return (Vec::new(), None),
        };
        let mut path = vec![Pos::of_cell(start.0, start.1)];
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(start);
        let mut cur = start;
        let mut heading = match way {
            Way::Toward(b) => Some(unit(b)),
            _ => None,
        };
        let mut end = None;
        while path.len() < max {
            let next = match way {
                Way::Downstream => {
                    let d = *w.water.down.get(cur.0, cur.1);
                    if d == u32::MAX {
                        None
                    } else {
                        Some((d as usize % s, d as usize / s))
                    }
                }
                Way::Upstream => w
                    .terrain
                    .height
                    .neighbours(cur.0, cur.1)
                    .filter(|&(nx, ny)| {
                        *w.water.down.get(nx, ny) == (cur.1 * s + cur.0) as u32
                            && self.has_edge(nx, ny, edge)
                    })
                    .max_by_key(|&(nx, ny)| (*w.water.flow.get(nx, ny), Reverse((ny, nx)))),
                Way::Toward(_) | Way::Onward => {
                    let here = Pos::of_cell(cur.0, cur.1);
                    let mut best: Option<((usize, usize), f64)> = None;
                    for (nx, ny) in w.terrain.height.neighbours(cur.0, cur.1) {
                        if seen.contains(&(nx, ny)) || !self.has_edge(nx, ny, edge) {
                            continue;
                        }
                        let p = Pos::of_cell(nx, ny);
                        let d = here.dist(p);
                        let v = (f64::from(p.x - here.x) / d, f64::from(p.y - here.y) / d);
                        let score = match heading {
                            Some(h) => v.0 * h.0 + v.1 * h.1,
                            None => 0.0,
                        };
                        if score < -0.2 {
                            continue;
                        }
                        if best.is_none_or(|(_, b)| score > b) {
                            best = Some(((nx, ny), score));
                        }
                    }
                    best.map(|(c, _)| c)
                }
            };
            let Some(n) = next.filter(|n| !seen.contains(n)) else {
                end = Some("end");
                break;
            };
            // Walking a river's bank is fine; open water is not.
            if let Some(o) =
                obstacle(n.0, n.1).filter(|o| *o != "river" || !self.has_edge(n.0, n.1, edge))
            {
                end = Some(o);
                break;
            }
            if matches!(way, Way::Downstream | Way::Upstream) && !self.has_edge(n.0, n.1, edge) {
                // The stream joins a river or fades out.
                if !(edge == 1 && self.has_edge(n.0, n.1, 0)) {
                    end = Some("end");
                    break;
                }
            }
            let (here, there) = (Pos::of_cell(cur.0, cur.1), Pos::of_cell(n.0, n.1));
            let d = here.dist(there);
            let v = (
                f64::from(there.x - here.x) / d,
                f64::from(there.y - here.y) / d,
            );
            heading = Some(match heading {
                // Keep a sense of direction along wiggles.
                Some(h) if matches!(way, Way::Onward) => {
                    (h.0 * 0.5 + v.0 * 0.5, h.1 * 0.5 + v.1 * 0.5)
                }
                Some(h) if matches!(way, Way::Toward(_)) => h,
                _ => v,
            });
            seen.insert(n);
            path.push(there);
            cur = n;
        }
        (path, end)
    }

    /// The nearest cell carrying an edge, within `radius` cells.
    pub fn nearest_edge(&self, p: Pos, edge: usize, radius: i64) -> Option<(usize, usize)> {
        let (cx, cy) = p.cell();
        let mut best: Option<(i64, (usize, usize))> = None;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (x, y) = (cx as i64 + dx, cy as i64 + dy);
                if x < 0 || y < 0 || x >= SIZE as i64 || y >= SIZE as i64 {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                if self.has_edge(x, y, edge) {
                    let d = Pos::of_cell(x, y).dist2(p);
                    if best.is_none_or(|(b, c)| (d, (y, x)) < (b, (c.1, c.0))) {
                        best = Some((d, (x, y)));
                    }
                }
            }
        }
        best.map(|(_, c)| c)
    }
}

/// Which way to follow an edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Way {
    Toward(usize),
    Upstream,
    Downstream,
    /// Whichever way the edge leads on.
    Onward,
}

fn kind_id(k: StructureKind) -> &'static str {
    k.id()
}

fn town_landmark(s: &Settlement) -> Landmark {
    let size = i64::from(s.size);
    let alive = s.abandoned.is_none();
    Landmark {
        kind: if alive { "town" } else { "ruins" },
        pos: Pos::of_cell(s.cell.ux(), s.cell.uy()),
        height: 4.0 + 3.0 * size as f64,
        settlement: Some(s.id),
        structure: None,
        feature: None,
        size,
        weight: if alive { 30.0 } else { 20.0 } + 15.0 * size as f64,
        traits: Default::default(),
    }
}

/// Summits that stand clear of their surroundings.
// DESIGN-Q: a summit is the highest point within 1.5 km and rises 120 m
// above the land within 2.4 km; "mountain" from 700 m up, else "hill".
fn peaks(w: &World, surface: &Grid<f64>) -> Vec<Landmark> {
    let s = SIZE as i64;
    let mut out = Vec::new();
    for y in 0..s {
        for x in 0..s {
            if !w.terrain.is_land(x as usize, y as usize) {
                continue;
            }
            let h = *surface.get(x as usize, y as usize);
            let mut top = true;
            let mut low = h;
            for dy in -8..=8i64 {
                for dx in -8..=8i64 {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= s || ny >= s || (dx == 0 && dy == 0) {
                        continue;
                    }
                    let nh = *surface.get(nx as usize, ny as usize);
                    low = f64::min(low, nh);
                    if dx.abs() <= 5 && dy.abs() <= 5 && (nh > h || (nh == h && (ny, nx) < (y, x)))
                    {
                        top = false;
                    }
                }
            }
            let prominence = h - low;
            if top && prominence >= 120.0 {
                out.push(Landmark {
                    kind: if h >= 700.0 { "mountain" } else { "hill" },
                    pos: Pos::of_cell(x as usize, y as usize),
                    height: 0.0,
                    settlement: None,
                    structure: None,
                    feature: None,
                    size: (prominence / 100.0).floor() as i64,
                    weight: prominence / 8.0,
                    traits: Default::default(),
                });
            }
        }
    }
    out
}

/// A landmark in view, with how it looks from here.
#[derive(Debug, Clone, PartialEq)]
pub struct InView {
    pub landmark: usize,
    pub bearing: usize,
    pub metres: f64,
    pub score: f64,
}

impl Land {
    /// Every landmark visible from `p`, most salient first. Salience grows
    /// with size and closeness and falls when many alike are in view.
    pub fn in_view(&self, w: &World, p: Pos, range: f64, skip_town: Option<usize>) -> Vec<InView> {
        let mut out: Vec<InView> = self
            .landmarks
            .iter()
            .enumerate()
            .filter(|(_, l)| skip_town.is_none() || l.settlement != skip_town)
            .filter_map(|(i, l)| {
                let metres = p.dist(l.pos);
                if metres < f64::from(LOCAL)
                    || !self.sight(w, p, EYE, l.pos, l.height.max(EYE), range)
                {
                    return None;
                }
                Some(InView {
                    landmark: i,
                    bearing: bearing(p, l.pos)?,
                    metres,
                    score: l.weight / (1.0 + metres / 2000.0),
                })
            })
            .collect();
        let mut alike: BTreeMap<&str, f64> = BTreeMap::new();
        for v in &out {
            *alike.entry(self.landmarks[v.landmark].kind).or_insert(0.0) += 1.0;
        }
        for v in &mut out {
            v.score /= alike[self.landmarks[v.landmark].kind].sqrt();
        }
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.landmark.cmp(&b.landmark))
        });
        out
    }

    /// Edges within a cell of `p`: (edge, bearing to it or `None` if here).
    pub fn edges_near(&self, p: Pos) -> Vec<(usize, Option<usize>)> {
        (0..EDGES.len())
            .filter_map(|e| {
                let (x, y) = self.nearest_edge(p, e, 1)?;
                let c = Pos::of_cell(x, y);
                let b = if c.dist(p) < f64::from(CELL) * 0.75 {
                    None
                } else {
                    bearing(p, c)
                };
                Some((e, b))
            })
            .collect()
    }

    /// The lie of the land at a point: "flat", "slope", "hilltop", "valley".
    pub fn terrain_here(&self, p: Pos) -> &'static str {
        let (x, y) = p.cell();
        let h = *self.surface.get(x, y);
        let nb: Vec<f64> = self
            .surface
            .neighbours(x, y)
            .map(|(a, b)| *self.surface.get(a, b))
            .collect();
        let higher = nb.iter().filter(|&&n| n > h + 5.0).count();
        let lower = nb.iter().filter(|&&n| n < h - 5.0).count();
        let spread = nb.iter().fold(0.0f64, |m, &n| m.max((n - h).abs()));
        if lower == nb.len() {
            "hilltop"
        } else if higher + 1 >= nb.len() {
            "valley"
        } else if spread > 40.0 {
            "slope"
        } else {
            "flat"
        }
    }

    /// Whether the point looks out over the land around (for a region view).
    pub fn high(&self, p: Pos) -> bool {
        let (x, y) = p.cell();
        let h = *self.surface.get(x, y);
        let r = 10i64;
        let (mut sum, mut n) = (0.0, 0.0);
        for dy in -r..=r {
            for dx in -r..=r {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0 || ny < 0 || nx >= SIZE as i64 || ny >= SIZE as i64 {
                    continue;
                }
                sum += *self.surface.get(nx as usize, ny as usize);
                n += 1.0;
            }
        }
        h - sum / n >= 60.0
    }

    /// Biomes in view from a high point, most widespread first, and
    /// whether the sea is among them.
    pub fn region(&self, w: &World, p: Pos, range: f64) -> (Vec<&'static str>, bool) {
        let (x, y) = p.cell();
        let r = ((range / f64::from(CELL)).floor() as i64).min(40);
        let mut count: BTreeMap<&'static str, u32> = BTreeMap::new();
        let mut sea = false;
        for dy in (-r..=r).step_by(2) {
            for dx in (-r..=r).step_by(2) {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0
                    || ny < 0
                    || nx >= SIZE as i64
                    || ny >= SIZE as i64
                    || dx * dx + dy * dy > r * r
                {
                    continue;
                }
                let b = *w.terrain.biome.get(nx as usize, ny as usize);
                if b == Biome::Sea {
                    sea = true;
                }
                *count.entry(biome_id(b)).or_insert(0) += 1;
            }
        }
        let mut v: Vec<(&str, u32)> = count.into_iter().collect();
        v.sort_by_key(|&(b, n)| (Reverse(n), b));
        (v.into_iter().take(3).map(|(b, _)| b).collect(), sea)
    }
}

pub fn biome_id(b: Biome) -> &'static str {
    match b {
        Biome::Sea => "sea",
        Biome::Lake => "lake",
        Biome::Shore => "shore",
        Biome::Marsh => "marsh",
        Biome::Grassland => "grassland",
        Biome::Scrub => "scrub",
        Biome::Desert => "desert",
        Biome::Forest => "forest",
        Biome::Pine => "pine",
        Biome::Tundra => "tundra",
        Biome::Rock => "rock",
        Biome::Snow => "snow",
    }
}

/// The shape of the world, for region views.
pub fn shape(w: &World) -> String {
    label(&w.terrain.shape)
}

/// Rotates a vector by `deg` degrees (clockwise on the map), by repeated
/// one-degree turns so only exact arithmetic is used.
pub fn rotate(v: (f64, f64), deg: i32) -> (f64, f64) {
    // cos and sin of one degree.
    const C: f64 = 0.999_847_695_156_391_3;
    const S: f64 = 0.017_452_406_437_283_51;
    let s = if deg < 0 { -S } else { S };
    let mut v = v;
    for _ in 0..deg.unsigned_abs().min(180) {
        v = (v.0 * C - v.1 * s, v.0 * s + v.1 * C);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearings_and_bands() {
        let o = Pos::new(10_000, 10_000);
        assert_eq!(bearing(o, Pos::new(10_000, 9_000)), Some(0));
        assert_eq!(bearing(o, Pos::new(11_000, 9_000)), Some(1));
        assert_eq!(bearing(o, Pos::new(11_000, 10_100)), Some(2));
        assert_eq!(bearing(o, Pos::new(9_000, 11_000)), Some(5));
        assert_eq!(bearing(o, o), None);
        for b in 0..8 {
            let (x, y) = unit(b);
            let p = o.moved(x * 1000.0, y * 1000.0);
            assert_eq!(bearing(o, p), Some(b));
        }
        assert_eq!(parse_bearing("NE"), None);
        assert_eq!(parse_bearing("ne"), Some(1));
        assert_eq!(parse_bearing("north-west"), Some(7));
        assert_eq!(rough_metres(1240.0), 1000);
        assert_eq!(rough_metres(260.0), 300);
        assert_eq!(rough_minutes(52), 45);
        assert_eq!(distance_band(5000.0), "far");
        let r = rotate((0.0, -1.0), 90);
        assert!((r.0 - 1.0).abs() < 1e-9 && r.1.abs() < 1e-9);
    }
}
