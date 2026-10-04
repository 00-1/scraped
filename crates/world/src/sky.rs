//! The sky (D06): stars in figures named in the old language, a still
//! star to steer by, a moon with phases, wandering planets, eclipses that
//! come round on a cycle, comets and showers of falling stars on a
//! schedule.
//!
//! Everything follows a fixed calendar: a year of four 90-day seasons, a
//! moon of 30 days. Given the seed and the minute, the sky is known
//! exactly, so a patient watcher can learn its cycles, and the old
//! culture's calendar and festivals (D05 calendar doors, D08 texts) can
//! follow it.

use serde::Serialize;

use scraped_lang::Language;

/// Days in a moon's cycle, from new to new.
pub const MONTH: u32 = 30;
/// Days in a year: four seasons of 90 days.
pub const YEAR: u32 = 360;

/// The shapes figures make. Ids for content.
pub const SHAPES: &[&str] = &[
    "hook", "line", "ring", "cross", "triangle", "crown", "zigzag", "bow", "pair", "cluster",
    "square", "spiral", "fan", "chain",
];
pub const BRIGHTNESS: &[&str] = &["faint", "clear", "brilliant"];
pub const PLANET_COLOURS: &[&str] = &["red", "white", "yellow", "blue-white", "amber"];
pub const PHASES: &[&str] = &[
    "new",
    "waxing crescent",
    "first quarter",
    "waxing gibbous",
    "full",
    "waning gibbous",
    "last quarter",
    "waning crescent",
];

/// What the figures are named after: things of the old world with words
/// in the language.
// DESIGN-Q: figures are named after a few concrete things the language
// has words for, some with a quality ("the great ox"); D07's cultural
// lexicon can widen this.
const FIGURES: &[&str] = &[
    "sheep", "ox", "goat", "fish", "king", "queen", "priest", "scribe", "smith", "jar", "lamp",
    "gate", "tower", "knife", "bowl", "seal", "tree", "mountain", "river", "well", "altar",
    "statue", "tablet", "house",
];
const QUALITIES: &[&str] = &["great", "small", "old", "white", "red", "holy"];

/// A figure of stars.
#[derive(Debug, Clone, Serialize)]
pub struct Constellation {
    pub id: usize,
    /// What the old culture saw in it: a concept id, with a quality.
    pub figure: &'static str,
    pub quality: Option<&'static str>,
    /// Its name in the oldest language, romanised (spoiler).
    pub name: String,
    pub shape: &'static str,
    pub stars: u8,
    pub bright: &'static str,
    /// The season in which it stands high at midnight.
    pub season: usize,
    /// It holds the still star, which never moves from the north.
    pub pole: bool,
}

/// A wandering star.
#[derive(Debug, Clone, Serialize)]
pub struct Planet {
    pub id: usize,
    pub colour: &'static str,
    /// Days to go once round the figures.
    pub period: u32,
    pub offset: u32,
    /// Never far from the sun: seen only at dusk or dawn.
    pub inner: bool,
    pub name: String,
}

/// The sky of a world.
#[derive(Debug, Clone, Serialize)]
pub struct Sky {
    pub constellations: Vec<Constellation>,
    pub planets: Vec<Planet>,
    /// Day of the first new moon.
    pub moon_offset: u32,
    /// Eclipses come every this many moons, on moon `eclipse_moon` of the
    /// cycle: the sun's at new moon, the moon's at the full moon of the
    /// same cycle.
    pub eclipse_every: u32,
    pub eclipse_moon: u32,
    /// A comet returns every this many days and shows for 20.
    pub comet_every: u32,
    pub comet_offset: u32,
    /// Days of the year when falling stars come, for three nights each.
    pub showers: Vec<u32>,
}

fn mix(parts: &[u64]) -> u64 {
    let mut v: u64 = 0x8a5c_d789_635d_2dff;
    for &p in parts {
        v ^= p;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v ^= v >> 31;
        v = v.wrapping_mul(0x94d0_49bb_1331_11eb);
        v ^= v >> 29;
    }
    v
}

/// The day of the calendar for a minute of play.
pub fn day(minutes: u32) -> u32 {
    minutes / 1440
}

fn hour(minutes: u32) -> u32 {
    minutes / 60 % 24
}

impl Sky {
    /// The sky of a world, named in its oldest language.
    // DESIGN-Q: twelve figures (one holding the still star), three to five
    // planets, eclipses every five to seven moons, a comet every 300–700
    // days, and two showers of falling stars a year.
    pub fn generate(seed: u64, oldest: Option<&Language>) -> Self {
        let h = |k: u64| mix(&[seed, 0x5c1e, k]);
        let name = |concept: &str, quality: Option<&str>| -> String {
            let Some(lang) = oldest else {
                return String::new();
            };
            let noun = lang.romanise(&lang.lexicon.root(concept));
            match quality {
                Some(q) => format!("{noun} {}", lang.romanise(&lang.lexicon.root(q))),
                None => noun,
            }
        };
        let mut constellations = Vec::new();
        let mut used: Vec<&str> = Vec::new();
        for i in 0..12usize {
            let r = h(i as u64);
            let mut figure = FIGURES[(r % FIGURES.len() as u64) as usize];
            let mut k = 1;
            while used.contains(&figure) {
                figure = FIGURES[((r >> 8) as usize + k) % FIGURES.len()];
                k += 1;
            }
            used.push(figure);
            let quality = ((r >> 16) % 3 == 0)
                .then(|| QUALITIES[((r >> 20) % QUALITIES.len() as u64) as usize]);
            constellations.push(Constellation {
                id: i,
                figure,
                quality,
                name: name(figure, quality),
                shape: SHAPES[((r >> 28) % SHAPES.len() as u64) as usize],
                stars: 4 + ((r >> 36) % 9) as u8,
                bright: if i == 0 {
                    "brilliant"
                } else {
                    BRIGHTNESS[((r >> 40) % 3) as usize]
                },
                // The pole figure is always up; the others take a season
                // each, three to a season.
                season: if i == 0 { 0 } else { (i - 1) % 4 },
                pole: i == 0,
            });
        }
        let n_planets = 3 + (h(100) % 3) as usize;
        let planets = (0..n_planets)
            .map(|i| {
                let r = h(200 + i as u64);
                let inner = i == 0;
                Planet {
                    id: i,
                    colour: PLANET_COLOURS[(r % PLANET_COLOURS.len() as u64) as usize],
                    period: if inner {
                        180 + (r >> 8) as u32 % 120
                    } else {
                        300 * (i as u32 + 1) + (r >> 8) as u32 % 400
                    },
                    offset: (r >> 24) as u32 % 1000,
                    inner,
                    name: name(
                        ["fire", "lamp", "seal", "god", "king"][((r >> 40) % 5) as usize],
                        Some(QUALITIES[((r >> 44) % QUALITIES.len() as u64) as usize]),
                    ),
                }
            })
            .collect();
        Sky {
            constellations,
            planets,
            moon_offset: (h(300) % u64::from(MONTH)) as u32,
            eclipse_every: 5 + (h(301) % 3) as u32,
            eclipse_moon: (h(302) % 5) as u32,
            comet_every: 300 + (h(303) % 400) as u32,
            comet_offset: (h(304) % 300) as u32,
            showers: vec![(h(305) % 180) as u32, 180 + (h(306) % 180) as u32],
        }
    }

    /// Days since the last new moon.
    pub fn moon_age(&self, minutes: u32) -> u32 {
        (day(minutes) + MONTH - self.moon_offset % MONTH) % MONTH
    }

    /// Which moon of the count this is (new moon to new moon).
    pub fn lunation(&self, minutes: u32) -> u32 {
        (day(minutes) + MONTH - self.moon_offset % MONTH) / MONTH
    }

    pub fn phase(&self, minutes: u32) -> &'static str {
        match self.moon_age(minutes) {
            0 | 1 | 29 => "new",
            2..=6 => "waxing crescent",
            7 | 8 => "first quarter",
            9..=13 => "waxing gibbous",
            14..=16 => "full",
            17..=21 => "waning gibbous",
            22 | 23 => "last quarter",
            _ => "waning crescent",
        }
    }

    /// Hours since the moon rose, if it is up: it rises with the sun at
    /// new moon and at sunset when full, and stays up twelve hours.
    pub fn moon_up(&self, minutes: u32) -> Option<u32> {
        if self.phase(minutes) == "new" {
            return None;
        }
        let rise = (6 + self.moon_age(minutes) * 24 / MONTH) % 24;
        let since = (hour(minutes) + 24 - rise) % 24;
        (since < 12).then_some(since)
    }

    /// Where the moon stands, if up: "east", "south" or "west".
    pub fn moon_side(&self, minutes: u32) -> Option<&'static str> {
        self.moon_up(minutes).map(|s| match s {
            0..=3 => "east",
            4..=7 => "south",
            _ => "west",
        })
    }

    /// Whether the moon gives light to walk by: near full, up, at night.
    pub fn moonlit(&self, minutes: u32) -> bool {
        matches!(
            self.phase(minutes),
            "full" | "waxing gibbous" | "waning gibbous"
        ) && self.moon_up(minutes).is_some()
    }

    fn eclipse_cycle(&self, minutes: u32) -> bool {
        self.lunation(minutes) % self.eclipse_every == self.eclipse_moon % self.eclipse_every
    }

    /// The sun darkened at midday on a new moon of the eclipse cycle.
    pub fn solar_eclipse(&self, minutes: u32) -> bool {
        self.eclipse_cycle(minutes)
            && self.moon_age(minutes) == 0
            && (11..13).contains(&hour(minutes))
    }

    /// The full moon reddened around midnight in the same cycle.
    pub fn lunar_eclipse(&self, minutes: u32) -> bool {
        self.eclipse_cycle(minutes)
            && self.moon_age(minutes) == 15
            && (hour(minutes) == 23 || hour(minutes) < 2)
    }

    /// The eclipses between two days: (day, "sun" or "moon").
    pub fn eclipses(&self, from: u32, to: u32) -> Vec<(u32, &'static str)> {
        let mut out = Vec::new();
        for d in from..to {
            if self.solar_eclipse(d * 1440 + 12 * 60) {
                out.push((d, "sun"));
            }
            if self.lunar_eclipse(d * 1440 + 23 * 60 + 30) {
                out.push((d, "moon"));
            }
        }
        out
    }

    /// The season of the calendar.
    pub fn season(minutes: u32) -> usize {
        (day(minutes) % YEAR / (YEAR / 4)) as usize
    }

    /// The figures up at night now, with where they stand: the still star
    /// in the north always; this season's figures high in the south; the
    /// next season's rising in the east after midnight; the last season's
    /// setting in the west in the evening.
    pub fn figures_up(&self, minutes: u32) -> Vec<(&Constellation, &'static str)> {
        let s = Self::season(minutes);
        let h = hour(minutes);
        let late = h < 5;
        self.constellations
            .iter()
            .filter_map(|c| {
                if c.pole {
                    Some((c, "north"))
                } else if c.season == s {
                    Some((c, "south"))
                } else if late && c.season == (s + 1) % 4 {
                    Some((c, "east"))
                } else if !late && c.season == (s + 3) % 4 {
                    Some((c, "west"))
                } else {
                    None
                }
            })
            .collect()
    }

    /// The figure a planet stands in now.
    pub fn planet_in(&self, p: &Planet, minutes: u32) -> &Constellation {
        let wheel = (self.constellations.len() - 1) as u32;
        let i = (day(minutes) + p.offset) % p.period * wheel / p.period;
        &self.constellations[1 + i as usize]
    }

    /// Whether an inner planet is the evening star now (else the morning
    /// star).
    pub fn evening_star(&self, p: &Planet, minutes: u32) -> bool {
        (day(minutes) + p.offset) % p.period < p.period / 2
    }

    /// Whether a comet shows: for 20 days of each return.
    pub fn comet(&self, minutes: u32) -> bool {
        (day(minutes) + self.comet_offset) % self.comet_every < 20
    }

    /// Whether falling stars come tonight.
    pub fn shower(&self, minutes: u32) -> bool {
        let d = day(minutes) % YEAR;
        self.showers.iter().any(|&s| d >= s && d < s + 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sky(seed: u64) -> Sky {
        let lang = Language::generate(seed);
        Sky::generate(seed, Some(&lang))
    }

    #[test]
    fn the_moon_goes_through_its_phases_each_month() {
        let s = sky(1);
        let phases: std::collections::BTreeSet<&str> =
            (0..MONTH).map(|d| s.phase(d * 1440)).collect();
        assert_eq!(phases.len(), PHASES.len());
        for d in 0..90 {
            assert_eq!(s.phase(d * 1440), s.phase((d + MONTH) * 1440));
        }
    }

    #[test]
    fn eclipses_are_predictable_from_the_cycle() {
        for seed in [1u64, 2, 3] {
            let s = sky(seed);
            let e = s.eclipses(0, 3 * YEAR);
            let suns: Vec<u32> = e
                .iter()
                .filter(|(_, k)| *k == "sun")
                .map(|(d, _)| *d)
                .collect();
            assert!(suns.len() >= 4, "{e:?}");
            // Every solar eclipse falls on a new moon, one cycle apart.
            for pair in suns.windows(2) {
                assert_eq!(pair[1] - pair[0], s.eclipse_every * MONTH);
            }
            for &d in &suns {
                assert_eq!(s.phase(d * 1440), "new");
            }
            for (d, k) in &e {
                if *k == "moon" {
                    assert_eq!(s.phase(d * 1440 + 23 * 60), "full");
                }
            }
        }
    }

    #[test]
    fn the_still_star_is_always_north_and_figures_change_with_the_seasons() {
        let s = sky(4);
        let night = |d: u32| d * 1440 + 23 * 60;
        for d in [0, 50, 100, 200, 300] {
            let up = s.figures_up(night(d));
            assert!(up.iter().any(|(c, side)| c.pole && *side == "north"));
        }
        let spring: Vec<usize> = s.figures_up(night(10)).iter().map(|(c, _)| c.id).collect();
        let autumn: Vec<usize> = s.figures_up(night(190)).iter().map(|(c, _)| c.id).collect();
        assert_ne!(spring, autumn);
    }

    #[test]
    fn figures_have_names_in_the_language() {
        let s = sky(5);
        assert!(s.constellations.iter().all(|c| !c.name.is_empty()));
        let names: std::collections::BTreeSet<&str> =
            s.constellations.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names.len(), s.constellations.len());
    }
}
