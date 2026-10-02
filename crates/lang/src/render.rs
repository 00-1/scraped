//! Meaning → words. Deterministic: the same meaning always renders the same.

use serde::Serialize;

use crate::concepts;
use crate::difficulty::{NameMarking, Separation};
use crate::meaning::{Clause, Head, Mood, NounPhrase, Role, Sentence};
use crate::morphology::{AffixPosition, Case, Morph};
use crate::phonology::Phonemes;
use crate::script::GlyphKey;
use crate::syntax::{Side, WordOrder};
use crate::Language;

/// One rendered word, kept as morphs so it can be glossed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    pub morphs: Vec<Morph>,
    /// Personal names can be flagged by a determinative sign.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub name: bool,
}

impl Word {
    fn plain(morphs: Vec<Morph>) -> Self {
        Word {
            morphs,
            name: false,
        }
    }

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

/// Romanised word separators for each separation dial.
fn separator(s: Separation) -> &'static str {
    match s {
        Separation::Spaces => " ",
        Separation::Dots => "·",
        Separation::None => "",
    }
}

/// Romanised word divider.
pub fn separator_glyph() -> &'static str {
    separator(Separation::Dots)
}

/// Romanised name determinative.
pub const DETERMINATIVE: &str = "°";

/// Renders meanings in one language, with a table of personal names.
pub struct Renderer<'a> {
    pub lang: &'a Language,
    pub names: &'a [Phonemes],
}

impl Renderer<'_> {
    pub fn render(&self, s: &Sentence) -> Rendered {
        Rendered {
            words: self.sentence(s),
        }
    }

    fn sentence(&self, s: &Sentence) -> Vec<Word> {
        match s {
            Sentence::Clause(c) => self.clause(c),
            Sentence::List(items) => items
                .iter()
                .flat_map(|np| self.noun_phrase(np, Case::Subject))
                .collect(),
            Sentence::Text(parts) => parts.iter().flat_map(|p| self.sentence(p)).collect(),
        }
    }

    /// Romanised text, with words separated and names marked according to
    /// the difficulty dials.
    pub fn surface(&self, r: &Rendered) -> String {
        let d = &self.lang.difficulty;
        r.words
            .iter()
            .map(|w| {
                let text = self.lang.romanise(&w.phonemes());
                if w.name && d.names == NameMarking::Determinative {
                    format!("{DETERMINATIVE}{text}")
                } else {
                    text
                }
            })
            .collect::<Vec<_>>()
            .join(separator(d.separation))
    }

    /// The text as glyphs, in reading order. `None` marks a gap between
    /// words when words are separated by spaces.
    pub fn glyphs(&self, r: &Rendered) -> Vec<Option<GlyphKey>> {
        let d = &self.lang.difficulty;
        let mut out = Vec::new();
        for (i, w) in r.words.iter().enumerate() {
            if i > 0 {
                match d.separation {
                    Separation::Spaces => out.push(None),
                    Separation::Dots => out.push(Some(GlyphKey::Divider)),
                    Separation::None => {}
                }
            }
            if w.name && d.names == NameMarking::Determinative {
                out.push(Some(GlyphKey::Determinative));
            }
            let ipa = self.lang.phonology.to_ipa(&w.phonemes());
            out.extend(self.lang.script.spell(&ipa).into_iter().map(Some));
        }
        out
    }

    /// The text as numbered glyphs laid out on the surface: one string per
    /// line, physical left-to-right order, `/` for a gap between words.
    pub fn glyph_lines(&self, r: &Rendered, width: usize) -> Vec<String> {
        self.lang
            .script
            .layout(&self.glyphs(r), width)
            .into_iter()
            .map(|line| {
                line.iter()
                    .map(|t| match t {
                        Some(k) => self.lang.script.index(k).to_string(),
                        None => "/".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
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
                out.extend(verb);
            }
            WordOrder::Svo => {
                out.extend(subject);
                out.extend(verb);
                out.extend(object);
                out.extend(recipient);
                out.extend(adverbs);
            }
            WordOrder::Vso => {
                out.extend(verb);
                out.extend(subject);
                out.extend(object);
                out.extend(recipient);
                out.extend(adverbs);
            }
        }
        if c.mood == Mood::Potent {
            out.insert(0, self.plain("pot.open"));
            out.push(self.plain("pot.close"));
        }
        out
    }

    /// The verb with any particles, and the potent particle before it.
    // DESIGN-Q: the potent particle always comes directly before the verb
    // group, whatever the word order.
    fn verb(&self, c: &Clause) -> Vec<Word> {
        self.verb_group(&c.predicate, c.tense, c.polarity, c.mood == Mood::Potent)
    }

    /// Places particle words next to their host: after it in suffixing
    /// languages, before it (mirrored) in prefixing ones.
    pub(crate) fn with_particles(
        &self,
        host: Word,
        particles: Vec<Morph>,
        pos: AffixPosition,
    ) -> Vec<Word> {
        let particles = particles
            .into_iter()
            .map(|p| Word::plain(vec![Morph { is_root: true, ..p }]));
        match pos {
            AffixPosition::Suffix => std::iter::once(host).chain(particles).collect(),
            AffixPosition::Prefix => {
                let mut v: Vec<Word> = particles.collect();
                v.reverse();
                v.push(host);
                v
            }
        }
    }

    /// An uninflected word (adjective, numeral, determiner, adverb, particle).
    /// An uninflected word, for callers outside the renderer.
    pub fn plain_word(&self, concept: &str) -> Word {
        self.plain(concept)
    }

    pub(crate) fn plain(&self, concept: &str) -> Word {
        Word::plain(vec![Morph {
            form: self.lang.lexicon.root(concept).clone(),
            gloss: concepts::gloss(concept),
            is_root: true,
        }])
    }

    /// A noun phrase's head word with its particles.
    pub(crate) fn head(
        &self,
        head: &Head,
        number: crate::morphology::Number,
        case: Case,
    ) -> Vec<Word> {
        let m = &self.lang.morphology;
        let word = match head {
            Head::Concept(id) => Word::plain(m.noun(self.lang.lexicon.root(id), id, number, case)),
            Head::Name(p) => Word {
                morphs: m.noun(&self.names[*p], &self.name(*p), number, case),
                name: true,
            },
        };
        self.with_particles(word, m.noun_particles(number, case), m.noun_position)
    }

    /// The verb group: the verb with its particles (and the potent particle).
    pub(crate) fn verb_group(
        &self,
        predicate: &str,
        tense: crate::morphology::Tense,
        polarity: crate::morphology::Polarity,
        potent: bool,
    ) -> Vec<Word> {
        let m = &self.lang.morphology;
        let root = self.lang.lexicon.root(predicate);
        let word = Word::plain(m.verb(root, predicate, tense, polarity));
        let mut out = self.with_particles(word, m.verb_particles(tense, polarity), m.verb_position);
        if potent {
            out.insert(0, self.plain("pot"));
        }
        out
    }

    pub(crate) fn noun_phrase(&self, np: &NounPhrase, case: Case) -> Vec<Word> {
        let head = self.head(&np.head, np.number, case);

        // Modifiers inside out: adjectives nearest the noun, then the
        // numeral, then the demonstrative. Mirrored when they follow.
        let mut inner: Vec<Vec<Word>> = Vec::new();
        inner.extend(np.adjectives.iter().map(|a| vec![self.plain(a)]));
        if let Some(n) = np.quantity {
            inner.push(
                self.lang
                    .numerals
                    .words(n)
                    .iter()
                    .map(|w| self.plain(w))
                    .collect(),
            );
        }
        if let Some(d) = &np.determiner {
            inner.push(vec![self.plain(d)]);
        }
        let mut core = Vec::new();
        match self.lang.syntax.modifiers {
            Side::Before => {
                core.extend(inner.into_iter().rev().flatten());
                core.extend(head);
            }
            Side::After => {
                core.extend(head);
                core.extend(inner.into_iter().flatten());
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
