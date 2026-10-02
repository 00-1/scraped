//! The interaction rule table: how local properties act on each other.
//!
//! Rules are data (`data/rules.toml`). Each watches a snapshot of one place's
//! properties and, while its conditions hold, yields an effect at a rate per
//! hour. The caller applies effects to its state, so the same table serves
//! the simulation and, in M08, writing (`Rules::trigger`).

use std::sync::OnceLock;

use serde::Deserialize;

/// What a rule does. Ids match `effect` in the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Ice thickens (cm per degree of frost per day).
    Ice,
    /// Ice thins.
    Melt,
    /// Wetness falls (points per hour).
    Dry,
    /// A fire uses fuel (minutes per hour).
    ConsumeFuel,
    /// A fire goes out.
    Extinguish,
    /// Wet material will not catch.
    ResistIgnition,
    TurnWheel,
    StopWheel,
    /// Features can't be seen.
    HideFeatures,
    /// Unstable stone falls.
    Collapse,
    /// Creatures nearby take notice.
    Disturb,
    /// Creatures nearby keep away.
    Deter,
}

/// A snapshot of one place's properties, as rules see them.
#[derive(Debug, Clone, PartialEq)]
pub struct Props {
    pub temperature: f64,
    /// "none", "still" or "flowing".
    pub water: &'static str,
    pub wetness: f64,
    pub fire: bool,
    pub fuel: f64,
    pub flow: f64,
    pub wheel: bool,
    /// 0 dark, 1 dim, 2 daylight.
    pub light: u8,
    /// 0 quiet to 3 very loud.
    pub noise: u8,
    /// 0 crumbling to 100 sound.
    pub stability: f64,
    pub creatures: u32,
}

impl Default for Props {
    fn default() -> Self {
        Props {
            temperature: 10.0,
            water: "none",
            wetness: 0.0,
            fire: false,
            fuel: 0.0,
            flow: 0.0,
            wheel: false,
            light: 2,
            noise: 0,
            stability: 100.0,
            creatures: 0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Operand {
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub id: String,
    when: Vec<(String, String, Operand)>,
    pub effect: Effect,
    pub rate: f64,
}

#[derive(Deserialize)]
struct RuleFile {
    rule: Vec<Rule>,
}

impl Props {
    fn number(&self, name: &str) -> Option<f64> {
        Some(match name {
            "temperature" => self.temperature,
            "wetness" => self.wetness,
            "fire" => f64::from(u8::from(self.fire)),
            "fuel" => self.fuel,
            "flow" => self.flow,
            "wheel" => f64::from(u8::from(self.wheel)),
            "light" => f64::from(self.light),
            "noise" => f64::from(self.noise),
            "stability" => self.stability,
            "creatures" => f64::from(self.creatures),
            _ => return None,
        })
    }

    fn text(&self, name: &str) -> Option<&str> {
        match name {
            "water" => Some(self.water),
            _ => None,
        }
    }
}

impl Rule {
    /// Whether every condition holds for these properties.
    pub fn holds(&self, p: &Props) -> bool {
        self.when.iter().all(|(prop, op, value)| match value {
            Operand::Number(v) => {
                let Some(x) = p.number(prop) else {
                    return false;
                };
                match op.as_str() {
                    "<" => x < *v,
                    ">" => x > *v,
                    "==" => x == *v,
                    "!=" => x != *v,
                    _ => false,
                }
            }
            Operand::Text(v) => {
                let Some(x) = p.text(prop) else { return false };
                match op.as_str() {
                    "==" => x == v,
                    "!=" => x != v,
                    _ => false,
                }
            }
        })
    }
}

/// The rule table.
pub struct Rules {
    pub rules: Vec<Rule>,
}

impl Rules {
    /// The table shipped in `data/rules.toml`.
    pub fn get() -> &'static Rules {
        static RULES: OnceLock<Rules> = OnceLock::new();
        RULES.get_or_init(|| Rules {
            rules: toml::from_str::<RuleFile>(include_str!("../data/rules.toml"))
                .expect("data/rules.toml is valid")
                .rule,
        })
    }

    /// Every effect that applies to a place, with its rate per hour.
    pub fn effects(&self, p: &Props) -> Vec<(Effect, f64)> {
        self.rules
            .iter()
            .filter(|r| r.holds(p))
            .map(|r| (r.effect, r.rate))
            .collect()
    }

    /// The summed rate of one effect for a place (0 if no rule applies).
    pub fn rate(&self, p: &Props, e: Effect) -> f64 {
        self.effects(p)
            .into_iter()
            .filter(|(x, _)| *x == e)
            .map(|(_, r)| r)
            .sum()
    }

    /// Whether any rule yields this effect for a place.
    pub fn yields(&self, p: &Props, e: Effect) -> bool {
        self.effects(p).iter().any(|(x, _)| *x == e)
    }

    /// An effect pushed from outside the table (writing, in M08): the same
    /// verbs, applied directly at a rate.
    pub fn trigger(&self, e: Effect, rate: f64) -> (Effect, f64) {
        (e, rate)
    }
}

/// Ice thickness in cm on water whose last 24 hourly temperatures are given
/// (oldest first), by running the freeze and thaw rules hour by hour.
pub fn ice_cm(water: &'static str, hourly: &[f64]) -> f64 {
    let rules = Rules::get();
    let mut ice = 0.0f64;
    for &t in hourly {
        let p = Props {
            temperature: t,
            water,
            ..Props::default()
        };
        let frost = if t < 0.0 { -t } else { 0.0 };
        let warmth = if t > 0.0 { t } else { 0.0 };
        ice += rules.rate(&p, Effect::Ice) * frost / 24.0;
        ice -= rules.rate(&p, Effect::Melt) * warmth / 24.0;
        if ice < 0.0 {
            ice = 0.0;
        }
    }
    ice
}

/// Ice that bears a person's weight, in cm.
// DESIGN-Q: 8 cm of ice bears the player; thinner ice breaks under them.
pub const ICE_BEARS: f64 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Props {
        Props::default()
    }

    #[test]
    fn table_parses_and_every_rule_can_fire() {
        let r = Rules::get();
        assert!(r.rules.len() >= 14);
        let cases: Vec<(&str, Props)> = vec![
            (
                "freeze_still",
                Props {
                    temperature: -2.0,
                    water: "still",
                    ..p()
                },
            ),
            (
                "freeze_flowing",
                Props {
                    temperature: -5.0,
                    water: "flowing",
                    ..p()
                },
            ),
            (
                "thaw",
                Props {
                    temperature: 4.0,
                    water: "still",
                    ..p()
                },
            ),
            (
                "fire_dries",
                Props {
                    fire: true,
                    fuel: 30.0,
                    ..p()
                },
            ),
            (
                "air_dries",
                Props {
                    wetness: 40.0,
                    ..p()
                },
            ),
            (
                "fire_burns_fuel",
                Props {
                    fire: true,
                    fuel: 30.0,
                    ..p()
                },
            ),
            (
                "fire_starves",
                Props {
                    fire: true,
                    fuel: 0.0,
                    ..p()
                },
            ),
            (
                "wet_wood_resists_fire",
                Props {
                    wetness: 80.0,
                    ..p()
                },
            ),
            ("flow_turns_wheel", Props { flow: 200.0, ..p() }),
            (
                "still_water_stops_wheel",
                Props {
                    flow: 10.0,
                    wheel: true,
                    ..p()
                },
            ),
            ("dark_hides", Props { light: 0, ..p() }),
            (
                "noise_dislodges",
                Props {
                    noise: 3,
                    stability: 20.0,
                    ..p()
                },
            ),
            (
                "noise_disturbs",
                Props {
                    noise: 2,
                    creatures: 1,
                    ..p()
                },
            ),
            (
                "fire_deters",
                Props {
                    fire: true,
                    fuel: 10.0,
                    creatures: 2,
                    ..p()
                },
            ),
        ];
        for rule in &r.rules {
            let (_, props) = cases
                .iter()
                .find(|(id, _)| *id == rule.id)
                .unwrap_or_else(|| panic!("no test case for rule {}", rule.id));
            assert!(rule.holds(props), "{} should hold", rule.id);
        }
    }

    #[test]
    fn interactions() {
        let r = Rules::get();
        // Cold freezes still water but not flowing water at -1.
        assert!(r.yields(
            &Props {
                temperature: -1.0,
                water: "still",
                ..p()
            },
            Effect::Ice
        ));
        assert!(!r.yields(
            &Props {
                temperature: -1.0,
                water: "flowing",
                ..p()
            },
            Effect::Ice
        ));
        // No water, no ice.
        assert!(!r.yields(
            &Props {
                temperature: -10.0,
                ..p()
            },
            Effect::Ice
        ));
        // Wet wood resists fire; dry wood does not.
        assert!(r.yields(
            &Props {
                wetness: 70.0,
                ..p()
            },
            Effect::ResistIgnition
        ));
        assert!(!r.yields(
            &Props {
                wetness: 10.0,
                ..p()
            },
            Effect::ResistIgnition
        ));
        // Noise brings down only unstable stone.
        assert!(!r.yields(
            &Props {
                noise: 3,
                stability: 90.0,
                ..p()
            },
            Effect::Collapse
        ));
        // A quiet dark room hides its features but nothing falls.
        let dark = Props {
            light: 0,
            stability: 10.0,
            ..p()
        };
        assert!(r.yields(&dark, Effect::HideFeatures) && !r.yields(&dark, Effect::Collapse));
        // Flow turns a wheel; slack water stops it.
        assert!(r.yields(&Props { flow: 100.0, ..p() }, Effect::TurnWheel));
        assert!(r.yields(
            &Props {
                flow: 20.0,
                wheel: true,
                ..p()
            },
            Effect::StopWheel
        ));
    }

    #[test]
    fn ice_builds_in_frost_and_melts_in_thaw() {
        let frost = [-6.0; 24];
        let thin = ice_cm("still", &[-1.0; 24]);
        let thick = ice_cm("still", &frost);
        assert!(thick > ICE_BEARS && thin < ICE_BEARS, "{thin} {thick}");
        assert!(ice_cm("flowing", &frost) < thick);
        let mut thaw = frost.to_vec();
        thaw.extend([8.0; 12]);
        assert!(ice_cm("still", &thaw[12..]) < thick);
        assert_eq!(ice_cm("none", &frost), 0.0);
    }
}
