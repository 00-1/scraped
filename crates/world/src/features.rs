//! Natural features and old marks on the land (D03): what makes the land
//! between settlements worth walking across.
//!
//! Every feature has a cause in the terrain, the water, the rock or the
//! history, and is placed where that cause holds: springs where water
//! comes to the surface at the foot of a slope, waterfalls where a river
//! drops, caves in limestone, cairns on summits, burial mounds near the
//! oldest towns. Nothing is scattered at random; the randomness only
//! breaks ties and thins out crowds.
//!
//! Where a feature has an inside (a cave, a sea cave) this records what it
//! should hold; D04 builds it.

use serde::Serialize;

use crate::geology::{Geology, Rock};
use crate::history::{Cell, EventKind, History};
use crate::structures::{Structure, StructureKind};
use crate::terrain::{Biome, Terrain, SIZE};
use crate::water::{Water, RIVER_FLOW};

/// What kind of thing a feature is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Water,
    Rock,
    Life,
    /// Old human marks on the land.
    Marks,
}

/// One kind of feature: its id (for content), group, how tall it stands
/// (metres, for being seen from afar) and how much it draws the eye.
pub struct Kind {
    pub id: &'static str,
    pub group: Group,
    pub height: f64,
    pub weight: f64,
}

/// Every feature kind. Those with a height of 0 are only found close by.
pub const KINDS: &[Kind] = &[
    k("spring", Group::Water, 0.0, 0.0),
    k("warm spring", Group::Water, 0.0, 0.0),
    k("waterfall", Group::Water, 12.0, 14.0),
    k("rapids", Group::Water, 0.0, 0.0),
    k("gorge", Group::Water, 0.0, 0.0),
    k("oxbow lake", Group::Water, 0.0, 0.0),
    k("delta", Group::Water, 0.0, 0.0),
    k("tidal flats", Group::Water, 0.0, 0.0),
    k("sea stack", Group::Water, 25.0, 16.0),
    k("sea cave", Group::Water, 0.0, 0.0),
    k("bog", Group::Water, 0.0, 0.0),
    k("cave mouth", Group::Rock, 0.0, 0.0),
    k("sinkhole", Group::Rock, 0.0, 0.0),
    k("ledged cliff", Group::Rock, 20.0, 10.0),
    k("natural arch", Group::Rock, 15.0, 16.0),
    k("scree", Group::Rock, 0.0, 0.0),
    k("rock pillar", Group::Rock, 18.0, 14.0),
    k("boulder field", Group::Rock, 0.0, 0.0),
    k("salt flat", Group::Rock, 0.0, 0.0),
    k("glacier", Group::Rock, 30.0, 18.0),
    k("snowfield", Group::Rock, 0.0, 0.0),
    k("ancient tree", Group::Life, 25.0, 12.0),
    k("grove", Group::Life, 15.0, 6.0),
    k("dead forest", Group::Life, 0.0, 0.0),
    k("petrified forest", Group::Life, 0.0, 0.0),
    k("reed bed", Group::Life, 0.0, 0.0),
    k("flower meadow", Group::Life, 0.0, 0.0),
    k("standing stones", Group::Marks, 4.0, 9.0),
    k("cairn", Group::Marks, 2.0, 6.0),
    k("terraces", Group::Marks, 0.0, 0.0),
    k("field walls", Group::Marks, 0.0, 0.0),
    k("old road", Group::Marks, 0.0, 0.0),
    k("cutting", Group::Marks, 0.0, 0.0),
    k("quarry", Group::Marks, 0.0, 0.0),
    k("spoil heap", Group::Marks, 6.0, 5.0),
    k("burial mound", Group::Marks, 5.0, 9.0),
];

const fn k(id: &'static str, group: Group, height: f64, weight: f64) -> Kind {
    Kind {
        id,
        group,
        height,
        weight,
    }
}

/// The kind with this id.
pub fn kind(id: &str) -> &'static Kind {
    KINDS
        .iter()
        .find(|k| k.id == id)
        .unwrap_or_else(|| panic!("no feature kind {id}"))
}

/// What a feature with an inside should hold, for D04 to build.
#[derive(Debug, Clone, Serialize)]
pub struct Inside {
    /// "cave" or "sea cave".
    pub kind: &'static str,
    /// Rough size, 1 (a hollow) to 4 (a system of many chambers).
    pub size: u8,
    /// What it should contain: ids such as "stream", "chambers", "bones",
    /// "old offerings", "tide pools".
    pub contents: Vec<&'static str>,
}

/// One feature on the land.
#[derive(Debug, Clone, Serialize)]
pub struct Feature {
    pub id: usize,
    pub kind: &'static str,
    pub cell: Cell,
    /// Why it is here: an id naming the cause ("river drop", "limestone",
    /// "summit", "old town"...), for tests and for agents.
    pub cause: &'static str,
    pub rock: Rock,
    /// For old marks: the era they were made in.
    pub era: Option<u32>,
    /// For old marks: the settlement that made them.
    pub settlement: Option<usize>,
    /// For old marks: the history event they are evidence of.
    pub event: Option<usize>,
    pub inside: Option<Inside>,
}

/// Height at a cell, or `None` off the map.
fn h_at(t: &Terrain, x: i64, y: i64) -> Option<f64> {
    (x >= 0 && y >= 0 && x < SIZE as i64 && y < SIZE as i64)
        .then(|| *t.height.get(x as usize, y as usize))
}

/// The biggest rise to any neighbour.
fn rise(t: &Terrain, x: usize, y: usize) -> f64 {
    let h = *t.height.get(x, y);
    t.height
        .neighbours(x, y)
        .map(|(a, b)| *t.height.get(a, b) - h)
        .fold(0.0, f64::max)
}

/// The biggest drop to any neighbour.
fn drop(t: &Terrain, x: usize, y: usize) -> f64 {
    let h = *t.height.get(x, y);
    t.height
        .neighbours(x, y)
        .map(|(a, b)| h - *t.height.get(a, b))
        .fold(0.0, f64::max)
}

fn near<F: Fn(usize, usize) -> bool>(x: usize, y: usize, r: i64, f: F) -> usize {
    let mut n = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if (dx, dy) != (0, 0)
                && nx >= 0
                && ny >= 0
                && nx < SIZE as i64
                && ny < SIZE as i64
                && f(nx as usize, ny as usize)
            {
                n += 1;
            }
        }
    }
    n
}

/// A candidate: score (higher first), cell, cause, and history links.
struct Cand {
    score: i64,
    x: usize,
    y: usize,
    cause: &'static str,
    era: Option<u32>,
    settlement: Option<usize>,
    event: Option<usize>,
}

fn cand(score: f64, x: usize, y: usize, cause: &'static str) -> Cand {
    Cand {
        score: (score * 1000.0) as i64,
        x,
        y,
        cause,
        era: None,
        settlement: None,
        event: None,
    }
}

/// How many of each kind at most, and how far apart (cells).
fn limits(kind: &str) -> (usize, i64) {
    match kind {
        "spring" => (8, 10),
        "rapids" | "cave mouth" | "sinkhole" | "flower meadow" | "grove" | "cairn" => (5, 14),
        "field walls" | "terraces" => (6, 8),
        "glacier" | "delta" | "petrified forest" | "salt flat" => (2, 20),
        _ => (4, 14),
    }
}

/// Places every feature the land and its history imply.
pub fn place(
    seed: u64,
    t: &Terrain,
    w: &Water,
    g: &Geology,
    h: &History,
    structures: &[Structure],
) -> Vec<Feature> {
    let s = SIZE;
    let mut by_kind: Vec<(&'static str, Vec<Cand>)> = Vec::new();
    let mut add = |kind: &'static str, c: Cand| {
        if let Some(v) = by_kind.iter_mut().find(|(k, _)| *k == kind) {
            v.1.push(c);
        } else {
            by_kind.push((kind, vec![c]));
        }
    };
    // A little seeded noise to break ties, so equal candidates don't all
    // fall on one side of the map.
    let jitter = |x: usize, y: usize, salt: u64| -> f64 {
        let mut v = seed ^ (x as u64) << 20 ^ (y as u64) << 40 ^ salt.wrapping_mul(0x9e37_79b9);
        v ^= v >> 29;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v ^= v >> 32;
        (v % 1000) as f64 / 1000.0
    };
    let river = |x: usize, y: usize| w.is_river(t, x, y);
    let sea = |x: usize, y: usize| *t.biome.get(x, y) == Biome::Sea;
    let lake = |x: usize, y: usize| *t.biome.get(x, y) == Biome::Lake;
    let forest = |x: usize, y: usize| matches!(t.biome.get(x, y), Biome::Forest | Biome::Pine);

    for y in 1..s - 1 {
        for x in 1..s - 1 {
            let b = *t.biome.get(x, y);
            let here = *t.height.get(x, y);
            let rock = g.at(x, y);
            let j = jitter(x, y, 1);
            if b == Biome::Sea {
                // Stacks stand off steep coasts.
                let cliff = t
                    .height
                    .neighbours(x, y)
                    .map(|(a, c)| *t.height.get(a, c))
                    .fold(f64::MIN, f64::max);
                if cliff > 40.0
                    && here > -40.0
                    && near(x, y, 1, |a, c| !t.biome.get(a, c).is_water()) <= 3
                {
                    add("sea stack", cand(cliff / 10.0 + j, x, y, "cliff coast"));
                }
                continue;
            }
            if b == Biome::Lake {
                continue;
            }
            let flow = *w.flow.get(x, y);
            let up = rise(t, x, y);
            let down = drop(t, x, y);
            // ---- water ----
            if river(x, y) {
                let d = w.down.get(x, y);
                if *d != u32::MAX {
                    let (dx, dy) = (*d as usize % s, *d as usize / s);
                    let fall = here - *t.height.get(dx, dy);
                    if fall > 25.0 {
                        add("waterfall", cand(fall + j, x, y, "river drop"));
                    } else if fall > 8.0 {
                        add("rapids", cand(fall + j, x, y, "river drop"));
                    }
                }
                // Walls on both sides.
                let walls = [(1i64, 0i64), (0, 1), (1, 1), (1, -1)]
                    .iter()
                    .any(|&(ax, ay)| {
                        let a = h_at(t, x as i64 + 2 * ax, y as i64 + 2 * ay);
                        let c = h_at(t, x as i64 - 2 * ax, y as i64 - 2 * ay);
                        matches!((a, c), (Some(a), Some(c)) if a - here > 60.0 && c - here > 60.0)
                    });
                if walls {
                    add("gorge", cand(up / 10.0 + j, x, y, "river cut deep"));
                }
                if here < 15.0 && flow >= RIVER_FLOW * 3 && near(x, y, 1, sea) > 0 {
                    add(
                        "delta",
                        cand(f64::from(flow) / 100.0 + j, x, y, "river meets sea"),
                    );
                }
            } else if flow < RIVER_FLOW / 4
                && up > 40.0
                && *t.moisture.get(x, y) > 0.45
                && !*w.pooled.get(x, y)
            {
                // Water comes to the surface at the foot of a slope.
                if rock == Rock::Basalt {
                    add(
                        "warm spring",
                        cand(up / 10.0 + j, x, y, "spring over basalt"),
                    );
                } else {
                    add(
                        "spring",
                        cand(up / 10.0 + *t.moisture.get(x, y) + j, x, y, "slope foot"),
                    );
                }
            }
            if here < 80.0 && near(x, y, 1, lake) > 0 && near(x, y, 2, river) > 0 {
                add("oxbow lake", cand(1.0 + j, x, y, "river loop cut off"));
            }
            if b == Biome::Shore && near(x, y, 1, sea) >= 3 && up < 3.0 && down < 3.0 {
                add(
                    "tidal flats",
                    cand(f64::from(near(x, y, 2, sea) as u32) + j, x, y, "flat shore"),
                );
            }
            if near(x, y, 1, sea) > 0
                && here > 30.0
                && matches!(rock, Rock::Limestone | Rock::Sandstone)
            {
                add("sea cave", cand(here / 10.0 + j, x, y, "soft rock coast"));
            }
            if (b == Biome::Marsh || *w.pooled.get(x, y)) && *t.temperature.get(x, y) < 6.0 {
                add("bog", cand(1.0 + j, x, y, "cold wet hollow"));
            }
            // ---- rock ----
            if rock.soluble() && up > 40.0 && !river(x, y) {
                add("cave mouth", cand(up / 10.0 + j, x, y, "limestone"));
            }
            if rock.soluble() && up < 8.0 && down < 8.0 && !river(x, y) && b != Biome::Marsh {
                add("sinkhole", cand(1.0 + j, x, y, "limestone"));
            }
            let steep = up.max(down);
            if steep > 90.0 && matches!(rock, Rock::Granite | Rock::Slate | Rock::Sandstone) {
                add(
                    "ledged cliff",
                    cand(steep / 10.0 + j, x, y, "hard rock face"),
                );
            }
            if steep > 60.0
                && matches!(rock, Rock::Sandstone | Rock::Limestone)
                && (near(x, y, 1, sea) > 0 || matches!(b, Biome::Desert | Biome::Scrub))
            {
                add(
                    "natural arch",
                    cand(steep / 10.0 + j, x, y, "worn soft rock"),
                );
            }
            if up > 120.0 && matches!(b, Biome::Rock | Biome::Tundra | Biome::Snow | Biome::Pine) {
                add("scree", cand(up / 10.0 + j, x, y, "below a steep face"));
            }
            if rock == Rock::Sandstone
                && matches!(b, Biome::Desert | Biome::Scrub)
                && down > 20.0
                && up < 5.0
            {
                add("rock pillar", cand(down / 10.0 + j, x, y, "worn sandstone"));
            }
            if rock == Rock::Granite
                && matches!(
                    b,
                    Biome::Tundra | Biome::Grassland | Biome::Rock | Biome::Scrub
                )
                && steep < 30.0
            {
                add("boulder field", cand(1.0 + j, x, y, "broken granite"));
            }
            if b == Biome::Desert && up > 0.0 && down < 2.0 {
                add("salt flat", cand(up / 10.0 + j, x, y, "dry hollow"));
            }
            if b == Biome::Snow {
                if here > 1500.0 && up > 30.0 {
                    add("glacier", cand(here / 100.0 + j, x, y, "high cold valley"));
                } else {
                    add("snowfield", cand(1.0 + j, x, y, "lasting snow"));
                }
            }
            // ---- life ----
            if b == Biome::Grassland {
                if near(x, y, 3, forest) == 0 && *t.moisture.get(x, y) > 0.3 {
                    add(
                        "ancient tree",
                        cand(*t.moisture.get(x, y) + j, x, y, "lone tree on open land"),
                    );
                }
                if near(x, y, 2, forest) > 0 && near(x, y, 2, forest) < 6 {
                    add("grove", cand(1.0 + j, x, y, "trees at the forest's edge"));
                }
                if *t.moisture.get(x, y) > 0.36 && steep < 15.0 {
                    add(
                        "flower meadow",
                        cand(*t.moisture.get(x, y) + j, x, y, "damp flat grassland"),
                    );
                }
            }
            if forest(x, y) && *t.moisture.get(x, y) < 0.47 {
                add(
                    "dead forest",
                    cand(1.0 - *t.moisture.get(x, y) + j, x, y, "forest gone dry"),
                );
            }
            if b == Biome::Desert && matches!(rock, Rock::Basalt | Rock::Sandstone) && j > 0.8 {
                add("petrified forest", cand(j, x, y, "buried old forest"));
            }
            if (near(x, y, 1, lake) > 0 && here < 200.0)
                || (b == Biome::Marsh && near(x, y, 2, river) > 0)
            {
                add("reed bed", cand(1.0 + j, x, y, "still shallow water"));
            }
            // ---- marks: summits and quarry stone ----
            let top = t
                .height
                .neighbours(x, y)
                .all(|(a, c)| *t.height.get(a, c) < here)
                && near(x, y, 3, |a, c| *t.height.get(a, c) > here) == 0;
            if top && here > 300.0 && !matches!(b, Biome::Snow) {
                add("cairn", cand(here / 100.0 + j, x, y, "summit"));
            }
        }
    }

    // ---- marks from history ----
    for st in &h.settlements {
        let (sx, sy) = (st.cell.ux(), st.cell.uy());
        let old = st.era == 0;
        let ring = |r0: i64, r1: i64| {
            let mut v = Vec::new();
            for dy in -r1..=r1 {
                for dx in -r1..=r1 {
                    let d2 = dx * dx + dy * dy;
                    if d2 < r0 * r0 || d2 > r1 * r1 {
                        continue;
                    }
                    let (nx, ny) = (sx as i64 + dx, sy as i64 + dy);
                    if nx < 1 || ny < 1 || nx >= s as i64 - 1 || ny >= s as i64 - 1 {
                        continue;
                    }
                    let (nx, ny) = (nx as usize, ny as usize);
                    if t.is_land(nx, ny) && !river(nx, ny) {
                        v.push((nx, ny));
                    }
                }
            }
            v
        };
        let link = |mut c: Cand, event: Option<usize>| {
            c.era = Some(st.era);
            c.settlement = Some(st.id);
            c.event = event;
            c
        };
        let founding = h
            .events
            .iter()
            .find(|e| matches!(e.kind, EventKind::Founding { settlement } if settlement == st.id))
            .map(|e| e.id);
        for (x, y) in ring(3, 6) {
            let here = *t.height.get(x, y);
            let j = jitter(x, y, 2);
            let up = rise(t, x, y);
            let down = drop(t, x, y);
            let b = *t.biome.get(x, y);
            if old && down > up && here > 20.0 {
                // The first people buried their dead on rises near home.
                let death = h.events.iter().find(|e| {
                    e.era == 0
                        && matches!(e.kind, EventKind::Death { person, .. } if h.people[person].settlement == st.id)
                });
                add(
                    "burial mound",
                    link(
                        cand(down / 10.0 + j, x, y, "old town's dead"),
                        death.map(|e| e.id).or(founding),
                    ),
                );
            }
            if old && matches!(b, Biome::Grassland | Biome::Scrub | Biome::Tundra) && up < 10.0 {
                add(
                    "standing stones",
                    link(cand(j, x, y, "old town's rites"), founding),
                );
            }
            if g.at(x, y).quarried() && up > 30.0 && st.size >= 2 {
                add(
                    "quarry",
                    link(cand(up / 10.0 + j, x, y, "building stone"), founding),
                );
            }
        }
        for (x, y) in ring(1, 4) {
            let j = jitter(x, y, 3);
            let up = rise(t, x, y);
            let b = *t.biome.get(x, y);
            let farmed = matches!(
                b,
                Biome::Grassland | Biome::Scrub | Biome::Forest | Biome::Shore
            );
            if farmed && up > 25.0 && up < 90.0 {
                add(
                    "terraces",
                    link(cand(up / 10.0 + j, x, y, "farmed slope"), founding),
                );
            }
            if farmed && up < 10.0 && st.size <= 2 {
                add(
                    "field walls",
                    link(cand(j + 1.0, x, y, "old fields"), founding),
                );
            }
        }
    }
    for r in &h.roads {
        // Roads of a town that is now abandoned, or of a past era, are old
        // roads; where one crosses a rise it was cut through.
        let gone =
            h.settlements[r.from].abandoned.is_some() || h.settlements[r.to].abandoned.is_some();
        let last_era = h.eras.len() as u32 - 1;
        if r.path.len() < 6 {
            continue;
        }
        if gone || r.era + 1 < last_era {
            let c = r.path[r.path.len() / 3];
            let mut cd = cand(
                r.path.len() as f64 / 10.0,
                c.ux(),
                c.uy(),
                "road of a past era",
            );
            cd.era = Some(r.era);
            cd.settlement = Some(r.from);
            add("old road", cd);
        }
        for (i, c) in r.path.iter().enumerate().skip(1) {
            let Some(n) = r.path.get(i + 1) else { break };
            let p = r.path[i - 1];
            let (ax, ay) = (
                i64::from(n.x) - i64::from(p.x),
                i64::from(n.y) - i64::from(p.y),
            );
            // Perpendicular to the road.
            let (px, py) = (-ay.signum(), ax.signum());
            let here = *t.height.get(c.ux(), c.uy());
            let a = h_at(t, i64::from(c.x) + px, i64::from(c.y) + py);
            let b = h_at(t, i64::from(c.x) - px, i64::from(c.y) - py);
            if let (Some(a), Some(b)) = (a, b) {
                if a - here > 30.0 && b - here > 30.0 {
                    let mut cd = cand(
                        (a + b - 2.0 * here) / 10.0,
                        c.ux(),
                        c.uy(),
                        "road through a rise",
                    );
                    cd.era = Some(r.era);
                    add("cutting", cd);
                }
            }
        }
    }
    for st in structures
        .iter()
        .filter(|st| st.kind == StructureKind::Mine)
    {
        let (x, y) = (st.cell.ux(), st.cell.uy());
        let mut cd = cand(1.0, x, y, "mine waste");
        cd.era = Some(st.era);
        cd.settlement = st.settlement;
        add("spoil heap", cd);
    }

    // Keep the best of each kind, spread apart, away from towns' hearts.
    let mut out: Vec<Feature> = Vec::new();
    let order: Vec<&'static str> = KINDS.iter().map(|k| k.id).collect();
    by_kind.sort_by_key(|(k, _)| order.iter().position(|o| o == k));
    for (kind, mut cands) in by_kind {
        let (max, apart) = limits(kind);
        cands.sort_by(|a, b| b.score.cmp(&a.score).then((a.y, a.x).cmp(&(b.y, b.x))));
        let mut kept: Vec<Cell> = Vec::new();
        for c in cands {
            if kept.len() >= max {
                break;
            }
            let cell = Cell::new(c.x, c.y);
            let in_town = h.settlements.iter().any(|st| st.cell.dist2(cell) <= 1);
            if in_town || kept.iter().any(|k| k.dist2(cell) < apart * apart) {
                continue;
            }
            if out.iter().any(|f| f.cell == cell) {
                continue;
            }
            kept.push(cell);
            let rock = g.at(c.x, c.y);
            let inside = inside_of(kind, rock, t, w, c.x, c.y, out.len() as u64 ^ seed);
            out.push(Feature {
                id: out.len(),
                kind,
                cell,
                cause: c.cause,
                rock,
                era: c.era,
                settlement: c.settlement,
                event: c.event,
                inside,
            });
        }
    }
    out
}

/// What a cave or sea cave should hold.
fn inside_of(
    kind: &str,
    rock: Rock,
    t: &Terrain,
    w: &Water,
    x: usize,
    y: usize,
    salt: u64,
) -> Option<Inside> {
    let mut contents = Vec::new();
    let roll = |n: u64| (salt.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> (n * 7)) % 100;
    match kind {
        "cave mouth" => {
            let wet = *t.moisture.get(x, y);
            if wet > 0.5 || *w.flow.get(x, y) > RIVER_FLOW / 8 {
                contents.push("stream");
            }
            contents.push("chambers");
            if roll(1) < 40 {
                contents.push("bones");
            }
            if roll(2) < 30 {
                contents.push("old offerings");
            }
            if rock.soluble() {
                contents.push("dripstone");
            }
            let size = 1 + (roll(3) % 4) as u8;
            Some(Inside {
                kind: "cave",
                size,
                contents,
            })
        }
        "sea cave" => {
            contents.push("tide pools");
            if roll(1) < 50 {
                contents.push("chambers");
            }
            if roll(2) < 25 {
                contents.push("wreckage");
            }
            Some(Inside {
                kind: "sea cave",
                size: 1 + (roll(3) % 2) as u8,
                contents,
            })
        }
        _ => None,
    }
}
