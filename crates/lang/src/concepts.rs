//! The starter list of concepts every language names, loaded from data.
//!
//! Concepts live in `data/concepts.toml` so the list can grow without code
//! changes. The file is embedded at compile time; no filesystem access.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const CONCEPTS_TOML: &str = include_str!("../data/concepts.toml");

/// Broad grouping, used to lay out the grammar sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    People,
    Nature,
    Places,
    Objects,
    Actions,
    Qualities,
    Deixis,
    Numbers,
}

/// Part of speech: decides which inflections and positions a word takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pos {
    Noun,
    Verb,
    Adj,
    Num,
    Det,
    Adv,
}

/// One meaning the language has a root for.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Concept {
    pub id: String,
    pub domain: Domain,
    pub pos: Pos,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objects: Vec<String>,
    #[serde(default)]
    pub applies: Vec<String>,
    #[serde(default)]
    pub value: Option<u8>,
    #[serde(default)]
    pub en_plural: Option<String>,
    #[serde(default)]
    pub en_3sg: Option<String>,
    #[serde(default)]
    pub en_past: Option<String>,
}

impl Concept {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Whether this verb can take `noun` as its object.
    pub fn accepts_object(&self, noun: &Concept) -> bool {
        self.objects.iter().any(|t| noun.has_tag(t))
    }

    /// Whether this adjective sensibly modifies `noun`.
    pub fn applies_to(&self, noun: &Concept) -> bool {
        self.applies.iter().any(|t| noun.has_tag(t))
    }
}

#[derive(Deserialize)]
struct ConceptFile {
    concept: Vec<Concept>,
}

/// The full concept list, parsed once.
pub fn all() -> &'static [Concept] {
    static CONCEPTS: OnceLock<Vec<Concept>> = OnceLock::new();
    CONCEPTS.get_or_init(|| {
        toml::from_str::<ConceptFile>(CONCEPTS_TOML)
            .expect("data/concepts.toml is valid")
            .concept
    })
}

/// Looks up a concept by id. Panics on unknown ids: they are programmer errors.
pub fn get(id: &str) -> &'static Concept {
    all()
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| panic!("unknown concept {id:?}"))
}

/// All concepts with the given part of speech.
pub fn with_pos(pos: Pos) -> impl Iterator<Item = &'static Concept> {
    all().iter().filter(move |c| c.pos == pos)
}

/// All nouns carrying `tag`.
pub fn nouns_tagged(tag: &str) -> Vec<&'static Concept> {
    with_pos(Pos::Noun).filter(|c| c.has_tag(tag)).collect()
}

/// The numeral concept for `n` (1–10).
pub fn numeral(n: u8) -> &'static Concept {
    with_pos(Pos::Num)
        .find(|c| c.value == Some(n))
        .unwrap_or_else(|| panic!("no numeral {n}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn list_loads_with_expected_size() {
        let n = all().len();
        assert!((60..=100).contains(&n), "{n} concepts");
    }

    #[test]
    fn ids_are_unique_and_clean() {
        let ids: BTreeSet<&str> = all().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids.len(), all().len());
        for id in ids {
            assert!(id.chars().all(|c| c.is_ascii_lowercase()), "{id}");
        }
    }

    #[test]
    fn numbers_one_to_ten() {
        for n in 1..=10 {
            assert_eq!(numeral(n).domain, Domain::Numbers);
        }
    }

    #[test]
    fn every_verb_object_tag_is_used_by_some_noun() {
        for v in with_pos(Pos::Verb) {
            for t in &v.objects {
                assert!(!nouns_tagged(t).is_empty(), "{}: {t}", v.id);
            }
        }
        for a in with_pos(Pos::Adj) {
            assert!(with_pos(Pos::Noun).any(|n| a.applies_to(n)), "{}", a.id);
        }
    }
}
