//! The slots of weather and sky (D06).

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::site::ctx;
use crate::skies::{COMING, MARKS, SIDES};
use scraped_world::sky::{BRIGHTNESS, PHASES, PLANET_COLOURS, SHAPES};

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

fn s_eclipse(_: u64) -> Vec<Context> {
    ["sun", "moon"]
        .iter()
        .map(|b| ctx(&[("body", Value::from(*b))]))
        .collect()
}

fn s_moon(_: u64) -> Vec<Context> {
    PHASES
        .iter()
        .enumerate()
        .map(|(i, p)| {
            ctx(&[
                ("phase", Value::from(*p)),
                ("side", Value::from(["east", "south", "west"][i % 3])),
                ("day", Value::Bool(i % 3 == 0)),
            ])
        })
        .collect()
}

fn s_figure(_: u64) -> Vec<Context> {
    SHAPES
        .iter()
        .enumerate()
        .map(|(i, sh)| {
            ctx(&[
                ("shape", Value::from(*sh)),
                ("stars", Value::from(["a few", "several", "many"][i % 3])),
                ("bright", Value::from(BRIGHTNESS[i % 3])),
                ("side", Value::from(SIDES[i % 4])),
                ("still", Value::Bool(i == 0)),
            ])
        })
        .collect()
}

fn s_planet(_: u64) -> Vec<Context> {
    PLANET_COLOURS
        .iter()
        .enumerate()
        .map(|(i, c)| {
            ctx(&[
                ("colour", Value::from(*c)),
                ("in_shape", Value::from(SHAPES[i])),
                ("low", Value::Bool(i == 0)),
            ])
        })
        .collect()
}

fn s_side(_: u64) -> Vec<Context> {
    SIDES
        .iter()
        .map(|s| ctx(&[("side", Value::from(*s))]))
        .collect()
}

fn s_none(_: u64) -> Vec<Context> {
    vec![ctx(&[])]
}

fn s_wonder(_: u64) -> Vec<Context> {
    scraped_world::phenomena::KINDS
        .iter()
        .enumerate()
        .map(|(i, k)| {
            ctx(&[
                ("kind", Value::from(*k)),
                ("bearing", Value::from(BEARINGS_HERE[i % 9])),
                (
                    "distance",
                    Value::from(["near", "short", "middle", "far"][i % 4]),
                ),
                (
                    "heard",
                    Value::Bool(matches!(
                        *k,
                        "booming dunes" | "tidal bore" | "echoing gorge" | "singing arch"
                    )),
                ),
            ])
        })
        .collect()
}

pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("wonder.noticed", "A natural wonder showing now: marsh lights over a bog at night, dunes booming in the wind, a tidal bore roaring up a river mouth, steam from vents, a gorge throwing back every sound, a mirage of water over hot ground, aurora over the cold north, a shore glowing at night, a fogbow, an arch singing in the wind. Natural, but strange: say what is sensed, never the explanation.")
            .var("kind", e(scraped_world::phenomena::KINDS), "Which wonder.")
            .var("bearing", e(BEARINGS_HERE), "Where (here: close by).")
            .var("distance", e(&["near", "short", "middle", "far", "horizon"]), "How far.")
            .var("heard", VarType::Bool, "Heard rather than seen.")
            .max_len(160)
            .sampler(s_wonder),
        SlotDef::new("sky.eclipse", "An eclipse: the sun darkens at midday (noticed outdoors without looking up), or the full moon goes dark red near midnight. They come round on a cycle a patient watcher can learn.")
            .var("body", e(&["sun", "moon"]), "Which is eclipsed.")
            .max_len(160)
            .sampler(s_eclipse),
        SlotDef::new("sky.moon", "The moon, on looking up: its phase and where it stands. By day only a pale moon.")
            .var("phase", e(PHASES), "Its phase: new (not seen), waxing crescent, first quarter, waxing gibbous, full, waning gibbous, last quarter, waning crescent.")
            .var("side", e(&["east", "south", "west"]), "Where it stands: rising in the east, high in the south, low in the west.")
            .var("day", VarType::Bool, "It is day (a pale moon).")
            .max_len(140)
            .sampler(s_moon),
        SlotDef::new("sky.figure", "A figure of stars on a clear night ('A hook of seven stars high in the south.'). One holds the still star, which never moves from the north: the night traveller's guide. The figures were named by the old culture; never say the names here.")
            .var("shape", e(SHAPES), "The shape the stars make.")
            .var("stars", e(&["a few", "several", "many"]), "How many stars, roughly.")
            .var("bright", e(BRIGHTNESS), "How bright.")
            .var("side", e(SIDES), "Where it stands: north (the still star's figure), high in the south (this season's), rising in the east (late at night), setting in the west (evening).")
            .var("still", VarType::Bool, "It holds the still star.")
            .max_len(160)
            .sampler(s_figure),
        SlotDef::new("sky.planet", "A wandering star: a bright point that is not where it was among the figures. The innermost shows only low at dusk or dawn.")
            .var("colour", e(PLANET_COLOURS), "Its colour.")
            .var("in_shape", e(SHAPES), "The figure it stands in now (by the figure's shape).")
            .var("low", VarType::Bool, "It is the low evening or morning star.")
            .max_len(140)
            .sampler(s_planet),
        SlotDef::new("sky.comet", "A comet with its tail, for some nights every year or two.")
            .var("side", e(SIDES), "Where it hangs.")
            .max_len(120)
            .sampler(s_side),
        SlotDef::new("sky.meteors", "A night of falling stars (a shower that comes on the same nights every year).").max_len(120).sampler(s_none),
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
