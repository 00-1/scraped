//! Roots for every concept, and personal names.
//!
//! Roots must stay apart from each other (edit distance ≥ 2) and no inflected
//! form of one root may equal any inflected form of another. That way every
//! surface word in a corpus has exactly one analysis, which is what makes the
//! language fair to decipher.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::concepts::{self, Pos};
use crate::morphology::Morphology;
use crate::phonology::{Phonemes, Phonology};
use crate::rng::Rng;

/// Minimum phoneme edit distance between any two roots or names.
pub const MIN_ROOT_DISTANCE: usize = 2;

/// Concept id → root, and how derived words and compounds are built from
/// roots (D07).
#[derive(Debug, Clone, Serialize, Default)]
pub struct Lexicon {
    pub roots: BTreeMap<String, Phonemes>,
    /// Derivational affix forms by gloss, and whether they are suffixes.
    #[serde(skip)]
    pub derivations: BTreeMap<String, Phonemes>,
    #[serde(skip)]
    pub suffixing: bool,
    #[serde(skip)]
    pub head_last: bool,
    /// Compounds whose form would collide with another word: left unused.
    #[serde(skip)]
    pub blocked: BTreeSet<String>,
}

impl Lexicon {
    /// A lexicon of roots, deriving and compounding as `m` does.
    pub fn new(roots: BTreeMap<String, Phonemes>, m: &Morphology) -> Self {
        let mut lex = Lexicon {
            roots,
            derivations: m
                .derivations
                .iter()
                .map(|a| (a.gloss.to_string(), a.form.clone()))
                .collect(),
            suffixing: m.noun_position == crate::morphology::AffixPosition::Suffix,
            head_last: m.compound_head_last,
            blocked: BTreeSet::new(),
        };
        lex.block_colliding(m);
        lex
    }

    /// Leaves out compounds that would read as another word.
    fn block_colliding(&mut self, m: &Morphology) {
        let mut taken: BTreeSet<Phonemes> = BTreeSet::new();
        for (id, root) in &self.roots {
            taken.extend(m.all_forms(root, concepts::get(id).pos));
        }
        for c in concepts::all() {
            let Some((a, b)) = c.id.split_once('~') else {
                continue;
            };
            if !self.roots.contains_key(a) || !self.roots.contains_key(b) {
                continue;
            }
            let forms = m.all_forms(&self.root(&c.id), Pos::Noun);
            if forms.iter().any(|f| taken.contains(f)) {
                self.blocked.insert(c.id.clone());
            } else {
                taken.extend(forms);
            }
        }
    }

    /// Whether the language has a word for this concept (number words
    /// depend on the numeral base; derived words and compounds need their
    /// roots).
    pub fn has(&self, concept: &str) -> bool {
        if let Some((base, d)) = concept.split_once('+') {
            return self.roots.contains_key(base)
                && self.derivations.contains_key(&d.to_uppercase());
        }
        if let Some((a, b)) = concept.split_once('~') {
            return self.roots.contains_key(a)
                && self.roots.contains_key(b)
                && !self.blocked.contains(concept);
        }
        self.roots.contains_key(concept)
    }

    /// Root for a concept, derived words and compounds built from their
    /// parts. Panics if the concept is unknown.
    pub fn root(&self, concept: &str) -> Phonemes {
        if let Some((base, d)) = concept.split_once('+') {
            let root = self.root(base);
            let affix = &self.derivations[&d.to_uppercase()];
            return if self.suffixing {
                root.iter().chain(affix).copied().collect()
            } else {
                affix.iter().chain(&root).copied().collect()
            };
        }
        if let Some((a, b)) = concept.split_once('~') {
            let (m, h) = (self.root(a), self.root(b));
            return if self.head_last {
                m.iter().chain(&h).copied().collect()
            } else {
                h.iter().chain(&m).copied().collect()
            };
        }
        self.roots
            .get(concept)
            .cloned()
            .unwrap_or_else(|| panic!("no root for {concept:?}"))
    }
}

/// Generates roots and names while guaranteeing they never collide.
pub struct WordMaker<'a> {
    phonology: &'a Phonology,
    morphology: &'a Morphology,
    taken_roots: Vec<Phonemes>,
    taken_forms: BTreeSet<Phonemes>,
}

impl<'a> WordMaker<'a> {
    pub fn new(phonology: &'a Phonology, morphology: &'a Morphology) -> Self {
        let mut maker = WordMaker {
            phonology,
            morphology,
            taken_roots: Vec::new(),
            taken_forms: BTreeSet::new(),
        };
        // Affixes are words of their own on the page only as part of a word,
        // but a root identical to an affix would still confuse a reader.
        // Fused forms are left out so the Fused dial does not reshuffle the
        // lexicon; exact collisions with them are still caught below.
        for affix in morphology.plain_affixes() {
            maker.taken_roots.push(affix.clone());
        }
        // Eroded affixes are separate words; an empty form is one not yet
        // coined.
        for p in morphology.particles() {
            if !p.form.is_empty() {
                maker.reserve(&p.form, Pos::Particle);
            }
        }
        maker
    }

    /// Takes an existing word (e.g. one derived by sound change) if none of
    /// its forms collide with anything taken. Unlike `make`, no minimum
    /// distance applies: sound change may bring words close together.
    pub fn try_reserve(&mut self, w: &Phonemes, pos: Pos) -> bool {
        let forms = self.morphology.all_forms(w, pos);
        let ok = !w.is_empty()
            && forms
                .iter()
                .all(|f| !self.taken_forms.contains(f) && self.phonology.is_valid(f));
        if ok {
            self.reserve(w, pos);
        }
        ok
    }

    /// Registers an existing root so later words avoid it.
    pub fn reserve(&mut self, root: &Phonemes, pos: Pos) {
        self.taken_forms
            .extend(self.morphology.all_forms(root, pos));
        self.taken_roots.push(root.clone());
    }

    /// A fresh word with roughly `syllables` syllables that collides with
    /// nothing generated so far. Lengthens if a short slot is crowded.
    pub fn make(&mut self, rng: &mut Rng, syllables: u32, pos: Pos) -> Phonemes {
        let mut syllables = syllables.max(1);
        loop {
            for _ in 0..60 {
                let w = self.phonology.random_word(rng, syllables);
                if self.acceptable(&w, pos) {
                    self.reserve(&w, pos);
                    return w;
                }
            }
            syllables += 1;
        }
    }

    fn acceptable(&self, w: &Phonemes, pos: Pos) -> bool {
        self.phonology.is_valid(w)
            && self
                .taken_roots
                .iter()
                .all(|r| edit_distance(r, w) >= MIN_ROOT_DISTANCE)
            && self
                .morphology
                .all_forms(w, pos)
                .iter()
                .all(|f| !self.taken_forms.contains(f) && self.phonology.is_valid(f))
    }
}

/// Typical length of a fresh word: function words and numbers are short in
/// most languages; content words vary.
pub fn syllables_for(rng: &mut Rng, pos: Pos) -> u32 {
    match pos {
        Pos::Particle => 1,
        Pos::Num | Pos::Det | Pos::Adv => rng.weighted(&[(1, 55), (2, 45)]),
        _ => rng.weighted(&[(1, 25), (2, 55), (3, 20)]),
    }
}

/// Builds a root for every concept in the starter list that `include`
/// accepts.
pub fn generate(
    rng: &mut Rng,
    maker: &mut WordMaker,
    include: &dyn Fn(&concepts::Concept) -> bool,
) -> Lexicon {
    let mut roots = BTreeMap::new();
    for concept in concepts::all()
        .iter()
        .filter(|c| !c.is_built() && include(c))
    {
        let syllables = syllables_for(rng, concept.pos);
        let root = maker.make(rng, syllables, concept.pos);
        roots.insert(concept.id.clone(), root);
    }
    Lexicon::new(roots, maker.morphology)
}

/// Levenshtein distance over phonemes.
pub fn edit_distance(a: &[u8], b: &[u8]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &x) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, &y) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(x != y);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_basics() {
        assert_eq!(edit_distance(&[1, 2, 3], &[1, 2, 3]), 0);
        assert_eq!(edit_distance(&[1, 2, 3], &[1, 9, 3]), 1);
        assert_eq!(edit_distance(&[1, 2], &[1, 2, 3]), 1);
        assert_eq!(edit_distance(&[], &[1, 2]), 2);
        assert_eq!(edit_distance(&[1, 2, 3], &[3, 2, 1]), 2);
    }
}

#[cfg(test)]
mod derivation_tests {
    use crate::concepts;
    use crate::morphology::{AffixPosition, DERIVATION_GLOSSES};
    use crate::Language;

    /// Derived words are their base plus the same affix in every era, and
    /// compounds the two roots in the language's order (D07).
    #[test]
    fn derivations_and_compounds_are_regular_in_every_era() {
        for seed in [1u64, 7, 42] {
            for lang in Language::generate(seed).eras() {
                let m = &lang.morphology;
                assert_eq!(m.derivations.len(), DERIVATION_GLOSSES.len());
                let mut seen = 0;
                for c in concepts::all()
                    .iter()
                    .filter(|c| c.is_built() && lang.lexicon.has(&c.id))
                {
                    let got = lang.lexicon.root(&c.id);
                    if let Some((base, d)) = c.id.split_once('+') {
                        let affix = &m
                            .derivations
                            .iter()
                            .find(|a| a.gloss == d.to_uppercase())
                            .unwrap()
                            .form;
                        let b = lang.lexicon.root(base);
                        let want: Vec<u8> = match m.noun_position {
                            AffixPosition::Suffix => b.iter().chain(affix).copied().collect(),
                            AffixPosition::Prefix => affix.iter().chain(&b).copied().collect(),
                        };
                        assert_eq!(got, want, "seed {seed} era {}: {}", lang.era, c.id);
                    } else if let Some((a, h)) = c.id.split_once('~') {
                        assert_eq!(
                            got,
                            m.compound(&lang.lexicon.root(a), &lang.lexicon.root(h))
                        );
                    }
                    seen += 1;
                }
                assert!(seen > 500, "seed {seed}: {seen} built words");
            }
        }
    }
}
