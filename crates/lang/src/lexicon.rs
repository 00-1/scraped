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

/// Concept id → root.
#[derive(Debug, Clone, Serialize)]
pub struct Lexicon {
    pub roots: BTreeMap<String, Phonemes>,
}

impl Lexicon {
    /// Root for a concept. Panics if the concept is unknown.
    pub fn root(&self, concept: &str) -> &Phonemes {
        self.roots
            .get(concept)
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
        for affix in morphology.affixes() {
            maker.taken_roots.push(affix.clone());
        }
        maker
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

/// Builds a root for every concept in the starter list.
pub fn generate(rng: &mut Rng, maker: &mut WordMaker) -> Lexicon {
    let mut roots = BTreeMap::new();
    for concept in concepts::all() {
        // Numbers and deictics are short in most languages; content words vary.
        let syllables = match concept.pos {
            Pos::Num | Pos::Det | Pos::Adv => rng.weighted(&[(1, 55), (2, 45)]),
            _ => rng.weighted(&[(1, 25), (2, 55), (3, 20)]),
        };
        let root = maker.make(rng, syllables, concept.pos);
        roots.insert(concept.id.clone(), root);
    }
    Lexicon { roots }
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
