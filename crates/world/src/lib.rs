//! World generation for *Scraped Again*.
//!
//! A bounded world from a seed: land and climate, rivers and lakes, a
//! civilisation's history across the language's eras, the buildings it left
//! with their interiors and writing, and the state they are in now. Pure
//! data; nothing here knows about a player.

pub mod debug;
pub mod decay;
pub mod history;
pub mod structures;
pub mod terrain;
pub mod texts;
pub mod water;

use serde::Serialize;

use scraped_lang::phonology::Phonemes;
use scraped_lang::render::{Rendered, Renderer};
use scraped_lang::Language;

use decay::Trace;
use history::{Effect, History};
use structures::Structure;
use terrain::Terrain;
use texts::Text;
use water::Water;

/// Everything generated for one seed.
#[derive(Debug, Clone, Serialize)]
pub struct World {
    pub seed: u64,
    pub terrain: Terrain,
    pub water: Water,
    pub history: History,
    pub structures: Vec<Structure>,
    pub texts: Vec<Text>,
    /// Physical evidence of events (ruins, walls).
    pub traces: Vec<Trace>,
    /// What old writing was meant to do to the land (mechanical in M08).
    pub effects: Vec<Effect>,
    /// The language at each era, oldest first.
    #[serde(skip)]
    pub languages: Vec<Language>,
    /// Every person's name, indexed like `history.people`.
    #[serde(skip)]
    pub names: Vec<Phonemes>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        Self::generate_with(seed, scraped_lang::difficulty::Difficulty::default())
    }

    /// A world whose language follows the given difficulty dials.
    pub fn generate_with(seed: u64, difficulty: scraped_lang::difficulty::Difficulty) -> Self {
        let languages = Language::generate_with(seed, difficulty).eras();
        let mut terrain = Terrain::generate(seed);
        let water = Water::generate(&mut terrain);
        let history = History::generate(seed, &terrain, &water, &languages);
        let mut structures = structures::place(seed, &terrain, &water, &history);
        let texts = texts::place(seed, &history, &mut structures);
        let (traces, effects) = decay::apply(seed, &terrain, &history, &mut structures);
        let names = history.people.iter().map(|p| p.name.clone()).collect();
        World {
            seed,
            terrain,
            water,
            history,
            structures,
            texts,
            traces,
            effects,
            languages,
            names,
        }
    }

    /// A text rendered in the language of its era.
    pub fn render(&self, text: &Text) -> Rendered {
        self.renderer(text.era).render(&text.meaning)
    }

    /// A renderer for one era, knowing every person's name.
    pub fn renderer(&self, era: u32) -> Renderer<'_> {
        Renderer {
            lang: &self.languages[era as usize],
            names: &self.names,
        }
    }

    /// A text's romanised surface form.
    pub fn surface(&self, text: &Text) -> String {
        let r = self.renderer(text.era);
        r.surface(&r.render(&text.meaning))
    }
}
