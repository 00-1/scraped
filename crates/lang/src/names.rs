//! Names built from words (D07): people named after qualities, animals and
//! things of their world, places named for what stands there.
//!
//! As in many real cultures, a name is ordinary words put together, so a
//! reader who has learnt "red" and "ford" can read "Red-Ford" off a
//! milestone, and a player who walked to the red ford has an anchor for
//! both. Each language has its own way of building personal names
//! (`NameStyle`). Every name keeps its meaning, so a place name can be
//! said again in a later era with that era's sound changes.

use serde::Serialize;

use crate::concepts::{self, Pos};
use crate::lexicon::WordMaker;
use crate::phonology::Phonemes;
use crate::rng::{Rng, Stream};
use crate::Language;

/// How a people build personal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NameStyle {
    /// Two words: a quality or a thing, then a beast or a thing ("Bright-Wolf").
    Compound,
    /// One word with a derivation ("Little-Hawk", "Oak-like").
    Derived,
    /// A mix: two words for some, one word with a derivation for others.
    Mixed,
}

impl NameStyle {
    // DESIGN-Q: naming styles and how often each is drawn.
    pub fn of(seed: u64) -> Self {
        let mut rng = Rng::new(seed, Stream::Names);
        rng.weighted(&[
            (NameStyle::Compound, 45),
            (NameStyle::Derived, 20),
            (NameStyle::Mixed, 35),
        ])
    }
}

/// A name and the words it is made of: concept ids, the derivation (if
/// any) marked as `base+gloss`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Name {
    pub form: Phonemes,
    pub meaning: Vec<String>,
}

/// Words fit to stand first in a personal name: qualities and things.
const FIRST: &[&str] = &[
    "great", "small", "old", "new", "holy", "dark", "good", "white", "red", "cold", "deep",
    "bright", "strong", "wise", "true", "sweet", "golden", "brave", "quiet", "sun", "moon", "star",
    "stone", "fire", "river", "sea", "mountain", "rain", "storm", "dawn", "iron", "gold", "silver",
    "oak",
];

/// Words fit to stand last in a personal name: beasts and things.
const LAST: &[&str] = &[
    "wolf", "bear", "hawk", "deer", "ox", "goat", "sheep", "fish", "raven", "crow", "owl", "fox",
    "lynx", "boar", "elk", "heart", "hand", "eye", "stone", "fire", "star", "tree", "oak", "river",
    "gate", "tower", "song", "flower", "leaf", "horn", "wing", "shield", "spear", "light",
];

/// Derivations used in one-word names.
const NAME_DERIVATIONS: &[&str] = &["dim", "adjz", "agt"];

impl Language {
    /// The root a name element stands for: a word or a derived word.
    fn element(&self, id: &str) -> Phonemes {
        self.lexicon.root(id)
    }

    fn named_from(&self, parts: Vec<String>) -> Name {
        let m = &self.morphology;
        let form = match parts.as_slice() {
            [one] => self.element(one),
            [a, b] => m.compound(&self.element(a), &self.element(b)),
            _ => Vec::new(),
        };
        Name {
            form,
            meaning: parts,
        }
    }

    /// The same name said in this era: its words, with this era's sounds
    /// (names of places live on and change with the language).
    pub fn say_name(&self, meaning: &[String]) -> Option<Phonemes> {
        if meaning.is_empty() {
            return None;
        }
        meaning
            .iter()
            .all(|p| self.lexicon.has(p))
            .then(|| self.named_from(meaning.to_vec()).form)
    }

    /// A word of an earlier era of this language, as it sounds now: the
    /// sound changes since `from` applied in order (D08: a coined name met
    /// in a later text). `None` if `from` is later than this era.
    pub fn carry(&self, from: &Language, w: &Phonemes) -> Option<Phonemes> {
        if from.era > self.era {
            return None;
        }
        let ipa = self
            .history
            .iter()
            .filter(|s| s.era > from.era && s.era <= self.era)
            .fold(from.phonology.to_ipa(w), |ipa, step| step.apply(&ipa));
        self.phonology.from_ipa(&ipa)
    }

    /// A personal name in this language's style, distinct from every word
    /// and name `maker` has taken. Falls back to a coined word when the
    /// language lacks the words or every pairing tried is taken.
    pub fn person_name(&self, rng: &mut Rng, maker: &mut WordMaker) -> Name {
        let style = NameStyle::of(self.seed);
        let has = |w: &&str| self.lexicon.has(w);
        let first: Vec<&str> = FIRST.iter().copied().filter(has).collect();
        let last: Vec<&str> = LAST.iter().copied().filter(has).collect();
        for _ in 0..40 {
            let two = match style {
                NameStyle::Compound => true,
                NameStyle::Derived => false,
                NameStyle::Mixed => rng.chance(60),
            };
            let parts: Vec<String> = if two && !first.is_empty() && !last.is_empty() {
                let a = *rng.pick(&first);
                let b = *rng.pick(&last);
                if a == b {
                    continue;
                }
                vec![a.to_string(), b.to_string()]
            } else if !last.is_empty() {
                let b = *rng.pick(&last);
                let d = *rng.pick(NAME_DERIVATIONS);
                let id = format!("{b}+{d}");
                if !self.lexicon.has(&id) || concepts::get(b).pos != Pos::Noun {
                    continue;
                }
                vec![id]
            } else {
                break;
            };
            let name = self.named_from(parts);
            if maker.try_reserve(&name.form, Pos::Noun) {
                return name;
            }
        }
        let syllables = rng.weighted(&[(2, 60), (3, 40)]);
        Name {
            form: maker.make(rng, syllables, Pos::Noun),
            meaning: Vec::new(),
        }
    }

    /// A place name: a head word for what the place is or stands by
    /// (`heads`, best first) with a modifier from `modifiers` (its look,
    /// its life, its founder's world), distinct from every word taken.
    pub fn place_name(
        &self,
        rng: &mut Rng,
        maker: &mut WordMaker,
        heads: &[&str],
        modifiers: &[&str],
    ) -> Name {
        let heads: Vec<&str> = heads
            .iter()
            .copied()
            .filter(|h| self.lexicon.has(h))
            .collect();
        let mods: Vec<&str> = modifiers
            .iter()
            .copied()
            .filter(|m| self.lexicon.has(m))
            .collect();
        if !heads.is_empty() && !mods.is_empty() {
            for k in 0..30 {
                // Mostly the best-fitting head, sometimes the next.
                let head = heads[if k < 10 { 0 } else { rng.index(heads.len()) }];
                let m = *rng.pick(&mods);
                if m == head {
                    continue;
                }
                let name = self.named_from(vec![m.to_string(), head.to_string()]);
                if maker.try_reserve(&name.form, Pos::Noun) {
                    return name;
                }
            }
        }
        Name {
            form: maker.make(rng, 2, Pos::Noun),
            meaning: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_words_put_together() {
        for seed in [1u64, 2, 3, 42] {
            let lang = Language::generate(seed);
            let mut maker = lang.word_maker();
            let mut rng = Rng::new(seed, Stream::Names);
            let mut meaningful = 0;
            for _ in 0..30 {
                let n = lang.person_name(&mut rng, &mut maker);
                if !n.meaning.is_empty() {
                    meaningful += 1;
                    assert_eq!(lang.say_name(&n.meaning), Some(n.form.clone()));
                }
            }
            assert!(meaningful >= 25, "seed {seed}: {meaningful}");
            let p = lang.place_name(
                &mut rng,
                &mut maker,
                &["ford", "river"],
                &["red", "white", "old"],
            );
            assert_eq!(p.meaning.len(), 2, "seed {seed}");
            assert!(["ford", "river"].contains(&p.meaning[1].as_str()));
        }
    }

    #[test]
    fn place_names_change_with_the_language() {
        let lang = Language::generate(5);
        let eras = lang.eras();
        let meaning = vec!["red".to_string(), "ford".to_string()];
        let forms: Vec<Phonemes> = eras.iter().filter_map(|l| l.say_name(&meaning)).collect();
        assert_eq!(forms.len(), eras.len());
        assert!(
            forms.windows(2).any(|w| w[0] != w[1]),
            "sound change touches names"
        );
    }
}
