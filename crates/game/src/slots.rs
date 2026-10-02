//! Every piece of text the game shows is a content slot declared here.
//! Samplers draw example situations from real generated sites.

use std::cell::RefCell;
use std::rc::Rc;

use scraped_content::{Context, Registry, SlotDef, Value, VarType};

use crate::outdoors::{
    bearing, distance_band, duration_band, rough_metres, rough_minutes, Pos, BEARINGS, DISTANCES,
    DURATIONS, EDGES,
};
use crate::site::{ctx, label, light, time_of_day, Place, Site};

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
];
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
        ("shape", Value::from(crate::outdoors::shape(&site.world))),
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
            .min_variants(1)
            .max_len(60)
            .sampler(s_thing),
        SlotDef::new("thing.examine", "What the player sees when examining a thing closely. If it bears writing, say so (and how it looks), and hint that it can be read.")
            .var("kind", e(KINDS), "What the thing is.")
            .var("material", e(MATERIALS), "What it is made of.")
            .var("written", VarType::Bool, "Whether there is writing on it.")
            .var("condition", e(CONDITIONS), "The condition of the building it is in.")
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
        SlotDef::new("travel.back_none", "The player asks to go back, but hasn't travelled anywhere yet.").sampler(s_none),
        SlotDef::new("say.loaded", "A saved game was loaded.").sampler(s_none),
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
