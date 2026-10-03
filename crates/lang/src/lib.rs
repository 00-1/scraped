//! Language engine for *Scraped Again*.
//!
//! Generates a consistent, decipherable language from a seed: sounds,
//! roots, affixes, word order, numerals, a script, and historical eras
//! linked by regular sound change, plus formulaic inscriptions rendered from
//! a structured meaning. Pure logic with no I/O, so it runs unchanged in a
//! WebAssembly front end.
//!
//! ```
//! let lang = scraped_lang::Language::generate(42);
//! let corpus = scraped_lang::corpus::Corpus::generate(&lang, 5);
//! assert_eq!(corpus.inscriptions.len(), 5);
//! let late = lang.at_era(2);
//! assert_eq!(late.era, 2);
//! ```

pub mod concepts;
pub mod corpus;
pub mod difficulty;
pub mod english;
pub mod history;
pub mod impression;
pub mod lexicon;
pub mod meaning;
pub mod morphology;
pub mod numerals;
pub mod parse;
pub mod phonology;
pub mod render;
pub mod rng;
pub mod script;
pub mod sheet;
pub mod slots;
pub mod syntax;

use std::collections::{BTreeMap, BTreeSet};

use concepts::Pos;
use difficulty::Difficulty;
use history::Step;
use lexicon::{Lexicon, WordMaker};
use morphology::Morphology;
use numerals::Numerals;
use phonology::{Phonemes, Phonology};
use rng::{Rng, Stream};
use script::Script;
use syntax::Syntax;

/// A complete generated language at one era.
#[derive(Debug, Clone)]
pub struct Language {
    pub seed: u64,
    /// Historical stage: 0 is the oldest (the proto-language).
    pub era: u32,
    pub difficulty: Difficulty,
    /// Spelling shared by every era, so a sound is always spelled alike.
    pub spelling: BTreeMap<&'static str, String>,
    pub phonology: Phonology,
    pub morphology: Morphology,
    pub syntax: Syntax,
    pub lexicon: Lexicon,
    pub numerals: Numerals,
    pub script: Script,
    /// The sound changes that led here, one step per era after the first.
    pub history: Vec<Step>,
    /// What changed in the step into this era, beyond regular sound change.
    pub changes: EraChanges,
}

/// Irregular-looking events of one era, all of which have regular causes.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct EraChanges {
    /// Concepts that got a new word (by chance, or because sound change
    /// made the old word collide with another).
    pub replaced: Vec<String>,
    /// Affixes that wore down and became separate particle words.
    pub eroded: Vec<&'static str>,
    /// Fused affix combinations that broke up again.
    pub unfused: usize,
}

impl Language {
    /// Generates the oldest era of the language for `seed` with default
    /// difficulty. Same seed, same language, always.
    pub fn generate(seed: u64) -> Self {
        Self::generate_with(seed, Difficulty::default())
    }

    /// Generates the oldest era with explicit difficulty dials.
    pub fn generate_with(seed: u64, difficulty: Difficulty) -> Self {
        let mut phonology = Phonology::generate(seed);

        // Spell every sound any era will have, together, so spellings never
        // shift between eras and stay uniquely decodable throughout.
        let mut sets = phonology.sets();
        let mut all_sounds: BTreeSet<&'static str> = sets.sounds.clone();
        let mut rules = Vec::new();
        for era in 1..difficulty.eras() {
            let step = Step::generate(seed, era, &sets, &rules);
            sets = step.apply_sets(&sets);
            rules.extend(step.rules);
            all_sounds.extend(sets.sounds.iter().copied());
        }
        let spelling = phonology::spelling_for(&all_sounds);
        phonology.respell(&spelling);

        let numerals = Numerals::generate(seed);
        let morphology = Morphology::generate(seed, &phonology, difficulty.regularity);
        let syntax = Syntax::generate(seed);
        let lexicon = {
            let mut rng = Rng::new(seed, Stream::Lexicon);
            let mut maker = WordMaker::new(&phonology, &morphology);
            let include = |c: &concepts::Concept| {
                c.pos != Pos::Num && c.id != "and" || numerals.needs(&c.id, &c.needs)
            };
            lexicon::generate(&mut rng, &mut maker, &include)
        };
        let script = Script::generate(
            &mut Rng::new(seed, Stream::Script),
            &phonology,
            &numerals,
            difficulty.script,
        );
        Language {
            seed,
            era: 0,
            difficulty,
            spelling,
            phonology,
            morphology,
            syntax,
            lexicon,
            numerals,
            script,
            history: Vec::new(),
            changes: EraChanges::default(),
        }
    }

    /// The same language at a later era (0 = oldest). Clamped to the number
    /// of eras the difficulty allows.
    pub fn at_era(&self, era: u32) -> Self {
        let target = era.min(self.difficulty.eras() - 1);
        let mut lang = if target < self.era {
            Self::generate_with(self.seed, self.difficulty)
        } else {
            self.clone()
        };
        while lang.era < target {
            lang = lang.evolve();
        }
        lang
    }

    /// Every era of the language, oldest first.
    pub fn eras(&self) -> Vec<Self> {
        (0..self.difficulty.eras())
            .map(|e| self.at_era(e))
            .collect()
    }

    /// The next era: regular sound change applied to every word and affix,
    /// then the repairs real languages make when change causes trouble.
    fn evolve(&self) -> Self {
        let era = self.era + 1;
        let earlier: Vec<history::Rule> =
            self.history.iter().flat_map(|s| s.rules.clone()).collect();
        let step = Step::generate(self.seed, era, &self.phonology.sets(), &earlier);
        let phonology =
            Phonology::from_sets(&step.apply_sets(&self.phonology.sets()), &self.spelling);
        let derive = |w: &Phonemes| -> Phonemes {
            let ipa = step.apply(&self.phonology.to_ipa(w));
            phonology
                .from_ipa(&ipa)
                .expect("sound change stays in the inventory")
        };
        let mut rng = Rng::new(self.seed, Stream::Evolution(era));
        let mut changes = EraChanges::default();

        // Affixes change like any other sounds.
        let mut morphology = self.morphology.clone();
        for a in morphology.affix_list_mut() {
            a.form = derive(&a.form);
        }
        for f in &mut morphology.fusions {
            f.form = derive(&f.form);
        }
        // Worn-down affixes that now collide become separate words; fused
        // forms that collide are levelled out first.
        let mut newly_eroded: Vec<&'static str> = Vec::new();
        while !morphology.affixes_are_distinct() {
            if morphology.fusions.pop().is_some() {
                changes.unfused += 1;
                continue;
            }
            let forms: Vec<Phonemes> = morphology.affixes().into_iter().cloned().collect();
            let victim = morphology
                .affix_list_mut()
                .into_iter()
                .filter(|a| !a.particle)
                .filter(|a| forms.iter().filter(|f| **f == a.form).count() > 1)
                .last()
                .map(|a| a.gloss);
            let victim = victim.unwrap_or_else(|| {
                morphology
                    .affix_list()
                    .into_iter()
                    .rev()
                    .find(|a| !a.particle)
                    .expect("distinct once all are particles")
                    .gloss
            });
            newly_eroded.push(victim);
            erode(&mut morphology, victim);
        }
        // DESIGN-Q: besides forced erosion, one affix in five eras erodes by
        // chance, so morphology visibly changes even without collisions.
        if rng.chance(20) {
            let candidates: Vec<&'static str> = morphology
                .affix_list()
                .into_iter()
                .filter(|a| !a.particle)
                .map(|a| a.gloss)
                .collect();
            if !candidates.is_empty() {
                let g = *rng.pick(&candidates);
                newly_eroded.push(g);
                erode(&mut morphology, g);
            }
        }

        let mut coined: Vec<(&'static str, Phonemes)> = Vec::new();
        let lexicon = {
            let mut maker = WordMaker::new(&phonology, &morphology);
            let mut roots = BTreeMap::new();
            for c in concepts::all().iter().filter(|c| self.lexicon.has(&c.id)) {
                let derived = derive(self.lexicon.root(&c.id));
                debug_assert!(phonology.is_valid(&derived), "era {era}: {} invalid", c.id);
                // DESIGN-Q: about one word in twenty-five is replaced per era
                // by chance, on top of replacements forced by collisions.
                let keep = !rng.chance(4) && maker.try_reserve(&derived, c.pos);
                let root = if keep {
                    derived
                } else {
                    changes.replaced.push(c.id.clone());
                    let syllables = lexicon::syllables_for(&mut rng, c.pos);
                    maker.make(&mut rng, syllables, c.pos)
                };
                roots.insert(c.id.clone(), root);
            }
            for g in &newly_eroded {
                coined.push((g, maker.make(&mut rng, 1, Pos::Particle)));
            }
            Lexicon { roots }
        };
        for (g, form) in coined {
            for a in morphology.affix_list_mut() {
                if a.gloss == g {
                    a.form = form.clone();
                }
            }
        }
        changes.eroded = newly_eroded;

        let script = self.script.evolve(&mut rng, &phonology, &self.numerals);
        let mut history = self.history.clone();
        history.push(step);
        Language {
            seed: self.seed,
            era,
            difficulty: self.difficulty,
            spelling: self.spelling.clone(),
            phonology,
            morphology,
            syntax: self.syntax.clone(),
            lexicon,
            numerals: self.numerals.clone(),
            script,
            history,
            changes,
        }
    }

    /// A word maker that already knows every root, for coining names that
    /// never collide with ordinary words.
    pub fn word_maker(&self) -> WordMaker<'_> {
        let mut maker = WordMaker::new(&self.phonology, &self.morphology);
        for (id, root) in &self.lexicon.roots {
            maker.reserve(root, concepts::get(id).pos);
        }
        maker
    }

    /// Romanised spelling of a phoneme sequence.
    pub fn romanise(&self, word: &Phonemes) -> String {
        self.phonology.romanise(word)
    }

    /// Every surface word of the language, mapped to its analyses
    /// (`root-AFFIX-AFFIX`). Used to check that words are unambiguous, and a
    /// first step towards parsing surface text back into meaning.
    pub fn analyses(&self, names: &[Phonemes]) -> BTreeMap<Phonemes, Vec<String>> {
        let mut out: BTreeMap<Phonemes, BTreeSet<String>> = BTreeMap::new();
        let mut add = |root: &Phonemes, gloss: &str, pos: Pos| {
            for morphs in self.morphology.paradigm(root, gloss, pos) {
                out.entry(morphology::join(&morphs))
                    .or_default()
                    .insert(render::gloss_of(&morphs));
            }
        };
        for (id, root) in &self.lexicon.roots {
            add(root, &concepts::gloss(id), concepts::get(id).pos);
        }
        for p in self.morphology.particles() {
            add(&p.form, p.gloss, Pos::Particle);
        }
        for n in names {
            add(n, &render::capitalise(&self.romanise(n)), Pos::Noun);
        }
        out.into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect()
    }
}

/// Turns an affix into a particle word; its form is coined afterwards.
fn erode(m: &mut Morphology, gloss: &str) {
    for a in m.affix_list_mut() {
        if a.gloss == gloss {
            a.particle = true;
            a.form = Vec::new();
        }
    }
}
