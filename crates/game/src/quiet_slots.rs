//! The slots of quiet text (D02): one short slot per kind of fact, chosen
//! by the attention model, and the slots that digging and the senses
//! answer with. Each says one thing; Jb writes many short pieces that
//! combine, instead of long templates that list everything.

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::attention::{season_evidence_ids, AMOUNTS, REGION_EVIDENCE};
use crate::site::ctx;
use crate::slots::{
    AIRS, BIOMES, CONDITIONS, KINDS, LIGHTS, MATERIALS, NEEDS, PURPOSES, STRUCTURES, TEMPERATURES,
    TERRAINS, TIMES, WETNESSES,
};
use scraped_sim::outdoors::BEARINGS;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

fn e_or_empty(values: &[&str]) -> VarType {
    VarType::Enum {
        values: std::iter::once("")
            .chain(values.iter().copied())
            .map(str::to_string)
            .collect(),
    }
}

/// Why a building stands out from the rest.
pub const STANDOUTS: &[&str] = &["tallest", "only", "best_kept", "worst_kept"];
/// What makes a sound.
pub const SOUNDS: &[&str] = &[
    "river", "stream", "sea", "lake", "wind", "rain", "fire", "creature", "birds", "insects",
    "dripping", "creaking", "rustling",
];
/// What makes a smell.
pub const SMELLS: &[&str] = &[
    "water",
    "salt",
    "smoke",
    "rot",
    "flowers",
    "resin",
    "animal",
    "damp_stone",
    "dust",
    "earth",
    "ash",
];
/// How strongly a sound or smell comes.
pub const STRENGTHS: &[&str] = &["faint", "clear", "strong"];
/// How a surface feels under the hand.
pub const TEXTURES: &[&str] = &[
    "rough",
    "smooth",
    "gritty",
    "grained",
    "chalky",
    "cold_smooth",
    "soft",
    "crumbling",
];
/// What the tongue says.
pub const TASTES: &[&str] = &["fresh_water", "brackish_water", "salt", "dust", "nothing"];
/// How the sun or the night sky stands.
pub const SKIES: &[&str] = &[
    "sun_low_east",
    "sun_high",
    "sun_low_west",
    "stars",
    "overcast_night",
    "grey",
];
/// Ages as the body feels them.
pub const AGES: &[&str] = &["young", "grown", "older", "old", "aged"];
/// How big a thing is.
pub const SIZES: &[&str] = &["small", "middling", "large", "huge"];

fn s_whole(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for (i, main) in STRUCTURES.iter().enumerate() {
        out.push(ctx(&[
            ("settlement", Value::from(["town", "ruins", "none"][i % 3])),
            ("size", Value::Number((i % 4) as i64 + 1)),
            ("abandoned", Value::Bool(i % 3 == 1)),
            ("amount", Value::from(AMOUNTS[2 + i % 6])),
            ("main", Value::from(*main)),
            ("main_amount", Value::from(AMOUNTS[1 + i % 7])),
            (
                "second",
                Value::from(STRUCTURES[(i + 4) % STRUCTURES.len()]),
            ),
            ("second_amount", Value::from(AMOUNTS[1 + i % 5])),
            ("kinds", Value::Number(1 + (i % 5) as i64)),
            ("condition", Value::from(CONDITIONS[i % CONDITIONS.len()])),
            ("biome", Value::from(BIOMES[2 + i % 10])),
        ]));
    }
    out
}

fn s_standout(_: u64) -> Vec<Context> {
    STRUCTURES
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                (
                    "name",
                    Value::from(format!("the {} {k}", CONDITIONS[i % 4])),
                ),
                ("kind", Value::from(*k)),
                ("condition", Value::from(CONDITIONS[i % 4])),
                ("why", Value::from(STANDOUTS[i % STANDOUTS.len()])),
            ])
        })
        .collect()
}

fn s_group(_: u64) -> Vec<Context> {
    STRUCTURES
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(*k)),
                ("amount", Value::from(AMOUNTS[2 + i % 6])),
                ("condition", Value::from(CONDITIONS[i % 4])),
                ("mixed", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_group_name(_: u64) -> Vec<Context> {
    STRUCTURES
        .iter()
        .map(|k| ctx(&[("kind", Value::from(*k))]))
        .collect()
}

fn s_member(_: u64) -> Vec<Context> {
    (0..4)
        .map(|i| {
            ctx(&[
                ("name", Value::from(format!("the {} tomb", CONDITIONS[i]))),
                (
                    "bearing",
                    Value::from(["here", "north", "east", "southwest"][i]),
                ),
                ("kind", Value::from("tomb")),
                ("condition", Value::from(CONDITIONS[i])),
                ("rank", Value::Number(i as i64 + 1)),
                ("more", Value::Number(3 - i as i64)),
            ])
        })
        .collect()
}

fn s_count(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("kind", Value::from("tomb")), ("count", Value::Number(14))]),
        ctx(&[("kind", Value::from("jar")), ("count", Value::Number(2))]),
    ]
}

fn s_thing(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("name", Value::from("a stone stele")),
            ("kind", Value::from("stele")),
        ]),
        ctx(&[
            ("name", Value::from("the well")),
            ("kind", Value::from("well")),
        ]),
    ]
}

fn s_ground(_: u64) -> Vec<Context> {
    BIOMES[2..]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            ctx(&[
                ("biome", Value::from(*b)),
                ("terrain", Value::from(TERRAINS[i % TERRAINS.len()])),
                ("high", Value::Bool(i % 3 == 0)),
            ])
        })
        .collect()
}

fn s_unseen(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("weather", Value::from("fog")),
            ("light", Value::from("daylight")),
        ]),
        ctx(&[
            ("weather", Value::from("clear")),
            ("light", Value::from("dark")),
        ]),
    ]
}

fn s_weather(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for (i, w) in ["clear", "rain", "fog"].iter().enumerate() {
        for (j, l) in LIGHTS.iter().enumerate() {
            out.push(ctx(&[
                ("weather", Value::from(*w)),
                ("light", Value::from(*l)),
                ("time", Value::from(TIMES[(i + j) % TIMES.len()])),
            ]));
        }
    }
    out
}

fn s_felt(_: u64) -> Vec<Context> {
    TEMPERATURES
        .iter()
        .enumerate()
        .map(|(i, t)| {
            ctx(&[
                ("temperature", Value::from(*t)),
                ("indoors", Value::Bool(i % 2 == 0)),
                ("extreme", Value::Bool(matches!(*t, "freezing" | "hot"))),
            ])
        })
        .collect()
}

fn s_moving(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("air", Value::from("windy")),
            ("indoors", Value::Bool(false)),
        ]),
        ctx(&[
            ("air", Value::from("draughty")),
            ("indoors", Value::Bool(true)),
        ]),
    ]
}

fn s_wet(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("wetness", Value::from("wet")),
            ("indoors", Value::Bool(false)),
        ]),
        ctx(&[
            ("wetness", Value::from("flooded")),
            ("indoors", Value::Bool(true)),
        ]),
    ]
}

fn s_indoors(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("indoors", Value::Bool(true))]),
        ctx(&[("indoors", Value::Bool(false))]),
    ]
}

fn s_uncanny(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("kind", Value::from("frost")),
            ("indoors", Value::Bool(false)),
        ]),
        ctx(&[
            ("kind", Value::from("warmth")),
            ("indoors", Value::Bool(true)),
        ]),
    ]
}

fn s_season(_: u64) -> Vec<Context> {
    crate::attention::SEASON_EVIDENCE
        .iter()
        .enumerate()
        .map(|(i, (_, ev, lands))| {
            ctx(&[
                ("evidence", Value::from(*ev)),
                (
                    "biome",
                    Value::from(crate::site::label(&lands[i % lands.len()])),
                ),
                ("time", Value::from(TIMES[i % TIMES.len()])),
            ])
        })
        .collect()
}

fn s_region(_: u64) -> Vec<Context> {
    REGION_EVIDENCE
        .iter()
        .enumerate()
        .map(|(i, (aspect, _, ev))| {
            ctx(&[
                ("evidence", Value::from(*ev)),
                ("aspect", Value::from(*aspect)),
                ("biome", Value::from(BIOMES[2 + i % 10])),
            ])
        })
        .collect()
}

fn s_room_whole(_: u64) -> Vec<Context> {
    PURPOSES
        .iter()
        .enumerate()
        .map(|(i, p)| {
            ctx(&[
                ("purpose", Value::from(*p)),
                ("structure", Value::from(STRUCTURES[i % STRUCTURES.len()])),
                ("condition", Value::from(CONDITIONS[i % CONDITIONS.len()])),
                ("level", Value::Number((i % 3) as i64 - 1)),
                ("light", Value::from(LIGHTS[i % 2])),
                ("time", Value::from(TIMES[i % TIMES.len()])),
            ])
        })
        .collect()
}

fn s_ways(_: u64) -> Vec<Context> {
    vec![
        ctx(&[(
            "ways",
            Value::List(vec!["a door north".into(), "the way out".into()]),
        )]),
        ctx(&[("ways", Value::List(vec!["a stair up".into()]))]),
    ]
}

fn s_room_group(_: u64) -> Vec<Context> {
    ["jar", "shelf", "gravestone", "tablet"]
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(*k)),
                ("amount", Value::from(AMOUNTS[2 + i])),
                ("material", Value::from(MATERIALS[i % MATERIALS.len()])),
            ])
        })
        .collect()
}

fn s_room_thing(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("name", Value::from("a stone altar")),
            ("kind", Value::from("altar")),
            ("item", Value::Bool(false)),
        ]),
        ctx(&[
            ("name", Value::from("a lamp")),
            ("kind", Value::from("lamp")),
            ("item", Value::Bool(true)),
        ]),
    ]
}

fn s_sound(_: u64) -> Vec<Context> {
    SOUNDS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            ctx(&[
                ("source", Value::from(*s)),
                (
                    "name",
                    Value::from(if *s == "creature" {
                        "the scavenger"
                    } else {
                        ""
                    }),
                ),
                ("strength", Value::from(STRENGTHS[i % 3])),
                ("bearing", Value::from(["here", BEARINGS[i % 8]][i % 2])),
                ("indoors", Value::Bool(i % 3 == 0)),
            ])
        })
        .collect()
}

fn s_smell(_: u64) -> Vec<Context> {
    SMELLS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            ctx(&[
                ("source", Value::from(*s)),
                ("strength", Value::from(STRENGTHS[i % 3])),
                ("bearing", Value::from(["here", BEARINGS[i % 8]][i % 2])),
                ("indoors", Value::Bool(i % 3 == 0)),
            ])
        })
        .collect()
}

fn s_touch(_: u64) -> Vec<Context> {
    TEXTURES
        .iter()
        .enumerate()
        .map(|(i, t)| {
            ctx(&[
                ("thing", Value::from("a stone stele")),
                ("material", Value::from(MATERIALS[i % MATERIALS.len()])),
                ("texture", Value::from(*t)),
                (
                    "temperature",
                    Value::from(TEMPERATURES[i % TEMPERATURES.len()]),
                ),
                ("wetness", Value::from(WETNESSES[i % 3])),
                ("marks", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_touch_air(_: u64) -> Vec<Context> {
    TEMPERATURES
        .iter()
        .enumerate()
        .map(|(i, t)| {
            ctx(&[
                ("temperature", Value::from(*t)),
                ("wetness", Value::from(WETNESSES[i % 4])),
                ("air", Value::from(AIRS[i % 3])),
                ("indoors", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_taste(_: u64) -> Vec<Context> {
    TASTES
        .iter()
        .map(|t| ctx(&[("taste", Value::from(*t))]))
        .collect()
}

fn s_sky(_: u64) -> Vec<Context> {
    SKIES
        .iter()
        .enumerate()
        .map(|(i, s)| {
            ctx(&[
                ("sky", Value::from(*s)),
                ("weather", Value::from(["clear", "rain", "fog"][i % 3])),
                ("time", Value::from(TIMES[i % TIMES.len()])),
                ("birds", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_down(_: u64) -> Vec<Context> {
    BIOMES[2..]
        .iter()
        .enumerate()
        .map(|(i, b)| {
            ctx(&[
                ("biome", Value::from(*b)),
                ("terrain", Value::from(TERRAINS[i % TERRAINS.len()])),
                ("wetness", Value::from(WETNESSES[i % 3])),
                ("indoors", Value::Bool(i % 4 == 0)),
            ])
        })
        .collect()
}

fn s_closer(_: u64) -> Vec<Context> {
    ["stele", "jar", "altar", "torch"]
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(*k)),
                ("material", Value::from(MATERIALS[i % MATERIALS.len()])),
                ("condition", Value::from(CONDITIONS[i % 4])),
                ("size", Value::from(SIZES[i % SIZES.len()])),
                ("written", Value::Bool(i % 2 == 0)),
                ("item", Value::Bool(i == 3)),
            ])
        })
        .collect()
}

fn s_felt_need(_: u64) -> Vec<Context> {
    let mut out = Vec::new();
    for (need, states) in crate::slots::need_states() {
        for s in states.iter().skip(1) {
            out.push(ctx(&[
                ("need", Value::from(need)),
                ("state", Value::from(*s)),
            ]));
        }
    }
    out
}

fn s_age(_: u64) -> Vec<Context> {
    AGES.iter()
        .map(|a| ctx(&[("age", Value::from(*a))]))
        .collect()
}

fn s_none(_: u64) -> Vec<Context> {
    vec![Context::new()]
}

/// Every slot of quiet text.
pub fn slots() -> Vec<SlotDef> {
    let ev = season_evidence_ids();
    let mut region: Vec<&str> = REGION_EVIDENCE.iter().map(|r| r.2).collect();
    region.sort_unstable();
    vec![
        // ---------- a place as a whole ----------
        SlotDef::new("place.whole", "A place of several buildings, taken in as one impression: what kind of place it is, never a list ('a ruined town of houses round a temple', 'a crowd of old tombs on the slope'). One short sentence. The arrival and 'look' start here; the player digs down with 'look closer' and 'look at the tombs'. Vague amounts only: one, two, a few, several, many, dozens, a crowd.")
            .var("settlement", e(&["town", "ruins", "none"]), "A living town, the ruins of one, or buildings standing on their own (none).")
            .var("size", VarType::Number, "Settlements: 1 hamlet to 4 city; 0 otherwise.")
            .var("abandoned", VarType::Bool, "Whether the settlement was abandoned.")
            .var("amount", e(&AMOUNTS), "How many buildings in all, roughly.")
            .var("main", e(STRUCTURES), "The commonest kind of building.")
            .var("main_amount", e(&AMOUNTS), "How many of those, roughly.")
            .var("second", e_or_empty(STRUCTURES), "The next commonest kind, or empty.")
            .var("second_amount", e(&AMOUNTS), "How many of those (none when there's no second kind).")
            .var("kinds", VarType::Number, "How many kinds of building there are.")
            .var("condition", e(CONDITIONS), "The state most of them are in.")
            .var("biome", e(BIOMES), "The land they stand in.")
            .max_len(200)
            .sampler(s_whole),
        SlotDef::new("place.standout", "One building that stands out from the rest of the place, named on its own ('Above them all, a worn temple.'). Only for the tallest, the only one of its kind, or one unlike the rest.")
            .var("name", VarType::Text, "Its name, from place.structure.")
            .var("kind", e(STRUCTURES), "What it is.")
            .var("condition", e(CONDITIONS), "Its state.")
            .var("why", e(STANDOUTS), "Why it stands out: tallest, only (of its kind), best_kept or worst_kept (against the rest).")
            .max_len(160)
            .sampler(s_standout),
        SlotDef::new("place.group", "Alike buildings taken together, when looking closer ('several houses, most of them worn'). Vague amounts only.")
            .var("kind", e(STRUCTURES), "What they are.")
            .var("amount", e(&AMOUNTS), "How many, roughly.")
            .var("condition", e(CONDITIONS), "The state most of them are in.")
            .var("mixed", VarType::Bool, "Whether their states differ.")
            .max_len(160)
            .sampler(s_group),
        SlotDef::new("place.group_name", "Alike buildings or things as a group, as the player refers to them ('the tombs').")
            .var("kind", VarType::Text, "What they are (a building or thing kind).")
            .max_len(60)
            .sampler(s_group_name),
        SlotDef::new("place.member", "One building of a group the player is looking at ('look at the tombs'), a few at a time: just this one, by what sets it apart.")
            .var("name", VarType::Text, "Its name, from place.structure.")
            .var("kind", e(STRUCTURES), "What it is.")
            .var("condition", e(CONDITIONS), "Its state.")
            .var("bearing", e(&["here", "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"]), "Which way it lies from the player (here: close by), which tells alike members apart.")
            .var("rank", VarType::Number, "Which of those shown this is (1, 2, 3…).")
            .var("more", VarType::Number, "How many of the group are left unmentioned after this batch (0 when none; only meaningful on the last of a batch).")
            .max_len(160)
            .sampler(s_member),
        SlotDef::new("place.count", "The player counts something ('count the tombs'): the exact number, the one time an exact number is given.")
            .var("kind", VarType::Text, "What was counted (a building or thing kind).")
            .var("count", VarType::Number, "How many.")
            .max_len(120)
            .sampler(s_count),
        SlotDef::new("place.thing", "Something standing in the open here (a well, a stele, a milestone), mentioned on its own.")
            .var("name", VarType::Text, "Its name.")
            .var("kind", VarType::Text, "What it is (a thing or mechanism kind).")
            .max_len(140)
            .sampler(s_thing),
        // ---------- the land ----------
        SlotDef::new("land.ground", "The ground underfoot outdoors, briefly: the land and its lie ('Long grass on a gentle slope.'). Said on arriving somewhere new, rarely after.")
            .var("biome", e(BIOMES), "The land.")
            .var("terrain", e(TERRAINS), "The lie of the ground.")
            .var("high", VarType::Bool, "A high place with a wide view.")
            .max_len(140)
            .sampler(s_ground),
        SlotDef::new("land.unseen", "Little can be seen further off: fog or darkness closes the view.")
            .var("weather", e(&["clear", "rain", "fog"]), "The weather.")
            .var("light", e(LIGHTS), "The light.")
            .max_len(140)
            .sampler(s_unseen),
        SlotDef::new("sky.weather", "The weather and the light, said only when it changes or matters ('Rain sets in.', 'The light is going.').")
            .var("weather", e(&["clear", "rain", "fog"]), "The weather.")
            .var("light", e(LIGHTS), "The light.")
            .var("time", e(TIMES), "Time of day.")
            .max_len(140)
            .sampler(s_weather),
        // ---------- what the body feels ----------
        SlotDef::new("air.felt", "The temperature as felt, in the body or the breath ('Your breath smokes.'), said when it's notable or has changed. Never the word for a number.")
            .var("temperature", e(TEMPERATURES), "How it feels.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .var("extreme", VarType::Bool, "Freezing or hot.")
            .max_len(140)
            .sampler(s_felt),
        SlotDef::new("air.moving", "Wind outdoors or a draught indoors.")
            .var("air", e(&["draughty", "windy"]), "How the air moves.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .max_len(120)
            .sampler(s_moving),
        SlotDef::new("ground.wet", "Wet or flooded ground or floor underfoot.")
            .var("wetness", e(&["wet", "flooded"]), "How wet.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .max_len(120)
            .sampler(s_wet),
        SlotDef::new("danger.unstable", "The stone overhead could fall: always said, it breaks through.").max_len(120).sampler(s_none),
        SlotDef::new("fire.near", "A fire burns here.").var("indoors", VarType::Bool, "Inside a building.").max_len(120).sampler(s_indoors),
        SlotDef::new("air.uncanny", "Warmth or frost where there should be none: the strangeness of the place, felt. Never say why.")
            .var("kind", e(&["warmth", "frost"]), "What is wrong with the air.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .max_len(140)
            .sampler(s_uncanny),
        // ---------- evidence instead of statements ----------
        SlotDef::new("evidence.season", "One sign of the season in this land, shown and never named ('Blossom whitens the thorn.'). Never say the season's name: the player works it out.")
            .var("evidence", e(&ev), "The sign: blossom, meltwater, long days, falling leaves, frost, snow cover…")
            .var("biome", e(BIOMES), "The land it's seen in.")
            .var("time", e(TIMES), "Time of day (birdsong at dawn, frost in the morning).")
            .max_len(160)
            .sampler(s_season),
        SlotDef::new("evidence.region", "One sign of how the land is faring, shown and never stated ('Cracked mud where water stood.'). Never say 'parched' or 'little grows'.")
            .var("evidence", e(&region), "The sign: cracked mud, dry stream beds, sodden ground, standing water, bare earth, withered growth, thick growth, rampant growth, fresh rockfalls, cracked ground, frost out of season, heat haze out of season.")
            .var("aspect", e(&scraped_sim::region::VARIABLES), "Which aspect of the land it shows: life, water, stability (of the ground) or climate.")
            .var("biome", e(BIOMES), "The land it's seen in.")
            .max_len(160)
            .sampler(s_region),
        // ---------- rooms ----------
        SlotDef::new("room.whole", "A room as one impression on entering ('A long hall, dim, the roof half fallen.'). Not its contents; those come as their own facts.")
            .var("purpose", e(PURPOSES), "What the room was for.")
            .var("structure", e(STRUCTURES), "The building it is in.")
            .var("condition", e(CONDITIONS), "The building's condition.")
            .var("level", VarType::Number, "Floor: 0 ground, below 0 underground, above 0 upstairs.")
            .var("light", e(LIGHTS), "How light it is.")
            .var("time", e(TIMES), "Time of day.")
            .max_len(200)
            .sampler(s_room_whole),
        SlotDef::new("room.ways", "The ways out of a room, as one fact ('Ways lead north and up.').")
            .var("ways", VarType::List, "Each way, rendered by place.exit (and place.out at an entrance).")
            .max_len(300)
            .sampler(s_ways),
        SlotDef::new("room.group", "Alike things in a room taken together ('Shelves of clay jars.'). Vague amounts only.")
            .var("kind", e(KINDS), "What they are.")
            .var("amount", e(&AMOUNTS), "How many, roughly.")
            .var("material", e(MATERIALS), "What they're made of.")
            .max_len(140)
            .sampler(s_room_group),
        SlotDef::new("room.thing", "One thing in a room, mentioned on its own because it stands out (big things before small).")
            .var("name", VarType::Text, "Its name.")
            .var("kind", VarType::Text, "What it is.")
            .var("item", VarType::Bool, "Something that could be carried.")
            .max_len(140)
            .sampler(s_room_thing),
        // ---------- the senses ----------
        SlotDef::new("sense.sound", "One thing heard ('listen'): its source, how strong, and from where. Loud or sudden sounds also break into other responses.")
            .var("source", e(SOUNDS), "What makes it.")
            .var("name", VarType::Text, "For a creature, its name; otherwise empty.")
            .var("strength", e(STRENGTHS), "How strongly it comes.")
            .var("bearing", e(&["here", "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"]), "Where from (here: all around, or close).")
            .var("indoors", VarType::Bool, "The listener is inside a building (sounds from outside come muffled).")
            .max_len(140)
            .sampler(s_sound),
        SlotDef::new("sense.silence", "Nothing to hear but the listener's own breath.").var("indoors", VarType::Bool, "Inside a building.").max_len(120).sampler(s_indoors),
        SlotDef::new("sense.smell", "One thing smelt ('smell'): its source, how strong, and from where (smells drift on the wind).")
            .var("source", e(SMELLS), "What gives it.")
            .var("strength", e(STRENGTHS), "How strongly.")
            .var("bearing", e(&["here", "north", "northeast", "east", "southeast", "south", "southwest", "west", "northwest"]), "Where from.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .max_len(140)
            .sampler(s_smell),
        SlotDef::new("sense.no_smell", "Nothing to smell worth noting.").var("indoors", VarType::Bool, "Inside a building.").max_len(120).sampler(s_indoors),
        SlotDef::new("sense.touch", "How a thing feels under the hand ('touch the stele'): texture, cold or warm, damp or dry, and whether marks can be felt in it.")
            .var("thing", VarType::Text, "What was touched.")
            .var("material", e(MATERIALS), "What it's made of.")
            .var("texture", e(TEXTURES), "How it feels.")
            .var("temperature", e(TEMPERATURES), "How cold or warm.")
            .var("wetness", e(WETNESSES), "How damp.")
            .var("marks", VarType::Bool, "Marks are cut or written into it, and can be felt.")
            .max_len(180)
            .sampler(s_touch),
        SlotDef::new("sense.touch_air", "Feeling the air and ground with nothing in particular to touch.")
            .var("temperature", e(TEMPERATURES), "How cold or warm.")
            .var("wetness", e(WETNESSES), "How damp.")
            .var("air", e(AIRS), "How the air moves.")
            .var("indoors", VarType::Bool, "Inside a building.")
            .max_len(160)
            .sampler(s_touch_air),
        SlotDef::new("sense.taste", "Tasting ('taste the water'): use sparingly.").var("taste", e(TASTES), "What the tongue says.").max_len(120).sampler(s_taste),
        SlotDef::new("sense.sky", "Looking up ('look up'): the sky, the sun's height or the stars, clouds, birds. Shows the time of day without saying the hour.")
            .var("sky", e(SKIES), "How the sun or night sky stands: sun_low_east, sun_high, sun_low_west, stars, overcast_night, grey.")
            .var("weather", e(&["clear", "rain", "fog"]), "The weather.")
            .var("time", e(TIMES), "Time of day.")
            .var("birds", VarType::Bool, "Birds are in the sky.")
            .max_len(180)
            .sampler(s_sky),
        SlotDef::new("sense.ground", "Looking down ('look down'): the ground or floor close up.")
            .var("biome", e(BIOMES), "The land (outdoors).")
            .var("terrain", e(TERRAINS), "The lie of the ground.")
            .var("wetness", e(WETNESSES), "How wet.")
            .var("indoors", VarType::Bool, "Inside a building: the floor.")
            .max_len(180)
            .sampler(s_down),
        SlotDef::new("thing.closer", "A second, closer look at a thing (examining it again): what the first glance missed: its size, wear, make.")
            .var("kind", VarType::Text, "What it is.")
            .var("material", e(MATERIALS), "What it's made of.")
            .var("condition", e(CONDITIONS), "The state of the building it is in.")
            .var("size", e(SIZES), "How big.")
            .var("written", VarType::Bool, "Whether there is writing on it.")
            .var("item", VarType::Bool, "Something that could be carried.")
            .max_len(240)
            .sampler(s_closer),
        // ---------- the body ----------
        SlotDef::new("body.felt", "One way the body feels, when the player checks themselves ('check myself'): a sensation, never a level or a number ('Your mouth is dry and your head aches.').")
            .var("need", e(NEEDS), "Which need.")
            .var("state", e(crate::slots::ALL_NEED_STATES), "Its state.")
            .max_len(160)
            .sampler(s_felt_need),
        SlotDef::new("body.well", "Checking themselves, the player finds nothing wrong.").max_len(120).sampler(s_none),
        SlotDef::new("body.age", "How the years feel in the body, when the player checks themselves: never the number.")
            .var("age", e(AGES), "young, grown, older, old or aged.")
            .max_len(140)
            .sampler(s_age),
    ]
}
