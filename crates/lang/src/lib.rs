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
pub mod sample;
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
    /// What its people have words for (D07).
    pub culture: Culture,
}

/// What a people have words for (D07): every core field, the fields their
/// way of life makes rich, some words from a few others, and the animals
/// and plants of their land.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Culture {
    /// Fields named fully.
    pub rich: Vec<String>,
    /// Fields named in part (about two words in three).
    pub some: Vec<String>,
    /// Species concept ids of their land.
    pub species: Vec<String>,
}

impl Culture {
    /// A culture drawn from the seed alone, for a language without a world:
    /// two rich fields, three partial ones, no species.
    pub fn from_seed(seed: u64) -> Self {
        Self::with_weights(seed, &[], Vec::new())
    }

    /// A culture whose rich fields lean towards `weights` (field, extra
    /// weight): a land with a long coast gives the sea, grassland herding.
    // DESIGN-Q: two rich fields and three partial ones, drawn by weight
    // (each field weight 10 plus the world's hints).
    pub fn with_weights(seed: u64, weights: &[(&str, u32)], species: Vec<String>) -> Self {
        let mut rng = Rng::new(seed, Stream::Culture);
        let mut left: Vec<(&str, u32)> = concepts::CULTURE_FIELDS
            .iter()
            .map(|f| {
                let extra = weights
                    .iter()
                    .filter(|(k, _)| k == f)
                    .map(|(_, w)| *w)
                    .sum::<u32>();
                (*f, 10 + extra)
            })
            .collect();
        let mut chosen = Vec::new();
        for _ in 0..5 {
            let total: u32 = left.iter().map(|(_, w)| w).sum();
            let mut r = rng.below(total);
            let i = left
                .iter()
                .position(|(_, w)| {
                    if r < *w {
                        true
                    } else {
                        r -= w;
                        false
                    }
                })
                .unwrap_or(0);
            chosen.push(left.remove(i).0.to_string());
        }
        Culture {
            rich: chosen[..2].to_vec(),
            some: chosen[2..].to_vec(),
            species,
        }
    }

    /// Whether the language has a word for a concept.
    pub fn names(&self, seed: u64, c: &concepts::Concept) -> bool {
        let f = c.field.as_str();
        if concepts::CORE_FIELDS.contains(&f) {
            return true;
        }
        if f == "fauna" || f == "flora" {
            return self.species.contains(&c.id);
        }
        if self.rich.iter().any(|r| r == f) {
            return true;
        }
        if self.some.iter().any(|r| r == f) {
            // A stable two in three, by concept.
            let h = c.id.bytes().fold(seed ^ 0x9e37_79b9, |a, b| {
                a.wrapping_mul(31).wrapping_add(u64::from(b))
            });
            return h % 3 != 0;
        }
        false
    }
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
        Self::generate_for(seed, difficulty, Culture::from_seed(seed))
    }

    /// Generates the oldest era of a people with a given culture (D07).
    pub fn generate_for(seed: u64, difficulty: Difficulty, culture: Culture) -> Self {
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
                (c.pos != Pos::Num && c.id != "and" || numerals.needs(&c.id, &c.needs))
                    && culture.names(seed, c)
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
            culture,
        }
    }

    /// The same language at a later era (0 = oldest). Clamped to the number
    /// of eras the difficulty allows.
    pub fn at_era(&self, era: u32) -> Self {
        let target = era.min(self.difficulty.eras() - 1);
        let mut lang = if target < self.era {
            Self::generate_for(self.seed, self.difficulty, self.culture.clone())
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
        // Derivational affixes change too; one worn into another affix's
        // shape gets a fresh form (derivations never become words).
        for a in &mut morphology.derivations {
            a.form = derive(&a.form);
        }
        for i in 0..morphology.derivations.len() {
            loop {
                let form = morphology.derivations[i].form.clone();
                let same = morphology
                    .affixes()
                    .into_iter()
                    .filter(|f| **f == form)
                    .count();
                if !form.is_empty() && same <= 1 {
                    break;
                }
                morphology.derivations[i].form = phonology.random_affix(&mut rng);
            }
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
            for c in concepts::all()
                .iter()
                .filter(|c| !c.is_built() && self.lexicon.has(&c.id))
            {
                let derived = derive(&self.lexicon.root(&c.id));
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
            Lexicon::new(roots, &morphology)
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
            culture: self.culture.clone(),
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
