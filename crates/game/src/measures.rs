//! Measures as a body knows them (S06): no hours, minutes or metres in
//! anything the player is told. Time and distance reach the text only as
//! coarse words a person would use.

/// How long something took, as it feels.
// DESIGN-Q: under ten minutes a moment; under half an hour a short while;
// under an hour and a half a while; under four hours a good while; under
// eight half a day; under fourteen most of a day; else more than a day.
pub const HOW_LONG: [&str; 7] = [
    "a moment",
    "a short while",
    "a while",
    "a good while",
    "half a day",
    "most of a day",
    "more than a day",
];

/// How far, as a walker judges it.
// DESIGN-Q: under 20 m a few steps; under 120 m a stone's throw; under
// 700 m a short walk; under 3 km a long walk; under 9 km most of a
// morning's walk; else a day's walk.
pub const HOW_FAR: [&str; 6] = [
    "a few steps",
    "a stone's throw",
    "a short walk",
    "a long walk",
    "most of a morning's walk",
    "a day's walk",
];

pub fn how_long(minutes: u32) -> &'static str {
    let i = match minutes {
        0..=9 => 0,
        10..=29 => 1,
        30..=89 => 2,
        90..=239 => 3,
        240..=479 => 4,
        480..=839 => 5,
        _ => 6,
    };
    HOW_LONG[i]
}

pub fn how_far(metres: f64) -> &'static str {
    let i = if metres < 20.0 {
        0
    } else if metres < 120.0 {
        1
    } else if metres < 700.0 {
        2
    } else if metres < 3000.0 {
        3
    } else if metres < 9000.0 {
        4
    } else {
        5
    };
    HOW_FAR[i]
}
