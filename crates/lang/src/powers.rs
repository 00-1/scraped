//! What writing can do (D09): the power of each concept, from the
//! principles in `data/powers.toml`.
//!
//! A concept's power follows from its meaning (water wets, a seal binds,
//! an eye reveals), so a player can guess a word's power from what it
//! means. Where a meaning suggests more than one power, each world's
//! culture settles on one, so worlds differ but stay logical.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::concepts::{self, Concept};

/// A quality of the world that writing can push one way or the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Openness,
    Heat,
    Stability,
    Light,
    Wetness,
    Flow,
    Sound,
    Growth,
    Lure,
    Calm,
    Weight,
    Visibility,
    Binding,
    Keeping,
    Rising,
}

impl Quality {
    pub const ALL: [Quality; 15] = [
        Quality::Openness,
        Quality::Heat,
        Quality::Stability,
        Quality::Light,
        Quality::Wetness,
        Quality::Flow,
        Quality::Sound,
        Quality::Growth,
        Quality::Lure,
        Quality::Calm,
        Quality::Weight,
        Quality::Visibility,
        Quality::Binding,
        Quality::Keeping,
        Quality::Rising,
    ];

    /// Its name, as in the principles file and in slot variables.
    pub fn name(self) -> &'static str {
        match self {
            Quality::Openness => "openness",
            Quality::Heat => "heat",
            Quality::Stability => "stability",
            Quality::Light => "light",
            Quality::Wetness => "wetness",
            Quality::Flow => "flow",
            Quality::Sound => "sound",
            Quality::Growth => "growth",
            Quality::Lure => "lure",
            Quality::Calm => "calm",
            Quality::Weight => "weight",
            Quality::Visibility => "visibility",
            Quality::Binding => "binding",
            Quality::Keeping => "keeping",
            Quality::Rising => "rising",
        }
    }

    pub fn from_name(s: &str) -> Option<Quality> {
        Quality::ALL.into_iter().find(|q| q.name() == s)
    }
}

/// A power: a quality and which way it pushes (+1 or -1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Power {
    pub quality: Quality,
    pub sign: i8,
}

impl Power {
    fn parse(s: &str) -> Option<Power> {
        let (name, sign) = match s.strip_suffix('+') {
            Some(n) => (n, 1),
            None => (s.strip_suffix('-')?, -1),
        };
        Some(Power {
            quality: Quality::from_name(name)?,
            sign,
        })
    }

    pub fn reversed(self) -> Power {
        Power {
            sign: -self.sign,
            ..self
        }
    }
}

#[derive(Deserialize)]
struct Rule {
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    fields: Vec<String>,
    #[serde(default)]
    powers: Vec<String>,
    #[serde(default)]
    carrier: Option<String>,
}

#[derive(Deserialize)]
struct RuleFile {
    rule: Vec<Rule>,
}

fn rules() -> &'static [Rule] {
    static R: OnceLock<Vec<Rule>> = OnceLock::new();
    R.get_or_init(|| {
        toml::from_str::<RuleFile>(include_str!("../data/powers.toml"))
            .expect("data/powers.toml is valid")
            .rule
    })
}

/// Every power a concept's meaning suggests, most fitting first.
pub fn offers(c: &Concept) -> Vec<Power> {
    let mut out: Vec<Power> = Vec::new();
    let mut add = |r: &Rule| {
        for p in r.powers.iter().filter_map(|p| Power::parse(p)) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
    };
    for r in rules().iter().filter(|r| r.ids.contains(&c.id)) {
        add(r);
    }
    for r in rules()
        .iter()
        .filter(|r| r.tags.iter().any(|t| c.has_tag(t)))
    {
        add(r);
    }
    for r in rules().iter().filter(|r| r.fields.contains(&c.field)) {
        add(r);
    }
    out
}

/// A verb that passes on its object's power: +1 gives it ("bring"), -1
/// takes it away ("take").
pub fn carrier(verb: &str) -> Option<i8> {
    rules()
        .iter()
        .find(|r| r.ids.iter().any(|i| i == verb))
        .and_then(|r| match r.carrier.as_deref() {
            Some("+") => Some(1),
            Some("-") => Some(-1),
            _ => None,
        })
}

/// One world's powers: which of each concept's offers its culture holds to.
#[derive(Debug, Clone, Copy)]
pub struct Powers {
    seed: u64,
}

// DESIGN-Q: a culture holds to a concept's most fitting power two times in
// three, else one of the others, chosen by the world's seed and the concept.
impl Powers {
    pub fn new(seed: u64) -> Self {
        Powers { seed }
    }

    /// The power of a concept in this world, if it has one.
    pub fn of(&self, id: &str) -> Option<Power> {
        let c = concepts::find(id)?;
        let offers = offers(c);
        let first = *offers.first()?;
        let h = id.bytes().fold(self.seed ^ 0x70_7e5, |a, b| {
            a.wrapping_mul(0x100_0000_01b3).wrapping_add(u64::from(b))
        });
        let h = h ^ (h >> 29);
        if offers.len() == 1 || h % 3 != 0 {
            return Some(first);
        }
        Some(offers[1 + ((h >> 8) % (offers.len() as u64 - 1)) as usize])
    }
}

/// How many distinct qualities the principles can reach.
pub fn qualities() -> usize {
    let mut seen: Vec<Quality> = Vec::new();
    for r in rules() {
        for p in r.powers.iter().filter_map(|p| Power::parse(p)) {
            if !seen.contains(&p.quality) {
                seen.push(p.quality);
            }
        }
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_names_real_concepts_and_powers() {
        for r in rules() {
            for p in &r.powers {
                assert!(Power::parse(p).is_some(), "bad power {p}");
            }
            for i in &r.ids {
                assert!(concepts::find(i).is_some(), "no concept {i}");
            }
            assert!(!r.powers.is_empty() || r.carrier.is_some());
        }
        assert_eq!(qualities(), Quality::ALL.len());
    }

    #[test]
    fn powers_follow_meaning() {
        let p = Powers::new(1);
        let open = p.of("open").unwrap();
        assert_eq!((open.quality, open.sign), (Quality::Openness, 1));
        assert_eq!(p.of("shine").unwrap().quality, Quality::Light);
        assert!(p.of("pray").is_none());
        assert_eq!(carrier("bring"), Some(1));
        // A concept's power is always one of its offers, in every world.
        for seed in 0..20 {
            let p = Powers::new(seed);
            for c in concepts::all() {
                if let Some(pw) = p.of(&c.id) {
                    assert!(offers(c).contains(&pw), "{}", c.id);
                }
            }
        }
    }

    #[test]
    fn many_concepts_have_powers() {
        let p = Powers::new(42);
        let n = concepts::all()
            .iter()
            .filter(|c| p.of(&c.id).is_some())
            .count();
        assert!(n >= 200, "{n}");
    }
}
