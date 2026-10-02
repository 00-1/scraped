//! Language engine for *Scraped Again*.
//!
//! Generates a consistent, decipherable language from a seed: sounds,
//! roots, affixes, word order, and formulaic inscriptions rendered from a
//! structured meaning. Pure logic with no I/O, so it runs unchanged in a
//! WebAssembly front end.
//!
//! ```
//! let lang = scraped_lang::Language::generate(42);
//! let corpus = scraped_lang::corpus::Corpus::generate(&lang, 5);
//! assert_eq!(corpus.inscriptions.len(), 5);
//! ```

pub mod concepts;
pub mod corpus;
pub mod english;
pub mod lexicon;
pub mod meaning;
pub mod morphology;
pub mod phonology;
pub mod render;
pub mod rng;
pub mod sheet;
pub mod syntax;

use concepts::Pos;
use lexicon::{Lexicon, WordMaker};
use morphology::Morphology;
use phonology::{Phonemes, Phonology};
use rng::{Rng, Stream};
use syntax::Syntax;

/// A complete generated language.
#[derive(Debug, Clone)]
pub struct Language {
    pub seed: u64,
    /// Historical stage of the language. Always 0 for now; sound-change
    /// eras (palimpsest layers) will derive later stages from earlier ones.
    pub era: u32,
    pub phonology: Phonology,
    pub morphology: Morphology,
    pub syntax: Syntax,
    pub lexicon: Lexicon,
}

impl Language {
    /// Generates the language for `seed`. Same seed, same language, always.
    pub fn generate(seed: u64) -> Self {
        let phonology = Phonology::generate(seed);
        let morphology = Morphology::generate(seed, &phonology);
        let syntax = Syntax::generate(seed);
        let mut rng = Rng::new(seed, Stream::Lexicon);
        let lexicon = {
            let mut maker = WordMaker::new(&phonology, &morphology);
            lexicon::generate(&mut rng, &mut maker)
        };
        Language {
            seed,
            era: 0,
            phonology,
            morphology,
            syntax,
            lexicon,
        }
    }

    /// A word maker that already knows every root, for coining names that
    /// never collide with ordinary words.
    pub fn word_maker(&self) -> WordMaker<'_> {
        let mut maker = WordMaker::new(&self.phonology, &self.morphology);
        for c in concepts::all() {
            maker.reserve(self.lexicon.root(&c.id), c.pos);
        }
        maker
    }

    /// Romanised spelling of a phoneme sequence.
    pub fn romanise(&self, word: &Phonemes) -> String {
        self.phonology.romanise(word)
    }

    /// Every surface form of every root, mapped to its analyses
    /// (`root-AFFIX-AFFIX`). Used to check that words are unambiguous, and a
    /// first step towards parsing surface text back into meaning.
    pub fn analyses(
        &self,
        names: &[Phonemes],
    ) -> std::collections::BTreeMap<Phonemes, Vec<String>> {
        let mut out: std::collections::BTreeMap<Phonemes, Vec<String>> = Default::default();
        let mut add = |root: &Phonemes, gloss: &str, pos: Pos| {
            for morphs in self.morphology.paradigm(root, gloss, pos) {
                let gloss = render::gloss_of(&morphs);
                out.entry(morphology::join(&morphs))
                    .or_default()
                    .push(gloss);
            }
        };
        for c in concepts::all() {
            add(self.lexicon.root(&c.id), &c.id, c.pos);
        }
        for n in names {
            add(n, &render::capitalise(&self.romanise(n)), Pos::Noun);
        }
        out
    }
}
