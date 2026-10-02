//! The player's body: warmth, water, food, rest, wetness and injury, felt
//! over hours. Coarse states for descriptions; whole numbers inside so the
//! state replays exactly.

use serde::{Deserialize, Serialize};

use crate::rules::{Effect, Props, Rules};

/// What the player is doing while time passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Resting,
    Walking,
    Sleeping,
}

/// What the surroundings do to the body.
#[derive(Debug, Clone, PartialEq)]
pub struct Exposure {
    pub temperature: f64,
    /// Degrees from clothing worn.
    pub clothing: i32,
    pub fire: bool,
    pub sheltered: bool,
    pub raining: bool,
    pub windy: bool,
    /// Wading or swimming.
    pub in_water: bool,
    pub activity: Activity,
}

/// Everything about the body that changes. Minutes and degree-minutes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Body {
    /// Minutes since a full drink, faster when walking or hot.
    pub thirst: u32,
    pub hunger: u32,
    /// Minutes awake, less what sleep has paid back.
    pub fatigue: u32,
    /// Degree-minutes of cold taken in and not yet warmed away.
    pub chill: u32,
    /// 0 dry to 100 soaked.
    pub wet: u32,
    /// 0 unhurt to 4 dead.
    pub injury: u32,
    /// Minutes towards healing one level of injury.
    pub healing: u32,
    /// Years of age; wounds heal more slowly as it rises.
    #[serde(default)]
    pub age: u32,
}

const HOUR: u32 = 60;

// DESIGN-Q: the pace of needs. Thirsty after 8 hours, dead of thirst after
// 60; hungry after 16, dead after 240; tired after 18 awake; cold below a
// felt 12 degrees, dead after 130 degree-hours of chill; injuries heal a
// level a day.
const COMFORT: f64 = 12.0;
const THIRST: [u32; 4] = [8 * HOUR, 18 * HOUR, 36 * HOUR, 60 * HOUR];
const HUNGER: [u32; 4] = [16 * HOUR, 48 * HOUR, 120 * HOUR, 240 * HOUR];
const FATIGUE: [u32; 3] = [18 * HOUR, 30 * HOUR, 42 * HOUR];
const CHILL: [u32; 4] = [15 * HOUR, 40 * HOUR, 80 * HOUR, 130 * HOUR];
const HEAL: u32 = 24 * HOUR;

/// State ids for content, per need.
pub const WARMTH_STATES: &[&str] = &["warm", "chilled", "shivering", "hypothermic"];
pub const THIRST_STATES: &[&str] = &["fine", "thirsty", "parched", "dying"];
pub const HUNGER_STATES: &[&str] = &["fine", "hungry", "weak", "starving"];
pub const REST_STATES: &[&str] = &["rested", "tired", "exhausted"];
pub const INJURY_STATES: &[&str] = &["unhurt", "bruised", "hurt", "badly_hurt"];
pub const WET_STATES: &[&str] = &["dry", "damp", "soaked"];
/// Ways the body can fail. Ids for content.
pub const DEATHS: &[&str] = &[
    "cold", "thirst", "hunger", "injury", "drowning", "fall", "collapse", "creature", "writing",
];

fn band(x: u32, limits: &[u32]) -> usize {
    limits.iter().take_while(|&&l| x >= l).count()
}

fn scaled(minutes: u32, factor: f64) -> u32 {
    (f64::from(minutes) * factor + 0.5).floor() as u32
}

impl Body {
    /// How warm the body feels, in degrees.
    pub fn felt(&self, e: &Exposure) -> f64 {
        let mut t = e.temperature + f64::from(e.clothing);
        if e.fire {
            t += 12.0;
        }
        if e.sheltered {
            t += 3.0;
        }
        if e.windy && !e.sheltered {
            t -= 3.0;
        }
        t -= f64::from(self.wet) * 6.0 / 100.0;
        if e.in_water {
            t -= 15.0;
        }
        t += match e.activity {
            Activity::Walking => 5.0,
            Activity::Sleeping => -2.0,
            Activity::Resting => 0.0,
        };
        t
    }

    /// Lets time pass. Returns the cause if the body gives out.
    pub fn pass(&mut self, minutes: u32, e: &Exposure) -> Option<&'static str> {
        let walking = e.activity == Activity::Walking;
        let sleeping = e.activity == Activity::Sleeping;
        let felt = self.felt(e);
        // Wetness: rain and water soak; fire and dry air dry, by the rules.
        if e.in_water {
            self.wet = 100;
        } else if e.raining && !e.sheltered {
            self.wet = (self.wet + minutes).min(100);
        } else {
            let p = Props {
                temperature: e.temperature,
                fire: e.fire,
                fuel: if e.fire { 60.0 } else { 0.0 },
                wetness: f64::from(self.wet),
                ..Props::default()
            };
            let dry = Rules::get().rate(&p, Effect::Dry);
            self.wet = self.wet.saturating_sub(scaled(minutes, dry / 60.0));
        }
        // Warmth.
        if felt < COMFORT {
            self.chill += scaled(minutes, COMFORT - felt);
        } else if felt > COMFORT + 4.0 {
            self.chill = self
                .chill
                .saturating_sub(scaled(minutes, (felt - COMFORT - 4.0) * 2.0));
        }
        // Water, food, rest.
        let hot = felt > 28.0;
        self.thirst += scaled(minutes, if walking || hot { 1.5 } else { 1.0 });
        self.hunger += scaled(minutes, if walking { 1.3 } else { 1.0 });
        if sleeping {
            self.fatigue = self.fatigue.saturating_sub(minutes * 3);
        } else {
            self.fatigue += scaled(minutes, if walking { 1.5 } else { 1.0 });
        }
        // Healing, if the body has the means.
        if self.injury > 0 && self.thirst_state() < 2 && self.hunger_state() < 3 {
            // DESIGN-Q: past 30, healing slows by a sixtieth a year (to
            // nothing at 90).
            let slow = self.age.saturating_sub(30).min(59);
            let gained = if sleeping { minutes * 2 } else { minutes };
            self.healing += gained * (60 - slow) / 60;
            if self.healing >= HEAL {
                self.injury -= 1;
                self.healing = 0;
            }
        }
        self.cause()
    }

    /// Why the body has failed, if it has.
    pub fn cause(&self) -> Option<&'static str> {
        if self.chill >= CHILL[3] {
            Some("cold")
        } else if self.thirst >= THIRST[3] {
            Some("thirst")
        } else if self.hunger >= HUNGER[3] {
            Some("hunger")
        } else if self.injury >= 4 {
            Some("injury")
        } else {
            None
        }
    }

    pub fn warmth_state(&self) -> usize {
        band(self.chill, &CHILL[..3])
    }
    pub fn thirst_state(&self) -> usize {
        band(self.thirst, &THIRST[..3])
    }
    pub fn hunger_state(&self) -> usize {
        band(self.hunger, &HUNGER[..3])
    }
    pub fn rest_state(&self) -> usize {
        band(self.fatigue, &FATIGUE[..2])
    }
    pub fn injury_state(&self) -> usize {
        self.injury.min(3) as usize
    }
    pub fn wet_state(&self) -> usize {
        band(self.wet, &[30, 70])
    }

    /// Whether the body can go no further and drops where it stands.
    pub fn collapses(&self) -> bool {
        self.fatigue >= FATIGUE[2]
    }

    /// The coarse state of every need, by id, for content and agents.
    pub fn states(&self) -> [(&'static str, &'static str); 6] {
        [
            ("warmth", WARMTH_STATES[self.warmth_state()]),
            ("thirst", THIRST_STATES[self.thirst_state()]),
            ("hunger", HUNGER_STATES[self.hunger_state()]),
            ("rest", REST_STATES[self.rest_state()]),
            ("injury", INJURY_STATES[self.injury_state()]),
            ("wet", WET_STATES[self.wet_state()]),
        ]
    }

    /// Slows walking when weak, exhausted or hurt (a factor ≥ 1).
    pub fn slowness(&self) -> f64 {
        let mut f = 1.0;
        if self.rest_state() >= 2 {
            f *= 1.4;
        }
        if self.hunger_state() >= 2 {
            f *= 1.2;
        }
        if self.injury >= 2 {
            f *= 1.3;
        }
        f
    }

    pub fn drink(&mut self, minutes_of_thirst: u32) {
        self.thirst = self.thirst.saturating_sub(minutes_of_thirst);
    }

    pub fn eat(&mut self, hours: u32) {
        self.hunger = self.hunger.saturating_sub(hours * HOUR);
    }

    pub fn hurt(&mut self, levels: u32) -> Option<&'static str> {
        self.injury += levels;
        self.healing = 0;
        self.cause()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(t: f64) -> Exposure {
        Exposure {
            temperature: t,
            clothing: 0,
            fire: false,
            sheltered: false,
            raining: false,
            windy: false,
            in_water: false,
            activity: Activity::Resting,
        }
    }

    #[test]
    fn a_cold_night_in_the_open_hurts_and_a_cloak_or_fire_helps() {
        let mut bare = Body::default();
        let mut cloaked = Body::default();
        let mut by_fire = Body::default();
        for _ in 0..10 {
            bare.pass(
                60,
                &Exposure {
                    activity: Activity::Sleeping,
                    ..out(2.0)
                },
            );
            cloaked.pass(
                60,
                &Exposure {
                    clothing: 8,
                    activity: Activity::Sleeping,
                    ..out(2.0)
                },
            );
            by_fire.pass(
                60,
                &Exposure {
                    fire: true,
                    activity: Activity::Sleeping,
                    ..out(2.0)
                },
            );
        }
        assert_eq!(bare.warmth_state(), 3, "{bare:?}");
        assert!(cloaked.warmth_state() <= 2);
        assert_eq!(by_fire.warmth_state(), 0);
        // Freezing water kills fast.
        let mut b = Body::default();
        let mut cause = None;
        for _ in 0..12 {
            cause = cause.or(b.pass(
                60,
                &Exposure {
                    in_water: true,
                    ..out(-2.0)
                },
            ));
        }
        assert_eq!(cause, Some("cold"));
    }

    #[test]
    fn needs_build_over_hours_and_are_met() {
        let mut b = Body::default();
        b.pass(9 * 60, &out(15.0));
        assert_eq!(b.thirst_state(), 1);
        b.drink(24 * 60);
        assert_eq!(b.thirst_state(), 0);
        b.pass(
            10 * 60,
            &Exposure {
                activity: Activity::Walking,
                ..out(15.0)
            },
        );
        assert_eq!(b.hunger_state(), 1);
        b.eat(12);
        assert_eq!(b.hunger_state(), 0);
        assert_eq!(b.rest_state(), 1);
        b.pass(
            8 * 60,
            &Exposure {
                activity: Activity::Sleeping,
                ..out(15.0)
            },
        );
        assert_eq!(b.rest_state(), 0);
        // Thirst kills in days, not hours.
        let mut t = Body::default();
        let mut hours = 0;
        while t.pass(60, &out(15.0)).is_none() {
            hours += 1;
        }
        assert!((50..70).contains(&hours), "{hours}");
    }

    #[test]
    fn rain_soaks_and_fire_dries_and_wounds_heal() {
        let mut b = Body::default();
        b.pass(
            120,
            &Exposure {
                raining: true,
                ..out(10.0)
            },
        );
        assert_eq!(b.wet_state(), 2);
        b.pass(
            120,
            &Exposure {
                fire: true,
                sheltered: true,
                ..out(10.0)
            },
        );
        assert_eq!(b.wet_state(), 0);
        b.hurt(2);
        assert_eq!(b.injury_state(), 2);
        b.pass(
            12 * 60,
            &Exposure {
                activity: Activity::Sleeping,
                fire: true,
                ..out(15.0)
            },
        );
        assert_eq!(b.injury_state(), 1);
        assert_eq!(b.hurt(3), Some("injury"));
    }
}
