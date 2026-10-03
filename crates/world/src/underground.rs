//! Where things run underground (D03), recorded for D04 to build:
//! cellars under buildings with rooms below ground, drains from a town's
//! square down to water, tunnels out under a walled town's walls,
//! catacombs joining a town's burial places, and mine workings running
//! into the rock. Nothing here is walkable yet.

use serde::Serialize;

use crate::features::Feature;
use crate::history::{Cell, History};
use crate::structures::{Family, Structure, StructureKind};
use crate::terrain::{Biome, Terrain, SIZE};
use crate::towns::Town;
use crate::water::Water;

/// Kinds of underground route. Ids for content.
pub const KINDS: [&str; 6] = [
    "cellar",
    "drain",
    "tunnel",
    "catacomb",
    "mine working",
    "cave system",
];

/// One underground route.
#[derive(Debug, Clone, Serialize)]
pub struct Route {
    pub kind: &'static str,
    pub settlement: Option<usize>,
    /// Cells it runs under, in order.
    pub path: Vec<Cell>,
    /// Buildings it opens into.
    pub structures: Vec<usize>,
    /// The feature it opens from (a cave mouth).
    pub feature: Option<usize>,
    /// How deep it lies, in levels below ground.
    pub depth: i8,
}

/// Steps from `from` toward `to`, one cell at a time.
fn line(from: Cell, to: Cell) -> Vec<Cell> {
    let mut out = vec![from];
    let (mut x, mut y) = (i64::from(from.x), i64::from(from.y));
    while (x, y) != (i64::from(to.x), i64::from(to.y)) && out.len() < 40 {
        x += (i64::from(to.x) - x).signum();
        y += (i64::from(to.y) - y).signum();
        out.push(Cell::new(x as usize, y as usize));
    }
    out
}

/// Plans every underground route the buildings and the land imply.
pub fn plan(
    t: &Terrain,
    w: &Water,
    h: &History,
    towns: &[Town],
    structures: &[Structure],
    features: &[Feature],
) -> Vec<Route> {
    let mut out = Vec::new();
    for st in structures {
        let deepest = st.interior.rooms.iter().map(|r| r.level).min().unwrap_or(0);
        if deepest < 0
            && matches!(
                st.kind.info().family,
                Family::Dwelling | Family::Food | Family::Store | Family::Rule
            )
        {
            out.push(Route {
                kind: "cellar",
                settlement: st.settlement,
                path: vec![st.cell],
                structures: vec![st.id],
                feature: None,
                depth: -deepest,
            });
        }
        if st.kind == StructureKind::Mine {
            // Workings follow the rock: toward the highest ground nearby.
            let mut best = st.cell;
            for dy in -4i64..=4 {
                for dx in -4i64..=4 {
                    let (x, y) = (i64::from(st.cell.x) + dx, i64::from(st.cell.y) + dy);
                    if x < 0 || y < 0 || x >= SIZE as i64 || y >= SIZE as i64 {
                        continue;
                    }
                    let c = Cell::new(x as usize, y as usize);
                    if *t.height.get(c.ux(), c.uy()) > *t.height.get(best.ux(), best.uy()) {
                        best = c;
                    }
                }
            }
            out.push(Route {
                kind: "mine working",
                settlement: st.settlement,
                path: line(st.cell, best),
                structures: vec![st.id],
                feature: None,
                depth: 2,
            });
        }
    }
    for town in towns {
        let s = &h.settlements[town.settlement];
        let mine: Vec<&Structure> = structures
            .iter()
            .filter(|st| st.settlement == Some(s.id))
            .collect();
        let square = town.districts[0].cell;
        if s.size >= 2 {
            // Drains follow the water down from the square, to a river,
            // the sea or a lake within a few cells.
            let mut path = vec![square];
            let mut c = square;
            for _ in 0..6 {
                let d = *w.down.get(c.ux(), c.uy());
                if d == u32::MAX {
                    break;
                }
                c = Cell::new(d as usize % SIZE, d as usize / SIZE);
                path.push(c);
                if w.is_river(t, c.ux(), c.uy()) || t.biome.get(c.ux(), c.uy()).is_water() {
                    break;
                }
            }
            out.push(Route {
                kind: "drain",
                settlement: Some(s.id),
                path,
                structures: Vec::new(),
                feature: None,
                depth: 1,
            });
        }
        if town.walled {
            // A way out under the walls, from the strongest building to
            // the far side from the town's first road.
            let from = mine
                .iter()
                .find(|st| {
                    matches!(
                        st.kind,
                        StructureKind::Palace | StructureKind::Gatehouse | StructureKind::Tower
                    )
                })
                .or_else(|| mine.iter().find(|st| st.kind == StructureKind::Temple));
            if let Some(from) = from {
                let away = h
                    .roads
                    .iter()
                    .find(|r| r.from == s.id || r.to == s.id)
                    .and_then(|r| r.path.get(r.path.len().min(3)).copied())
                    .unwrap_or(square);
                let ox = (2 * i64::from(square.x) - i64::from(away.x)).clamp(1, SIZE as i64 - 2);
                let oy = (2 * i64::from(square.y) - i64::from(away.y)).clamp(1, SIZE as i64 - 2);
                let mut to = Cell::new(ox as usize, oy as usize);
                if to == square {
                    to = Cell::new((square.ux() + 3).min(SIZE - 2), square.uy());
                }
                out.push(Route {
                    kind: "tunnel",
                    settlement: Some(s.id),
                    path: line(from.cell, to),
                    structures: vec![from.id],
                    feature: None,
                    depth: 1,
                });
            }
        }
        // Catacombs join the burial places that lie underground.
        let burial: Vec<&&Structure> = mine
            .iter()
            .filter(|st| {
                matches!(
                    st.kind,
                    StructureKind::Catacombs | StructureKind::Ossuary | StructureKind::Mausoleum
                ) || (st.kind == StructureKind::Temple
                    && st.interior.rooms.iter().any(|r| r.purpose == "crypt"))
            })
            .collect();
        if burial.len() >= 2 && burial.iter().any(|st| st.kind == StructureKind::Catacombs) {
            let mut path = Vec::new();
            for pair in burial.windows(2) {
                path.extend(line(pair[0].cell, pair[1].cell));
            }
            path.dedup();
            out.push(Route {
                kind: "catacomb",
                settlement: Some(s.id),
                path,
                structures: burial.iter().map(|st| st.id).collect(),
                feature: None,
                depth: 2,
            });
        }
    }
    for f in features.iter().filter(|f| f.inside.is_some()) {
        let inside = f.inside.as_ref().expect("inside");
        // A cave runs back into the hill, as far as its size.
        let mut c = f.cell;
        let mut path = vec![c];
        for _ in 0..inside.size {
            let next = t
                .height
                .neighbours(c.ux(), c.uy())
                .filter(|&(x, y)| *t.biome.get(x, y) != Biome::Sea)
                .max_by(|a, b| {
                    t.height
                        .get(a.0, a.1)
                        .partial_cmp(t.height.get(b.0, b.1))
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then(b.cmp(a))
                });
            let Some((x, y)) = next else { break };
            c = Cell::new(x, y);
            if path.contains(&c) {
                break;
            }
            path.push(c);
        }
        out.push(Route {
            kind: "cave system",
            settlement: None,
            path,
            structures: Vec::new(),
            feature: Some(f.id),
            depth: inside.size as i8,
        });
    }
    out
}
