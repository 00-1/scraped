//! Scenes (D03): small arrangements of things that show what happened in
//! a place before any text explains it: a barricade from the inside, a
//! meal left on the table, a plague pit, scorch marks after a siege.
//!
//! Each scene is a structured fact (what, where, which event, which
//! things), rendered through content slots, never prose. Each traces to a
//! history event or to the place's own history (its last days, a river's
//! floods), and is tied to a building's room or a spot outside, so D04's
//! new interiors can keep it where it belongs.

use serde::Serialize;

use crate::features::Feature;
use crate::history::{Cause, Cell, EventKind, History, Role};
use crate::structures::{Family, Structure, StructureKind};
use crate::terrain::Terrain;
use crate::towns::{riverside, Town};
use crate::water::Water;

/// Every kind of scene, and the things that make it. Ids for content.
pub const KINDS: &[(&str, &[&str])] = &[
    ("barricade", &["table", "chest", "beam"]),
    ("siege marks", &["scorch marks", "arrowheads"]),
    ("plague pit", &["lime", "bones"]),
    ("sealed door", &["daubed mark", "nailed boards"]),
    ("empty granary", &["scraped bins", "tally marks"]),
    ("bones with belongings", &["bones", "bundle", "bowl"]),
    ("meal left", &["bowls", "cups", "dust of bread"]),
    ("tools dropped", &["tools", "half-made work"]),
    ("packed bundle", &["bundle", "cloak"]),
    ("hurried burial", &["shallow graves", "markers"]),
    ("flood line", &["flood line", "silt"]),
    ("shrine still kept", &["offerings", "swept floor"]),
    ("funeral offerings", &["offering bowls", "beads"]),
    ("lost traveller", &["bones", "pack", "staff"]),
    (
        "battlefield",
        &["arrowheads", "broken shields", "heaped stones"],
    ),
    ("grave goods", &["pots", "beads", "bronze"]),
    ("abandoned workings", &["picks", "baskets"]),
];

/// The things that make a kind of scene.
pub fn parts(kind: &str) -> &'static [&'static str] {
    KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(&[], |(_, p)| p)
}

/// Where a scene is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "at")]
pub enum SceneAt {
    Room { structure: usize, room: usize },
    Outside { x: u16, y: u16 },
}

/// One scene.
#[derive(Debug, Clone, Serialize)]
pub struct Scene {
    pub id: usize,
    pub kind: &'static str,
    pub at: SceneAt,
    pub settlement: Option<usize>,
    /// The history event it shows, if one.
    pub event: Option<usize>,
    /// Why it is: "war", "plague", "famine", "last days", "flight",
    /// "flood", "devotion", "death", "road", "old town's dead".
    pub cause: &'static str,
}

/// The first room of a building that hasn't fallen in, preferring one of
/// the given purposes.
fn room_in(st: &Structure, purposes: &[&str]) -> usize {
    st.interior
        .rooms
        .iter()
        .enumerate()
        .find(|(_, r)| !r.collapsed && purposes.contains(&r.purpose))
        .map_or(0, |(i, _)| i)
}

/// Places every scene the history and the land imply.
pub fn place(
    seed: u64,
    t: &Terrain,
    w: &Water,
    h: &History,
    towns: &[Town],
    structures: &[Structure],
    features: &[Feature],
) -> Vec<Scene> {
    let mut out: Vec<Scene> = Vec::new();
    let mut add = |kind: &'static str,
                   at: SceneAt,
                   settlement: Option<usize>,
                   event: Option<usize>,
                   cause: &'static str| {
        // One scene to a spot.
        if out.iter().any(|s| s.at == at) {
            return false;
        }
        let id = out.len();
        out.push(Scene {
            id,
            kind,
            at,
            settlement,
            event,
            cause,
        });
        true
    };
    let pick = |n: usize, salt: u64| -> usize {
        let mut v = seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        v ^= v >> 31;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        (v >> 7) as usize % n.max(1)
    };
    let outside = |c: Cell| SceneAt::Outside { x: c.x, y: c.y };
    for town in towns {
        let s = &h.settlements[town.settlement];
        let here: Vec<&Structure> = structures
            .iter()
            .filter(|st| st.settlement == Some(s.id))
            .collect();
        let of = |kinds: &[StructureKind]| -> Vec<&Structure> {
            here.iter()
                .copied()
                .filter(|st| kinds.contains(&st.kind))
                .collect()
        };
        let houses = of(&[StructureKind::House]);
        let house = |n: usize| houses.get(n % houses.len().max(1)).copied();
        let mut count = 0;
        // Scenes from the town's events.
        for e in &h.events {
            let (kind, cause, at) = match e.kind {
                EventKind::War { settlement, .. } if settlement == s.id => {
                    let walls = of(&[
                        StructureKind::Wall,
                        StructureKind::Gatehouse,
                        StructureKind::Tower,
                    ]);
                    let at = walls.first().map(|st| outside(st.cell));
                    ("siege marks", "war", at)
                }
                EventKind::Plague { settlement } if settlement == s.id => {
                    let graves = of(&[StructureKind::Cemetery]);
                    let at = graves
                        .first()
                        .map(|st| outside(Cell::new(st.cell.ux() + 1, st.cell.uy())));
                    ("plague pit", "plague", at)
                }
                EventKind::Famine { settlement } if settlement == s.id => {
                    let stores = of(&[StructureKind::Granary, StructureKind::Storehouse]);
                    let at = stores.first().map(|st| SceneAt::Room {
                        structure: st.id,
                        room: room_in(st, &["grain-floor", "storeroom"]),
                    });
                    ("empty granary", "famine", at)
                }
                EventKind::Abandonment { settlement, cause } if settlement == s.id => {
                    let n = pick(houses.len(), e.id as u64);
                    let kind = match cause {
                        Cause::War => "barricade",
                        Cause::Plague => "sealed door",
                        Cause::Famine => "bones with belongings",
                        _ => "meal left",
                    };
                    let at = house(n).map(|st| SceneAt::Room {
                        structure: st.id,
                        room: room_in(st, &["hall"]),
                    });
                    (
                        kind,
                        if kind == "meal left" {
                            "last days"
                        } else {
                            cause_id(cause)
                        },
                        at,
                    )
                }
                EventKind::Death { person, cause } if h.people[person].settlement == s.id => {
                    if h.people[person].role == Role::Ruler {
                        let tomb = structures
                            .iter()
                            .find(|st| st.kind == StructureKind::Tomb && st.person == Some(person));
                        let at = tomb.map(|st| SceneAt::Room {
                            structure: st.id,
                            room: room_in(st, &["burial", "offerings"]),
                        });
                        ("funeral offerings", "death", at)
                    } else if matches!(cause, Cause::War | Cause::Plague) {
                        let graves = of(&[StructureKind::Cemetery]);
                        let at = graves.first().map(|st| outside(st.cell));
                        ("hurried burial", cause_id(cause), at)
                    } else {
                        continue;
                    }
                }
                EventKind::Migration { from, .. } if from == s.id => {
                    let at = house(1 + e.id).map(|st| SceneAt::Room {
                        structure: st.id,
                        room: room_in(st, &["sleeping", "hall"]),
                    });
                    ("packed bundle", "flight", at)
                }
                _ => continue,
            };
            // A town shows a handful of its events, not all of them.
            if count >= 3 + usize::from(s.size) {
                break;
            }
            if let Some(at) = at {
                if add(kind, at, Some(s.id), Some(e.id), cause) {
                    count += 1;
                }
            }
        }
        // The town's last days, and its place's own history, until it has
        // at least three (more in larger towns).
        let last = h
            .events
            .iter()
            .find(|e| matches!(e.kind, EventKind::Abandonment { settlement, .. } if settlement == s.id))
            .map(|e| e.id)
            .unwrap_or(h.root);
        let want = 3 + usize::from(s.size) / 2;
        let crafts: Vec<&Structure> = here
            .iter()
            .copied()
            .filter(|st| {
                matches!(st.kind.info().family, Family::Craft | Family::Food)
                    || st.kind == StructureKind::Mine
            })
            .collect();
        let holy = of(&[
            StructureKind::Temple,
            StructureKind::WaysideShrine,
            StructureKind::Hermitage,
        ]);
        let mut options: Vec<(&'static str, &'static str, Option<SceneAt>, Option<usize>)> = vec![
            (
                "meal left",
                "last days",
                house(pick(houses.len(), 11)).map(|st| SceneAt::Room {
                    structure: st.id,
                    room: room_in(st, &["hall"]),
                }),
                Some(last),
            ),
            (
                if crafts
                    .first()
                    .is_some_and(|st| st.kind == StructureKind::Mine)
                {
                    "abandoned workings"
                } else {
                    "tools dropped"
                },
                "last days",
                crafts.first().map(|st| SceneAt::Room {
                    structure: st.id,
                    room: room_in(
                        st,
                        &["gallery", "forge", "kiln-room", "loom-room", "bakery"],
                    ),
                }),
                Some(last),
            ),
            (
                "bones with belongings",
                "last days",
                house(pick(houses.len(), 13) + 2).map(|st| SceneAt::Room {
                    structure: st.id,
                    room: room_in(st, &["sleeping", "store", "hall"]),
                }),
                Some(last),
            ),
        ];
        if riverside(t, w, s.cell) || *t.height.get(s.cell.ux(), s.cell.uy()) < 25.0 {
            let low = here.iter().min_by(|a, b| {
                t.height
                    .get(a.cell.ux(), a.cell.uy())
                    .partial_cmp(t.height.get(b.cell.ux(), b.cell.uy()))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.id.cmp(&b.id))
            });
            options.insert(
                1,
                (
                    "flood line",
                    "flood",
                    low.map(|st| SceneAt::Room {
                        structure: st.id,
                        room: 0,
                    }),
                    None,
                ),
            );
        }
        if s.abandoned.is_some() {
            options.push((
                "shrine still kept",
                "devotion",
                holy.first().map(|st| SceneAt::Room {
                    structure: st.id,
                    room: room_in(st, &["sanctum", "shrine", "cell"]),
                }),
                Some(last),
            ));
        }
        // Then any building left: a meal in another house, belongings in
        // a store, tools in another workshop.
        for (n, st) in here.iter().enumerate() {
            let (kind, purposes): (&'static str, &[&str]) = match st.kind.info().family {
                Family::Dwelling => (
                    if n % 2 == 0 {
                        "meal left"
                    } else {
                        "bones with belongings"
                    },
                    &["hall", "sleeping"],
                ),
                Family::Craft | Family::Food => ("tools dropped", &[]),
                Family::Store => ("bones with belongings", &[]),
                _ => continue,
            };
            options.push((
                kind,
                "last days",
                Some(SceneAt::Room {
                    structure: st.id,
                    room: room_in(st, purposes),
                }),
                Some(last),
            ));
        }
        for (kind, cause, at, event) in options {
            if count >= want {
                break;
            }
            if let Some(at) = at {
                if add(kind, at, Some(s.id), event, cause) {
                    count += 1;
                }
            }
        }
    }
    // Out on the land.
    for e in &h.events {
        if let EventKind::War { settlement, .. } = e.kind {
            // Where the attackers met the defenders: on the first road out.
            let s = &h.settlements[settlement];
            let road = h
                .roads
                .iter()
                .find(|r| r.from == settlement || r.to == settlement);
            if let Some(r) = road {
                let i = if r.from == settlement {
                    4.min(r.path.len() - 1)
                } else {
                    r.path.len().saturating_sub(5)
                };
                let c = r.path[i];
                if c.dist2(s.cell) >= 4 {
                    add("battlefield", outside(c), None, Some(e.id), "war");
                }
            }
        }
    }
    for st in structures.iter().filter(|st| st.settlement.is_none()) {
        match st.kind {
            StructureKind::Waystation => {
                let c = Cell::new(st.cell.ux() + 1, st.cell.uy());
                add("lost traveller", outside(c), None, None, "road");
            }
            StructureKind::Hermitage | StructureKind::WaysideShrine => {
                add(
                    "shrine still kept",
                    SceneAt::Room {
                        structure: st.id,
                        room: 0,
                    },
                    None,
                    None,
                    "devotion",
                );
            }
            _ => {}
        }
    }
    for f in features.iter().filter(|f| f.kind == "burial mound") {
        add(
            "grave goods",
            outside(f.cell),
            f.settlement,
            f.event,
            "old town's dead",
        );
    }
    out
}

fn cause_id(c: Cause) -> &'static str {
    match c {
        Cause::War => "war",
        Cause::Plague => "plague",
        Cause::Famine => "famine",
        Cause::Age => "last days",
        Cause::Writing => "last days",
    }
}
