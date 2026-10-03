//! The slots of places with character (D03): natural features, a town's
//! districts and layout, and scenes. Each is one kind of fact for the
//! attention model, with a digging layer (`feature.closer`), so most of it
//! is found by walking and looking closer rather than announced.

use scraped_content::{Context, SlotDef, Value, VarType};
use scraped_sim::outdoors::{BEARINGS, DISTANCES};
use scraped_world::features::KINDS as FEATURES;
use scraped_world::geology::Rock;
use scraped_world::scenes::KINDS as SCENES;
use scraped_world::towns::{TownRole, DISTRICTS, PLANS, REASONS, STREETS};

use crate::site::ctx;
use crate::slots::PURPOSES;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

fn features() -> Vec<&'static str> {
    FEATURES.iter().map(|k| k.id).collect()
}

fn rocks() -> Vec<&'static str> {
    Rock::ALL.iter().map(|r| r.id()).collect()
}

fn roles() -> Vec<&'static str> {
    TownRole::ALL.iter().map(|r| r.id()).collect()
}

fn scenes() -> Vec<&'static str> {
    SCENES.iter().map(|(k, _)| *k).collect()
}

/// Every thing a scene can be made of.
pub fn scene_parts() -> Vec<&'static str> {
    let mut v: Vec<&str> = SCENES.iter().flat_map(|(_, p)| p.iter().copied()).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// What a scene shows happened. Ids for content.
pub const SCENE_CAUSES: &[&str] = &[
    "war",
    "plague",
    "famine",
    "last days",
    "flight",
    "flood",
    "devotion",
    "death",
    "road",
    "old town's dead",
];

/// The groups of feature, for content.
pub const FEATURE_GROUPS: &[&str] = &["water", "rock", "life", "marks"];

fn s_feature(_: u64) -> Vec<Context> {
    FEATURES
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(k.id)),
                ("group", Value::from(scraped_sim::outdoors::label(&k.group))),
                (
                    "bearing",
                    Value::from(if i % 3 == 0 { "here" } else { BEARINGS[i % 8] }),
                ),
                ("distance", Value::from(DISTANCES[i % 3])),
                ("rock", Value::from(Rock::ALL[i % 6].id())),
            ])
        })
        .collect()
}

fn s_feature_name(_: u64) -> Vec<Context> {
    FEATURES
        .iter()
        .map(|k| ctx(&[("kind", Value::from(k.id))]))
        .collect()
}

fn s_feature_closer(_: u64) -> Vec<Context> {
    FEATURES
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(k.id)),
                ("group", Value::from(scraped_sim::outdoors::label(&k.group))),
                ("rock", Value::from(Rock::ALL[i % 6].id())),
                (
                    "inside",
                    Value::Bool(matches!(k.id, "cave mouth" | "sea cave")),
                ),
                ("old", Value::Bool(i % 2 == 0)),
            ])
        })
        .collect()
}

fn s_district(_: u64) -> Vec<Context> {
    DISTRICTS
        .iter()
        .enumerate()
        .map(|(i, d)| {
            ctx(&[
                ("kind", Value::from(*d)),
                ("role", Value::from(TownRole::ALL[i % 8].id())),
                ("street", Value::from(STREETS[i % STREETS.len()])),
                (
                    "ways",
                    Value::List(
                        DISTRICTS
                            .iter()
                            .skip(i + 1)
                            .take(1 + i % 3)
                            .map(|d| Value::from(*d))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect()
}

fn s_district_name(_: u64) -> Vec<Context> {
    DISTRICTS
        .iter()
        .map(|d| ctx(&[("kind", Value::from(*d))]))
        .collect()
}

fn s_layout(_: u64) -> Vec<Context> {
    TownRole::ALL
        .iter()
        .enumerate()
        .map(|(i, r)| {
            ctx(&[
                ("role", Value::from(r.id())),
                ("reason", Value::from(REASONS[i % REASONS.len()])),
                ("plan", Value::from(PLANS[i % PLANS.len()])),
                ("walled", Value::Bool(i % 3 == 0)),
                (
                    "gates",
                    Value::Number(if i % 3 == 0 { 1 + (i % 3) as i64 } else { 0 }),
                ),
                (
                    "districts",
                    Value::List(
                        DISTRICTS
                            .iter()
                            .skip(1 + i % 4)
                            .take(2 + i % 3)
                            .map(|d| Value::from(*d))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect()
}

fn s_scene(_: u64) -> Vec<Context> {
    SCENES
        .iter()
        .enumerate()
        .map(|(i, (k, parts))| {
            ctx(&[
                ("kind", Value::from(*k)),
                (
                    "parts",
                    Value::List(parts.iter().map(|p| Value::from(*p)).collect()),
                ),
                ("cause", Value::from(SCENE_CAUSES[i % SCENE_CAUSES.len()])),
                ("purpose", Value::from(PURPOSES[i % PURPOSES.len()])),
            ])
        })
        .collect()
}

/// Every slot of places with character.
pub fn slots() -> Vec<SlotDef> {
    let parts = scene_parts();
    vec![
        // ---------- natural features ----------
        SlotDef::new("land.feature", "A natural feature or old mark on the land close by or in view, as one fact ('A spring wells up at the foot of the slope.', 'Off to the west, a cairn on the ridge.'). Said when the player comes near it; 'look closer' digs in. Never say why it is there: the player works that out.")
            .var("kind", e(&features()), "What it is.")
            .var("group", e(FEATURE_GROUPS), "What sort of thing: water, rock, life, or old marks people left on the land.")
            .var("bearing", e(&[&["here"][..], &BEARINGS].concat()), "Which way it lies, or here when the player stands at it.")
            .var("distance", e(&DISTANCES), "How far by eye (near when here).")
            .var("rock", e(&rocks()), "The rock of the ground there.")
            .max_len(160)
            .sampler(s_feature),
        SlotDef::new("land.feature_name", "A natural feature or old mark close by as the player refers to it ('the spring', 'the cairn'). Include the kind word.")
            .var("kind", e(&features()), "What it is.")
            .max_len(40)
            .sampler(s_feature_name),
        SlotDef::new("feature.closer", "What looking closer at a natural feature or old mark finds: the second layer of detail, from what it is and the rock it is in. A cave or sea cave has an inside, which the player cannot yet enter. Never explain its cause outright.")
            .var("kind", e(&features()), "What it is.")
            .var("group", e(FEATURE_GROUPS), "Water, rock, life or old marks.")
            .var("rock", e(&rocks()), "The rock there: granite, slate, limestone, sandstone, basalt or clay.")
            .var("inside", VarType::Bool, "Whether it opens into a space beyond (a cave).")
            .var("old", VarType::Bool, "For old marks: whether they date from the first people (the oldest era).")
            .max_len(220)
            .sampler(s_feature_closer),
        // ---------- towns ----------
        SlotDef::new("place.district", "The part of a town the player stands in, as one fact ('Here the workshops crowd a narrow lane.'). Said on arriving in it; the streets lead to the others.")
            .var("kind", e(&DISTRICTS), "Which part of town: the square at its heart, a temple quarter, the market, the harbour, workshops, the palace quarter, a garrison, gardens, the graves, the mines, farmyards, a scholars' quarter, a sacred way, the upper town, or the old town.")
            .var("role", e(&roles()), "What the town was for.")
            .var("street", e(&STREETS), "The kind of street that leads here from the square (main street for the square itself).")
            .var("ways", VarType::List, "The other districts a street leads to from here, as district kinds.")
            .max_len(180)
            .sampler(s_district),
        SlotDef::new("place.district_name", "A part of a town as the player refers to it and goes to it ('the market', 'the harbour'). Include the district's kind word.")
            .var("kind", e(&DISTRICTS), "Which part of town.")
            .max_len(40)
            .sampler(s_district_name),
        SlotDef::new("place.layout", "How a town lies, for 'look around' in it: its shape and why (strung along the road, ringed round a hilltop, crowded on a harbour), its walls and gates, and which parts it has. One or two short sentences; the parts are for going to.")
            .var("role", e(&roles()), "What the town was for: capital, port, holy city, mining camp, fortress, market town, farming village, refuge.")
            .var("reason", e(&REASONS), "Why it has its shape: a river crossing, a harbour, a hilltop, a crossroads, a valley floor, a spring, or the open plain.")
            .var("plan", e(&PLANS), "How its streets run.")
            .var("walled", VarType::Bool, "Whether walls ring it.")
            .var("gates", VarType::Number, "Gates in the walls (0 when unwalled).")
            .var("districts", VarType::List, "Its parts other than the square, as district kinds.")
            .max_len(260)
            .sampler(s_layout),
        // ---------- scenes ----------
        SlotDef::new("place.scene", "A scene: a small arrangement of things that shows what happened here, before any writing explains it (a barricade from the inside, a meal left on the table, scorch marks and arrowheads). Show the things; never say what happened. Usually found by looking closer.")
            .var("kind", e(&scenes()), "Which scene.")
            .var("parts", VarType::List, &format!("The things that make it, as ids: {}.", parts.join(", ")))
            .var("cause", e(SCENE_CAUSES), "What it shows (for choosing words; never to be said outright).")
            .var("purpose", VarType::Text, "Indoors: the room's purpose; outdoors: empty.")
            .max_len(220)
            .sampler(s_scene),
    ]
}
