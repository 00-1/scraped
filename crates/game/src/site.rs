//! The playable site: the starting settlement's buildings, rooms and things,
//! taken from the generated world, and the variables descriptions need.

use scraped_content::{Context, Value};
use scraped_world::history::Settlement;
use scraped_world::structures::{Condition, Exit, Material, PassageState, Structure};
use scraped_world::World;

/// Where the player is.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case", tag = "at")]
pub enum Place {
    /// The open ground of the site, between its buildings.
    Outside,
    Room {
        structure: usize,
        room: usize,
    },
}

/// Something the player can examine, read or carry.
#[derive(Debug, Clone)]
pub struct Thing {
    pub id: usize,
    pub kind: &'static str,
    pub material: Material,
    /// Where it started.
    pub home: Place,
    pub portable: bool,
    /// Indices into `World::texts`.
    pub texts: Vec<usize>,
}

/// Feature kinds that can be picked up.
const PORTABLE: &[&str] = &["jar", "tablet", "scroll"];

/// A world narrowed to the site being played.
pub struct Site {
    pub world: World,
    pub settlement: usize,
    /// Structures of the site, by world index.
    pub structures: Vec<usize>,
    pub things: Vec<Thing>,
}

/// One way out of a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Way {
    /// Index of the link in the structure's interior.
    pub link: usize,
    pub exit: Exit,
    pub to: Place,
    pub passage: Option<scraped_world::structures::Passage>,
    pub state: PassageState,
    pub collapsed: bool,
}

pub fn label<T: serde::Serialize>(x: &T) -> String {
    serde_json::to_value(x)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

impl Site {
    /// The site for a seed: the living settlement with the most writing
    /// (the capital only if nothing else qualifies).
    // DESIGN-Q: until M12's authored frame, play starts in the most-written
    // living town that is not the capital, so the deepest text is elsewhere.
    pub fn new(seed: u64) -> Self {
        let world = World::generate(seed);
        let score = |s: &Settlement| {
            let texts: usize = world
                .structures
                .iter()
                .filter(|st| st.settlement == Some(s.id))
                .map(text_count)
                .sum();
            (
                s.abandoned.is_none(),
                !s.capital,
                texts,
                std::cmp::Reverse(s.id),
            )
        };
        let settlement = world
            .history
            .settlements
            .iter()
            .max_by_key(|s| score(s))
            .map(|s| s.id)
            .unwrap_or(0);
        let structures: Vec<usize> = world
            .structures
            .iter()
            .filter(|st| st.settlement == Some(settlement))
            .map(|st| st.id)
            .collect();
        let mut things = Vec::new();
        for &sid in &structures {
            let st = &world.structures[sid];
            for (ri, room) in st.interior.rooms.iter().enumerate() {
                for f in &room.features {
                    things.push(Thing {
                        id: things.len(),
                        kind: f.kind,
                        material: f.material,
                        home: Place::Room {
                            structure: sid,
                            room: ri,
                        },
                        portable: PORTABLE.contains(&f.kind),
                        texts: f.texts.clone(),
                    });
                }
            }
            if !st.outside.is_empty() {
                things.push(Thing {
                    id: things.len(),
                    kind: "inscription",
                    material: Material::Stone,
                    home: Place::Outside,
                    portable: false,
                    texts: st.outside.clone(),
                });
            }
        }
        Site {
            world,
            settlement,
            structures,
            things,
        }
    }

    pub fn structure(&self, i: usize) -> &Structure {
        &self.world.structures[i]
    }

    /// Whether a structure can be entered at all.
    pub fn enterable(&self, i: usize) -> bool {
        self.structure(i).condition != Condition::Buried
    }

    /// Ways out of a place.
    pub fn ways(&self, place: Place) -> Vec<Way> {
        let Place::Room { structure, room } = place else {
            return Vec::new();
        };
        let st = self.structure(structure);
        let mut out = Vec::new();
        for (link, l) in st.interior.links.iter().enumerate() {
            let (other, exit) = if l.a == room {
                (l.b, l.exit)
            } else if l.b == room {
                (l.a, l.exit.opposite())
            } else {
                continue;
            };
            out.push(Way {
                link,
                exit,
                to: Place::Room {
                    structure,
                    room: other,
                },
                passage: Some(l.passage),
                state: l.state,
                collapsed: st.interior.rooms[other].collapsed,
            });
        }
        out
    }

    /// Variables for describing a structure's name.
    pub fn structure_vars(&self, i: usize) -> Context {
        let st = self.structure(i);
        ctx(&[
            ("kind", Value::from(label(&st.kind))),
            ("condition", Value::from(label(&st.condition))),
        ])
    }

    /// Variables for a thing's name and description.
    pub fn thing_vars(&self, t: &Thing) -> Context {
        let condition = match t.home {
            Place::Room { structure, .. } => label(&self.structure(structure).condition),
            Place::Outside => "worn".to_string(),
        };
        ctx(&[
            ("kind", Value::from(t.kind)),
            ("material", Value::from(label(&t.material))),
            ("written", Value::Bool(!t.texts.is_empty())),
            ("condition", Value::from(condition)),
        ])
    }

    /// Variables for an exit phrase.
    pub fn way_vars(&self, w: &Way) -> Context {
        let state = if w.collapsed {
            "collapsed".to_string()
        } else {
            label(&w.state)
        };
        ctx(&[
            ("direction", Value::from(label(&w.exit))),
            (
                "passage",
                Value::from(
                    w.passage
                        .map(|p| label(&p))
                        .unwrap_or_else(|| "opening".into()),
                ),
            ),
            ("state", Value::from(state)),
        ])
    }

    /// The biome at the site.
    pub fn biome(&self) -> String {
        let c = self.world.history.settlements[self.settlement].cell;
        label(self.world.terrain.biome.get(c.ux(), c.uy()))
    }
}

fn text_count(st: &Structure) -> usize {
    st.outside.len()
        + st.interior
            .rooms
            .iter()
            .flat_map(|r| &r.features)
            .map(|f| f.texts.len())
            .sum::<usize>()
}

/// Builds a context from pairs.
pub fn ctx(pairs: &[(&str, Value)]) -> Context {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
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

/// How light a place is.
// DESIGN-Q: until light and lamps exist (M07), underground rooms and night
// are "dim" but still readable.
pub fn light(place: Place, site: &Site, minutes: u32) -> &'static str {
    let night = matches!(time_of_day(minutes), "night");
    match place {
        Place::Outside if night => "dim",
        Place::Outside => "daylight",
        Place::Room { structure, room } => {
            let r = &site.structure(structure).interior.rooms[room];
            if r.level < 0 || night {
                "dim"
            } else {
                "daylight"
            }
        }
    }
}
