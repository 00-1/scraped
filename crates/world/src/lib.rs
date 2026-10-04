//! World generation for *Scraped Again*.
//!
//! A bounded world from a seed: land and climate, rivers and lakes, a
//! civilisation's history across the language's eras, the buildings it left
//! with their interiors and writing, and the state they are in now. Pure
//! data; nothing here knows about a player.

pub mod debug;
pub mod decay;
pub mod features;
pub mod genres;
pub mod geology;
pub mod history;
pub mod interiors;
pub mod life;
pub mod objects;
pub mod phenomena;
pub mod scenes;
pub mod sky;
pub mod society;
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
    /// Every name as said in each era (D08): people, indexed like
    /// `history.people`, then towns. A name made of words is said with
    /// that era's words; a coined one carries the sound changes since it
    /// was given.
    #[serde(skip)]
    pub names: Vec<Vec<Phonemes>>,
}

impl World {
    pub fn generate(seed: u64) -> Self {
        Self::generate_with(seed, scraped_lang::difficulty::Difficulty::default())
    }

    /// A world whose language follows the given difficulty dials.
    pub fn generate_with(seed: u64, difficulty: scraped_lang::difficulty::Difficulty) -> Self {
        let mut terrain = Terrain::generate(seed);
        let water = Water::generate(&mut terrain);
        // The land and its life come first, so the language can have words
        // for what its people lived among (D07).
        let life = life::Life::generate(seed, &terrain, &water);
        let culture = culture(seed, &terrain, &water, &life);
        let languages = Language::generate_for(seed, difficulty, culture).eras();
        let history = History::generate(seed, &terrain, &water, &languages);
        let geology = geology::Geology::generate(seed, &terrain, &water);
        let towns = towns::plan(seed, &terrain, &water, &geology, &history);
        let mut structures = structures::place(seed, &terrain, &water, &history);
        structures::place_more(seed, &terrain, &water, &history, &towns, &mut structures);
        let mut texts = texts::place(seed, &history, &mut structures);
        genres::write(seed, &history, &languages, &mut structures, &mut texts);
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
        let sky = sky::Sky::generate(seed, languages.first());
        let phenomena = phenomena::place(seed, &terrain, &water, &geology, &features);
        // People first, then towns (D08): a town is named as `people + id`.
        let given: Vec<(u32, &Phonemes, &[String])> = history
            .people
            .iter()
            .map(|p| (p.era, &p.name, p.name_meaning.as_slice()))
            .chain(
                history
                    .settlements
                    .iter()
                    .map(|s| (s.era, &s.name, s.name_meaning.as_slice())),
            )
            .collect();
        let names = languages
            .iter()
            .map(|lang| {
                given
                    .iter()
                    .map(|&(era, form, meaning)| {
                        if era == lang.era {
                            return form.clone();
                        }
                        let first = &languages[era as usize];
                        // A later name never stands in an earlier text, but
                        // the era's reader still needs a spelling for it.
                        let spellable = || {
                            let ipa: Vec<&str> = first
                                .phonology
                                .to_ipa(form)
                                .into_iter()
                                .filter(|i| lang.phonology.from_ipa(&[i]).is_some())
                                .collect();
                            lang.phonology.from_ipa(&ipa).filter(|w| !w.is_empty())
                        };
                        (!meaning.is_empty())
                            .then(|| lang.say_name(meaning))
                            .flatten()
                            .or_else(|| lang.carry(first, form))
                            .or_else(spellable)
                            .unwrap_or_else(|| form.clone())
                    })
                    .collect()
            })
            .collect();
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

    /// A cheap fingerprint of everything generated (FNV-1a over its JSON):
    /// any change to generation changes it (C01: such a change is a major
    /// version).
    pub fn fingerprint(&self) -> String {
        let json = serde_json::to_string(self).expect("a world serialises");
        let h = json.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
        format!("{h:016x}")
    }

    /// A text rendered in the language of its era.
    pub fn render(&self, text: &Text) -> Rendered {
        self.renderer(text.era).render(&text.meaning)
    }

    /// A renderer for one era, knowing every person's name.
    pub fn renderer(&self, era: u32) -> Renderer<'_> {
        Renderer {
            lang: &self.languages[era as usize],
            names: &self.names[era as usize],
        }
    }

    /// A text's romanised surface form.
    pub fn surface(&self, text: &Text) -> String {
        let r = self.renderer(text.era);
        r.surface(&r.render(&text.meaning))
    }
}

/// What a world's people had words for (D07): the sea where the coast is
/// long, herding on open grassland, farming by rivers and in forests, the
/// rest left to chance; and the animals and plants of the land.
fn culture(
    seed: u64,
    t: &terrain::Terrain,
    w: &water::Water,
    life: &life::Life,
) -> scraped_lang::Culture {
    use terrain::{Biome, SIZE};
    let (mut land, mut coast, mut open, mut river) = (0u32, 0u32, 0u32, 0u32);
    for y in 0..SIZE {
        for x in 0..SIZE {
            if !t.is_land(x, y) {
                continue;
            }
            land += 1;
            match t.biome.get(x, y) {
                Biome::Shore | Biome::Marsh => coast += 1,
                Biome::Grassland | Biome::Scrub => open += 1,
                _ => {}
            }
            if w.is_river(t, x, y) {
                river += 1;
            }
        }
    }
    let share = |n: u32| n * 100 / land.max(1);
    let weights = [
        ("sea", share(coast) * 3),
        ("herding", share(open)),
        ("farming", share(river) * 4),
    ];
    scraped_lang::Culture::with_weights(seed, &weights, life.concepts())
}
