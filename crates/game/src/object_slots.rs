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
        SlotDef::new("emblem.describe", "An emblem as it looks: a motif, how it is set, its border ('a twin falcon in a beaded border'). A phrase. The same emblem is always worded the same.")
            .var("motif", e(MOTIFS), "What it shows.")
            .var("device", e(DEVICES), "How the motif is set: single, twin, crowned, in a ring, on a bar, crossed.")
            .var("border", e(BORDERS), "Its border.")
            .max_len(80)
            .sampler(s_emblem),
    ]
}
