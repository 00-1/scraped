//! What sets a landmark apart (D01): traits read from the world itself, so
//! that two hills, two towns or two towers never look the same from afar,
//! and the same landmark reads the same from every side, which is what
//! makes a paper map possible.
//!
//! Every trait has a cause in the terrain or the history: a summit's shape
//! is the shape of the ground around its top, a town's walls are the walls
//! history built there. Each landmark also gets a `mark`: the trait that
//! tells it apart from others of its kind nearby, for short names.

use serde::Serialize;

use scraped_world::structures::{Condition, StructureKind};
use scraped_world::terrain::{Biome, Grid, SIZE};
use scraped_world::World;

use crate::outdoors::{Landmark, Pos};

/// Shapes of a summit, from the ground around its top.
pub const SHAPES: [&str; 5] = ["peaked", "rounded", "flat", "twin", "long"];
/// Heights of a summit above the sea.
pub const HEIGHTS: [&str; 3] = ["low", "high", "towering"];
/// What covers a summit, from its biome.
pub const COVERS: [&str; 5] = ["snowy", "bare", "wooded", "grassy", "sandy"];
/// Where a place stands.
pub const SETTINGS: [&str; 6] = [
    "riverside",
    "lakeside",
    "coastal",
    "hilltop",
    "valley",
    "plain",
];
/// The state of a building or a settlement's buildings.
pub const CONDITIONS: [&str; 5] = ["intact", "worn", "damaged", "ruined", "buried"];
/// Every word a `mark` can be: the union of the trait words above, plus
/// "walled" and the kinds of the tallest building or what stands on a top.
pub const MARKS: [&str; 35] = [
    "peaked",
    "rounded",
    "flat",
    "twin",
    "long",
    "low",
    "high",
    "towering",
    "snowy",
    "bare",
    "wooded",
    "grassy",
    "sandy",
    "riverside",
    "lakeside",
    "coastal",
    "hilltop",
    "valley",
    "plain",
    "intact",
    "worn",
    "damaged",
    "ruined",
    "buried",
    "walled",
    "small",
    "large",
    "towered",
    "templed",
    "tombed",
    "lone",
    "upland",
    "lowland",
    "crossroads",
    "bridged",
];

/// A landmark's traits. Text values are ids from the lists above; "" when
/// a trait doesn't apply to this kind of landmark.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Traits {
    /// Summits: the shape of the top.
    pub shape: &'static str,
    /// Summits: height band above the sea.
    pub height: &'static str,
    /// Summits: what covers it.
    pub cover: &'static str,
    /// Summits: a building on or by the top (a structure kind), or "".
    pub top: &'static str,
    /// Settlements: whether walls stand there.
    pub walls: bool,
    /// Settlements: the kind of the tallest building, or "".
    pub tallest: &'static str,
    /// Settlements: how many roads lead in.
    pub roads: usize,
    /// Settlements: whether a bridge stands there.
    pub bridged: bool,
    /// Where it stands.
    pub setting: &'static str,
    /// Buildings and settlements: their state.
    pub condition: &'static str,
    /// The trait that sets it apart from others of its kind nearby, and a
    /// second when one isn't enough ("" when not needed).
    pub mark: &'static str,
    pub mark2: &'static str,
}

fn height_of(k: StructureKind) -> f64 {
    k.info().height
}

fn kind_word(k: StructureKind) -> &'static str {
    k.id()
}

fn condition_word(c: Condition) -> &'static str {
    match c {
        Condition::Intact => "intact",
        Condition::Worn => "worn",
        Condition::Damaged => "damaged",
        Condition::Ruined => "ruined",
        Condition::Buried => "buried",
    }
}

fn cell_of(p: Pos) -> (usize, usize) {
    p.cell()
}

/// Where a cell stands: by water, on a height, in a hollow, or on the flat.
fn setting(w: &World, surface: &Grid<f64>, (x, y): (usize, usize)) -> &'static str {
    let s = SIZE as i64;
    let mut sea = false;
    let mut lake = false;
    let mut river = false;
    let h = *surface.get(x, y);
    let mut sum = 0.0;
    let mut n = 0.0;
    for dy in -3..=3i64 {
        for dx in -3..=3i64 {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < 0 || ny < 0 || nx >= s || ny >= s {
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            let near = dx.abs() <= 1 && dy.abs() <= 1;
            match w.terrain.biome.get(ux, uy) {
                Biome::Sea if near => sea = true,
                Biome::Lake if near => lake = true,
                _ => {}
            }
            if near && w.water.is_river(&w.terrain, ux, uy) {
                river = true;
            }
            sum += *surface.get(ux, uy);
            n += 1.0;
        }
    }
    let rise = h - sum / n;
    if sea {
        "coastal"
    } else if lake {
        "lakeside"
    } else if river {
        "riverside"
    } else if rise > 25.0 {
        "hilltop"
    } else if rise < -25.0 {
        "valley"
    } else {
        "plain"
    }
}

fn cover(b: Biome) -> &'static str {
    match b {
        Biome::Snow => "snowy",
        Biome::Rock | Biome::Tundra => "bare",
        Biome::Forest | Biome::Pine => "wooded",
        Biome::Desert => "sandy",
        _ => "grassy",
    }
}

/// The shape of the ground around a summit at `(x, y)`.
fn shape(surface: &Grid<f64>, (x, y): (usize, usize), twin: bool) -> &'static str {
    if twin {
        return "twin";
    }
    let s = SIZE as i64;
    let h = *surface.get(x, y);
    let at = |dx: i64, dy: i64| {
        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
        if nx < 0 || ny < 0 || nx >= s || ny >= s {
            h - 1000.0
        } else {
            *surface.get(nx as usize, ny as usize)
        }
    };
    // Near-top cells along each axis, and the drop one and three cells out.
    let mut near_top = 0;
    let (mut along_x, mut along_y) = (0, 0);
    for d in -3..=3i64 {
        for e in -3..=3i64 {
            if at(d, e) >= h - 30.0 {
                near_top += 1;
                if e == 0 {
                    along_x += 1;
                }
                if d == 0 {
                    along_y += 1;
                }
            }
        }
    }
    let drop = (h - (at(-2, 0) + at(2, 0) + at(0, -2) + at(0, 2)) / 4.0) / 2.0;
    if near_top >= 14 {
        "flat"
    } else if along_x >= along_y + 3 || along_y >= along_x + 3 {
        "long"
    } else if drop >= 45.0 {
        "peaked"
    } else {
        "rounded"
    }
}

/// Fills in every landmark's traits, then picks marks.
pub fn assign(w: &World, surface: &Grid<f64>, landmarks: &mut [Landmark], structure_pos: &[Pos]) {
    let n = landmarks.len();
    for i in 0..n {
        let l = &landmarks[i];
        let cell = cell_of(l.pos);
        let ground = *surface.get(cell.0, cell.1);
        let mut t = Traits {
            setting: setting(w, surface, cell),
            cover: cover(*w.terrain.biome.get(cell.0, cell.1)),
            height: if ground < 400.0 {
                "low"
            } else if ground < 1200.0 {
                "high"
            } else {
                "towering"
            },
            ..Traits::default()
        };
        match (l.settlement, l.structure) {
            (None, None) if l.feature.is_some() => {
                // Features keep their setting, cover and height.
            }
            (Some(s), _) => {
                let mut tallest: Option<(f64, StructureKind)> = None;
                let mut worst = 0usize;
                let mut count = 0usize;
                let mut total = 0usize;
                for st in w.structures.iter().filter(|st| st.settlement == Some(s)) {
                    if st.kind == StructureKind::Wall {
                        t.walls = true;
                    }
                    let h = height_of(st.kind);
                    if st.condition != Condition::Buried
                        && tallest.is_none_or(|(th, tk)| h > th || (h == th && st.kind < tk))
                    {
                        tallest = Some((h, st.kind));
                    }
                    let c = st.condition as usize;
                    worst = worst.max(c);
                    total += c;
                    count += 1;
                }
                t.tallest = tallest.map_or("", |(_, k)| kind_word(k));
                t.roads = w
                    .history
                    .roads
                    .iter()
                    .filter(|r| r.from == s || r.to == s)
                    .count();
                t.bridged = w
                    .structures
                    .iter()
                    .any(|st| st.settlement == Some(s) && st.kind == StructureKind::Bridge);
                t.condition = (total + count / 2)
                    .checked_div(count)
                    .map_or("intact", |c| CONDITIONS[c]);
            }
            (None, Some(st)) => {
                t.condition = condition_word(w.structures[st].condition);
            }
            (None, None) => {
                let h = ground;
                let twin = landmarks.iter().enumerate().any(|(j, o)| {
                    j != i
                        && o.settlement.is_none()
                        && o.structure.is_none()
                        && o.feature.is_none()
                        && o.pos.dist(l.pos) < 2500.0
                        && (*surface.get(o.pos.cell().0, o.pos.cell().1) - h).abs() < 80.0
                });
                t.shape = shape(surface, cell, twin);
                // Something built on or by the top.
                t.top = w
                    .structures
                    .iter()
                    .filter(|st| st.settlement.is_none() && st.kind != StructureKind::Bridge)
                    .filter(|st| structure_pos[st.id].dist(l.pos) < 450.0)
                    .max_by(|a, b| {
                        height_of(a.kind)
                            .partial_cmp(&height_of(b.kind))
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(b.id.cmp(&a.id))
                    })
                    .map_or("", |st| kind_word(st.kind));
            }
        }
        landmarks[i].traits = t;
    }
    pick_marks(landmarks);
}

/// The words that could set a landmark apart, best first.
fn candidates(l: &Landmark) -> Vec<&'static str> {
    let t = &l.traits;
    let mut out = Vec::new();
    match (l.settlement, l.structure) {
        (None, None) if l.feature.is_some() => {
            out.push(t.setting);
            out.push(t.cover);
            out.push(match t.height {
                "low" => "lowland",
                _ => "upland",
            });
            out.push("lone");
        }
        (Some(_), _) => {
            if t.walls {
                out.push("walled");
            }
            out.push(match l.size {
                1 => "small",
                4 => "large",
                _ => "",
            });
            out.push(t.setting);
            out.push(match t.tallest {
                "tower" => "towered",
                "temple" => "templed",
                _ => "",
            });
            if l.kind == "ruins" {
                out.push(t.condition);
            }
            if t.roads >= 3 {
                out.push("crossroads");
            }
            if t.bridged {
                out.push("bridged");
            }
            out.push(t.cover);
            out.push(match t.height {
                "low" => "lowland",
                _ => "upland",
            });
        }
        (None, Some(_)) => {
            out.push(t.setting);
            out.push(t.condition);
            out.push(t.cover);
            out.push(match t.height {
                "low" => "lowland",
                _ => "upland",
            });
            out.push("lone");
        }
        (None, None) => {
            out.push(t.shape);
            out.push(match t.top {
                "tower" => "towered",
                "temple" => "templed",
                "tomb" => "tombed",
                _ => "",
            });
            out.push(t.cover);
            out.push(t.height);
            out.push(t.setting);
        }
    }
    out.retain(|w| !w.is_empty());
    out
}

/// How far apart two alike landmarks must be before sharing a name stops
/// mattering: twice the furthest anything is seen (20 km on a clear day),
/// so no one spot ever sees both.
const APART: f64 = 2.0 * 20_000.0 + 1.0;

/// Gives each landmark, most striking first, the first of its trait words
/// that no alike landmark nearby already goes by; failing that, the first
/// pair of them no alike landmark nearby goes by.
fn pick_marks(landmarks: &mut [Landmark]) {
    let n = landmarks.len();
    let cands: Vec<Vec<&'static str>> = landmarks.iter().map(candidates).collect();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        landmarks[b]
            .weight
            .partial_cmp(&landmarks[a].weight)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut chosen: Vec<Option<(&'static str, &'static str)>> = vec![None; n];
    for &i in &order {
        let taken: Vec<(&'static str, &'static str)> = (0..n)
            .filter(|&j| {
                landmarks[j].kind == landmarks[i].kind
                    && landmarks[j].pos.dist(landmarks[i].pos) < APART
            })
            .filter_map(|j| chosen[j])
            .collect();
        let mine = &cands[i];
        let free = |m: (&'static str, &'static str)| !taken.contains(&m);
        let single = mine.iter().map(|&w| (w, "")).find(|&m| free(m));
        let pair = || {
            mine.iter()
                .enumerate()
                .find_map(|(a, &w1)| mine[a + 1..].iter().map(|&w2| (w1, w2)).find(|&m| free(m)))
        };
        let m = single
            .or_else(pair)
            .unwrap_or((mine.first().copied().unwrap_or(""), ""));
        chosen[i] = Some(m);
        landmarks[i].traits.mark = m.0;
        landmarks[i].traits.mark2 = m.1;
    }
}
