//! The slots of objects (D05): an object examined, looked at closer, and
//! the emblems people put on what they owned.

use scraped_content::{Context, SlotDef, Value, VarType};
use scraped_world::objects::{stuffs, BORDERS, CONDITIONS, DEVICES, KINDS, MOTIFS};

use crate::site::ctx;

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

/// Object families, as content sees them.
pub const FAMILIES: &[&str] = &[
    "vessel",
    "tool",
    "coin",
    "seal",
    "measure",
    "jewellery",
    "figurine",
    "game",
    "instrument",
    "weapon",
    "armour",
    "clothing",
    "light",
    "box",
    "sky",
    "medical",
    "writing",
    "everyday",
];

fn kinds() -> Vec<&'static str> {
    KINDS.iter().map(|k| k.id).collect()
}

fn s_object(_: u64) -> Vec<Context> {
    KINDS
        .iter()
        .step_by(7)
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(k.id)),
                ("family", Value::from(FAMILIES[i % FAMILIES.len()])),
                ("material", Value::from(k.stuffs[0])),
                ("condition", Value::from(CONDITIONS[i % CONDITIONS.len()])),
                (
                    "emblem",
                    Value::from(if i % 2 == 0 {
                        "a twin falcon in a beaded border"
                    } else {
                        ""
                    }),
                ),
                (
                    "owner",
                    Value::from(["family", "", "temple", "people"][i % 4]),
                ),
                ("left", Value::Bool(i % 3 == 0)),
                (
                    "maker",
                    Value::from(if i % 3 == 1 { "a crowned bee" } else { "" }),
                ),
                ("border", Value::from(BORDERS[i % BORDERS.len()])),
            ])
        })
        .collect()
}

fn s_emblem(_: u64) -> Vec<Context> {
    (0..8)
        .map(|i| {
            ctx(&[
                ("motif", Value::from(MOTIFS[(i * 5) % MOTIFS.len()])),
                ("device", Value::from(DEVICES[i % DEVICES.len()])),
                ("border", Value::from(BORDERS[i % BORDERS.len()])),
            ])
        })
        .collect()
}

fn s_thing(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("thing", Value::from("a bronze casket"))]),
        ctx(&[("thing", Value::from("a door north"))]),
    ]
}

fn s_locked(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("thing", Value::from("a wood coffer")),
        ("emblem", Value::from("a twin falcon in a beaded border")),
    ])]
}

fn s_opened(_: u64) -> Vec<Context> {
    [
        vec![],
        vec!["a bone comb"],
        vec!["a gold ring", "a bronze coin"],
    ]
    .into_iter()
    .map(|c| {
        ctx(&[
            ("thing", Value::from("a wood casket")),
            ("count", Value::Number(c.len() as i64)),
            (
                "contents",
                Value::List(c.into_iter().map(Value::from).collect()),
            ),
        ])
    })
    .collect()
}

fn s_cache(_: u64) -> Vec<Context> {
    ["floor", "wall"]
        .iter()
        .map(|p| {
            ctx(&[
                ("place", Value::from(*p)),
                ("emblem", Value::from("a crowned bee")),
                ("thing", Value::from("a silver coin")),
            ])
        })
        .collect()
}

fn s_dig(_: u64) -> Vec<Context> {
    vec![
        ctx(&[("things", Value::List(vec![Value::from("a gold diadem")]))]),
        ctx(&[("indoors", Value::Bool(false))]),
        ctx(&[("indoors", Value::Bool(true))]),
    ]
}

fn s_map(_: u64) -> Vec<Context> {
    vec![
        ctx(&[
            (
                "places",
                Value::List(vec![
                    Value::from("a town to the north"),
                    Value::from("a town to the east, far"),
                ]),
            ),
            ("cross", Value::Bool(true)),
            ("cross_bearing", Value::from("southwest")),
            ("cross_distance", Value::from("middle")),
            ("cross_by", Value::from("standing stones")),
            ("border", Value::from("beaded")),
        ]),
        ctx(&[
            ("places", Value::List(vec![])),
            ("cross", Value::Bool(false)),
            ("cross_bearing", Value::from("here")),
            ("cross_distance", Value::from("near")),
            ("cross_by", Value::from("")),
            ("border", Value::from("plain")),
        ]),
    ]
}

fn s_place(_: u64) -> Vec<Context> {
    (0..4)
        .map(|i| {
            ctx(&[
                ("what", Value::from("town")),
                (
                    "bearing",
                    Value::from(scraped_sim::outdoors::BEARINGS[i * 2]),
                ),
                (
                    "distance",
                    Value::from(["near", "short", "middle", "far"][i]),
                ),
                ("gone", Value::Bool(i == 2)),
            ])
        })
        .collect()
}

/// Every slot of objects.
pub fn slots() -> Vec<SlotDef> {
    let obj = |s: SlotDef| {
        s.var("kind", e(&kinds()), "What it is.")
            .var("family", e(FAMILIES), "What sort of thing: a vessel, a tool, a coin, a seal, a measure, jewellery, a figurine, a game, an instrument, a weapon, armour, clothing, a light, a box, a thing for the sky, a medical thing, a writing thing, an everyday thing.")
            .var("material", e(&stuffs()), "What it is made of.")
            .var("condition", e(CONDITIONS), "How it has fared.")
            .var("emblem", VarType::Text, "The emblem on it (from emblem.describe), or empty. The same owner always has the same emblem: a player can match things to owners by it.")
            .var("owner", e(&["", "people", "family", "temple", "era"]), "Whose emblem it is (for the writer only; never say it outright): a people, a family line, a temple. Empty when unmarked.")
            .var("left", VarType::Bool, "Whether an event left it here (war, plague, flight): a hint of haste.")
    };
    vec![
        obj(SlotDef::new("object.examine", "The player examines a found object: what it is, what it's made of, how it has fared, and the emblem on it if there is one. One or two sentences; a closer look gives more."))
            .max_len(200)
            .sampler(s_object),
        obj(SlotDef::new("object.closer", "Examining an object again, closer: small details. A maker's mark (another emblem, if 'maker' is set), wear, a repair (condition 'mended'), and the style of its border ('border'), which tells the era it was made in to a player who compares."))
            .var("maker", VarType::Text, "The maker's mark (from emblem.describe), or empty.")
            .var("border", e(BORDERS), "The border style of the era it was made in: the same in every object of that era.")
            .max_len(240)
            .sampler(s_object),
        SlotDef::new("object.not_open", "The player tries to open something that doesn't open or hold anything.")
            .var("thing", VarType::Text, "The thing, as named.")
            .max_len(80)
            .sampler(s_thing),
        SlotDef::new("object.locked", "A box or chest is locked. Its lock carries an emblem: the key, wherever it is, carries the same one. Describe the lock and its emblem.")
            .var("thing", VarType::Text, "The thing, as named.")
            .var("emblem", VarType::Text, "The emblem on the lock (from emblem.describe).")
            .max_len(160)
            .sampler(s_locked),
        SlotDef::new("object.unlocked", "A key the player carries turns in a lock (a box's or a door's).")
            .var("thing", VarType::Text, "What it unlocks, as named.")
            .max_len(100)
            .sampler(s_thing),
        SlotDef::new("door.unlocked", "A key the player carries turns in a door's lock.")
            .var("thing", VarType::Text, "The door, as named.")
            .max_len(100)
            .sampler(s_thing),
        SlotDef::new("object.opened", "The player opens a box, jar or bag: what is inside (each named), or that it is empty.")
            .var("thing", VarType::Text, "The container, as named.")
            .var("count", VarType::Number, "How many things inside.")
            .var("contents", VarType::List, "What is inside, each by thing.name.")
            .max_len(200)
            .sampler(s_opened),
        SlotDef::new("cache.sign", "A sign that something is hidden here: the owner's emblem cut into a floor stone or the wall, as one fact. Low-key; looking closer finds what's there.")
            .var("place", e(&["floor", "wall"]), "Where: a floor stone or the wall.")
            .var("emblem", VarType::Text, "The emblem, from emblem.describe.")
            .max_len(140)
            .sampler(s_cache),
        SlotDef::new("cache.found", "Looking closely, the player finds something hidden under a floor stone or in the wall, marked with an emblem.")
            .var("place", e(&["floor", "wall"]), "Where it was hidden.")
            .var("emblem", VarType::Text, "The emblem marking the spot.")
            .var("thing", VarType::Text, "What was hidden, by thing.name.")
            .max_len(160)
            .sampler(s_cache),
        SlotDef::new("dig.found", "Digging by a feature out on the land, the player turns up something buried.")
            .var("things", VarType::List, "What came up, by thing.name.")
            .max_len(160)
            .sampler(s_dig),
        SlotDef::new("dig.nothing", "The player digs and finds nothing (or tries to dig indoors).")
            .var("indoors", VarType::Bool, "Whether they tried indoors.")
            .max_len(100)
            .sampler(s_dig),
        SlotDef::new("map.read", "An old map, drawn as the land was in its era around the town it was made for: the towns it shows (each from map.place; some may be gone now, and towns founded since are missing), and a cross where something lies. Shows, never explains.")
            .var("places", VarType::List, "The towns it shows, each from map.place.")
            .var("cross", VarType::Bool, "Whether it has a cross.")
            .var("cross_bearing", e(&[&["here"][..], &scraped_sim::outdoors::BEARINGS].concat()), "Which way the cross lies from the map's centre.")
            .var("cross_distance", e(&["near", "short", "middle", "far", "horizon"]), "How far, as the map draws it.")
            .var("cross_by", VarType::Text, "What the cross is drawn by (a feature kind, e.g. standing stones), or empty.")
            .var("border", e(BORDERS), "The border style of its era.")
            .max_len(400)
            .sampler(s_map),
        SlotDef::new("map.place", "One town drawn on an old map: which way and how far from the map's centre. A phrase.")
            .var("what", e(&["town"]), "What is drawn.")
            .var("bearing", e(&[&["here"][..], &scraped_sim::outdoors::BEARINGS].concat()), "Which way.")
            .var("distance", e(&["near", "short", "middle", "far", "horizon"]), "How far.")
            .var("gone", VarType::Bool, "Whether it has since been abandoned (the writer knows; the map doesn't).")
            .max_len(60)
            .sampler(s_place),
        SlotDef::new("emblem.describe", "An emblem as it looks: a motif, how it is set, its border ('a twin falcon in a beaded border'). A phrase. The same emblem is always worded the same.")
            .var("motif", e(MOTIFS), "What it shows.")
            .var("device", e(DEVICES), "How the motif is set: single, twin, crowned, in a ring, on a bar, crossed.")
            .var("border", e(BORDERS), "Its border.")
            .max_len(80)
            .sampler(s_emblem),
    ]
}
