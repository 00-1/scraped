//! The slots of weather and sky (D06).

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::site::ctx;
use crate::skies::{COMING, MARKS};

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

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

fn s_mark(_: u64) -> Vec<Context> {
    MARKS
        .iter()
        .enumerate()
        .map(|(i, m)| {
            ctx(&[
                ("mark", Value::from(*m)),
                ("bearing", Value::from(BEARINGS_HERE[i % 9])),
            ])
        })
        .collect()
}

fn s_coming(_: u64) -> Vec<Context> {
    COMING
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(*k)),
                ("bearing", Value::from(BEARINGS_HERE[1 + i * 2])),
                ("soon", Value::Bool(i % 2 == 0)),
                ("dark", Value::Bool(i == 1)),
            ])
        })
        .collect()
}

pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("weather.mark", "What the weather has done close by, seen on the land: a ford running too deep after rain, a river running high, storm-felled trees across the way, snow lying, a lake drawn back from its shore in a dry summer.")
            .var("mark", e(MARKS), "What it is: flooded ford, high water, fallen trees, snow dusting, snow lying, deep snow (closes high ground), low lake.")
            .var("bearing", e(BEARINGS_HERE), "Where (here: underfoot or all about).")
            .max_len(160)
            .sampler(s_mark),
        SlotDef::new("weather.coming", "Weather on its way, seen on one side of the sky: a bank of cloud, a storm building, fog rolling in. Said quietly unless the player looks around.")
            .var("kind", e(COMING), "What is coming: rain, storm, fog or snow.")
            .var("bearing", e(&BEARINGS_HERE[1..]), "The side of the sky it comes from.")
            .var("soon", VarType::Bool, "Within the hour.")
            .var("dark", VarType::Bool, "It is night (only a storm shows then, by its lightning).")
            .max_len(160)
            .sampler(s_coming),
    ]
}
