//! One regular, agglutinative inflection system.
//!
//! Each grammatical category has exactly one affix and unmarked values take
//! none, so a reader comparing forms can peel affixes off one at a time.
//! There are no irregular forms and no sandhi: a word is literally
//! root + affixes, in a fixed order.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::concepts::Pos;
use crate::phonology::{Phonemes, Phonology};
use crate::rng::{Rng, Stream};

/// Grammatical role of a noun phrase, marked on its head noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    /// Unmarked: the doer, or the one who is.
    Subject,
    /// The thing acted on.
    Object,
    /// The owner in "the king's gate", "child of X".
    Genitive,
    /// The beneficiary in "made this for X".
    // DESIGN-Q: the milestone asks for 2–3 cases, but the dedication formula
    // needs "for [person/place]". A dative case is the simplest regular way
    // to express it; an adposition would be the alternative.
    Dative,
}

impl Case {
    pub const ALL: [Case; 4] = [Case::Subject, Case::Object, Case::Genitive, Case::Dative];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Number {
    Singular,
    Plural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tense {
    NonPast,
    Past,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Polarity {
    Positive,
    Negative,
}

/// Whether a word class puts its affixes before or after the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AffixPosition {
    Prefix,
    Suffix,
}

/// A grammatical marker and its Leipzig-style gloss label.
#[derive(Debug, Clone, Serialize)]
pub struct Affix {
    pub gloss: &'static str,
    pub form: Phonemes,
}

/// The affixes of one language and where they go.
#[derive(Debug, Clone, Serialize)]
pub struct Morphology {
    pub noun_position: AffixPosition,
    pub verb_position: AffixPosition,
    pub plural: Affix,
    pub object: Affix,
    pub genitive: Affix,
    pub dative: Affix,
    pub past: Affix,
    pub negative: Affix,
}

/// One piece of a word: a root or an affix, with its gloss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Morph {
    pub form: Phonemes,
    pub gloss: String,
    pub is_root: bool,
}

impl Morphology {
    /// Generates affixes for a language. Affixes are distinct, and every
    /// combination within a paradigm spells out differently.
    pub fn generate(seed: u64, phonology: &Phonology) -> Self {
        let mut rng = Rng::new(seed, Stream::Morphology);
        let noun_position = position(&mut rng);
        let verb_position = position(&mut rng);
        loop {
            let mut make = |gloss| Affix {
                gloss,
                form: phonology.random_affix(&mut rng),
            };
            let m = Morphology {
                noun_position,
                verb_position,
                plural: make("PL"),
                object: make("ACC"),
                genitive: make("GEN"),
                dative: make("DAT"),
                past: make("PST"),
                negative: make("NEG"),
            };
            if m.affixes_are_distinct() {
                return m;
            }
        }
    }

    /// All six affix forms.
    pub fn affixes(&self) -> Vec<&Phonemes> {
        self.affix_list().into_iter().map(|a| &a.form).collect()
    }

    /// All six affixes with glosses, in a fixed order.
    pub fn affix_list(&self) -> [&Affix; 6] {
        [
            &self.plural,
            &self.object,
            &self.genitive,
            &self.dative,
            &self.past,
            &self.negative,
        ]
    }

    fn affixes_are_distinct(&self) -> bool {
        let forms: BTreeSet<&Phonemes> = self.affixes().into_iter().collect();
        if forms.len() != 6 {
            return false;
        }
        // Every inflection of a dummy root must be unique within its class.
        let dummy: Phonemes = vec![];
        let nouns = self.all_forms(&dummy, Pos::Noun);
        let verbs = self.all_forms(&dummy, Pos::Verb);
        nouns.iter().collect::<BTreeSet<_>>().len() == nouns.len()
            && verbs.iter().collect::<BTreeSet<_>>().len() == verbs.len()
    }

    fn case_affix(&self, case: Case) -> Option<&Affix> {
        match case {
            Case::Subject => None,
            Case::Object => Some(&self.object),
            Case::Genitive => Some(&self.genitive),
            Case::Dative => Some(&self.dative),
        }
    }

    /// Inflects a noun. Number sits next to the root, case outside it.
    pub fn noun(&self, root: &Phonemes, gloss: &str, number: Number, case: Case) -> Vec<Morph> {
        let mut inner = Vec::new();
        if number == Number::Plural {
            inner.push(&self.plural);
        }
        inner.extend(self.case_affix(case));
        assemble(root, gloss, &inner, self.noun_position)
    }

    /// Inflects a verb. Tense sits next to the root, negation outside it.
    pub fn verb(
        &self,
        root: &Phonemes,
        gloss: &str,
        tense: Tense,
        polarity: Polarity,
    ) -> Vec<Morph> {
        let mut inner = Vec::new();
        if tense == Tense::Past {
            inner.push(&self.past);
        }
        if polarity == Polarity::Negative {
            inner.push(&self.negative);
        }
        assemble(root, gloss, &inner, self.verb_position)
    }

    /// Every inflected form of a root, as morphs.
    pub fn paradigm(&self, root: &Phonemes, gloss: &str, pos: Pos) -> Vec<Vec<Morph>> {
        let mut out = Vec::new();
        match pos {
            Pos::Noun => {
                for number in [Number::Singular, Number::Plural] {
                    for case in Case::ALL {
                        out.push(self.noun(root, gloss, number, case));
                    }
                }
            }
            Pos::Verb => {
                for tense in [Tense::NonPast, Tense::Past] {
                    for pol in [Polarity::Positive, Polarity::Negative] {
                        out.push(self.verb(root, gloss, tense, pol));
                    }
                }
            }
            _ => out.push(assemble(root, gloss, &[], AffixPosition::Suffix)),
        }
        out
    }

    /// Every surface form a root can take, for collision checks.
    pub fn all_forms(&self, root: &Phonemes, pos: Pos) -> Vec<Phonemes> {
        self.paradigm(root, "", pos)
            .iter()
            .map(|m| join(m))
            .collect()
    }
}

fn position(rng: &mut Rng) -> AffixPosition {
    // Suffixing is far more common in the world's languages.
    rng.weighted(&[(AffixPosition::Suffix, 70), (AffixPosition::Prefix, 30)])
}

/// Places affixes around a root. `inner` lists affixes from the root outward,
/// so prefixing languages mirror suffixing ones.
fn assemble(root: &Phonemes, gloss: &str, inner: &[&Affix], pos: AffixPosition) -> Vec<Morph> {
    let root = Morph {
        form: root.clone(),
        gloss: gloss.to_string(),
        is_root: true,
    };
    let affixes = inner.iter().map(|a| Morph {
        form: a.form.clone(),
        gloss: a.gloss.to_string(),
        is_root: false,
    });
    match pos {
        AffixPosition::Suffix => std::iter::once(root).chain(affixes).collect(),
        AffixPosition::Prefix => {
            let mut v: Vec<Morph> = affixes.collect();
            v.reverse();
            v.push(root);
            v
        }
    }
}

/// Concatenates morphs into one phoneme sequence.
pub fn join(morphs: &[Morph]) -> Phonemes {
    morphs.iter().flat_map(|m| m.form.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affixes_never_collide() {
        for seed in 0..100 {
            let p = Phonology::generate(seed);
            let m = Morphology::generate(seed, &p);
            assert!(m.affixes_are_distinct(), "seed {seed}");
            for a in m.affixes() {
                assert!(p.is_valid(a), "seed {seed}");
            }
        }
    }

    #[test]
    fn suffix_order_is_root_number_case() {
        let p = Phonology::generate(3);
        let mut m = Morphology::generate(3, &p);
        m.noun_position = AffixPosition::Suffix;
        let root = vec![0, p.inventory.vowels().next().unwrap()];
        let glosses: Vec<String> = m
            .noun(&root, "gate", Number::Plural, Case::Object)
            .into_iter()
            .map(|x| x.gloss)
            .collect();
        assert_eq!(glosses, ["gate", "PL", "ACC"]);
        m.noun_position = AffixPosition::Prefix;
        let glosses: Vec<String> = m
            .noun(&root, "gate", Number::Plural, Case::Object)
            .into_iter()
            .map(|x| x.gloss)
            .collect();
        assert_eq!(glosses, ["ACC", "PL", "gate"]);
    }

    #[test]
    fn unmarked_forms_are_bare_roots() {
        let p = Phonology::generate(5);
        let m = Morphology::generate(5, &p);
        let root = vec![0, p.inventory.vowels().next().unwrap()];
        assert_eq!(
            join(&m.noun(&root, "x", Number::Singular, Case::Subject)),
            root
        );
        assert_eq!(
            join(&m.verb(&root, "x", Tense::NonPast, Polarity::Positive)),
            root
        );
    }
}
