//! World generation for *Scraped Again*.
//!
//! A bounded world from a seed: land and climate, rivers and lakes, a
//! civilisation's history across the language's eras, the buildings it left
//! with their interiors and writing, and the state they are in now. Pure
//! data; nothing here knows about a player.

pub mod debug;
pub mod decay;
pub mod features;
pub mod geology;
pub mod history;
pub mod interiors;
pub mod life;
pub mod objects;
pub mod phenomena;
pub mod scenes;
pub mod sky;
pub mod structures;
pub mod terrain;
pub mod texts;
pub mod towns;
pub mod underground;
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
    /// The rock under the land (D03).
    pub geology: geology::Geology,
    /// Natural features and old marks on the land (D03).
    pub features: Vec<features::Feature>,
    /// Each settlement's role and layout, indexed like settlements (D03).
    pub towns: Vec<towns::Town>,
    /// Where cellars, drains, tunnels, catacombs and mine workings run,
    /// for D04 to build (D03).
    pub underground: Vec<underground::Route>,
    /// Small arrangements of things that show what happened (D03).
    pub scenes: Vec<scenes::Scene>,
    /// The great interiors (D04): structures grown into vast places, and
    /// the caves among them, with their kinds.
    pub greats: Vec<(usize, interiors::GreatKind)>,
    /// Things people made, used and left behind, with their owners (D05).
    pub objects: Vec<objects::Object>,
    /// Locked doors and their keys (D05).
    pub locks: Vec<objects::DoorLock>,
    /// Animals and plants, and where they make their homes (D06).
    pub life: life::Life,
    /// Stars, moon, planets and their cycles (D06).
    pub sky: sky::Sky,
    /// Natural wonders: oddities with no writing cause (D06).
    pub phenomena: Vec<phenomena::Phenomenon>,
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
        let geology = geology::Geology::generate(seed, &terrain, &water);
        let towns = towns::plan(seed, &terrain, &water, &geology, &history);
        let mut structures = structures::place(seed, &terrain, &water, &history);
        structures::place_more(seed, &terrain, &water, &history, &towns, &mut structures);
        let texts = texts::place(seed, &history, &mut structures);
        let (traces, effects) = decay::apply(seed, &terrain, &history, &mut structures);
        let features = features::place(seed, &terrain, &water, &geology, &history, &structures);
        let greats = interiors::grow(
            seed,
            &terrain,
            &geology,
            &history,
            &towns,
            &features,
            &mut structures,
        );
        let underground =
            underground::plan(&terrain, &water, &history, &towns, &structures, &features);
        let scenes = scenes::place(
            seed,
            &terrain,
            &water,
            &history,
            &towns,
            &structures,
            &features,
        );
        let mut objects = objects::place(seed, &history, &structures);
        let locks = objects::hide_and_lock(seed, &history, &structures, &features, &mut objects);
        let life = life::Life::generate(seed, &terrain, &water);
        let sky = sky::Sky::generate(seed, languages.first());
        let phenomena = phenomena::place(seed, &terrain, &water, &geology, &features);
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
            geology,
            features,
            towns,
            underground,
            scenes,
            greats,
            objects,
            locks,
            life,
            sky,
            phenomena,
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
