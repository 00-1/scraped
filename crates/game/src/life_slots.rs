//! The slots of living things (D06): signs, homes, plants and animals
//! glimpsed. Every one is said through the attention model.

use scraped_content::{Context, SlotDef, Value, VarType};
use scraped_world::life::{
    calls, colours_all, forms, homes, marks_all, signs, Kingdom, FEET, FRUITS, ROLES, SIZES, STATES,
};

use crate::site::ctx;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

/// What an animal can be doing when seen.
pub const DOINGS: &[&str] = &[
    "grazing",
    "browsing",
    "standing watchful",
    "moving off",
    "watching",
    "slipping away",
    "nosing about",
    "trotting off",
    "sitting up",
    "diving into a hole",
    "singing",
    "calling",
    "flying over",
    "feeding",
    "perched",
    "on the wing",
    "at the flowers",
    "rising",
    "holding in the current",
    "basking",
    "still",
];
pub const COUNTS: &[&str] = &["one", "a pair", "a few", "many"];
const GROUNDS: &[&str] = &["", "mud", "snow", "sand"];
const BEARINGS_HERE: &[&str] = &[
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

fn s_sign(_: u64) -> Vec<Context> {
    let signs = signs();
    signs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            ctx(&[
                ("sign", Value::from(*s)),
                ("foot", Value::from(FEET[i % FEET.len()])),
                ("size", Value::from(SIZES[i % 3])),
                (
                    "ground",
                    Value::from(if *s == "tracks" { "mud" } else { "" }),
                ),
                ("fresh", Value::Bool(i % 2 == 0)),
                ("role", Value::from(ROLES[i % 11])),
            ])
        })
        .collect()
}

fn s_home(_: u64) -> Vec<Context> {
    homes()
        .iter()
        .enumerate()
        .map(|(i, h)| {
            ctx(&[
                ("home", Value::from(*h)),
                ("role", Value::from(ROLES[i % 11])),
                ("size", Value::from(SIZES[i % 3])),
                ("foot", Value::from(FEET[i % FEET.len()])),
                ("occupied", Value::Bool(i % 2 == 1)),
                ("bearing", Value::from(BEARINGS_HERE[i % 9])),
            ])
        })
        .collect()
}

fn s_plant(_: u64) -> Vec<Context> {
    let forms = forms(Kingdom::Plant);
    let colours = colours_all();
    let marks = marks_all();
    forms
        .iter()
        .step_by(5)
        .enumerate()
        .map(|(i, f)| {
            ctx(&[
                ("form", Value::from(*f)),
                ("role", Value::from(ROLES[11 + i % 9])),
                ("state", Value::from(STATES[i % STATES.len()])),
                ("colour", Value::from(colours[i % colours.len()])),
                ("mark", Value::from(marks[i * 7 % marks.len()])),
                ("fruit", Value::from(FRUITS[i % FRUITS.len()])),
            ])
        })
        .collect()
}

fn s_seen(_: u64) -> Vec<Context> {
    let forms = forms(Kingdom::Animal);
    let colours = colours_all();
    let marks = marks_all();
    forms
        .iter()
        .step_by(6)
        .enumerate()
        .map(|(i, f)| {
            ctx(&[
                ("form", Value::from(*f)),
                ("role", Value::from(ROLES[i % 11])),
                ("colour", Value::from(colours[i % colours.len()])),
                ("mark", Value::from(marks[i * 5 % marks.len()])),
                ("size", Value::from(SIZES[i % 3])),
                ("doing", Value::from(DOINGS[i % DOINGS.len()])),
                ("count", Value::from(COUNTS[i % COUNTS.len()])),
                ("bearing", Value::from(BEARINGS_HERE[1 + i % 8])),
                ("distance", Value::from(["near", "short", "middle"][i % 3])),
            ])
        })
        .collect()
}

fn s_none(_: u64) -> Vec<Context> {
    vec![ctx(&[])]
}

fn s_catch(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            ("form", Value::from("trout")),
            ("colour", Value::from("speckled")),
            ("mark", Value::from("spotted")),
            ("water", Value::from("river")),
        ]),
        ctx(&[
            ("form", Value::from("rabbit")),
            ("colour", Value::from("grey")),
            ("mark", Value::from("short ears")),
            ("water", Value::from("lake")),
        ]),
    ]
}

fn s_water(_: u64) -> Vec<Context> {
    WATERS
        .iter()
        .map(|w| ctx(&[("water", Value::from(*w))]))
        .collect()
}

const WATERS: &[&str] = &["river", "stream", "lake", "sea"];

pub fn slots() -> Vec<SlotDef> {
    let animals = forms(Kingdom::Animal);
    vec![
        SlotDef::new("fish.no_water", "The player tries to fish with no water close enough.").max_len(100).sampler(s_none),
        SlotDef::new("fish.caught", "An hour and a half at the water's edge catches a fish (made into a 'fish' item).")
            .var("form", e(&animals), "The fish's body plan: trout, pike, herring…")
            .var("colour", e(&colours_all()), "Its colour.")
            .var("mark", e(&marks_all()), "What sets it apart.")
            .var("water", e(WATERS), "Where it was caught.")
            .max_len(140)
            .sampler(s_catch),
        SlotDef::new("fish.none", "An hour and a half at the water's edge catches nothing.")
            .var("water", e(WATERS), "Where.")
            .max_len(100)
            .sampler(s_water),
        SlotDef::new("snare.set", "The player sets a snare here (it can hold something after some hours; 'snare' again close by checks it).").max_len(120).sampler(s_none),
        SlotDef::new("snare.waiting", "The player checks a snare set too lately to have caught anything.").max_len(100).sampler(s_none),
        SlotDef::new("snare.caught", "A snare has caught a small animal (made into a 'game' item); the snare is set again.")
            .var("form", e(&animals), "The animal's body plan: rabbit, hare, vole…")
            .var("colour", e(&colours_all()), "Its coat.")
            .var("mark", e(&marks_all()), "What sets it apart.")
            .max_len(140)
            .sampler(s_catch),
        SlotDef::new("snare.empty", "A snare has caught nothing; it is set again.").max_len(100).sampler(s_none),
        SlotDef::new("life.sign", "A sign an animal left, found by looking closer or down ('Cloven prints in the mud, fresh.'). The animal itself is not named: the player learns what leaves what.")
            .var("sign", e(&signs()), "What was found: tracks, droppings, browsed bark, burrows, feathers…")
            .var("foot", e(FEET), "For tracks: what made them (hoof, paw, small paw, bird, tiny, claw).")
            .var("size", e(SIZES), "How big the animal that left it.")
            .var("ground", e(GROUNDS), "For tracks: what holds them (mud, snow, sand); empty for other signs.")
            .var("fresh", VarType::Bool, "Left lately (the animal is about).")
            .var("role", e(ROLES), "What kind of animal (grazer, predator, bird…), for wording only: not to be named outright.")
            .max_len(140)
            .sampler(s_sign),
        SlotDef::new("life.home", "A den, nest, warren, roost or wallow close by ('A warren in the bank, the earth fresh at its mouths.').")
            .var("home", e(&homes()), "What kind of home.")
            .var("role", e(ROLES), "What kind of animal keeps it.")
            .var("size", e(SIZES), "How big the animal.")
            .var("foot", e(FEET), "What its tracks are.")
            .var("occupied", VarType::Bool, "The animal is likely inside now (it is not about at this hour).")
            .var("bearing", e(BEARINGS_HERE), "Where (here: right by the player).")
            .max_len(160)
            .sampler(s_home),
        SlotDef::new("life.plant", "A plant noticed, in its season ('Birches, bare.', 'Yellow flowers in the grass.'). The form is a body plan; name it as this world's plant, with a familiar word or a new one.")
            .var("form", e(&forms(Kingdom::Plant)), "The body plan: oak, reed, puffball…")
            .var("role", e(&ROLES[11..]), "What kind of plant.")
            .var("state", e(STATES), "How it is this season.")
            .var("colour", e(&colours_all()), "Its colour (flowers, bark or cap).")
            .var("mark", e(&marks_all()), "What sets it apart.")
            .var("fruit", e(FRUITS), "What it bears.")
            .max_len(140)
            .sampler(s_plant),
        SlotDef::new("life.seen", "An animal glimpsed by someone looking about ('Three dun deer at the wood's edge, grazing.'). Rare for most animals: their signs come first. The form is a body plan; name it as this world's beast, with a familiar word or a new one.")
            .var("form", e(&forms(Kingdom::Animal)), "The body plan: deer, crow, pike…")
            .var("role", e(&ROLES[..11]), "What kind of animal.")
            .var("colour", e(&colours_all()), "Coat, plumage or scales.")
            .var("mark", e(&marks_all()), "What sets it apart.")
            .var("size", e(SIZES), "How big.")
            .var("doing", e(DOINGS), "What it is doing.")
            .var("count", e(COUNTS), "How many.")
            .var("bearing", e(&BEARINGS_HERE[1..]), "Which way.")
            .var("distance", e(&["near", "short", "middle"]), "How far by eye.")
            .max_len(160)
            .sampler(s_seen),
    ]
}

/// Calls, for the sound slot.
pub fn call_values() -> Vec<&'static str> {
    let mut v = vec![""];
    v.extend(calls());
    v
}
