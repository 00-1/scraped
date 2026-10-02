//! Meaning → words. Deterministic: the same meaning always renders the same.

use serde::Serialize;

use crate::concepts;
use crate::meaning::{Clause, Head, Mood, NounPhrase, Role, Sentence};
use crate::morphology::{Case, Morph};
use crate::phonology::Phonemes;
use crate::syntax::{Side, WordOrder};
use crate::Language;

/// One rendered word, kept as morphs so it can be glossed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    pub morphs: Vec<Morph>,
}

impl Word {
    pub fn phonemes(&self) -> Phonemes {
        crate::morphology::join(&self.morphs)
    }

    /// Leipzig-style gloss, e.g. `gate-PL-ACC`.
    pub fn gloss(&self) -> String {
        gloss_of(&self.morphs)
    }
}

/// Joins morph glosses with hyphens.
pub fn gloss_of(morphs: &[Morph]) -> String {
    morphs
        .iter()
        .map(|m| m.gloss.as_str())
        .collect::<Vec<_>>()
        .join("-")
}

/// A rendered sentence.
#[derive(Debug, Clone, Serialize)]
pub struct Rendered {
    pub words: Vec<Word>,
}

/// Renders meanings in one language, with a table of personal names.
pub struct Renderer<'a> {
    pub lang: &'a Language,
    pub names: &'a [Phonemes],
}

impl Renderer<'_> {
    pub fn render(&self, s: &Sentence) -> Rendered {
        let words = match s {
            Sentence::Clause(c) => self.clause(c),
            Sentence::List(items) => items
                .iter()
                .flat_map(|np| self.noun_phrase(np, Case::Subject))
                .collect(),
        };
        Rendered { words }
    }

    /// Romanised text, words separated by spaces.
    // DESIGN-Q: words are space-separated and names are not marked (no
    // capitals, no determinative). Real inscriptions often run words together
    // or flag names; either would change difficulty noticeably.
    pub fn surface(&self, r: &Rendered) -> String {
        r.words
            .iter()
            .map(|w| self.lang.romanise(&w.phonemes()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Romanised words with morph boundaries marked by hyphens.
    pub fn segmented(&self, w: &Word) -> String {
        w.morphs
            .iter()
            .map(|m| self.lang.romanise(&m.form))
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Display form of a person's name (capitalised romanisation).
    pub fn name(&self, person: usize) -> String {
        capitalise(&self.lang.romanise(&self.names[person]))
    }

    fn clause(&self, c: &Clause) -> Vec<Word> {
        let verb = self.verb(c);
        let np = |role, case| {
            c.arg(role)
                .map(|np| self.noun_phrase(np, case))
                .unwrap_or_default()
        };
        // Commands have no subject: the reader is the one addressed.
        let subject = if c.mood == Mood::Imperative {
            Vec::new()
        } else {
            np(Role::Subject, Case::Subject)
        };
        let object = np(Role::Object, Case::Object);
        let recipient = np(Role::Recipient, Case::Dative);
        let adverbs: Vec<Word> = c.adverbs.iter().map(|a| self.plain(a)).collect();

        let mut out = Vec::new();
        match self.lang.syntax.word_order {
            WordOrder::Sov => {
                out.extend(subject);
                out.extend(recipient);
                out.extend(object);
                out.extend(adverbs);
                out.push(verb);
            }
            WordOrder::Svo => {
                out.extend(subject);
                out.push(verb);
                out.extend(object);
                out.extend(recipient);
                out.extend(adverbs);
            }
            WordOrder::Vso => {
                out.push(verb);
                out.extend(subject);
                out.extend(object);
                out.extend(recipient);
                out.extend(adverbs);
            }
        }
        out
    }

    fn verb(&self, c: &Clause) -> Word {
        let root = self.lang.lexicon.root(&c.predicate);
        Word {
            morphs: self
                .lang
                .morphology
                .verb(root, &c.predicate, c.tense, c.polarity),
        }
    }

    /// An uninflected word (adjective, numeral, determiner, adverb).
    fn plain(&self, concept: &str) -> Word {
        Word {
            morphs: vec![Morph {
                form: self.lang.lexicon.root(concept).clone(),
                gloss: concept.to_string(),
                is_root: true,
            }],
        }
    }

    fn noun_phrase(&self, np: &NounPhrase, case: Case) -> Vec<Word> {
        let head = match &np.head {
            Head::Concept(id) => Word {
                morphs: self
                    .lang
                    .morphology
                    .noun(self.lang.lexicon.root(id), id, np.number, case),
            },
            Head::Name(p) => Word {
                morphs: self
                    .lang
                    .morphology
                    .noun(&self.names[*p], &self.name(*p), np.number, case),
            },
        };

        // Modifiers inside out: adjectives nearest the noun, then the
        // numeral, then the demonstrative. Mirrored when they follow.
        let mut inner: Vec<Word> = Vec::new();
        inner.extend(np.adjectives.iter().map(|a| self.plain(a)));
        if let Some(n) = np.quantity {
            inner.push(self.plain(&concepts::numeral(n).id));
        }
        if let Some(d) = &np.determiner {
            inner.push(self.plain(d));
        }
        let mut core = Vec::new();
        match self.lang.syntax.modifiers {
            Side::Before => {
                core.extend(inner.into_iter().rev());
                core.push(head);
            }
            Side::After => {
                core.push(head);
                core.extend(inner);
            }
        }

        let mut out = Vec::new();
        let possessor = np
            .possessor
            .as_ref()
            .map(|p| self.noun_phrase(p, Case::Genitive))
            .unwrap_or_default();
        match self.lang.syntax.genitive {
            Side::Before => {
                out.extend(possessor);
                out.extend(core);
            }
            Side::After => {
                out.extend(core);
                out.extend(possessor);
            }
        }
        // DESIGN-Q: appositions always follow their head, in every language.
        // Head-final languages could plausibly put titles first instead.
        for a in &np.apposition {
            out.extend(self.noun_phrase(a, case));
        }
        out
    }
}

/// Uppercases the first letter, skipping leading punctuation such as `'`.
pub fn capitalise(s: &str) -> String {
    let mut done = false;
    s.chars()
        .flat_map(|ch| {
            if !done && ch.is_alphabetic() {
                done = true;
                ch.to_uppercase().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}
