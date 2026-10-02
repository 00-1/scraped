//! Every piece of text the game shows is a content slot declared here.
//! Samplers draw example situations from real generated sites.

use std::cell::RefCell;
use std::rc::Rc;

use scraped_content::{Context, Registry, SlotDef, Value, VarType};

use crate::site::{ctx, label, light, time_of_day, Place, Site};
use scraped_sim::outdoors::{
    bearing, distance_band, duration_band, rough_metres, rough_minutes, Pos, BEARINGS, DISTANCES,
    DURATIONS, EDGES,
};
use scraped_sim::region::VARIABLES;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

pub const BIOMES: &[&str] = &[
    "sea",
    "lake",
    "shore",
    "marsh",
    "grassland",
    "scrub",
    "desert",
    "forest",
    "pine",
    "tundra",
    "rock",
    "snow",
];
pub const STRUCTURES: &[&str] = &[
    "house",
    "temple",
    "storehouse",
    "archive",
    "tomb",
    "cemetery",
    "tower",
    "wall",
    "waystation",
    "bridge",
    "mine",
];
pub const CONDITIONS: &[&str] = &["intact", "worn", "damaged", "ruined", "buried"];
pub const PURPOSES: &[&str] = &[
    "hall",
    "store",
    "sleeping",
    "forecourt",
    "sanctum",
    "chapel",
    "crypt",
    "entrance",
    "reading",
    "stacks",
    "vault",
    "tomb-entrance",
    "passage",
    "burial",
    "offerings",
    "guardroom",
    "watch",
    "lookout",
    "storeroom",
    "tally-room",
    "graves",
    "gatehouse",
    "shelter",
    "span",
    "adit",
    "gallery",
];
pub const KINDS: &[&str] = &[
    "hearth",
    "wall",
    "shelf",
    "jar",
    "stele",
    "basin",
    "statue",
    "altar",
    "niche",
    "sarcophagus",
    "lintel",
    "table",
    "rack",
    "tablet",
    "scroll",
    "chest",
    "door-slab",
    "gravestone",
    "gate",
    "milestone",
    "parapet",
    "beam",
    "inscription",
    "torch",
    "lamp",
    "oil",
    "firesteel",
    "wood",
    "waterskin",
    "cloak",
    "provisions",
    "berries",
    "pry_bar",
    "scraper",
    "stylus",
    "lens",
    "fine_scraper",
    "old_scraper",
    "first_scraper",
    "first_lens",
];
pub const NEEDS: &[&str] = &["warmth", "thirst", "hunger", "rest", "injury", "wet"];
pub const ALL_NEED_STATES: &[&str] = &[
    "warm",
    "chilled",
    "shivering",
    "hypothermic",
    "fine",
    "thirsty",
    "parched",
    "dying",
    "hungry",
    "weak",
    "starving",
    "rested",
    "tired",
    "exhausted",
    "unhurt",
    "bruised",
    "hurt",
    "badly_hurt",
    "dry",
    "damp",
    "soaked",
];
pub const TEMPERATURES: &[&str] = &["freezing", "cold", "cool", "mild", "warm", "hot"];
pub const WETNESSES: &[&str] = &["dry", "damp", "wet", "flooded"];
pub const AIRS: &[&str] = &["still", "draughty", "windy"];
pub const MECH_STATES: &[&str] = &[
    "water", "dry", "open", "shut", "turning", "still", "up", "pulled", "raised", "lowered", "lit",
    "cold",
];
pub const FIRE_PLACES: &[&str] = &["campfire", "hearth", "brazier"];
pub const FIRE_FAILS: &[&str] = &[
    "burning",
    "no_place",
    "no_flame",
    "no_fuel",
    "rain",
    "wet",
    "no_fire",
    "not_lightable",
    "not_lit",
];
pub const SOURCES: &[&str] = &["well", "river", "stream", "lake", "flood", "container"];
pub const ARCHETYPES: &[&str] = &["scavenger", "grazer", "predator", "deep"];
pub const MADE: &[&str] = &["torch", "shelter", "lamp_filled"];
pub const MAKE_WHAT: &[&str] = &["torch", "shelter", "unknown"];
pub const MAKE_WHY: &[&str] = &["no_wood", "indoors", "unknown"];
pub const CROSS_HOW: &[&str] = &["ice", "wade", "swim"];
pub const CROSS_FAILS: &[&str] = &["no_water", "too_wide"];
pub const MATERIALS: &[&str] = &["stone", "clay", "wood", "metal", "plaster", "vellum"];
pub const DIRECTIONS: &[&str] = &["north", "south", "east", "west", "up", "down"];
/// Directions a player may type, compass diagonals included.
pub const ALL_DIRECTIONS: &[&str] = &[
    "north",
    "south",
    "east",
    "west",
    "up",
    "down",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
];
pub const LANDMARKS: &[&str] = &[
    "town",
    "ruins",
    "hill",
    "mountain",
    "house",
    "temple",
    "storehouse",
    "archive",
    "tomb",
    "cemetery",
    "tower",
    "wall",
    "waystation",
    "bridge",
    "mine",
];
pub const TERRAINS: &[&str] = &["flat", "slope", "hilltop", "valley"];
pub const SIDES: &[&str] = &[
    "here",
    "north",
    "northeast",
    "east",
    "southeast",
    "south",
    "southwest",
    "west",
    "northwest",
];
pub const MODES: &[&str] = &["walk", "head", "follow", "back"];
pub const OBSTACLES: &[&str] = &["sea", "lake", "river"];
pub const EDGE_ENDS: &[&str] = &["end", "sea", "lake", "river"];
pub const SHAPES: &[&str] = &["island", "basin"];
pub const PASSAGES: &[&str] = &["door", "arch", "stair", "opening"];
pub const STATES: &[&str] = &["open", "closed", "blocked", "collapsed"];
pub const LIGHTS: &[&str] = &["daylight", "dim", "dark"];
pub const WEATHERS: &[&str] = &["clear", "rain", "fog"];
pub const TIMES: &[&str] = &["dawn", "morning", "afternoon", "evening", "night"];
pub const WRITING: &[&str] = &["left_to_right", "right_to_left", "boustrophedon"];

thread_local! {
    static SITES: RefCell<Vec<(u64, Rc<Site>)>> = const { RefCell::new(Vec::new()) };
}

/// A cached site for samplers, so lint and previews stay fast.
pub fn sample_site(seed: u64) -> Rc<Site> {
    SITES.with(|c| {
        let mut c = c.borrow_mut();
        if let Some((_, s)) = c.iter().find(|(s, _)| *s == seed) {
            return s.clone();
        }
        let site = Rc::new(Site::new(seed));
        c.push((seed, site.clone()));
        // Room for every sample seed: lint walks them all, slot after slot.
        if c.len() > 8 {
            c.remove(0);
        }
        site
    })
}

fn rooms(site: &Site) -> Vec<Place> {
    site.structures
        .iter()
        .flat_map(|&s| {
            (0..site.structure(s).interior.rooms.len()).map(move |r| Place::Room {
                structure: s,
                room: r,
            })
        })
        .collect()
}

fn s_site(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let names: Vec<Value> = site
        .structures
        .iter()
        .map(|&s| Value::from(label(&site.structure(s).kind)))
        .collect();
    (0..3)
        .map(|i| {
            ctx(&[
                ("biome", Value::from(site.biome())),
                ("structures", Value::List(names.clone())),
                ("count", Value::Number(names.len() as i64)),
                (
                    "abandoned",
                    Value::Bool(
                        site.world.history.settlements[site.settlement]
                            .abandoned
                            .is_some(),
                    ),
                ),
                ("time", Value::from(TIMES[i % TIMES.len()])),
            ])
        })
        .collect()
}

fn s_structure(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    site.structures
        .iter()
        .map(|&s| site.structure_vars(s))
        .collect()
}

fn s_room(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    rooms(&site)
        .into_iter()
        .map(|p| {
            let Place::Room { structure, room } = p else {
                unreachable!()
            };
            let st = site.structure(structure);
            let r = &st.interior.rooms[room];
            let things: Vec<Value> = site
                .things
                .iter()
                .filter(|t| t.home == p)
                .map(|t| Value::from(format!("{} {}", label(&t.material), t.kind)))
                .collect();
            let exits: Vec<Value> = site
                .ways(p)
                .iter()
                .map(|w| Value::from(label(&w.exit)))
                .collect();
            ctx(&[
                ("purpose", Value::from(r.purpose)),
                ("structure", Value::from(label(&st.kind))),
                ("condition", Value::from(label(&st.condition))),
                ("level", Value::Number(i64::from(r.level))),
                ("light", Value::from(light(p, &site, 9 * 60))),
                ("things", Value::List(things)),
                ("exits", Value::List(exits)),
                ("time", Value::from(time_of_day(9 * 60))),
            ])
        })
        .collect()
}

fn s_exit(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let mut out: Vec<Context> = rooms(&site)
        .into_iter()
        .flat_map(|p| site.ways(p))
        .map(|w| site.way_vars(&w))
        .collect();
    out.dedup();
    out
}

fn s_thing(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    site.things.iter().map(|t| site.thing_vars(t)).collect()
}

fn s_named(seed: u64) -> Vec<Context> {
    s_thing(seed)
        .into_iter()
        .map(|c| ctx(&[("thing", Value::from(format!("the {}", c["kind"].text())))]))
        .collect()
}

fn s_read(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    site.things
        .iter()
        .filter(|t| !t.texts.is_empty())
        .take(8)
        .map(|t| {
            let era = site.world.texts[t.texts[0]].era as usize;
            ctx(&[
                ("thing", Value::from(format!("the {}", t.kind))),
                ("material", Value::from(label(&t.material))),
                ("glyphs", Value::Number(24)),
                ("page", Value::Number(1)),
                ("pages", Value::Number(2)),
                ("texts", Value::Number(t.texts.len() as i64)),
                (
                    "direction",
                    Value::from(label(&site.world.languages[era].script.direction)),
                ),
                (
                    "hand",
                    Value::from(crate::writing::HANDS[t.id % crate::writing::HANDS.len()]),
                ),
            ])
        })
        .collect()
}

fn s_glyph(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    site.world.languages[0]
        .script
        .glyphs
        .iter()
        .take(6)
        .enumerate()
        .map(|(i, (_, g))| {
            let known = i % 3 == 0;
            ctx(&[
                ("number", Value::Number(i as i64 + 1)),
                ("description", Value::from(g.describe())),
                ("label", Value::from(if known { "ka" } else { "" })),
                ("known", Value::Bool(known)),
            ])
        })
        .collect()
}

fn s_list(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("items", Value::List(vec![])), ("count", Value::Number(0))]),
        ctx(&[
            ("items", Value::List(vec!["a clay tablet".into()])),
            ("count", Value::Number(1)),
        ]),
        ctx(&[
            (
                "items",
                Value::List(vec!["a clay tablet".into(), "a clay jar".into()]),
            ),
            ("count", Value::Number(2)),
        ]),
    ]
}

fn s_options(_: u64) -> Vec<Context> {
    vec![ctx(&[(
        "options",
        Value::List(vec!["the plaster wall".into(), "the clay wall".into()]),
    )])]
}

fn s_word(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("word", Value::from("dance"))]),
        ctx(&[("word", Value::from("fly"))]),
    ]
}

fn s_words(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("words", Value::from("throne"))]),
        ctx(&[("words", Value::from("golden key"))]),
    ]
}

fn s_verb(_: u64) -> Vec<Context> {
    ["examine", "take", "read", "go"]
        .iter()
        .map(|v| ctx(&[("verb", Value::from(*v))]))
        .collect()
}

fn s_define(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("number", Value::Number(3)),
        ("label", Value::from("ka")),
        ("count", Value::Number(14)),
        ("input", Value::from("3 as ka")),
    ])]
}

fn s_minutes(_: u64) -> Vec<Context> {
    vec![ctx(&[("minutes", Value::Number(30))])]
}

fn s_none(_: u64) -> Vec<Context> {
    vec![Context::new()]
}

fn s_remaining(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("remaining", Value::Number(1))]),
        ctx(&[("remaining", Value::Number(3))]),
    ]
}

fn s_way(seed: u64) -> Vec<Context> {
    s_exit(seed)
}

fn s_land_name(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    site.land
        .landmarks
        .iter()
        .map(|l| {
            ctx(&[
                ("kind", Value::from(l.kind)),
                ("size", Value::Number(l.size)),
                ("biome", Value::from(site.biome_at(l.pos))),
            ])
        })
        .collect()
}

fn s_landmark(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let from = site.start();
    site.land
        .in_view(
            &site.world,
            from,
            20000.0,
            site.land.town(&site.world, from),
        )
        .iter()
        .take(8)
        .map(|v| {
            let kind = site.land.landmarks[v.landmark].kind;
            ctx(&[
                ("name", Value::from(format!("the {kind}"))),
                ("kind", Value::from(kind)),
                ("bearing", Value::from(BEARINGS[v.bearing])),
                ("distance", Value::from(distance_band(v.metres))),
            ])
        })
        .collect()
}

fn s_horizon(seed: u64) -> Vec<Context> {
    let all: Vec<Value> = s_landmark(seed)
        .iter()
        .map(|c| {
            Value::from(format!(
                "{} to the {}",
                c["name"].text(),
                c["bearing"].text()
            ))
        })
        .collect();
    [
        (0, "fog", "daylight"),
        (3, "clear", "daylight"),
        (6, "clear", "dim"),
        (1, "rain", "dark"),
    ]
    .iter()
    .map(|&(n, w, l)| {
        let shown: Vec<Value> = all.iter().take(n).cloned().collect();
        let count = shown.len() as i64;
        ctx(&[
            ("landmarks", Value::List(shown)),
            ("count", Value::Number(count)),
            ("weather", Value::from(w)),
            ("light", Value::from(l)),
        ])
    })
    .collect()
}

fn s_weather(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for w in WEATHERS {
        for (l, t) in [
            ("daylight", "morning"),
            ("dim", "evening"),
            ("dark", "night"),
        ] {
            out.push(ctx(&[
                ("weather", Value::from(*w)),
                ("light", Value::from(l)),
                ("time", Value::from(t)),
            ]));
        }
    }
    out
}

/// Sample spots: the start, and the places of landmarks.
fn spots(site: &Site) -> Vec<Pos> {
    let mut out = vec![site.start()];
    out.extend(site.land.landmarks.iter().take(10).map(|l| l.pos));
    out
}

fn s_area(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    spots(&site)
        .into_iter()
        .map(|p| {
            let edges: Vec<Value> = site
                .land
                .edges_near(p)
                .into_iter()
                .map(|(e, b)| {
                    Value::from(match b {
                        None => format!("a {} here", EDGES[e]),
                        Some(b) => format!("a {} to the {}", EDGES[e], BEARINGS[b]),
                    })
                })
                .collect();
            let town = site.land.town(&site.world, p);
            ctx(&[
                ("biome", Value::from(site.biome_at(p))),
                ("terrain", Value::from(site.land.terrain_here(p))),
                ("edges", Value::List(edges)),
                ("near", Value::List(Vec::new())),
                ("high", Value::Bool(site.land.high(p))),
                ("in_town", Value::Bool(town.is_some())),
                ("time", Value::from("morning")),
            ])
        })
        .collect()
}

fn s_edge_name(_: u64) -> Vec<Context> {
    EDGES
        .iter()
        .map(|e| ctx(&[("kind", Value::from(*e))]))
        .collect()
}

fn s_edge(_: u64) -> Vec<Context> {
    EDGES
        .iter()
        .zip(SIDES.iter().cycle())
        .map(|(e, s)| {
            ctx(&[
                ("name", Value::from(format!("the {e}"))),
                ("kind", Value::from(*e)),
                ("side", Value::from(*s)),
            ])
        })
        .collect()
}

fn s_region(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let (biomes, sea) = site.land.region(&site.world, site.start(), 20000.0);
    let main = biomes.first().copied().unwrap_or("grassland");
    vec![ctx(&[
        (
            "biomes",
            Value::List(biomes.into_iter().map(Value::from).collect()),
        ),
        ("main", Value::from(main)),
        (
            "shape",
            Value::from(scraped_sim::outdoors::shape(&site.world)),
        ),
        ("sea", Value::Bool(sea)),
    ])]
}

fn s_report(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let from = site.start();
    let mut out = Vec::new();
    for (i, l) in site.land.landmarks.iter().take(6).enumerate() {
        let m = from.dist(l.pos);
        let minutes = (m / 60.0) as u32;
        out.push(ctx(&[
            ("mode", Value::from(MODES[i % MODES.len()])),
            (
                "bearing",
                Value::from(bearing(from, l.pos).map_or("nowhere", |b| BEARINGS[b])),
            ),
            ("distance", Value::from(distance_band(m))),
            ("metres", Value::Number(rough_metres(m))),
            ("duration", Value::from(duration_band(minutes))),
            ("minutes", Value::Number(rough_minutes(minutes))),
            (
                "edge",
                Value::from(if i % MODES.len() == 2 {
                    "river"
                } else {
                    "none"
                }),
            ),
        ]));
    }
    out
}

fn s_lost(_: u64) -> Vec<Context> {
    [
        ("fog", "daylight", "forest"),
        ("clear", "dark", "grassland"),
        ("rain", "dim", "marsh"),
    ]
    .iter()
    .map(|&(w, l, b)| {
        ctx(&[
            ("weather", Value::from(w)),
            ("light", Value::from(l)),
            ("biome", Value::from(b)),
        ])
    })
    .collect()
}

fn s_place_name(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("name", Value::from("the tall hill"))]),
        ctx(&[("name", Value::from("the gap"))]),
    ]
}

fn s_interrupt(seed: u64) -> Vec<Context> {
    s_landmark(seed)
}

fn s_blocked(_: u64) -> Vec<Context> {
    OBSTACLES
        .iter()
        .zip(BEARINGS.iter())
        .map(|(o, b)| ctx(&[("by", Value::from(*o)), ("bearing", Value::from(*b))]))
        .collect()
}

fn s_edge_end(_: u64) -> Vec<Context> {
    EDGES
        .iter()
        .zip(EDGE_ENDS.iter().cycle())
        .map(|(e, b)| ctx(&[("edge", Value::from(*e)), ("by", Value::from(*b))]))
        .collect()
}

fn s_indoors(_: u64) -> Vec<Context> {
    ["head", "follow", "back", "name"]
        .iter()
        .map(|v| ctx(&[("verb", Value::from(*v))]))
        .collect()
}

fn s_input(_: u64) -> Vec<Context> {
    vec![ctx(&[("input", Value::from("this place"))])]
}

fn s_all_directions(_: u64) -> Vec<Context> {
    ALL_DIRECTIONS
        .iter()
        .map(|d| ctx(&[("direction", Value::from(*d))]))
        .collect()
}

fn s_cues(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for (i, t) in TEMPERATURES.iter().enumerate() {
        out.push(ctx(&[
            ("temperature", Value::from(*t)),
            ("wetness", Value::from(WETNESSES[i % 4])),
            ("air", Value::from(AIRS[i % 3])),
            ("unstable", Value::Bool(i == 2)),
            ("fire", Value::Bool(i == 1)),
            ("dark", Value::Bool(i == 3)),
            ("indoors", Value::Bool(i % 2 == 0)),
            ("uncanny", Value::from(["none", "warmth", "frost"][i % 3])),
        ]));
    }
    out
}

fn s_dark(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("level", Value::Number(-1)),
            ("exits", Value::List(vec!["a stair up".into()])),
        ]),
        ctx(&[("level", Value::Number(0)), ("exits", Value::List(vec![]))]),
    ]
}

fn s_mech(_: u64) -> Vec<Context> {
    scraped_sim::fixtures::MECHANISMS
        .iter()
        .map(|k| ctx(&[("kind", Value::from(*k))]))
        .collect()
}

fn s_mech_state(_: u64) -> Vec<Context> {
    [
        ("well", "water"),
        ("well", "dry"),
        ("sluice", "open"),
        ("sluice", "shut"),
        ("wheel", "turning"),
        ("wheel", "still"),
        ("drain_lever", "pulled"),
        ("bridge_lever", "lowered"),
        ("bridge_lever", "raised"),
        ("brazier", "lit"),
        ("brazier", "cold"),
    ]
    .iter()
    .map(|(k, st)| ctx(&[("kind", Value::from(*k)), ("state", Value::from(*st))]))
    .collect()
}

fn s_fire_place(_: u64) -> Vec<Context> {
    FIRE_PLACES
        .iter()
        .map(|w| ctx(&[("where", Value::from(*w))]))
        .collect()
}

fn s_fire_lit(_: u64) -> Vec<Context> {
    FIRE_PLACES
        .iter()
        .map(|w| ctx(&[("where", Value::from(*w)), ("fuel", Value::Number(120))]))
        .collect()
}

fn s_fire_fail(_: u64) -> Vec<Context> {
    FIRE_FAILS
        .iter()
        .map(|r| ctx(&[("reason", Value::from(*r))]))
        .collect()
}

fn s_fuel(_: u64) -> Vec<Context> {
    vec![ctx(&[("fuel", Value::Number(240))])]
}

fn s_item_kind(_: u64) -> Vec<Context> {
    ["torch", "lamp"]
        .iter()
        .map(|k| ctx(&[("kind", Value::from(*k))]))
        .collect()
}

fn s_item_examine(_: u64) -> Vec<Context> {
    scraped_sim::items::ITEMS
        .iter()
        .map(|k| {
            ctx(&[
                ("kind", Value::from(k.id)),
                ("lit", Value::Bool(k.light)),
                ("fuel", Value::Number(i64::from(k.burns))),
                ("water", Value::Number(i64::from(k.holds / 2))),
                ("holds", Value::Number(i64::from(k.holds))),
                ("worn", Value::Bool(k.warmth > 0)),
            ])
        })
        .collect()
}

fn s_made(_: u64) -> Vec<Context> {
    MADE.iter()
        .map(|k| ctx(&[("kind", Value::from(*k))]))
        .collect()
}

fn s_make_fail(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("what", Value::from("torch")),
            ("reason", Value::from("no_wood")),
        ]),
        ctx(&[
            ("what", Value::from("shelter")),
            ("reason", Value::from("indoors")),
        ]),
        ctx(&[
            ("what", Value::from("unknown")),
            ("reason", Value::from("unknown")),
        ]),
    ]
}

fn s_heavy(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("thing", Value::from("a bundle of wood")),
        ("weight", Value::Number(14)),
        ("limit", Value::Number(15)),
    ])]
}

fn s_drink(_: u64) -> Vec<Context> {
    SOURCES
        .iter()
        .map(|src| {
            ctx(&[
                ("source", Value::from(*src)),
                ("full", Value::Bool(*src != "container")),
            ])
        })
        .collect()
}

fn s_fill(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("thing", Value::from("the waterskin")),
        ("drinks", Value::Number(4)),
    ])]
}

fn s_fill_none(_: u64) -> Vec<Context> {
    ["container", "water"]
        .iter()
        .map(|r| {
            ctx(&[
                ("thing", Value::from("the tablet")),
                ("reason", Value::from(*r)),
            ])
        })
        .collect()
}

fn s_eat(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("thing", Value::from("the provisions")),
            ("hours", Value::Number(12)),
        ]),
        ctx(&[
            ("thing", Value::from("some berries")),
            ("hours", Value::Number(4)),
        ]),
    ]
}

fn s_sleep(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("hours", Value::Number(8)), ("woken", Value::Bool(false))]),
        ctx(&[("hours", Value::Number(2)), ("woken", Value::Bool(true))]),
    ]
}

fn s_biome(_: u64) -> Vec<Context> {
    BIOMES
        .iter()
        .map(|b| ctx(&[("biome", Value::from(*b))]))
        .collect()
}

fn s_found(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("kind", Value::from("berries")),
        ("biome", Value::from("forest")),
    ])]
}

fn s_body_change(_: u64) -> Vec<Context> {
    let groups: [(&str, &[&str]); 6] = [
        ("warmth", scraped_sim::body::WARMTH_STATES),
        ("thirst", scraped_sim::body::THIRST_STATES),
        ("hunger", scraped_sim::body::HUNGER_STATES),
        ("rest", scraped_sim::body::REST_STATES),
        ("injury", scraped_sim::body::INJURY_STATES),
        ("wet", scraped_sim::body::WET_STATES),
    ];
    let mut out = Vec::new();
    for (need, states) in groups {
        for (i, st) in states.iter().enumerate() {
            out.push(ctx(&[
                ("need", Value::from(need)),
                ("state", Value::from(*st)),
                ("worse", Value::Bool(i > 0)),
            ]));
        }
    }
    out
}

fn s_status(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("warmth", Value::from("warm")),
            ("thirst", Value::from("fine")),
            ("hunger", Value::from("fine")),
            ("rest", Value::from("rested")),
            ("injury", Value::from("unhurt")),
            ("wet", Value::from("dry")),
        ]),
        ctx(&[
            ("warmth", Value::from("shivering")),
            ("thirst", Value::from("parched")),
            ("hunger", Value::from("weak")),
            ("rest", Value::from("exhausted")),
            ("injury", Value::from("badly_hurt")),
            ("wet", Value::from("soaked")),
        ]),
    ]
}

fn s_hurt(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("hurt", Value::Number(1))]),
        ctx(&[("hurt", Value::Number(2))]),
    ]
}

fn s_cross(_: u64) -> Vec<Context> {
    CROSS_HOW
        .iter()
        .map(|h| ctx(&[("how", Value::from(*h)), ("by", Value::from("river"))]))
        .collect()
}

fn s_cross_fail(_: u64) -> Vec<Context> {
    CROSS_FAILS
        .iter()
        .map(|r| ctx(&[("reason", Value::from(*r))]))
        .collect()
}

fn s_by(_: u64) -> Vec<Context> {
    ["river", "lake"]
        .iter()
        .map(|b| ctx(&[("by", Value::from(*b))]))
        .collect()
}

fn s_creature_name(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("archetype", Value::from("scavenger")),
            ("biome", Value::from("grassland")),
        ]),
        ctx(&[
            ("archetype", Value::from("grazer")),
            ("biome", Value::from("tundra")),
        ]),
        ctx(&[
            ("archetype", Value::from("predator")),
            ("biome", Value::from("forest")),
        ]),
        ctx(&[
            ("archetype", Value::from("deep")),
            ("biome", Value::from("underground")),
        ]),
    ]
}

fn s_creature_seen(_: u64) -> Vec<Context> {
    ARCHETYPES
        .iter()
        .zip(DISTANCES.iter())
        .map(|(a, d)| {
            ctx(&[
                ("name", Value::from(format!("the {a}"))),
                ("archetype", Value::from(*a)),
                ("bearing", Value::from("north")),
                ("distance", Value::from(*d)),
            ])
        })
        .collect()
}

fn s_creature_near(_: u64) -> Vec<Context> {
    vec![ctx(&[(
        "creatures",
        Value::List(vec!["a scavenger to the north (middle)".into()]),
    )])]
}

fn s_struck(_: u64) -> Vec<Context> {
    ARCHETYPES
        .iter()
        .map(|a| {
            ctx(&[
                ("name", Value::from(format!("the {a}"))),
                ("archetype", Value::from(*a)),
                ("harm", Value::Number(if *a == "grazer" { 1 } else { 2 })),
            ])
        })
        .collect()
}

fn s_stole(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("name", Value::from("the scavenger")),
        ("thing", Value::from("the provisions")),
    ])]
}

fn s_creature(_: u64) -> Vec<Context> {
    vec![ctx(&[("name", Value::from("the scavenger"))])]
}

fn s_death(_: u64) -> Vec<Context> {
    scraped_sim::body::DEATHS
        .iter()
        .enumerate()
        .map(|(i, c)| {
            ctx(&[
                ("cause", Value::from(*c)),
                ("doing", Value::from("head north")),
                ("day", Value::Number(i as i64 + 1)),
                ("indoors", Value::Bool(i % 2 == 1)),
            ])
        })
        .collect()
}

fn s_end(_: u64) -> Vec<Context> {
    let mut out: Vec<Context> = scraped_sim::body::DEATHS
        .iter()
        .enumerate()
        .map(|(i, c)| {
            ctx(&[
                ("ending", Value::from("death")),
                ("cause", Value::from(*c)),
                ("days", Value::Number(i as i64 + 1)),
                ("years", Value::Number(25)),
                ("places", Value::Number(i as i64 * 2)),
                ("named", Value::Number(i as i64 % 3)),
                ("read", Value::Number(i as i64 * 3)),
                ("wrote", Value::Number(i as i64 % 2)),
                ("released", Value::Number(i as i64 % 4)),
            ])
        })
        .collect();
    for (i, e) in crate::ending::ENDINGS[1..].iter().enumerate() {
        out.push(ctx(&[
            ("ending", Value::from(*e)),
            ("cause", Value::from(*e)),
            ("days", Value::Number(40 + i as i64 * 400)),
            (
                "years",
                Value::Number(if *e == "old_age" { 80 } else { 26 }),
            ),
            ("places", Value::Number(12)),
            ("named", Value::Number(3)),
            ("read", Value::Number(30)),
            ("wrote", Value::Number(4)),
            ("released", Value::Number(5)),
        ]));
    }
    out
}

fn s_ending(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("day", Value::Number(12)),
            ("indoors", Value::Bool(true)),
            ("years", Value::Number(25)),
        ]),
        ctx(&[
            ("day", Value::Number(20_000)),
            ("indoors", Value::Bool(false)),
            ("years", Value::Number(80)),
        ]),
    ]
}

fn s_end_region(_: u64) -> Vec<Context> {
    let row = |biome: &str, bearing: &str, aspect: &str, b: &str, a: &str, w: &str, c: &str| {
        ctx(&[
            ("count", Value::Number(if c == "world" { 4 } else { 1 })),
            ("biomes", Value::List(vec![Value::from(biome)])),
            ("bearings", Value::List(vec![Value::from(bearing)])),
            ("aspect", Value::from(aspect)),
            ("before", Value::from(b)),
            ("after", Value::from(a)),
            ("without", Value::from(w)),
            ("cause", Value::from(c)),
        ])
    };
    vec![
        row(
            "grassland",
            "north",
            "life",
            "middling",
            "low",
            "low",
            "world",
        ),
        row("forest", "here", "water", "low", "high", "low", "you"),
        row(
            "marsh",
            "southwest",
            "stability",
            "high",
            "high",
            "low",
            "you",
        ),
        row(
            "scrub", "east", "climate", "usual", "colder", "usual", "you",
        ),
        row("desert", "west", "life", "low", "very_low", "low", "both"),
    ]
}

fn s_end_act(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("day", Value::Number(3)),
            ("act", Value::from("silenced")),
            ("scale", Value::from("great")),
            ("aspect", Value::from("stability")),
            ("rising", Value::Bool(false)),
            ("bearing", Value::from("north")),
        ]),
        ctx(&[
            ("day", Value::Number(9)),
            ("act", Value::from("released")),
            ("scale", Value::from("region")),
            ("aspect", Value::from("water")),
            ("rising", Value::Bool(true)),
            ("bearing", Value::from("here")),
        ]),
    ]
}

fn s_chronicle(_: u64) -> Vec<Context> {
    vec![ctx(&[("words", Value::Number(14))])]
}

fn s_section(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("section", Value::from("transcript"))]),
        ctx(&[("section", Value::from("names"))]),
    ]
}

fn s_glyph_number(_: u64) -> Vec<Context> {
    vec![ctx(&[("number", Value::Number(4))])]
}

fn s_scraped(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("material", Value::from("stone")),
            ("lost", Value::Number(7)),
            ("glyphs", Value::Number(18)),
        ]),
        ctx(&[
            ("material", Value::from("plaster")),
            ("lost", Value::Number(12)),
            ("glyphs", Value::Number(20)),
        ]),
    ]
}

fn s_ghosts(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("count", Value::Number(1))]),
        ctx(&[("count", Value::Number(3))]),
    ]
}

fn s_scrape(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("thing", Value::from("the stone stele")),
        ("material", Value::from("stone")),
    ])]
}

fn s_effect(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for (p, c) in [
        ("heat", "land"),
        ("heat", "room"),
        ("openness", "passage"),
        ("stability", "structure"),
    ] {
        for rising in [true, false] {
            out.push(ctx(&[
                ("property", Value::from(p)),
                ("rising", Value::Bool(rising)),
                ("class", Value::from(c)),
                ("indoors", Value::Bool(c != "land")),
            ]));
        }
    }
    out
}

fn s_tool(_: u64) -> Vec<Context> {
    ["scraper", "stylus", "lens"]
        .iter()
        .map(|k| ctx(&[("kind", Value::from(*k))]))
        .collect()
}

fn s_write(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("thing", Value::from("the plaster wall")),
        ("material", Value::from("plaster")),
        (
            "glyphs",
            Value::List(vec!["a bar, centre".into(), "a hook turned left".into()]),
        ),
    ])]
}

fn s_write_refused(_: u64) -> Vec<Context> {
    ["nothing", "no_tool", "dark", "not_surface", "covered"]
        .iter()
        .map(|r| {
            ctx(&[
                ("reason", Value::from(*r)),
                ("thing", Value::from("the stele")),
            ])
        })
        .collect()
}

fn s_mark(_: u64) -> Vec<Context> {
    vec![ctx(&[("mark", Value::from("zz"))])]
}

fn s_count(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("count", Value::Number(1))]),
        ctx(&[("count", Value::Number(3))]),
    ]
}

pub const BANDS: &[&str] = &["very_low", "low", "middling", "high", "very_high"];
use scraped_sim::region::SEASONS;

fn s_region_state(seed: u64) -> Vec<Context> {
    let site = sample_site(seed);
    let st = &site.regions.initial;
    site.regions
        .regions
        .iter()
        .take(6)
        .enumerate()
        .map(|(i, r)| {
            let v = st.vars[r.id];
            ctx(&[
                ("life", Value::from(scraped_sim::region::band(0, v[0]))),
                ("water", Value::from(scraped_sim::region::band(1, v[1]))),
                ("stability", Value::from(scraped_sim::region::band(2, v[2]))),
                ("climate", Value::from(["usual", "colder", "warmer"][i % 3])),
                ("season", Value::from(SEASONS[i % 4])),
            ])
        })
        .collect()
}

fn s_changed(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("aspect", Value::from("water")),
            ("before", Value::from("high")),
            ("after", Value::from("low")),
        ]),
        ctx(&[
            ("aspect", Value::from("life")),
            ("before", Value::from("low")),
            ("after", Value::from("middling")),
        ]),
        ctx(&[
            ("aspect", Value::from("climate")),
            ("before", Value::from("usual")),
            ("after", Value::from("colder")),
        ]),
    ]
}

fn s_scale(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("scale", Value::from("region"))]),
        ctx(&[("scale", Value::from("great"))]),
    ]
}

fn s_time(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("season", Value::from("spring")),
            ("age", Value::from("young")),
            ("years", Value::Number(25)),
            ("day", Value::Number(1)),
        ]),
        ctx(&[
            ("season", Value::from("winter")),
            ("age", Value::from("old")),
            ("years", Value::Number(61)),
            ("day", Value::Number(12900)),
        ]),
    ]
}

/// Every slot the game declares.
pub fn slots() -> Vec<SlotDef> {
    let thing = "The thing's name as the game refers to it, e.g. 'the stone altar'.";
    vec![
        SlotDef::new("place.site", "What the player sees standing in the open among the site's buildings (the 'look' outside). Sets the scene of a ruined settlement; lists what can be entered. Must not explain what the writing means.")
            .var("biome", e(BIOMES), "The land the site stands in.")
            .var("structures", VarType::List, "Names of the buildings here, already rendered by place.structure.")
            .var("count", VarType::Number, "How many buildings.")
            .var("abandoned", VarType::Bool, "Whether the settlement was abandoned in its history.")
            .var("time", e(TIMES), "Time of day.")
            .max_len(600)
            .sampler(s_site),
        SlotDef::new("place.structure", "A building's short name as used in lists and by the parser ('the ruined temple'). The player types words from it to refer to the building, so include the kind word.")
            .var("kind", e(STRUCTURES), "What kind of building.")
            .var("condition", e(CONDITIONS), "What time has done to it.")
            .min_variants(1)
            .max_len(60)
            .sampler(s_structure),
        SlotDef::new("place.room", "The 'look' inside a room: its feel, its things and its ways out. Things and exits arrive already rendered. Must not say what writing means.")
            .var("purpose", e(PURPOSES), "What the room was for.")
            .var("structure", e(STRUCTURES), "The building it is in.")
            .var("condition", e(CONDITIONS), "The building's condition.")
            .var("level", VarType::Number, "Floor: 0 ground, below 0 underground, above 0 upstairs.")
            .var("light", e(LIGHTS), "How light it is.")
            .var("things", VarType::List, "What is here, rendered by thing.name.")
            .var("exits", VarType::List, "Ways out, rendered by place.exit.")
            .var("time", e(TIMES), "Time of day.")
            .max_len(800)
            .sampler(s_room),
        SlotDef::new("place.exit", "One way out of a room, used inside place.room ('a narrow stair leading down'). Include the direction word so the player knows what to type.")
            .var("direction", e(DIRECTIONS), "Which way.")
            .var("passage", e(PASSAGES), "What kind of passage.")
            .var("state", e(STATES), "Open, closed, blocked by rubble, or leading to a collapsed room.")
            .min_variants(1)
            .max_len(80)
            .sampler(s_exit),
        SlotDef::new("place.out", "The way out of a building's entrance into the open, used in place.room's exits.")
            .min_variants(1)
            .max_len(60)
            .sampler(s_none),
        SlotDef::new("thing.name", "A thing's short name in lists and for the parser ('a clay tablet'). The player types words from it, so include the kind word. Mention writing only as visible marks, never meaning.")
            .var("kind", e(KINDS), "What the thing is.")
            .var("material", e(MATERIALS), "What it is made of.")
            .var("written", VarType::Bool, "Whether there is writing on it.")
            .var("condition", e(CONDITIONS), "The condition of the building it is in.")
            .var("item", VarType::Bool, "A carryable item (torch, wood, cloak…) rather than part of a building or a jar or tablet; its material matters less than its kind. Kinds with two words use an underscore (pry_bar).")
            .min_variants(1)
            .max_len(60)
            .sampler(s_thing),
        SlotDef::new("thing.examine", "What the player sees when examining a thing closely. If it bears writing, say so (and how it looks), and hint that it can be read.")
            .var("kind", e(KINDS), "What the thing is.")
            .var("material", e(MATERIALS), "What it is made of.")
            .var("written", VarType::Bool, "Whether there is writing on it.")
            .var("condition", e(CONDITIONS), "The condition of the building it is in.")
            .var("item", VarType::Bool, "A carryable item rather than part of a building (items show their state through item.examine instead).")
            .max_len(400)
            .sampler(s_thing),
        SlotDef::new("read.frame", "Introduces reading a piece of writing, before its glyphs are listed ('Carved into the stone, in a cramped hand:'). Never reveals meaning.")
            .var("thing", VarType::Text, thing)
            .var("material", e(MATERIALS), "The surface.")
            .var("glyphs", VarType::Number, "How many glyphs the writing has.")
            .var("page", VarType::Number, "Which page of the reading this is (from 1).")
            .var("pages", VarType::Number, "How many pages in all.")
            .var("texts", VarType::Number, "How many separate pieces of writing are on the thing.")
            .var("direction", e(WRITING), "Which way the writing runs.")
            .var("hand", e(crate::writing::HANDS), "The look of the hand it was written in (the most recent visible layer). The same scribe always has the same hand, so attentive players can tell writers apart.")
            .min_variants(1)
            .max_len(200)
            .sampler(s_read),
        SlotDef::new("read.glyph", "One glyph in a reading. 'description' is the glyph described in words (from glyph.describe). If the player has labelled this glyph, 'label' holds their label and 'known' is true. Keep the number visible: players use it with 'define 3 as ka'.")
            .var("number", VarType::Number, "The glyph's position in the writing, from 1.")
            .var("description", VarType::Text, "The glyph, described.")
            .var("label", VarType::Text, "The player's own label for it, or empty.")
            .var("known", VarType::Bool, "Whether the player has labelled it.")
            .min_variants(1)
            .max_len(400)
            .sampler(s_glyph),
        SlotDef::new("read.more", "After a page of reading, when more remains: tells the player to type 'more'.")
            .var("remaining", VarType::Number, "Pages left.")
            .min_variants(1)
            .sampler(s_remaining),
        SlotDef::new("read.end", "After the last page of a reading.").min_variants(1).sampler(s_none),
        SlotDef::new("read.nothing", "The player tries to read something with no writing on it.")
            .var("thing", VarType::Text, thing)
            .sampler(s_named),
        SlotDef::new("read.no_more", "The player types 'more' when nothing is being read.").min_variants(1).sampler(s_none),
        SlotDef::new("say.inventory", "What the player carries.")
            .var("items", VarType::List, "What is carried, rendered by thing.name.")
            .var("count", VarType::Number, "How many things.")
            .sampler(s_list),
        SlotDef::new("say.take", "The player picks something up.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("say.take_fixed", "The player tries to pick up something that cannot be carried (an altar, a wall).")
            .var("thing", VarType::Text, thing)
            .sampler(s_named),
        SlotDef::new("say.take_held", "The player tries to pick up something already carried.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("say.drop", "The player puts something down.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("say.drop_unheld", "The player tries to drop something not carried.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("say.no_exit", "There is no way to go in that direction (indoors, or up and down outdoors).").var("direction", e(ALL_DIRECTIONS), "Which way the player tried.").sampler(s_all_directions),
        SlotDef::new("say.blocked", "The way is closed, blocked, or leads into a collapsed room.")
            .var("direction", e(DIRECTIONS), "Which way.")
            .var("passage", e(PASSAGES), "What kind of passage.")
            .var("state", e(STATES), "Why it cannot be passed.")
            .sampler(s_way),
        SlotDef::new("say.door_open", "The player opens a door.").var("thing", VarType::Text, "The door, as named by place.exit.").sampler(s_named),
        SlotDef::new("say.door_close", "The player closes a door.").var("thing", VarType::Text, "The door, as named by place.exit.").sampler(s_named),
        SlotDef::new("say.door_already_open", "The door is already open.").var("thing", VarType::Text, "The door.").sampler(s_named),
        SlotDef::new("say.door_already_closed", "The door is already closed.").var("thing", VarType::Text, "The door.").sampler(s_named),
        SlotDef::new("say.door_stuck", "The way cannot be opened or closed: it is not a door, or it is blocked or collapsed.")
            .var("thing", VarType::Text, "The way.")
            .sampler(s_named),
        SlotDef::new("say.buried", "The player tries to enter a building too buried to get into.")
            .var("thing", VarType::Text, "The building's name.")
            .sampler(s_named),
        SlotDef::new("say.outside_already", "The player tries to go out while already outside.").sampler(s_none),
        SlotDef::new("say.not_entrance", "The player types 'out' in a room deeper inside a building; they must find their way back to the entrance first.").sampler(s_none),
        SlotDef::new("say.not_here", "The player names something that isn't here (or that the game can't match).")
            .var("words", VarType::Text, "What the player typed.")
            .sampler(s_words),
        SlotDef::new("say.which", "Several things match; ask which one the player means. The player can answer with a distinguishing word or 'first', 'second'…")
            .var("options", VarType::List, "The matching things' names.")
            .sampler(s_options),
        SlotDef::new("say.unknown_verb", "The first word of the command isn't a known verb.").var("word", VarType::Text, "The word.").sampler(s_word),
        SlotDef::new("say.need_object", "A verb needs something to act on ('take what?').").var("verb", VarType::Text, "The verb.").sampler(s_verb),
        SlotDef::new("say.empty", "The player entered nothing.").sampler(s_none),
        SlotDef::new("say.define", "The player labels a glyph of the writing they last read ('define 3 as ka').")
            .var("number", VarType::Number, "The glyph's position.")
            .var("label", VarType::Text, "The player's label.")
            .var("count", VarType::Number, "How many glyphs the writing has.")
            .var("input", VarType::Text, "What the player typed after 'define'.")
            .sampler(s_define),
        SlotDef::new("say.define_bad", "A 'define' command the game can't follow; explain the form 'define 3 as ka', after reading something.")
            .var("number", VarType::Number, "The glyph's position.")
            .var("label", VarType::Text, "The player's label.")
            .var("count", VarType::Number, "How many glyphs the last writing read has (0 if none).")
            .var("input", VarType::Text, "What the player typed after 'define'.")
            .sampler(s_define),
        SlotDef::new("say.wait", "Time passes while the player waits.").var("minutes", VarType::Number, "How long.").sampler(s_minutes),
        SlotDef::new("say.help", "Help: the kinds of commands the game understands. Plain and short.").max_len(900).sampler(s_none),
        SlotDef::new("say.intro", "The opening of a new game, before the first look. M12 replaces this with the authored frame.")
            .var("biome", e(BIOMES), "The land the site stands in.")
            .max_len(900)
            .sampler(s_site),
        SlotDef::new("say.saved", "The game was saved.").sampler(s_none),
        SlotDef::new("say.named", "The player names the spot they stand on ('name this place the gap'). They can later 'go to the gap' from anywhere; the game remembers its true position, but getting there still takes finding the way.")
            .var("name", VarType::Text, "The name the player chose.")
            .sampler(s_place_name),
        SlotDef::new("say.name_bad", "A 'name' command with no name: explain the form 'name this place the gap'.")
            .var("input", VarType::Text, "What the player typed after 'name'.")
            .sampler(s_input),
        SlotDef::new("land.weather", "Weather and light at a glance, opening the look outdoors. Short.")
            .var("weather", e(WEATHERS), "Clear, rain or fog.")
            .var("light", e(LIGHTS), "Daylight, dim (dawn and dusk) or dark (night).")
            .var("time", e(TIMES), "Time of day.")
            .max_len(200)
            .sampler(s_weather),
        SlotDef::new("land.area", "The ground where the player stands outdoors (the local 100–300 m): land, lie of the ground, edges close by (rivers, roads, coast…), and anything near enough to walk to. When 'in_town' is true, place.site follows with the settlement's buildings, so don't list them here.")
            .var("biome", e(BIOMES), "The land here.")
            .var("terrain", e(TERRAINS), "The lie of the ground: flat, a slope, a hilltop or a valley floor.")
            .var("edges", VarType::List, "Edges close by, each rendered by land.edge.")
            .var("near", VarType::List, "Buildings and things close by outside a settlement, rendered by their name slots.")
            .var("high", VarType::Bool, "Whether this is a high point that looks out over the region (land.region follows).")
            .var("in_town", VarType::Bool, "Whether the player stands in a settlement.")
            .var("time", e(TIMES), "Time of day.")
            .max_len(600)
            .sampler(s_area),
        SlotDef::new("land.edge_name", "An edge's short name for the parser ('the river', 'the old road'). The player types words from it ('follow the river'), so include the kind word.")
            .var("kind", e(&EDGES), "What kind of edge.")
            .min_variants(1)
            .max_len(40)
            .sampler(s_edge_name),
        SlotDef::new("land.edge", "One edge close by, inside land.area: where it runs relative to the player ('a river runs just to the east').")
            .var("name", VarType::Text, "The edge's name, from land.edge_name.")
            .var("kind", e(&EDGES), "What kind of edge.")
            .var("side", e(SIDES), "Where it is: 'here' (underfoot or alongside) or a compass point.")
            .max_len(120)
            .sampler(s_edge),
        SlotDef::new("land.name", "A landmark's short name as seen from afar and typed by the player ('the twin-peaked hill', 'the tower'). Include the kind word. Never use the place's real name: the player can't know it.")
            .var("kind", e(LANDMARKS), "What it is: a town, ruins, a hill or mountain, or a lone building.")
            .var("size", VarType::Number, "Towns: 1 hamlet to 4 city. Hills: height above the land around, in hundreds of metres.")
            .var("biome", e(BIOMES), "The land it stands in.")
            .min_variants(1)
            .max_len(60)
            .sampler(s_land_name),
        SlotDef::new("land.landmark", "One distant landmark in a look: what it is, which way, how far by eye ('a tower stands far off to the north-east'). Never exact numbers.")
            .var("name", VarType::Text, "The landmark, from land.name.")
            .var("kind", e(LANDMARKS), "What it is.")
            .var("bearing", e(&BEARINGS), "Which way it lies.")
            .var("distance", e(&DISTANCES), "How far by eye: near (under ~400 m), short (~1 km), middle (a few km), far (up to ~10 km), horizon (beyond).")
            .max_len(160)
            .sampler(s_landmark),
        SlotDef::new("land.horizon", "What stands out further away, closing the look outdoors: the most striking landmarks in view, or how little can be seen (fog, night).")
            .var("landmarks", VarType::List, "Landmarks in view, most striking first, each rendered by land.landmark.")
            .var("count", VarType::Number, "How many.")
            .var("weather", e(WEATHERS), "The weather, which limits the view.")
            .var("light", e(LIGHTS), "The light, which limits the view.")
            .max_len(900)
            .sampler(s_horizon),
        SlotDef::new("land.region", "From a high point: the lie of the whole region (what land spreads out below, whether the sea is in view).")
            .var("biomes", VarType::List, "The commonest kinds of land in view, commonest first (biome ids).")
            .var("main", e(BIOMES), "The commonest.")
            .var("shape", e(SHAPES), "Island (sea all round) or basin (ringed by mountains).")
            .var("sea", VarType::Bool, "Whether the sea is in view.")
            .max_len(400)
            .sampler(s_region),
        SlotDef::new("travel.report", "Opens the account of a journey: which way and how far the player went, as they judge it (they may be wrong if they got lost). Players map from this, so give the bearing and a rough distance or time; never exact figures beyond what's given.")
            .var("mode", e(MODES), "How they travelled: walking to a place, heading a direction, following an edge, or going back.")
            .var("bearing", e(&[&BEARINGS[..], &["nowhere"]].concat()), "The overall direction travelled ('nowhere' if they ended where they began).")
            .var("distance", e(&DISTANCES), "Rough distance band.")
            .var("metres", VarType::Number, "Distance as the player judges it, rounded (to 100 m under a kilometre, 500 m above).")
            .var("duration", e(&DURATIONS), "Rough time taken.")
            .var("minutes", VarType::Number, "Time taken, rounded to quarter hours.")
            .var("edge", e(&[&EDGES[..], &["none"]].concat()), "The edge followed, or 'none'.")
            .max_len(300)
            .sampler(s_report),
        SlotDef::new("travel.lost", "A getting-lost cue: on the way, nothing could be seen to steer by (fog, darkness, deep forest), so the player is less sure where they are. Must not say where they truly are.")
            .var("weather", e(WEATHERS), "The weather now.")
            .var("light", e(LIGHTS), "The light now.")
            .var("biome", e(BIOMES), "The land they are in now.")
            .max_len(300)
            .sampler(s_lost),
        SlotDef::new("travel.arrive", "The player reaches where they were going.")
            .var("name", VarType::Text, "The destination's name.")
            .max_len(200)
            .sampler(s_place_name),
        SlotDef::new("travel.not_there", "The player walked as far as they thought they needed to, but the place they were heading for is not here: they have drifted off course.")
            .var("name", VarType::Text, "Where they meant to go.")
            .max_len(200)
            .sampler(s_place_name),
        SlotDef::new("travel.interrupt", "Something comes into view for the first time, and the player stops to look.")
            .var("name", VarType::Text, "What they see, from land.name.")
            .var("kind", e(LANDMARKS), "What it is.")
            .var("bearing", e(&BEARINGS), "Which way.")
            .var("distance", e(&DISTANCES), "How far by eye.")
            .max_len(200)
            .sampler(s_interrupt),
        SlotDef::new("travel.blocked", "Water bars the way: sea, a lake, or a river too deep to wade, with no ford or bridge here.")
            .var("by", e(OBSTACLES), "What blocks the way.")
            .var("bearing", e(&BEARINGS), "Which way the player was going.")
            .max_len(200)
            .sampler(s_blocked),
        SlotDef::new("travel.edge_end", "The edge the player is following ends or can be followed no further (a river reaches the sea or a lake, a road peters out).")
            .var("edge", e(&EDGES), "The edge being followed.")
            .var("by", e(EDGE_ENDS), "Why: it simply ends, or water.")
            .max_len(200)
            .sampler(s_edge_end),
        SlotDef::new("travel.already", "The player asks to go somewhere they are already at.")
            .var("name", VarType::Text, "The place.")
            .sampler(s_place_name),
        SlotDef::new("travel.no_route", "There is no way to walk to that place from here (across the sea, say).")
            .var("name", VarType::Text, "The place.")
            .sampler(s_place_name),
        SlotDef::new("travel.unseen", "The player names somewhere to go that they can't see from here and haven't named ('you can't see that from here').")
            .var("words", VarType::Text, "What the player typed.")
            .sampler(s_words),
        SlotDef::new("travel.no_edge", "The player asks to follow something that isn't close by (no river here, say), or it's unclear which.")
            .var("words", VarType::Text, "What the player typed.")
            .sampler(s_words),
        SlotDef::new("travel.indoors", "An outdoor command (head, follow, back, name) typed indoors: go outside first.")
            .var("verb", VarType::Text, "The command.")
            .sampler(s_indoors),
        SlotDef::new("prop.cues", "Short cues about the feel of a place, after its look: cold, damp, draughty, dark, unstable stone, a fire. Only what the senses give; often nothing needs saying.")
            .var("temperature", e(TEMPERATURES), "How warm the air is.")
            .var("wetness", e(WETNESSES), "How wet the place is (flooded: standing water).")
            .var("air", e(AIRS), "Still, draughty or windy.")
            .var("unstable", VarType::Bool, "Loose stone overhead that noise could bring down.")
            .var("fire", VarType::Bool, "A fire burns here.")
            .var("dark", VarType::Bool, "Too dark to see.")
            .var("indoors", VarType::Bool, "In a room rather than outdoors.")
            .var("uncanny", e(&["none", "warmth", "frost"]), "Heat or cold that doesn't belong here (writing's doing): an unnatural warmth, or frost against the season. Never say why.")
            .max_len(300)
            .sampler(s_cues),
        SlotDef::new("place.dark", "The look in a room too dark to see: no things, only the ways out the player can feel. Should make the player want light.")
            .var("level", VarType::Number, "Floor: 0 ground, below 0 underground.")
            .var("exits", VarType::List, "Ways out, rendered by place.exit.")
            .max_len(300)
            .sampler(s_dark),
        SlotDef::new("read.dark", "The player tries to read in the dark.")
            .var("thing", VarType::Text, "The thing's name.")
            .sampler(s_named),
        SlotDef::new("mech.name", "A mechanism's short name for lists and the parser ('the well', 'the sluice gate'). Include the kind word (well, sluice, wheel, lever, brazier).")
            .var("kind", e(scraped_sim::fixtures::MECHANISMS), "What it is: a well, a sluice gate on a river, a water wheel, a lever that drains a flooded room, a lever that raises and lowers a bridge, or a brazier.")
            .min_variants(1)
            .max_len(60)
            .sampler(s_mech),
        SlotDef::new("mech.examine", "A mechanism looked at closely: what it is and what state it is in. Hint at what it does without explaining the puzzle.")
            .var("kind", e(scraped_sim::fixtures::MECHANISMS), "What it is.")
            .var("state", e(MECH_STATES), "Its state: a well with water or dry; a sluice open or shut; a wheel turning or still; a drain lever up or pulled; a bridge raised or lowered; a brazier lit or cold.")
            .max_len(300)
            .sampler(s_mech_state),
        SlotDef::new("mech.operate", "The player works a mechanism and it moves: a sluice opens or shuts, a drain lever is pulled (the water starts to go), a bridge comes down or goes up.")
            .var("kind", e(scraped_sim::fixtures::MECHANISMS), "What it is.")
            .var("state", e(MECH_STATES), "Its new state.")
            .max_len(300)
            .sampler(s_mech_state),
        SlotDef::new("mech.already", "The mechanism is already the way the player wants it.")
            .var("kind", e(scraped_sim::fixtures::MECHANISMS), "What it is.")
            .var("state", e(MECH_STATES), "Its state.")
            .sampler(s_mech_state),
        SlotDef::new("mech.cannot", "This mechanism can't be worked by hand (a water wheel turns only with the river).")
            .var("kind", e(scraped_sim::fixtures::MECHANISMS), "What it is.")
            .var("state", e(MECH_STATES), "Its state.")
            .sampler(s_mech_state),
        SlotDef::new("fire.name", "The fire burning here, as the parser names it ('the fire'). Include the word 'fire'.")
            .var("where", e(FIRE_PLACES), "A campfire outdoors, a hearth, or a brazier.")
            .min_variants(1)
            .max_len(40)
            .sampler(s_fire_place),
        SlotDef::new("fire.lit", "A fire catches.")
            .var("where", e(FIRE_PLACES), "Where it burns.")
            .var("fuel", VarType::Number, "Minutes it will burn without more wood.")
            .sampler(s_fire_lit),
        SlotDef::new("fire.fail", "A fire or flame can't be made or worked: something is missing or wrong. Say what, plainly, so the player knows what to find.")
            .var("reason", e(FIRE_FAILS), "Why: one is already burning; no hearth or brazier here (indoors); nothing to strike a flame with; no wood; rain; the wood is too wet (as wet as the player); no fire here; that can't be lit; that isn't lit.")
            .max_len(200)
            .sampler(s_fire_fail),
        SlotDef::new("fire.fed", "More wood goes on the fire.").var("fuel", VarType::Number, "Minutes it will now burn.").sampler(s_fuel),
        SlotDef::new("fire.out", "A fire nearby burns down and goes out.").var("where", e(FIRE_PLACES), "Which fire.").sampler(s_fire_place),
        SlotDef::new("fire.doused", "The player puts a fire out.").var("where", e(FIRE_PLACES), "Which fire.").sampler(s_fire_place),
        SlotDef::new("item.examine", "An item looked at closely: what it is, and its state (lit, how much burning is left, how much water it holds, whether worn). Only mention what applies.")
            .var("kind", e(&scraped_sim::items::ids()), "What it is.")
            .var("lit", VarType::Bool, "Burning now (torch, lamp).")
            .var("fuel", VarType::Number, "Minutes of burning left (torch, lamp), or what it adds to a fire (wood, oil).")
            .var("water", VarType::Number, "Drinks of water in it.")
            .var("holds", VarType::Number, "Drinks it can hold (0 if not a container).")
            .var("worn", VarType::Bool, "Worn now (cloak).")
            .max_len(300)
            .sampler(s_item_examine),
        SlotDef::new("item.lit", "A torch or lamp is lit (or already was).").var("kind", e(&["torch", "lamp"]), "Which.").sampler(s_item_kind),
        SlotDef::new("item.out", "A torch burns out (it is used up) or a lamp runs dry.").var("kind", e(&["torch", "lamp"]), "Which.").sampler(s_item_kind),
        SlotDef::new("item.doused", "The player puts out a torch or lamp.").var("kind", e(&["torch", "lamp"]), "Which.").sampler(s_item_kind),
        SlotDef::new("item.made", "The player makes something: a torch from wood, a rough shelter, or fills a lamp with oil.").var("kind", e(MADE), "What was made.").sampler(s_made),
        SlotDef::new("item.make_fail", "Something can't be made: what's missing.")
            .var("what", e(MAKE_WHAT), "What the player tried to make ('unknown' for anything the game doesn't know how to make).")
            .var("reason", e(MAKE_WHY), "Why not: no wood, or a shelter indoors.")
            .sampler(s_make_fail),
        SlotDef::new("item.too_heavy", "The player can't carry any more.")
            .var("thing", VarType::Text, "What they tried to pick up.")
            .var("weight", VarType::Number, "What they carry now.")
            .var("limit", VarType::Number, "The most they can carry.")
            .sampler(s_heavy),
        SlotDef::new("item.wear", "The player puts on clothing.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("item.remove", "The player takes clothing off.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("item.cannot_use", "The player tries to use, wear or pry with something in a way it can't be used.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("drink.done", "The player drinks: deeply from a well, river, stream or lake, or a mouthful from a container.")
            .var("source", e(SOURCES), "Where the water comes from (flood: standing water in a room).")
            .var("full", VarType::Bool, "A full drink (from a source) rather than a mouthful.")
            .sampler(s_drink),
        SlotDef::new("drink.none", "There is nothing to drink here and nothing carried.").sampler(s_none),
        SlotDef::new("fill.done", "A container is filled with water.")
            .var("thing", VarType::Text, thing)
            .var("drinks", VarType::Number, "Drinks it now holds.")
            .sampler(s_fill),
        SlotDef::new("fill.none", "A container can't be filled: it isn't one (or isn't carried), or there is no water here.")
            .var("thing", VarType::Text, thing)
            .var("reason", e(&["container", "water"]), "Why.")
            .sampler(s_fill_none),
        SlotDef::new("eat.done", "The player eats.")
            .var("thing", VarType::Text, thing)
            .var("hours", VarType::Number, "Hours of hunger it takes away.")
            .sampler(s_eat),
        SlotDef::new("eat.none", "Nothing to eat (or that isn't food).").sampler(s_none),
        SlotDef::new("sleep.done", "The player wakes after sleeping (or is woken early).")
            .var("hours", VarType::Number, "Hours slept.")
            .var("woken", VarType::Bool, "Woken before rested.")
            .sampler(s_sleep),
        SlotDef::new("forage.found", "An hour's foraging finds something to eat.")
            .var("kind", e(&["berries"]), "What was found.")
            .var("biome", e(BIOMES), "Where.")
            .sampler(s_found),
        SlotDef::new("forage.none", "An hour's foraging finds nothing.").var("biome", e(BIOMES), "Where.").sampler(s_biome),
        SlotDef::new("gather.found", "The player gathers a bundle of firewood.").var("biome", e(BIOMES), "Where.").sampler(s_biome),
        SlotDef::new("gather.none", "There is no wood to gather here (open, bare or frozen land).").var("biome", e(BIOMES), "Where.").sampler(s_biome),
        SlotDef::new("body.change", "A need changes: the player grows thirsty, starts shivering, warms up again, is hurt, heals, dries off. Said once, when the state changes. Coarse, felt, never numbers.")
            .var("need", e(NEEDS), "Which need.")
            .var("state", e(ALL_NEED_STATES), "Its new state (warm/chilled/shivering/hypothermic; fine/thirsty/parched/dying; fine/hungry/weak/starving; rested/tired/exhausted; unhurt/bruised/hurt/badly_hurt; dry/damp/soaked).")
            .var("worse", VarType::Bool, "Whether it got worse (or better).")
            .max_len(200)
            .sampler(s_body_change),
        SlotDef::new("body.status", "How the player feels, all needs at once (the 'status' command). Felt, not numbers.")
            .var("warmth", e(scraped_sim::body::WARMTH_STATES), "Warmth.")
            .var("thirst", e(scraped_sim::body::THIRST_STATES), "Thirst.")
            .var("hunger", e(scraped_sim::body::HUNGER_STATES), "Hunger.")
            .var("rest", e(scraped_sim::body::REST_STATES), "Rest.")
            .var("injury", e(scraped_sim::body::INJURY_STATES), "Injury.")
            .var("wet", e(scraped_sim::body::WET_STATES), "Wetness.")
            .max_len(500)
            .sampler(s_status),
        SlotDef::new("body.collapse", "Exhaustion: the player drops where they stand and sleeps.").sampler(s_none),
        SlotDef::new("hazard.fall", "A fall on a stair in the dark.").var("hurt", VarType::Number, "Levels of injury (1 or 2).").sampler(s_hurt),
        SlotDef::new("hazard.collapse", "Noise brings loose stone down in this room: a way may close, another open, and the player may be hurt.").var("hurt", VarType::Number, "Levels of injury.").sampler(s_hurt),
        SlotDef::new("hazard.flooded", "The way leads into a room under water; the player can't go that way until it drains.").var("thing", VarType::Text, "The way, from place.exit.").sampler(s_named),
        SlotDef::new("hazard.barred", "A door is barred or stuck fast: it won't open by hand. (A pry bar would do it.)").var("thing", VarType::Text, "The door, from place.exit.").sampler(s_named),
        SlotDef::new("door.pried", "The player forces a barred door open with a pry bar.").var("thing", VarType::Text, "The door.").sampler(s_named),
        SlotDef::new("shout.done", "The player shouts or makes a din.").sampler(s_none),
        SlotDef::new("cross.done", "The player gets across water: walking on ice, wading, or swimming.")
            .var("how", e(CROSS_HOW), "How.")
            .var("by", e(&["river", "lake"]), "What was crossed.")
            .sampler(s_cross),
        SlotDef::new("cross.fail", "The player can't cross here: no water close by, or it is too wide to swim.").var("reason", e(CROSS_FAILS), "Why.").sampler(s_cross_fail),
        SlotDef::new("cross.fell_through", "Thin ice gives way under the player. They are soaked and cold, and may not get out.").var("by", e(&["river", "lake"]), "Where.").sampler(s_by),
        SlotDef::new("cross.swept", "The current takes the player off their feet.").var("by", e(&["river", "lake"]), "Where.").sampler(s_by),
        SlotDef::new("creature.name", "A creature's short name ('the scavenger', 'the thing in the dark'). Invent the beasts of this world in words, never a real species; include a word the player can type.")
            .var("archetype", e(ARCHETYPES), "What kind: a scavenger (steals food, scared off by fire and noise), a grazer (charges if you come close), a predator (strikes from hiding, kept off by fire), or something deep (lives in dark underground rooms, fears light).")
            .var("biome", e(&[BIOMES, &["underground"]].concat()), "Where it lives.")
            .min_variants(1)
            .max_len(60)
            .sampler(s_creature_name),
        SlotDef::new("creature.seen", "One creature in view, inside a look ('a scavenger to the north, not far').")
            .var("name", VarType::Text, "Its name, from creature.name.")
            .var("archetype", e(ARCHETYPES), "What kind.")
            .var("bearing", e(&BEARINGS), "Which way.")
            .var("distance", e(&DISTANCES), "How far by eye.")
            .max_len(120)
            .sampler(s_creature_seen),
        SlotDef::new("creature.near", "Creatures in view, after the look outdoors.").var("creatures", VarType::List, "Each rendered by creature.seen.").sampler(s_creature_near),
        SlotDef::new("creature.sighted", "A creature comes into view and the player stops (travel interrupted).")
            .var("name", VarType::Text, "Its name.")
            .var("archetype", e(ARCHETYPES), "What kind.")
            .var("bearing", e(&BEARINGS), "Which way.")
            .var("distance", e(&DISTANCES), "How far by eye.")
            .sampler(s_creature_seen),
        SlotDef::new("creature.struck", "A creature strikes the player: a grazer's charge, a predator from hiding, something in the dark.")
            .var("name", VarType::Text, "Its name.")
            .var("archetype", e(ARCHETYPES), "What kind.")
            .var("harm", VarType::Number, "Levels of injury (1 or 2).")
            .sampler(s_struck),
        SlotDef::new("creature.stole", "A scavenger makes off with the player's food.")
            .var("name", VarType::Text, "Its name.")
            .var("thing", VarType::Text, "What it took.")
            .sampler(s_stole),
        SlotDef::new("creature.fled", "Fire, light or noise drives a creature off.").var("name", VarType::Text, "Its name.").sampler(s_creature),
        SlotDef::new("death.narrate", "The player dies. One short paragraph per cause; the run ends. Should feel earned, not cruel.")
            .var("cause", e(scraped_sim::body::DEATHS), "What killed them: cold, thirst, hunger, injury (wounds), drowning, a fall, falling stone (collapse), or a creature.")
            .var("doing", VarType::Text, "The command they were carrying out.")
            .var("day", VarType::Number, "Which day of the run (from 1).")
            .var("indoors", VarType::Bool, "Whether they died indoors.")
            .max_len(600)
            .sampler(s_death),
        SlotDef::new("end.summary", "The run is over: the opening of the end-of-run summary, after the death or ending narration. The whole picture follows (region by region, the player's acts, the chronicle); this frames it with what the player did in plain counts.")
            .var("ending", e(crate::ending::ENDINGS), "How it ended: death, leaving (left), writing yourself into the world (written_in), old age, or being overtaken by the land's collapse.")
            .var("cause", e(&[scraped_sim::body::DEATHS, crate::ending::ENDINGS].concat()), "The cause of death, or the ending again.")
            .var("days", VarType::Number, "Days of the run, from 1.")
            .var("years", VarType::Number, "The player's age at the end.")
            .var("places", VarType::Number, "Buildings entered.")
            .var("named", VarType::Number, "Places the player named.")
            .var("read", VarType::Number, "Texts read.")
            .var("wrote", VarType::Number, "Texts the player wrote.")
            .var("released", VarType::Number, "Texts the player scraped (claims released).")
            .max_len(600)
            .sampler(s_end),
        SlotDef::new("end.left", "The player scrapes the departure claim they wrote and leaves the world: the run ends by choice. The world stays as it is now.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.written_in", "The player scrapes a claim they wrote about themselves (not the departure): they become part of the world, a trace in it, and the run ends.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.old_age", "The player's life ends of old age.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.overtaken", "The region the player stands in collapses (its ground and its life both give way) and the player is overtaken by it.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.region", "One line of the end summary: one aspect of the land, as it was at the start of the run and at its end, in one or more regions that changed alike, and whether that was the player's doing. 'without' is how it would be had the player done nothing (equal to 'after' when cause is world; when the player held a region steady, before equals after). The player's doing comes first.")
            .var("count", VarType::Number, "How many regions changed this way.")
            .var("bearings", VarType::List, "Which ways they lie from where the run began (compass points, or 'here'), each once.")
            .var("biomes", VarType::List, "Their commonest lands (biome ids), each once.")
            .var("aspect", e(&VARIABLES), "What: life, water, stability, climate.")
            .var("before", e(&[BANDS, &["colder", "usual", "warmer"]].concat()), "How it was at the start.")
            .var("after", e(&[BANDS, &["colder", "usual", "warmer"]].concat()), "How it is at the end.")
            .var("without", e(&[BANDS, &["colder", "usual", "warmer"]].concat()), "How it would be without the player.")
            .var("cause", e(crate::ending::CAUSES), "world: it would have gone so anyway; you: the player's doing; both.")
            .max_len(300)
            .sampler(s_end_region),
        SlotDef::new("end.calm", "In the end summary: nothing changed across the land that anyone would notice, by the player's hand or otherwise.").max_len(300).sampler(s_none),
        SlotDef::new("end.act", "In the end summary, the causal chain: one thing the player did that began or ended a push on a region's course (scraping their own writing with a strong scraper, or silencing a great inscription).")
            .var("day", VarType::Number, "Day of the run.")
            .var("act", e(&["released", "silenced"]), "Began a push (released) or ended one (silenced).")
            .var("scale", e(&["region", "great"]), "A region, or several (a great inscription's reach).")
            .var("aspect", e(&VARIABLES), "What it pushes.")
            .var("rising", VarType::Bool, "Toward more (true) or less.")
            .var("bearing", e(&[&BEARINGS[..], &["here"]].concat()), "Which way from where the run began.")
            .max_len(300)
            .sampler(s_end_act),
        SlotDef::new("end.chronicle", "Before the chronicle: a short text in the language, written as by those who came after, about the player's time. It follows as glyph numbers (or the player's labels), '/' between words. Present it as found writing; never say what it means.")
            .var("words", VarType::Number, "How many words it has.")
            .max_len(300)
            .sampler(s_chronicle),
        SlotDef::new("read.legacy", "Among the faint layers the lens shows: one older than anything, in a hand the player may recognise. It is the final inscription of a previous run. Never say so outright.").max_len(300).sampler(s_none),
        SlotDef::new("notebook.heading", "A heading in the exported notebook: the transcript of the run, or the list of places the player named.")
            .var("section", e(&["transcript", "names"]), "Which file.")
            .max_len(80)
            .sampler(s_section),
        SlotDef::new("read.lost", "A glyph of a scraped layer that can't be made out any more, in a reading. Keep the number visible.")
            .var("number", VarType::Number, "The glyph's position.")
            .min_variants(1)
            .max_len(120)
            .sampler(s_glyph_number),
        SlotDef::new("read.scraped", "Before a scraped layer's glyphs: this writing was scraped, and only part of it survives ('Beneath the scouring, a few strokes survive:').")
            .var("material", e(MATERIALS), "The surface.")
            .var("lost", VarType::Number, "How many glyphs are lost to the eye (more survive in better light).")
            .var("glyphs", VarType::Number, "How many glyphs there were.")
            .max_len(200)
            .sampler(s_scraped),
        SlotDef::new("read.ghosts", "Fainter marks lie beneath what can be read: older layers, too faint to make out yet.")
            .var("count", VarType::Number, "How many older layers.")
            .max_len(200)
            .sampler(s_ghosts),
        SlotDef::new("scrape.done", "The act of scraping a whole text off a surface. Only the act; whatever it releases is told by effect.change.")
            .var("thing", VarType::Text, thing)
            .var("material", e(MATERIALS), "The surface.")
            .max_len(300)
            .sampler(s_scrape),
        SlotDef::new("scrape.no_tool", "The player tries to scrape writing with nothing to scrape it with.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("scrape.bare", "Nothing fresh to scrape: the surface's writing is already scraped.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("effect.change", "The moment a claim takes effect (or stops) where the player is: a physical change only, never what the writing said. Heat rising or falling; doors swinging open or slamming shut; stone groaning loose or settling firm; frost or warmth spreading over the land.")
            .var("property", e(&["heat", "openness", "stability"]), "What changes.")
            .var("rising", VarType::Bool, "Warmer, more open, sounder (true) or the reverse.")
            .var("class", e(&["passage", "room", "land", "structure"]), "What it acts on: doors and gates, rooms, the open land, stone.")
            .var("indoors", VarType::Bool, "Whether the player is indoors.")
            .max_len(300)
            .sampler(s_effect),
        SlotDef::new("effect.held", "A door won't move, though nothing bars it: it is held (writing's doing; never say so).").var("thing", VarType::Text, "The door, from place.exit.").sampler(s_named),
        SlotDef::new("tool.found", "The player first picks up one of the three writing tools: the scraper, the stylus or the lens. A moment of discovery; don't explain what it does.")
            .var("kind", e(&["scraper", "stylus", "lens", "fine_scraper", "old_scraper", "first_scraper", "first_lens"]), "Which tool: the scraper, the stylus, the lens, one of the stronger scrapers (fine, old, and the first, strongest of all), or the first lens, which reads the faintest layers.")
            .max_len(400)
            .sampler(s_tool),
        SlotDef::new("write.done", "The player writes new text on a surface. Echo the glyphs back as they were cut or painted (descriptions given), never what they mean.")
            .var("thing", VarType::Text, thing)
            .var("material", e(MATERIALS), "The surface.")
            .var("glyphs", VarType::List, "Each glyph written, described (from glyph.describe).")
            .max_len(1200)
            .sampler(s_write),
        SlotDef::new("write.refused", "Writing can't begin: no stylus, too dark, nothing to write, not a surface that takes writing, or fresh writing already covers it (it must be scraped first).")
            .var("reason", e(&["nothing", "no_tool", "dark", "not_surface", "covered", "too_great"]), "Why (too_great: the writing here is too great for the scrapers carried).")
            .var("thing", VarType::Text, "The surface named, if any.")
            .max_len(200)
            .sampler(s_write_refused),
        SlotDef::new("write.unknown_mark", "The player names a glyph that isn't a glyph number of the script or one of their own labels.")
            .var("mark", VarType::Text, "What they typed.")
            .sampler(s_mark),
        SlotDef::new("write.hesitate", "The player's hand won't commit marks they don't know well enough: some words in what they want to write haven't been seen in enough writing yet. Physical, not a rule; don't say which words.")
            .var("count", VarType::Number, "How many unfamiliar words.")
            .max_len(200)
            .sampler(s_count),
        SlotDef::new("write.smudge", "The new writing won't take over the trace beneath it: it beads, runs or flakes away, because it doesn't fit what was there before. Physical, never the rule.")
            .var("thing", VarType::Text, thing)
            .var("material", e(MATERIALS), "The surface.")
            .max_len(200)
            .sampler(s_scrape),
        SlotDef::new("write.backlash", "A malformed potent inscription turns on its writer as it is scraped: a jolt, a burn, something that hurts (one level of injury).").sampler(s_none),
        SlotDef::new("scrape.wet", "The player tries to scrape writing whose ink or cuts are still fresh; it must dry first.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("read.deep", "Through the lens, a fainter layer shows beneath the top one, before its glyphs are listed. Through the first lens, every faint layer down to the oldest shows.")
            .var("count", VarType::Number, "How many faint layers show (more than one only through the first lens).")
            .max_len(200)
            .sampler(s_count),
        SlotDef::new("region.cues", "The state of the wider land, seen from here, after the look outdoors: how much grows, how much water there is, whether the ground is sound, whether it is colder or warmer than the land should be, and the season. Only what can be seen; often little needs saying.")
            .var("life", e(BANDS), "How much grows and lives in this region.")
            .var("water", e(BANDS), "How much water: rivers, pools, damp ground.")
            .var("stability", e(BANDS), "How sound the ground and stone are (very_low: cracking, slumping).")
            .var("climate", e(&["colder", "usual", "warmer"]), "Colder or warmer than this land should be.")
            .var("season", e(&SEASONS), "The season.")
            .max_len(400)
            .sampler(s_region_state),
        SlotDef::new("region.changed", "The player returns to a place and it has changed since they last saw it: contrast what they remember with what is here now ('The pool you remember is gone; cracked mud remains'). Said only when a real change happened.")
            .var("aspect", e(&["life", "water", "stability", "climate"]), "What changed.")
            .var("before", e(&[BANDS, &["colder", "usual", "warmer"]].concat()), "How it was.")
            .var("after", e(&[BANDS, &["colder", "usual", "warmer"]].concat()), "How it is now.")
            .max_len(300)
            .sampler(s_changed),
        SlotDef::new("great.site", "The room holds one of the great inscriptions: writing whose force is felt across the land. Something palpable, never its meaning.").max_len(400).sampler(s_none),
        SlotDef::new("great.release", "The player scrapes writing with a scraper strong enough that its claim reaches across a region, or further. The feeling of something vast letting go.")
            .var("scale", e(&["region", "great"]), "How far it reaches: a region, or the great scale.")
            .max_len(400)
            .sampler(s_scale),
        SlotDef::new("scrape.too_weak", "The scraper the player carries is not strong enough for this writing: it skids and won't bite.").var("thing", VarType::Text, thing).sampler(s_named),
        SlotDef::new("time.status", "The season and the player's age, with the 'status' command.")
            .var("season", e(&SEASONS), "The season.")
            .var("age", e(crate::trajectory::AGES), "How old the player feels.")
            .var("years", VarType::Number, "Years of age.")
            .var("day", VarType::Number, "Day of the run, from 1.")
            .max_len(200)
            .sampler(s_time),
        SlotDef::new("travel.back_none", "The player asks to go back, but hasn't travelled anywhere yet.").sampler(s_none),
        SlotDef::new("say.loaded", "A saved game was loaded.").sampler(s_none),
        SlotDef::new("say.export_offer", "After the end of a run, in the terminal client: the player can type 'export' to save their notebook (the transcript, the places they named, and the run record) as files.").max_len(300).sampler(s_none),
        SlotDef::new("say.exported", "The notebook was saved as files.").max_len(200).sampler(s_none),
        SlotDef::new("say.legacy_kept", "Legacy is on and the run ended with an inscription of the player's: it will lie, faint and very old, somewhere in the next world.").max_len(300).sampler(s_none),
        SlotDef::new("say.pack_changed", "A loaded save was made with different text (content pack) than now: the story replays the same, but wording may differ.")
            .sampler(s_none),
    ]
}

/// The registry of every slot: language engine and game.
pub fn registry() -> Registry {
    let mut all = scraped_lang::slots::slots();
    all.extend(slots());
    Registry::new(all)
}
