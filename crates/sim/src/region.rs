//! A world on a trajectory: regions with slow variables (life, water,
//! stability, climate) that drift day by day, coupled to their neighbours
//! and driven by the great inscriptions of history and by the player's
//! largest writing.
//!
//! The simulation is coarse on purpose: a few dozen regions, one step a
//! day, values kept in whole thousandths so the state is exact and the same
//! on every platform. Skipping time is just stepping days quickly.

use serde::{Deserialize, Serialize};

use scraped_world::history::{EventKind, Trajectory};
use scraped_world::terrain::{Biome, Grid, SIZE};
use scraped_world::World;

use crate::outdoors::{label, Pos};
use crate::writing::{Claim, Property};

/// The four regional variables, in order.
pub const VARIABLES: [&str; 4] = ["life", "water", "stability", "climate"];
pub const LIFE: usize = 0;
pub const WATER: usize = 1;
pub const STABILITY: usize = 2;
/// Thousandths of a degree away from the land's own climate.
pub const CLIMATE: usize = 3;

/// Days in a season, and the four seasons. Ids for content.
// DESIGN-Q: a 360-day year of four 90-day seasons, starting in spring;
// summer 5° warmer, winter 6° colder.
pub const SEASON_DAYS: u32 = 90;
pub const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

pub fn season(minutes: u32) -> usize {
    ((minutes / 1440 / SEASON_DAYS) % 4) as usize
}

pub fn season_offset(minutes: u32) -> f64 {
    [0.0, 5.0, 0.0, -6.0][season(minutes)]
}

/// One region of the land.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Region {
    pub id: usize,
    pub cells: u32,
    pub centre: Pos,
    /// Where its water goes.
    pub downstream: Option<usize>,
    pub upstream: Vec<usize>,
    pub neighbours: Vec<usize>,
    /// Its natural state, in thousandths: life and water.
    pub natural: [i32; 2],
    /// Its commonest biome.
    pub biome: String,
}

/// A great inscription: a world-scale claim from history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Great {
    /// What it does to its regions, by id: "winter", "drought", "flood",
    /// "drying", "crevasse", "binding", "thaw", "greening".
    pub kind: String,
    /// The text that carries it.
    pub text: usize,
    pub region: usize,
    /// How many regions out it reaches.
    pub reach: usize,
    pub root: bool,
}

/// The regions of a world.
#[derive(Debug, Clone, Serialize)]
pub struct Regions {
    #[serde(skip)]
    pub of_cell: Grid<u16>,
    pub regions: Vec<Region>,
    pub trajectory: Trajectory,
    /// The state play begins in, which the generated land already shows.
    pub initial: RegionState,
}

/// Region state: four variables per region, in thousandths.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RegionState {
    /// The last day stepped.
    pub day: u32,
    pub vars: Vec<[i32; 4]>,
}

/// What pushed a variable on the last step, for debugging trade-offs.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cause {
    pub region: usize,
    pub variable: &'static str,
    pub why: String,
    pub amount: i32,
}

/// A regional push from a claim: which variable, which way, how hard.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Driver {
    pub region: usize,
    pub reach: usize,
    pub variable: usize,
    /// Target shift in thousandths (degrees×1000 for climate).
    pub target: i32,
    pub why: String,
}

/// The regional variable and target a claim pushes.
// DESIGN-Q: warmth pushes climate by 6°; opening lets water run (+0.3),
// sealing holds it back (-0.3); breaking cracks the ground (-0.4). D09:
// wetting and flowing move water (+0.3, +0.2), growth moves life (+0.3);
// other qualities stay local.
pub fn regional_push(property: Property, amount: i32) -> Option<(usize, i32)> {
    let sign = amount.signum();
    Some(match property {
        Property::Heat => (CLIMATE, 6000 * sign),
        Property::Openness => (WATER, 300 * sign),
        Property::Stability => (STABILITY, 400 * sign),
        Property::Wetness => (WATER, 300 * sign),
        Property::Flow => (WATER, 200 * sign),
        Property::Growth => (LIFE, 300 * sign),
        _ => return None,
    })
}

impl Regions {
    /// Regions from watersheds: each land cell belongs to the basin it drains
    /// to, large basins are cut into blocks and small ones join a neighbour.
    // DESIGN-Q: regions are drainage basins cut into 32-cell blocks (about
    // 10 km), with basins under 40 cells merged into a neighbour.
    pub fn new(w: &World) -> Self {
        let s = SIZE;
        let idx = |x: usize, y: usize| y * s + x;
        // Outlet of each land cell.
        let mut outlet = vec![usize::MAX; s * s];
        for y in 0..s {
            for x in 0..s {
                if !w.terrain.is_land(x, y) {
                    continue;
                }
                let (mut cx, mut cy) = (x, y);
                for _ in 0..s * s {
                    let d = *w.water.down.get(cx, cy);
                    if d == u32::MAX {
                        break;
                    }
                    let (nx, ny) = (d as usize % s, d as usize / s);
                    if !w.terrain.is_land(nx, ny) {
                        break;
                    }
                    cx = nx;
                    cy = ny;
                }
                outlet[idx(x, y)] = idx(cx, cy);
            }
        }
        // Basin × block keys, numbered in scan order.
        let mut key_of = vec![u64::MAX; s * s];
        for y in 0..s {
            for x in 0..s {
                let o = outlet[idx(x, y)];
                if o != usize::MAX {
                    key_of[idx(x, y)] = (o as u64) << 16 | ((y / 32) * 8 + x / 32) as u64;
                }
            }
        }
        let mut keys: Vec<u64> = key_of.iter().copied().filter(|&k| k != u64::MAX).collect();
        keys.sort_unstable();
        keys.dedup();
        let mut id_of = vec![u16::MAX; s * s];
        for i in 0..s * s {
            if key_of[i] != u64::MAX {
                id_of[i] = keys.binary_search(&key_of[i]).expect("known key") as u16;
            }
        }
        // Merge small regions into their largest neighbour, repeatedly.
        for _ in 0..4 {
            let mut size = vec![0u32; keys.len()];
            for &r in &id_of {
                if r != u16::MAX {
                    size[r as usize] += 1;
                }
            }
            let mut into: Vec<u16> = (0..keys.len() as u16).collect();
            for r in 0..keys.len() {
                if size[r] == 0 || size[r] >= 40 {
                    continue;
                }
                let mut best: Option<(u32, u16)> = None;
                for y in 0..s {
                    for x in 0..s {
                        if id_of[idx(x, y)] as usize != r {
                            continue;
                        }
                        for (nx, ny) in w.terrain.height.neighbours(x, y) {
                            let n = id_of[idx(nx, ny)];
                            if n != u16::MAX
                                && n as usize != r
                                && best.is_none_or(|(b, bi)| {
                                    (size[n as usize], std::cmp::Reverse(n))
                                        > (b, std::cmp::Reverse(bi))
                                })
                            {
                                best = Some((size[n as usize], n));
                            }
                        }
                    }
                }
                if let Some((_, n)) = best {
                    into[r] = n;
                }
            }
            for r in id_of.iter_mut() {
                if *r != u16::MAX {
                    let mut t = *r;
                    for _ in 0..8 {
                        if into[t as usize] == t {
                            break;
                        }
                        t = into[t as usize];
                    }
                    *r = t;
                }
            }
        }
        // Renumber densely.
        let mut used: Vec<u16> = id_of.iter().copied().filter(|&r| r != u16::MAX).collect();
        used.sort_unstable();
        used.dedup();
        for r in id_of.iter_mut() {
            if *r != u16::MAX {
                *r = used.binary_search(r).expect("used") as u16;
            }
        }
        let n = used.len();
        let mut of_cell = Grid::new(s, u16::MAX);
        let mut cells = vec![0u32; n];
        let (mut sx, mut sy) = (vec![0u64; n], vec![0u64; n]);
        let mut moist = vec![0.0f64; n];
        let mut temp = vec![0.0f64; n];
        let mut biomes: Vec<std::collections::BTreeMap<String, u32>> = vec![Default::default(); n];
        let mut down_votes: Vec<std::collections::BTreeMap<usize, u32>> =
            vec![Default::default(); n];
        let mut nb: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); n];
        for y in 0..s {
            for x in 0..s {
                let r = id_of[idx(x, y)];
                if r == u16::MAX {
                    continue;
                }
                of_cell.set(x, y, r);
                let r = r as usize;
                cells[r] += 1;
                sx[r] += x as u64;
                sy[r] += y as u64;
                moist[r] += *w.terrain.moisture.get(x, y);
                temp[r] += *w.terrain.temperature.get(x, y);
                *biomes[r]
                    .entry(label(w.terrain.biome.get(x, y)))
                    .or_insert(0) += 1;
                let d = *w.water.down.get(x, y);
                if d != u32::MAX {
                    let o = id_of[d as usize];
                    if o != u16::MAX && o as usize != r {
                        *down_votes[r].entry(o as usize).or_insert(0) += *w.water.flow.get(x, y);
                    }
                }
                for (nx, ny) in w.terrain.height.neighbours(x, y) {
                    let o = id_of[idx(nx, ny)];
                    if o != u16::MAX && o as usize != r {
                        nb[r].insert(o as usize);
                    }
                }
            }
        }
        let mut regions: Vec<Region> = (0..n)
            .map(|r| {
                let c = f64::from(cells[r].max(1));
                let m = moist[r] / c;
                let t = temp[r] / c;
                // DESIGN-Q: natural water follows moisture; natural life
                // follows water and warmth (best near 15°).
                let water = (m * 1000.0) as i32;
                let warmth = 1.0 - ((t - 15.0) / 25.0).abs().min(1.0);
                let life = (m * warmth * 1000.0).clamp(50.0, 950.0) as i32;
                let biome = biomes[r]
                    .iter()
                    .max_by_key(|(b, k)| (**k, std::cmp::Reverse((*b).clone())))
                    .map(|(b, _)| b.clone())
                    .unwrap_or_default();
                Region {
                    id: r,
                    cells: cells[r],
                    centre: Pos::of_cell(
                        (sx[r] / u64::from(cells[r].max(1))) as usize,
                        (sy[r] / u64::from(cells[r].max(1))) as usize,
                    ),
                    downstream: down_votes[r]
                        .iter()
                        .max_by_key(|(k, v)| (**v, std::cmp::Reverse(**k)))
                        .map(|(k, _)| *k),
                    upstream: Vec::new(),
                    neighbours: nb[r].iter().copied().collect(),
                    natural: [life, water.clamp(50, 950)],
                    biome,
                }
            })
            .collect();
        // No cycles downstream: water only goes to a region that drains
        // further down (lower id order breaks ties).
        for r in 0..n {
            let mut seen = vec![r];
            let mut cur = regions[r].downstream;
            while let Some(d) = cur {
                if seen.contains(&d) {
                    regions[r].downstream = None;
                    break;
                }
                seen.push(d);
                cur = regions[d].downstream;
            }
        }
        for r in 0..n {
            if let Some(d) = regions[r].downstream {
                regions[d].upstream.push(r);
            }
        }
        let mut out = Regions {
            of_cell,
            regions,
            trajectory: w.history.trajectory,
            initial: RegionState::default(),
        };
        out.initial = out.start();
        out
    }

    pub fn at(&self, p: Pos) -> Option<usize> {
        let (x, y) = p.cell();
        let r = *self.of_cell.get(x, y);
        (r != u16::MAX).then_some(r as usize)
    }

    /// Regions within `reach` steps of `r`.
    pub fn around(&self, r: usize, reach: usize) -> Vec<usize> {
        let mut out = vec![r];
        let mut i = 0;
        let mut depth = vec![0usize];
        while i < out.len() {
            let (cur, d) = (out[i], depth[i]);
            i += 1;
            if d >= reach {
                continue;
            }
            for &n in &self.regions[cur].neighbours {
                if !out.contains(&n) {
                    out.push(n);
                    depth.push(d + 1);
                }
            }
        }
        out
    }

    /// The world as history left it: natural state, worn by its trajectory.
    pub fn start(&self) -> RegionState {
        // DESIGN-Q: a dying world starts with a quarter less life and water
        // than its land would hold; a recovering one a tenth less.
        let wear = match self.trajectory {
            Trajectory::Dying => 750,
            Trajectory::Recovering => 900,
            Trajectory::Stagnant => 850,
            Trajectory::Balanced => 1000,
        };
        RegionState {
            day: 0,
            vars: self
                .regions
                .iter()
                .map(|r| {
                    [
                        r.natural[0] * wear / 1000,
                        r.natural[1] * wear / 1000,
                        // DESIGN-Q (M14 balance): ground starts in the
                        // middle of "high" (0.7), not on the edge of
                        // "very high", so a little drift isn't news.
                        700,
                        0,
                    ]
                })
                .collect(),
        }
    }

    /// The state play begins in, settled: history's great inscriptions have
    /// acted for centuries, so water, ground and climate start where they
    /// have long pushed them. Life stays as worn as the trajectory left it;
    /// its slow drift is the story a run sees.
    // DESIGN-Q (M14 balance): two years of settling.
    pub fn settle(&self, drivers: &[Driver], days: u32) -> RegionState {
        let mut st = self.initial.clone();
        let life: Vec<i32> = st.vars.iter().map(|v| v[LIFE]).collect();
        for _ in 0..days {
            st.day += 1;
            self.step(&mut st, drivers);
            for (v, l) in st.vars.iter_mut().zip(&life) {
                v[LIFE] = *l;
            }
        }
        st.day = 0;
        st
    }

    /// Steps the regions forward to `day`, one day at a time.
    pub fn advance(&self, state: &mut RegionState, day: u32, drivers: &[Driver]) -> Vec<Cause> {
        let mut causes = Vec::new();
        while state.day < day {
            state.day += 1;
            causes = self.step(state, drivers);
        }
        causes
    }

    /// One day.
    // DESIGN-Q: the coupling. Water relaxes towards what rain and upstream
    // give, minus what warmth takes; life towards what water and warmth
    // allow, spreading from neighbours; stability falls with drought and
    // spreads its cracks; climate relaxes back unless pushed. Each moves a
    // few percent a day, so change takes weeks.
    fn step(&self, st: &mut RegionState, drivers: &[Driver]) -> Vec<Cause> {
        let n = self.regions.len();
        let prev = st.vars.clone();
        let mut causes = Vec::new();
        let season = season(st.day * 1440);
        // Targets from writing.
        let mut push = vec![[0i32; 4]; n];
        for d in drivers {
            for r in self.around(d.region, d.reach) {
                push[r][d.variable] += d.target;
                causes.push(Cause {
                    region: r,
                    variable: VARIABLES[d.variable],
                    why: d.why.clone(),
                    amount: d.target,
                });
            }
        }
        // Where the land is heading of itself, as a share of what it could
        // hold. DESIGN-Q: a dying world heads for 60% of its natural life
        // and water, a stagnant one stays at 85%, a recovering one grows
        // past its worn state (towards 115%), a balanced one holds at full.
        let heading = match self.trajectory {
            Trajectory::Dying => 0.6,
            Trajectory::Stagnant => 0.85,
            Trajectory::Recovering => 1.15,
            Trajectory::Balanced => 1.0,
        };
        for r in 0..n {
            let reg = &self.regions[r];
            let [life, water, stab, clim] = prev[r].map(f64::from);
            let natural_life = f64::from(reg.natural[0]) * heading;
            let natural_water = f64::from(reg.natural[1]) * heading;
            // Climate.
            let clim_target = f64::from(push[r][CLIMATE]);
            let clim2 = clim + (clim_target - clim) * 0.05;
            // Water: rain, upstream, warmth, and writing.
            let upstream = if reg.upstream.is_empty() {
                0.0
            } else {
                reg.upstream
                    .iter()
                    .map(|&u| prev[u][WATER] as f64 - f64::from(self.regions[u].natural[1]))
                    .sum::<f64>()
                    / reg.upstream.len() as f64
            };
            let evaporation =
                (clim / 1000.0).max(0.0) * 40.0 + if season == 1 { 20.0 } else { 0.0 };
            let water_target =
                natural_water + 0.4 * upstream - evaporation + f64::from(push[r][WATER]);
            let water2 = water + (water_target - water) * 0.05;
            // Life follows water and warmth, and its neighbours.
            let cold = (-clim / 1000.0).max(0.0) * 25.0;
            let dry = (natural_water - water).max(0.0);
            let life_target = natural_life - dry * 0.5 - cold - (clim / 1000.0).max(0.0) * 10.0;
            let nb_life = if reg.neighbours.is_empty() {
                life
            } else {
                reg.neighbours
                    .iter()
                    .map(|&k| prev[k][LIFE] as f64)
                    .sum::<f64>()
                    / reg.neighbours.len() as f64
            };
            // DESIGN-Q (M14 balance): life moves half a percent of the way
            // a day (a quarter in winter), so it takes seasons, not weeks.
            let growth = if season == 3 { 0.0025 } else { 0.005 };
            // Values are whole thousandths, so a slow step under half a unit
            // would round away and stall; it moves at least one unit.
            let toward = (life_target - life) * growth;
            let toward = if toward.abs() < 0.5 && (life_target - life).abs() >= 1.0 {
                (life_target - life).signum()
            } else {
                toward
            };
            let life2 = life + toward + (nb_life - life) * 0.0025;
            // Stability falls with drought and spreads its cracks.
            let weakest = reg
                .neighbours
                .iter()
                .map(|&k| prev[k][STABILITY])
                .min()
                .unwrap_or(prev[r][STABILITY]) as f64;
            let stab_target =
                800.0 * heading + 0.2 * (life - 500.0) - 0.3 * dry + f64::from(push[r][STABILITY]);
            let stab2 = stab + (stab_target - stab) * 0.02 + (weakest - stab).min(0.0) * 0.005;
            st.vars[r] = [
                (life2.clamp(0.0, 1000.0) + 0.5).floor() as i32,
                (water2.clamp(0.0, 1000.0) + 0.5).floor() as i32,
                (stab2.clamp(0.0, 1000.0) + 0.5).floor() as i32,
                (clim2.clamp(-20000.0, 20000.0) + 0.5).floor() as i32,
            ];
        }
        causes
    }

    /// Mean of a variable over the land, weighted by size.
    pub fn mean(&self, st: &RegionState, v: usize) -> f64 {
        let total: u64 = self.regions.iter().map(|r| u64::from(r.cells)).sum();
        self.regions
            .iter()
            .zip(&st.vars)
            .map(|(r, x)| f64::from(x[v]) * f64::from(r.cells))
            .sum::<f64>()
            / total.max(1) as f64
    }
}

/// A coarse band of a variable, for descriptions and revisit memory.
pub fn band(v: usize, value: i32) -> &'static str {
    match v {
        CLIMATE => match value {
            x if x <= -3000 => "colder",
            x if x >= 3000 => "warmer",
            _ => "usual",
        },
        _ => match value {
            x if x < 200 => "very_low",
            x if x < 400 => "low",
            x if x < 600 => "middling",
            x if x < 800 => "high",
            _ => "very_high",
        },
    }
}

/// Which history writing events become great inscriptions: the root, and
/// the farthest-reaching others, so each trajectory has its drivers. Only
/// writing that can be reached (through rubble, but not through fallen
/// rooms) qualifies.
// DESIGN-Q: the root plus the two widest-reaching other writing events.
pub fn great_events(w: &World) -> Vec<usize> {
    let reachable = |e: usize| {
        w.texts
            .iter()
            .find(|t| t.event == Some(e))
            .is_some_and(|t| match t.room {
                None => true,
                Some(room) => through_rubble(&w.structures[t.structure], room),
            })
    };
    let mut others: Vec<(u16, usize)> = w
        .history
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Writing {
                effect,
                root: false,
                ..
            } => Some((effect.radius, e.id)),
            _ => None,
        })
        .filter(|&(_, id)| reachable(id))
        .collect();
    others.sort_by_key(|&(r, id)| (std::cmp::Reverse(r), id));
    let mut out = Vec::new();
    if reachable(w.history.root) {
        out.push(w.history.root);
    }
    out.extend(others.into_iter().take(2).map(|(_, id)| id));
    out
}

/// Whether a room can be reached from a building's entrance through any
/// passage (rubble can be dug) but not through fallen rooms.
fn through_rubble(st: &scraped_world::structures::Structure, target: usize) -> bool {
    let rooms = &st.interior.rooms;
    if rooms.is_empty() || rooms[0].collapsed || rooms[target].collapsed {
        return false;
    }
    let mut seen = vec![false; rooms.len()];
    seen[0] = true;
    let mut queue = vec![0usize];
    while let Some(r) = queue.pop() {
        if r == target {
            return true;
        }
        for l in &st.interior.links {
            let other = if l.a == r {
                l.b
            } else if l.b == r {
                l.a
            } else {
                continue;
            };
            if !seen[other] && !rooms[other].collapsed {
                seen[other] = true;
                queue.push(other);
            }
        }
    }
    false
}

/// The name of a great inscription's effect, from its claim.
pub fn great_kind(c: &Claim) -> &'static str {
    match (c.property, c.amount > 0) {
        (Property::Heat, false) => "winter",
        (Property::Heat, true) => "drought",
        (Property::Openness, true) => "flood",
        (Property::Openness, false) => "binding",
        (Property::Stability, false) => "crevasse",
        (Property::Stability, true) => "holding",
        (Property::Wetness | Property::Flow, true) => "flood",
        (Property::Wetness | Property::Flow, false) => "drought",
        (Property::Growth, true) => "greening",
        (Property::Growth, false) => "blight",
        _ => "holding",
    }
}

/// Spoiler view: every region's variables now and after `days` more with
/// the given drivers, and what pushed each one.
pub fn debug(r: &Regions, st: &RegionState, drivers: &[Driver], days: u32) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "TRAJECTORY {:?}  day {}  ({} regions)",
        r.trajectory,
        st.day,
        r.regions.len()
    ); // DEBUG-TEXT
    let _ = writeln!(out, "DRIVERS"); // DEBUG-TEXT
    for d in drivers {
        let _ = writeln!(
            out,
            "  region {:>3} (+{} around): {} {:+}  {}",
            d.region, d.reach, VARIABLES[d.variable], d.target, d.why
        ); // DEBUG-TEXT
    }
    let mut later = st.clone();
    let causes = r.advance(&mut later, st.day + days, drivers);
    let _ = writeln!(out, "\nREGION  biome        life      water     stability climate   (now → after {days} days)  causes"); // DEBUG-TEXT
    for reg in &r.regions {
        let a = st.vars[reg.id];
        let b = later.vars[reg.id];
        let why: Vec<String> = causes
            .iter()
            .filter(|c| c.region == reg.id)
            .map(|c| format!("{} {:+} ({})", c.variable, c.amount, c.why))
            .collect(); // DEBUG-TEXT
        let _ = writeln!(
            out,
            "{:>6}  {:<10} {:>4}→{:<4} {:>4}→{:<4} {:>4}→{:<4} {:>+6.1}→{:<+6.1} {}{}", // DEBUG-TEXT
            reg.id,
            reg.biome,
            a[0],
            b[0],
            a[1],
            b[1],
            a[2],
            b[2],
            f64::from(a[3]) / 1000.0,
            f64::from(b[3]) / 1000.0,
            reg.downstream.map_or(String::new(), |d| format!("→{d} ")),
            why.join("; ")
        );
    }
    out
}

/// Whether a biome is water (for maps).
pub fn is_water(b: Biome) -> bool {
    b.is_water()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(seed: u64) -> World {
        World::generate(seed)
    }

    #[test]
    fn regions_cover_the_land_and_flow_downhill_without_cycles() {
        let w = world(42);
        let r = Regions::new(&w);
        assert!(r.regions.len() >= 5, "{}", r.regions.len());
        for y in 0..SIZE {
            for x in 0..SIZE {
                assert_eq!(w.terrain.is_land(x, y), *r.of_cell.get(x, y) != u16::MAX);
            }
        }
        for reg in &r.regions {
            let mut cur = reg.downstream;
            let mut steps = 0;
            while let Some(d) = cur {
                steps += 1;
                assert!(steps <= r.regions.len());
                cur = r.regions[d].downstream;
            }
        }
    }

    #[test]
    fn ten_years_alone_is_stable() {
        for seed in [1u64, 7, 42] {
            let w = world(seed);
            let r = Regions::new(&w);
            let mut st = r.start();
            let mut last = st.clone();
            for year in 1..=10u32 {
                r.advance(&mut st, year * 360, &[]);
                for (a, b) in st.vars.iter().zip(&last.vars) {
                    for v in 0..3 {
                        assert!((0..=1000).contains(&a[v]));
                        if year > 6 {
                            // Settled: little change from year to year.
                            assert!(
                                (a[v] - b[v]).abs() <= 60,
                                "seed {seed} year {year}: {a:?} vs {b:?}"
                            );
                        }
                    }
                }
                last = st.clone();
            }
        }
    }

    #[test]
    fn skipping_days_equals_stepping_them() {
        let w = world(7);
        let r = Regions::new(&w);
        let d = vec![Driver {
            region: 0,
            reach: 2,
            variable: CLIMATE,
            target: -6000,
            why: "test".into(),
        }];
        let mut a = r.start();
        r.advance(&mut a, 200, &d);
        let mut b = r.start();
        for day in 1..=200 {
            r.advance(&mut b, day, &d);
        }
        assert_eq!(a, b);
    }

    #[test]
    fn trajectories_go_their_way() {
        for seed in 1u64..40 {
            let w = world(seed);
            let r = Regions::new(&w);
            let mut st = r.start();
            let before = r.mean(&st, LIFE);
            r.advance(&mut st, 360, &[]);
            let after = r.mean(&st, LIFE);
            match r.trajectory {
                Trajectory::Dying => {
                    assert!(after < before * 0.95, "seed {seed}: {before} → {after}")
                }
                Trajectory::Recovering => {
                    assert!(after > before * 1.05, "seed {seed}: {before} → {after}")
                }
                // Stagnant and balanced worlds hold roughly where they are,
                // their great inscriptions aside.
                _ => {}
            }
        }
    }

    #[test]
    fn writing_drives_regions_and_costs_downstream() {
        let w = world(42);
        let r = Regions::new(&w);
        // Holding water back in a region with a downstream neighbour dries it.
        let Some(up) = r.regions.iter().find(|x| x.downstream.is_some()) else {
            return;
        };
        let down = up.downstream.unwrap();
        let mut free = r.start();
        r.advance(&mut free, 120, &[]);
        let mut held = r.start();
        let d = vec![Driver {
            region: up.id,
            reach: 0,
            variable: WATER,
            target: 300,
            why: "test".into(),
        }];
        r.advance(&mut held, 120, &d);
        assert!(held.vars[up.id][WATER] > free.vars[up.id][WATER]);
        assert!(
            held.vars[down][WATER] >= free.vars[down][WATER],
            "more water upstream flows down"
        );
    }
}
