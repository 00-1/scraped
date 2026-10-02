//! The playable site: the starting settlement's buildings, rooms and things,
//! taken from the generated world, and the variables descriptions need.

use scraped_content::{Context, Value};
use scraped_world::history::Settlement;
use scraped_world::structures::{Condition, Exit, Material, PassageState, Structure};
use scraped_world::World;

use scraped_sim::fixtures::{Fixtures, Spot};
pub use scraped_sim::outdoors::{label, time_of_day};
use scraped_sim::outdoors::{outdoor_light, Land, Pos};
use scraped_sim::region::{great_events, great_kind, Great, Regions};
use scraped_sim::writing::{claim_of, Writing};

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
    /// Text ids (world texts, then latent inscriptions), oldest layer first.
    pub texts: Vec<usize>,
    /// Where on the land it is (its building's spot).
    pub pos: Pos,
    /// The written surface it carries, by index into `Writing::surfaces`.
    pub surface: Option<usize>,
}

/// Feature kinds that can be picked up.
const PORTABLE: &[&str] = &["jar", "tablet", "scroll"];

/// A generated world as the game uses it: the starting settlement, every
/// thing that can be examined, and the land prepared for travel.
pub struct Site {
    pub world: World,
    /// The settlement play starts in.
    pub settlement: usize,
    /// Structures of the starting settlement, by world index.
    pub structures: Vec<usize>,
    /// Every thing in the world.
    pub things: Vec<Thing>,
    pub land: Land,
    /// Items, mechanisms, creatures and obstacles of the physical game.
    pub fixtures: Fixtures,
    /// Every written surface and its layers.
    pub writing: Writing,
    /// Regions of the land, for the slow simulation.
    pub regions: Regions,
    /// The great inscriptions of history.
    pub greats: Vec<Great>,
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
        let land = Land::new(&world);
        let mut things = Vec::new();
        // (structure, room, feature) of each thing that is part of a building.
        let mut feature_of: Vec<(usize, Option<usize>, Option<usize>)> = Vec::new();
        for st in &world.structures {
            let sid = st.id;
            let pos = land.structure_pos[sid];
            for (ri, room) in st.interior.rooms.iter().enumerate() {
                for (fi, f) in room.features.iter().enumerate() {
                    feature_of.push((sid, Some(ri), Some(fi)));
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
                        pos,
                        surface: None,
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
                    pos,
                    surface: None,
                });
                feature_of.push((sid, None, None));
            }
        }
        let fixtures = Fixtures::new(&world, &land, settlement);
        for p in &fixtures.items {
            things.push(Thing {
                id: things.len(),
                kind: p.kind,
                material: item_material(p.kind),
                home: place_of(p.at),
                portable: true,
                texts: Vec::new(),
                pos: p.pos,
                surface: None,
            });
        }
        let regions = Regions::new(&world);
        let writing = Writing::new(&world, &land, &fixtures, settlement, Some(&regions));
        // DESIGN-Q: the root reaches three regions out, other great
        // inscriptions two.
        let greats: Vec<Great> = great_events(&world)
            .into_iter()
            .filter_map(|e| {
                let t = world.texts.iter().find(|t| t.event == Some(e))?;
                let c = claim_of(&world, &land, t, t.id)?;
                let root = e == world.history.root;
                Some(Great {
                    kind: great_kind(&c).to_string(),
                    text: t.id,
                    region: regions.at(c.pos)?,
                    reach: if root { 3 } else { 2 },
                    root,
                })
            })
            .collect();
        for (si, s) in writing.surfaces.iter().enumerate() {
            if let Some(t) = feature_of
                .iter()
                .position(|&f| f == (s.structure, s.room, s.feature))
            {
                things[t].surface = Some(si);
                things[t].texts = s.layers.clone();
            }
        }
        Site {
            world,
            settlement,
            structures,
            things,
            land,
            fixtures,
            writing,
            regions,
            greats,
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
            (
                "item",
                Value::Bool(
                    t.texts.is_empty()
                        && scraped_sim::items::kind(t.kind).is_some()
                        && t.kind != "jar",
                ),
            ),
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

    /// The biome at the starting settlement.
    pub fn biome(&self) -> String {
        self.biome_at(self.start())
    }

    pub fn biome_at(&self, p: Pos) -> String {
        let (x, y) = p.cell();
        label(self.world.terrain.biome.get(x, y))
    }

    /// Where play starts: the middle of the starting settlement.
    pub fn start(&self) -> Pos {
        let c = self.world.history.settlements[self.settlement].cell;
        Pos::of_cell(c.ux(), c.uy())
    }

    /// The centre of a settlement.
    pub fn town_pos(&self, settlement: usize) -> Pos {
        let c = self.world.history.settlements[settlement].cell;
        Pos::of_cell(c.ux(), c.uy())
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

/// How light a place is.
// DESIGN-Q: until light and lamps exist (M07), underground rooms and night
// indoors are "dim" but still readable.
pub fn light(place: Place, site: &Site, minutes: u32) -> &'static str {
    let night = matches!(time_of_day(minutes), "night");
    match place {
        Place::Outside => outdoor_light(minutes),
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

/// The game's place for a sim spot.
pub fn place_of(s: Spot) -> Place {
    match s {
        Spot::Out { .. } => Place::Outside,
        Spot::Room { structure, room } => Place::Room { structure, room },
    }
}

/// What an item is mostly made of, for descriptions.
pub fn item_material(kind: &str) -> Material {
    match kind {
        "torch" | "wood" | "berries" => Material::Wood,
        "lamp" | "firesteel" | "pry_bar" | "scraper" | "stylus" | "lens" => Material::Metal,
        "waterskin" | "cloak" => Material::Vellum,
        _ => Material::Clay,
    }
}
