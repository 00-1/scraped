//! The game core for *Scraped Again*: state, commands, and what the player
//! is told.
//!
//! Pure logic, no I/O. A game is fully defined by its seed, the content
//! pack and the commands typed, so a save is just those and loading replays
//! them. Every sentence comes from a content slot; nothing in this crate is
//! prose.

pub use scraped_sim::outdoors;
pub mod attention;
#[cfg(test)]
mod attention_tests;
pub mod bots;
pub mod composing;
#[cfg(test)]
mod composing_tests;
pub mod coverage;
#[cfg(test)]
mod coverage_tests;
pub mod depth;
pub mod ending;
#[cfg(test)]
mod ending_tests;
pub mod fairness;
#[cfg(test)]
mod fairness_tests;
mod interior;
mod interior_slots;
#[cfg(test)]
mod interior_tests;
mod life;
mod life_slots;
#[cfg(test)]
mod life_tests;
pub mod mapper;
mod object_slots;
#[cfg(test)]
mod object_tests;
mod objects;
pub mod parser;
mod physical;
mod place_slots;
mod places;
#[cfg(test)]
mod places_tests;
pub mod quiet_slots;
pub mod reading;
mod reading_slots;
#[cfg(test)]
mod reading_tests;
#[cfg(test)]
mod sealed_tests;
pub mod seedcode;
mod senses;
pub mod site;
mod skies;
mod sky_slots;
pub mod slots;
#[cfg(test)]
mod storylet_tests;
pub mod storylets;
#[cfg(test)]
mod survival;
pub mod trajectory;
#[cfg(test)]
mod trajectory_tests;
mod travel;
mod works;
#[cfg(test)]
mod works_tests;
mod writing;
#[cfg(test)]
mod writing_tests;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};

use scraped_content::{Context, Pack, Registry, Renderer, Value};
use scraped_lang::slots::{describe_glyph, LangHooks};
use scraped_world::structures::{Exit, Passage, PassageState};

use outdoors::Pos;
use parser::{resolve, Candidate, Command, ParseError, Resolution};
use scraped_sim::body::{Activity, Body};
use scraped_sim::creatures::Creature;
use scraped_sim::env::SimState;
use scraped_sim::writing::Claim;
use site::{ctx, label, Place, Site, Thing, Way};

/// Verbs that are compass points or up and down.
const DIRECTION_VERBS: &[&str] = &[
    "north",
    "south",
    "east",
    "west",
    "up",
    "down",
    "northeast",
    "northwest",
    "southeast",
    "southwest",
];

/// Glyphs shown per page of reading.
pub const PAGE: usize = 16;

/// Something a command can refer to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "id")]
pub enum Target {
    Thing(usize),
    Structure(usize),
    /// A way out of the current room, by its link in the interior (D04:
    /// several ways may face one direction).
    Way(usize),
    /// Something seen in the distance, by index into `Land::landmarks`.
    Landmark(usize),
    /// A place the player named, by index into `State::names`.
    Named(usize),
    /// A followable edge nearby, by index into `outdoors::EDGES`.
    Edge(usize),
    /// A mechanism, by index into `Fixtures::mechanisms`.
    Mechanism(usize),
    /// The fire burning here.
    Fire,
    /// Alike buildings here, as a group, by kind (index into
    /// `slots::STRUCTURES`), and perhaps only those in one condition
    /// (index into `slots::CONDITIONS`): "the worn tombs".
    Buildings(usize, Option<usize>),
    /// Alike things here, as a group, by kind (index into `slots::kinds()`).
    Things(usize),
    /// A part of the town underfoot, by index into its districts (D03).
    District(usize),
    /// A natural feature or old mark close by, by index into
    /// `World::features` (D03).
    Feature(usize),
    /// A space of this building the player has been in (D04).
    Room(usize),
    /// A sign of the last thing read, by its position from 1 (S01).
    Sign(usize),
}

/// How a run ended, for the end-of-run summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Death {
    pub cause: String,
    /// The command the player was carrying out.
    pub doing: String,
    pub minutes: u32,
    /// Where, as a debug id.
    pub place: String,
}

/// A question the game is waiting for an answer to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pending {
    pub verb: String,
    pub options: Vec<Target>,
}

/// What the player is reading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    pub thing: usize,
    pub page: usize,
}

/// Everything that changes during play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub place: Place,
    pub carried: Vec<usize>,
    /// Things put down somewhere other than where they started.
    pub moved: BTreeMap<usize, Place>,
    /// Where things put down outdoors lie.
    pub dropped: BTreeMap<usize, Pos>,
    /// Where the player is on the land (indoors: the building's spot).
    pub pos: Pos,
    /// Landmarks the player has seen, so only new ones interrupt travel.
    pub seen: BTreeSet<usize>,
    /// Where each journey started, for `go back`.
    pub trail: Vec<Pos>,
    /// Places the player has named, with where they truly are.
    pub names: Vec<(String, Pos)>,
    /// Minutes since midnight of the first day.
    pub minutes: u32,
    /// The last thing referred to, for "it".
    pub it: Option<Target>,
    pub reading: Option<Reading>,
    /// The last thing read, whose signs can be examined and traced.
    pub last_read: Option<usize>,
    /// Signs whose sound the player has heard (era, index in its script):
    /// the one thing the game keeps for them (S01).
    #[serde(default)]
    pub heard: BTreeSet<(u32, usize)>,
    /// Snares set (D06): where, and the minute they were set.
    #[serde(default)]
    pub snares: Vec<(Pos, u32)>,
    /// Containers opened, caches uncovered and doors unlocked (D05).
    #[serde(default)]
    pub opened: BTreeSet<usize>,
    #[serde(default)]
    pub uncovered: BTreeSet<usize>,
    #[serde(default)]
    pub unlocked: BTreeSet<(usize, usize)>,
    /// When the player came into foul air, while they stay in it (D04).
    #[serde(default)]
    pub foul_since: Option<u32>,
    /// A text being traced: the thing, and the next sign to trace.
    #[serde(default)]
    pub tracing: Option<(usize, usize)>,
    /// Doors the player has opened or closed: "structure:link" → open.
    pub doors: BTreeMap<String, bool>,
    pub pending: Option<Pending>,
    /// The physical world as the player has changed it.
    pub sim: SimState,
    pub body: Body,
    pub creatures: Vec<Creature>,
    /// Torches and lamps burning.
    pub lit: BTreeSet<usize>,
    /// Minutes of burning left in torches and lamps.
    pub fuel: BTreeMap<usize, u32>,
    /// Drinks of water in carried containers.
    pub water: BTreeMap<usize, u32>,
    /// Clothing worn.
    pub worn: BTreeSet<usize>,
    /// Things used up: eaten, burnt.
    pub gone: BTreeSet<usize>,
    /// Kinds of things made or found during play; their ids follow the
    /// world's things.
    pub made: Vec<String>,
    /// How loud the player is being right now (0–3).
    pub noise: u8,
    /// Need states last told to the player.
    pub felt: Vec<usize>,
    pub dead: Option<Death>,
    /// Texts scraped: history's casts, and the player's. Nothing is ever
    /// removed from this set or from any surface (nothing is lost).
    pub scraped: BTreeSet<usize>,
    /// Writing tools the player has come across (for the first-find note).
    pub found: BTreeSet<String>,
    /// Surfaces cleaned of moss, soot or dust (D10).
    #[serde(default)]
    pub cleaned: BTreeSet<usize>,
    /// What the player has written, in order. These layers enter history.
    pub written: Vec<composing::Written>,
    /// Roots the player has met, and in which texts (tracked silently).
    pub encountered: BTreeMap<String, BTreeSet<usize>>,
    /// The regions' slow variables.
    pub regions: scraped_sim::region::RegionState,
    /// Texts the player released, with the power of the scraper used.
    pub released: BTreeMap<usize, u8>,
    /// How each outdoor place looked when last seen: regional bands by cell.
    pub memory: BTreeMap<String, Vec<String>>,
    /// Buildings entered.
    #[serde(default)]
    pub visited: BTreeSet<usize>,
    /// Texts read (in part or whole).
    #[serde(default)]
    pub read: BTreeSet<usize>,
    /// What the player's scrapes did to the regional pushes.
    #[serde(default)]
    pub acts: Vec<ending::Act>,
    /// What storylets have done.
    #[serde(default)]
    pub story: storylets::StoryState,
    /// What the player has been told about each thing, by fact key: the
    /// attention model's memory (D02).
    #[serde(default)]
    pub told: BTreeMap<String, attention::Told>,
    /// Hidden ways found, by (structure, link) (D04).
    #[serde(default)]
    pub revealed: BTreeSet<(usize, usize)>,
    /// Rooms the player has marked (chalk, scratches), by (structure, room).
    #[serde(default)]
    pub marks: BTreeSet<(usize, usize)>,
    /// Rooms the player has stood in, by (structure, room): what they can
    /// find their way back to.
    #[serde(default)]
    pub rooms_seen: BTreeSet<(usize, usize)>,
}

/// A brief, machine-readable summary of what the player can perceive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub place: String,
    pub things: Vec<String>,
    pub carried: Vec<String>,
    pub exits: Vec<String>,
    /// Things here that can be worked: doors, levers, sluices.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mechanisms: Vec<String>,
    pub minutes: u32,
    /// Landmarks in view, most salient first (outdoors).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub landmarks: Vec<Sighting>,
    /// Edges nearby (outdoors), by kind id.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<String>,
    /// Weather and light outdoors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<String>,
    /// The last journey as the player perceived it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub travelled: Option<Travelled>,
    /// What the last `write` did.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrote: Option<composing::WriteReport>,
    /// Whether the last `scrape` released anything the player could feel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scraped: Option<bool>,
    /// The body's needs, by coarse state.
    pub body: BTreeMap<String, String>,
    /// "daylight", "dim" or "dark".
    pub light: String,
    /// Weight carried, and the most that can be.
    pub load: (u32, u32),
    /// How the run ended, if it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dead: Option<String>,
}

/// A landmark as the player sees it: rough bearing and distance only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sighting {
    pub name: String,
    pub bearing: String,
    pub distance: String,
}

/// A journey as the player perceived it (drift included).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Travelled {
    /// Compass point of the whole journey, or empty if back where it began.
    pub bearing: String,
    pub metres: i64,
    pub minutes: i64,
}

/// The response to one command.
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    /// Exactly what a human player sees.
    pub text: String,
    pub state: Summary,
    /// Ground truth (only filled when asked for: spoilers).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth: Option<Json>,
    /// The slot renders behind the text (only when `Game::trace` is set).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub renders: Vec<Rendered>,
}

pub mod saves;
pub mod shared;

/// A saved game: the world's seed and settings, the moves played, and
/// (C01) a snapshot of play to load from, the builds it was played on, a
/// turn counter and a chain over the moves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Save {
    pub seed: u64,
    pub pack_version: String,
    pub commands: Vec<String>,
    /// The previous run's legacy this world was made with, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<ending::Legacy>,
    /// The difficulty preset.
    #[serde(default = "standard")]
    pub difficulty: String,
    /// The engine that made the world, and the one that last wrote it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub created: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub engine: String,
    /// Moves played, and the chain over them (C01: no going back).
    #[serde(default)]
    pub turn: usize,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chain: String,
    /// The builds each stretch of play ran on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<saves::Stretch>,
    /// Play as it stands, loaded instead of replaying the moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<Box<saves::Snapshot>>,
}

fn standard() -> String {
    "standard".to_string()
}

/// A game in progress.
pub struct Game {
    pub site: Site,
    registry: Registry,
    pack: Pack,
    pub state: State,
    memory: BTreeMap<String, String>,
    log: Vec<String>,
    /// Whether outputs include ground truth.
    pub spoil: bool,
    /// Every fact the attention model weighed this command, said or not,
    /// for the JSON protocol's truth (only gathered when spoiling), so
    /// agents aren't limited by the human budget.
    attended: Vec<Json>,
    /// Debug and tests: fixed weather and light outdoors.
    pub forced: Option<(&'static str, &'static str)>,
    glyph_cache: BTreeMap<(u32, usize), String>,
    /// Every sign's impression, per era, as rendered (S01).
    /// Each era's sign impressions, and how many of its signs were heard
    /// when they were made (a related sign names a heard base by sound).
    impression_cache: BTreeMap<u32, (usize, Vec<String>)>,
    /// Whether the last command was reading, so `look closer` reads on.
    reading_now: bool,
    /// A scrape done by cleaning with a tool: told as cleaning (S04).
    cleaning: bool,
    /// Facts already said in this response before the description (a move
    /// report inside), taken off its budget (D04).
    said_already: usize,
    /// For the depth metrics: per species, the minute its first sign and
    /// the minute it was first seen were put before the player (D06).
    pub life_log: BTreeMap<usize, [Option<u32>; 2]>,
    /// The facts about where a journey just arrived, said first (S01).
    arrival_keys: Vec<String>,
    /// Whether the last command was `listen`, and so whether the player
    /// is listening during this one (S01: signs' sounds while scraping).
    listening: bool,
    attentive: bool,
    last_travel: Option<Travelled>,
    /// Things made during play (ids after the world's things).
    extra: Vec<Thing>,
    /// What happened while time passed, for the next output.
    notes: Vec<String>,
    /// Something happened that should stop a journey or sleep.
    interrupted: bool,
    /// Claims of live writing, recomputed whenever something is scraped.
    claims: Vec<Claim>,
    /// Texts the player has written, rebuilt from the state.
    player_texts: Vec<scraped_world::texts::Text>,
    /// How many texts a root must be met in before it can be written.
    pub threshold: usize,
    last_write: Option<composing::WriteReport>,
    last_scrape: Option<usize>,
    /// Whether the last scrape changed anything the player could feel.
    scrape_felt: bool,
    /// Regional pushes acting now.
    drivers: Vec<scraped_sim::region::Driver>,
    /// What moved the regions on the last day stepped (debug).
    causes: Vec<scraped_sim::region::Cause>,
    /// The pushes acting when play began: what the world does alone.
    initial_drivers: Vec<scraped_sim::region::Driver>,
    /// Commands and what the game said, for the notebook.
    transcript: Vec<(String, String)>,
    /// Whether the end of the run has been summed up yet.
    summarised: bool,
    legacy: Option<ending::Legacy>,
    /// Where the pack's storylets were placed in this world.
    pub placed: Vec<storylets::Placed>,
    /// Beats of the spine announced so far.
    pub hooks_seen: BTreeSet<String>,
    /// The storylets' rules, fixed when the world was made: a hot reload
    /// of the pack changes their text, never where or when they happen.
    storylets: Vec<scraped_content::Storylet>,
    /// The difficulty preset the world was made at.
    pub preset: String,
    /// Whether outputs list the slot renders that made them.
    pub trace: bool,
    /// Slot renders since the last command began (or the whole run, when
    /// `keep_renders` is set), with their variables.
    renders: Vec<Rendered>,
    pub keep_renders: bool,
    /// Bots that measure the late game rather than survival keep the
    /// body well after every command. Never set in play. // DEBUG-TEXT
    pub sustain: bool,
    /// The way last gone through inside, so `follow` keeps on (D04).
    last_link: Option<usize>,
    /// A light that never fails, for bots that measure places (D04).
    pub forced_light: bool,
    /// The builds this world was played on before this one, and the engine
    /// that made it (C01).
    history: Vec<saves::Stretch>,
    created: String,
}

/// A slot rendered during play, with the variables it was given: what the
/// authoring tool needs to answer "why did I see this?".
#[derive(Debug, Clone, Serialize)]
pub struct Rendered {
    #[serde(flatten)]
    pub trace: scraped_content::Trace,
    pub vars: Context,
    /// Game time, in minutes.
    pub minutes: u32,
    /// The renderer's seed, so the text can be rendered again exactly.
    pub seed: u64,
}

/// One glyph of a reading, or a gap between words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    Glyph {
        era: u32,
        index: usize,
        /// Scraped away: the reader can't make it out.
        lost: bool,
    },
    Gap,
    /// Between separate pieces of writing on one thing.
    Break,
}

impl Game {
    /// A new game for `seed`, with Jb's content.
    pub fn new(seed: u64, pack: Pack) -> Self {
        Self::with_legacy(seed, pack, None)
    }

    /// A new game in a world carrying a previous run's legacy.
    pub fn with_legacy(seed: u64, pack: Pack, legacy: Option<ending::Legacy>) -> Self {
        Self::create(seed, pack, "standard", legacy)
    }

    /// A new game at a difficulty preset ("gentle", "standard",
    /// "archaeologist"), with a previous run's legacy if any.
    pub fn create(seed: u64, pack: Pack, preset: &str, legacy: Option<ending::Legacy>) -> Self {
        let preset = if scraped_lang::difficulty::PRESETS.contains(&preset) {
            preset
        } else {
            "standard"
        };
        let mut site = Site::create(seed, preset, legacy.as_ref().map(|l| &l.meaning));
        let placed = storylets::place(&mut site, &pack);
        let state = State {
            place: Place::Outside,
            carried: Vec::new(),
            moved: BTreeMap::new(),
            dropped: BTreeMap::new(),
            pos: site.start(),
            seen: BTreeSet::new(),
            trail: Vec::new(),
            names: Vec::new(),
            minutes: 8 * 60,
            it: None,
            reading: None,
            last_read: None,
            heard: BTreeSet::new(),
            snares: Vec::new(),
            foul_since: None,
            opened: BTreeSet::new(),
            uncovered: BTreeSet::new(),
            unlocked: BTreeSet::new(),
            tracing: None,
            doors: BTreeMap::new(),
            pending: None,
            sim: SimState::default(),
            body: Body::default(),
            creatures: site.fixtures.creatures.iter().map(Creature::new).collect(),
            lit: BTreeSet::new(),
            fuel: BTreeMap::new(),
            water: BTreeMap::new(),
            worn: BTreeSet::new(),
            gone: BTreeSet::new(),
            made: Vec::new(),
            noise: 0,
            felt: vec![0; 6],
            dead: None,
            scraped: site.writing.scraped.clone(),
            found: BTreeSet::new(),
            cleaned: BTreeSet::new(),
            written: Vec::new(),
            encountered: BTreeMap::new(),
            regions: site.regions.initial.clone(),
            released: BTreeMap::new(),
            memory: BTreeMap::new(),
            visited: BTreeSet::new(),
            read: BTreeSet::new(),
            acts: Vec::new(),
            story: storylets::StoryState::default(),
            told: BTreeMap::new(),
            revealed: BTreeSet::new(),
            marks: BTreeSet::new(),
            rooms_seen: BTreeSet::new(),
        };
        let claims = site
            .writing
            .live_claims(&site.world, &site.land, &state.scraped);
        let mut g = Game {
            site,
            registry: slots::registry_for(&pack),
            pack,
            state,
            memory: BTreeMap::new(),
            log: Vec::new(),
            spoil: false,
            attended: Vec::new(),
            forced: None,
            glyph_cache: BTreeMap::new(),
            impression_cache: BTreeMap::new(),
            reading_now: false,
            cleaning: false,
            listening: false,
            attentive: false,
            arrival_keys: Vec::new(),
            said_already: 0,
            life_log: BTreeMap::new(),
            last_travel: None,
            extra: Vec::new(),
            notes: Vec::new(),
            interrupted: false,
            claims,
            player_texts: Vec::new(),
            threshold: composing::THRESHOLD,
            last_write: None,
            last_scrape: None,
            scrape_felt: false,
            drivers: Vec::new(),
            causes: Vec::new(),
            initial_drivers: Vec::new(),
            transcript: Vec::new(),
            summarised: false,
            legacy,
            placed,
            hooks_seen: BTreeSet::new(),
            storylets: Vec::new(),
            preset: preset.to_string(),
            trace: false,
            renders: Vec::new(),
            keep_renders: false,
            sustain: false,
            last_link: None,
            forced_light: false,
            history: Vec::new(),
            created: String::new(),
        };
        g.storylets = g.pack.storylets().cloned().collect();
        g.recompute_drivers();
        g.initial_drivers = g.drivers.clone();
        g.site.regions.initial = g.site.regions.settle(&g.drivers, 720);
        g.state.regions = g.site.regions.initial.clone();
        g
    }

    pub fn seed(&self) -> u64 {
        self.site.world.seed
    }

    /// Whether outputs carry ground truth: never in the player build (C01),
    /// where the `spoilers` feature is left out.
    pub fn spoiling(&self) -> bool {
        cfg!(feature = "spoilers") && self.spoil
    }

    /// The opening text.
    pub fn start(&mut self) -> Output {
        // The opening beat replaces the bare wake-up when Jb has written it.
        let mut intro = self.hook("opening", "", "");
        if intro.is_empty() {
            intro.push(self.say(
                "say.intro",
                ctx(&[("biome", Value::from(self.site.biome()))]),
            ));
        }
        let look = self.look();
        intro.push(look);
        self.output(intro, None)
    }

    /// The content pack's version.
    pub fn pack_version(&self) -> String {
        self.pack.version()
    }

    /// The code, said through its slot.
    pub fn say_code(&mut self, code: &str) -> String {
        self.say("say.seed_code", ctx(&[("code", Value::from(code))]))
    }

    /// The shareable code for this world (seed, difficulty, pack).
    pub fn seed_code(&self) -> String {
        seedcode::encode(self.seed(), &self.preset, &self.pack.version())
    }

    /// Saves the game: its seed, settings and moves, a snapshot of play,
    /// and the builds it has been played on (C01).
    pub fn save(&self) -> Save {
        let mut history = self.history.clone();
        saves::extend(
            &mut history,
            saves::ENGINE,
            &self.pack.version(),
            self.log.len(),
        );
        Save {
            seed: self.seed(),
            pack_version: self.pack.version(),
            commands: self.log.clone(),
            legacy: self.legacy.clone(),
            difficulty: self.preset.clone(),
            created: if self.created.is_empty() {
                saves::ENGINE.to_string()
            } else {
                self.created.clone()
            },
            engine: saves::ENGINE.to_string(),
            turn: self.log.len(),
            chain: saves::chain(&self.log),
            history,
            snapshot: Some(Box::new(self.snapshot())),
        }
    }

    /// Loads a save: from its snapshot where it has one, else by replaying
    /// its moves. Also returns whether the content pack differs from the
    /// one the save was made with.
    pub fn load(save: &Save, pack: Pack) -> (Self, bool) {
        let changed = save.pack_version != pack.version();
        let mut g = Game::create(save.seed, pack, &save.difficulty, save.legacy.clone());
        match &save.snapshot {
            Some(snap) => g.restore(snap, &save.commands),
            None => {
                g.start();
                for c in &save.commands {
                    g.step(c);
                }
            }
        }
        g.history = save.history.clone();
        g.created = save.created.clone();
        (g, changed)
    }

    /// Opens a save in a client (C01): refuses one this version cannot carry
    /// on (see [`saves::may_open`]), and returns the loaded game with the
    /// text that says so: loaded, and any upgrade or change of wording.
    pub fn open(save: &Save, pack: Pack) -> Result<(Self, String), String> {
        let upgraded = saves::may_open(&save.engine)?;
        let (mut g, changed) = Game::load(save, pack);
        let mut text = g.message("say.loaded");
        if upgraded {
            let c = ctx(&[
                ("from", Value::from(save.engine.as_str())),
                ("to", Value::from(saves::ENGINE)),
            ]);
            text.push_str("\n\n");
            text.push_str(&g.say("say.upgraded", c));
        }
        if changed {
            text.push_str("\n\n");
            text.push_str(&g.message("say.pack_changed"));
        }
        Ok((g, text))
    }

    /// The current state summary with no text, for session-level replies.
    pub fn step_quiet(&mut self) -> Output {
        self.output(Vec::new(), None)
    }

    /// Text for a game-level event the client handles (saved, loaded…).
    pub fn message(&mut self, slot: &str) -> String {
        self.say(slot, Context::new())
    }

    // ---------- rendering ----------

    fn renderer_say(&mut self, slot: &str, c: &Context, seed: u64, keep_memory: bool) -> String {
        let lang = &self.site.world.languages[0];
        let hooks = LangHooks { lang };
        let mut r = Renderer::new(&self.registry, &self.pack, seed, &hooks);
        if keep_memory {
            r.set_memory(std::mem::take(&mut self.memory));
        }
        let out = r.render(slot, c);
        if keep_memory {
            self.memory = r.memory();
        }
        let minutes = self.state.minutes;
        let traces = std::mem::take(&mut r.trace);
        drop(r);
        self.renders
            .extend(traces.into_iter().map(|trace| Rendered {
                trace,
                vars: c.clone(),
                minutes,
                seed,
            }));
        out
    }

    /// Replaces the content pack mid-game: text changes at once; the game's
    /// state, and where and when storylets happen, do not.
    pub fn set_pack(&mut self, pack: Pack) {
        let mut reg_pack = pack.clone();
        // Slots for the storylets this world was made with, whatever the
        // new pack says.
        reg_pack.files.push(scraped_content::PackFile {
            path: String::new(),
            notes: None,
            variants: Vec::new(),
            storylets: self.storylets.clone(),
        });
        self.registry = slots::registry_for(&reg_pack);
        self.pack = pack;
        self.glyph_cache.clear();
        self.impression_cache.clear();
    }

    /// The renders kept so far (see `keep_renders`).
    pub fn renders(&self) -> &[Rendered] {
        &self.renders
    }

    /// Renders a description; variants may vary from turn to turn.
    /// The command as the player typed it, to quote back ("to head north",
    /// not "to head"; S04).
    /// The building whose door the player stands at (S05): within 30 m,
    /// and nearer than any other.
    fn building_at_hand(&self, buildings: &[Target]) -> Option<Target> {
        let mut near: Vec<(f64, Target)> = buildings
            .iter()
            .filter_map(|&b| match b {
                Target::Structure(s) => {
                    Some((self.site.land.structure_pos[s].dist(self.state.pos), b))
                }
                _ => None,
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        match near.as_slice() {
            [(d, b), rest @ ..] if *d <= 30.0 && rest.first().is_none_or(|r| r.0 > *d) => Some(*b),
            _ => None,
        }
    }

    pub(crate) fn typed(&self, verb: &str) -> String {
        self.log
            .last()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| verb.to_string())
    }

    fn say(&mut self, slot: &str, mut c: Context) -> String {
        // The player's own words quoted back lose their article: "You see no
        // sinkhole here", not "no the sinkhole" (S04).
        if let Some(Value::Text(w)) = c.get("words") {
            let bare = ["the ", "a ", "an ", "some "]
                .iter()
                .find_map(|a| w.strip_prefix(a))
                .map(str::to_string);
            if let Some(b) = bare {
                c.insert("words".into(), Value::from(b));
            }
        }
        let seed = self.seed() ^ u64::from(self.state.minutes);
        self.renderer_say(slot, &c, seed, true)
    }

    /// Renders a name or other text that must never change, so the player
    /// (and the parser) can rely on it.
    fn stable(&mut self, slot: &str, c: Context, key: u64) -> String {
        let seed = self.seed() ^ key.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        self.renderer_say(slot, &c, seed, false)
    }

    fn thing_name(&mut self, id: usize) -> String {
        let c = self.site.thing_vars(self.thing(id));
        self.stable("thing.name", c, 1_000 + id as u64)
    }

    fn structure_name(&mut self, id: usize) -> String {
        let c = self.site.structure_vars(id);
        self.stable("place.structure", c, 500_000 + id as u64)
    }

    fn way_name(&mut self, w: &Way) -> String {
        let c = self.site.way_vars(w);
        let key = match w.to {
            Place::Room { structure, room } => 900_000 + structure as u64 * 64 + room as u64,
            Place::Outside => 0,
        };
        self.stable("place.exit", c, key)
    }

    fn glyph_description(&mut self, era: u32, index: usize) -> String {
        if let Some(d) = self.glyph_cache.get(&(era, index)) {
            return d.clone();
        }
        let lang = &self.site.world.languages[era as usize];
        let glyph = lang.script.glyphs[index].1.clone();
        let hooks = LangHooks { lang };
        let seed = self.seed() ^ (u64::from(era) << 32 | index as u64);
        let mut r = Renderer::new(&self.registry, &self.pack, seed, &hooks);
        let d = describe_glyph(&mut r, &glyph);
        let traces = std::mem::take(&mut r.trace);
        drop(r);
        let minutes = self.state.minutes;
        self.renders
            .extend(traces.into_iter().map(|trace| Rendered {
                trace,
                vars: Context::new(),
                minutes,
                seed,
            }));
        self.glyph_cache.insert((era, index), d.clone());
        d
    }

    // ---------- the world as the player sees it ----------

    /// Ways out of the current place, with doors the player has moved.
    pub(crate) fn ways(&self) -> Vec<Way> {
        self.ways_at(self.state.place)
    }

    /// Ways out of a room the player knows of, with doors as they stand.
    pub(crate) fn ways_at(&self, place: Place) -> Vec<Way> {
        let Place::Room { structure, .. } = place else {
            return Vec::new();
        };
        let held = self.doors_held();
        self.site
            .ways(place)
            .into_iter()
            .filter(|w| !w.hidden || self.state.revealed.contains(&(structure, w.link)))
            .map(|mut w| {
                match self.state.doors.get(&format!("{structure}:{}", w.link)) {
                    Some(true) if w.state == PassageState::Closed => w.state = PassageState::Open,
                    Some(false) if w.state == PassageState::Open => w.state = PassageState::Closed,
                    _ => {}
                }
                if (self.state.sim.opened.contains(&(structure, w.link))
                    || self.site.fixtures.cleared.contains(&(structure, w.link)))
                    && w.state == PassageState::Blocked
                {
                    w.state = PassageState::Open;
                }
                if self.state.sim.fallen.contains(&(structure, w.link)) {
                    w.state = PassageState::Blocked;
                }
                // A gate the works raise, or a door with its day (D05).
                if self.sealed(structure, w.link).is_some() && w.state == PassageState::Open {
                    w.state = PassageState::Closed;
                }
                if w.passage == Some(scraped_world::structures::Passage::Door) && !w.collapsed {
                    match held {
                        Some(h) if h > 0 && w.state == PassageState::Closed => {
                            w.state = PassageState::Open
                        }
                        Some(h) if h < 0 && w.state == PassageState::Open => {
                            w.state = PassageState::Closed
                        }
                        _ => {}
                    }
                }
                w
            })
            .collect()
    }

    fn where_is(&self, thing: usize) -> Option<Place> {
        if self.state.carried.contains(&thing) || self.state.gone.contains(&thing) {
            return None;
        }
        Some(
            *self
                .state
                .moved
                .get(&thing)
                .unwrap_or(&self.thing(thing).home),
        )
    }

    fn here(&self) -> Vec<usize> {
        (0..self.thing_count())
            .filter(|&t| {
                self.where_is(t) == Some(self.state.place)
                    && (self.state.place != Place::Outside || self.local(self.thing_pos(t)))
                    && self.in_sight(t)
            })
            .collect()
    }

    fn thing_pos(&self, t: usize) -> Pos {
        *self.state.dropped.get(&t).unwrap_or(&self.thing(t).pos)
    }

    fn visible_targets(&mut self) -> Vec<Candidate<Target>> {
        let mut out = Vec::new();
        let seen: Vec<usize> = if self.is_dark() {
            Vec::new()
        } else {
            self.here()
        };
        for t in seen.into_iter().chain(self.state.carried.clone()) {
            let name = self.thing_name(t);
            let kind = self.thing(t).kind.replace('_', " ");
            out.push(Candidate::new(Target::Thing(t), &name, &[&kind]));
        }
        for m in self.mechanisms_here() {
            let name = self.mech_name(m);
            let kind = label(&self.site.fixtures.mechanisms[m].kind).replace('_', " ");
            out.push(Candidate::new(Target::Mechanism(m), &name, &[&kind]));
        }
        if self.fire_here_pub() {
            let name = self.fire_name();
            out.push(Candidate::new(Target::Fire, &name, &["fire"]));
        }
        if !self.is_dark() {
            out.extend(self.sign_targets());
        }
        for w in self.ways() {
            let name = self.way_name(&w);
            let dir = label(&w.exit);
            out.push(Candidate::new(Target::Way(w.link), &name, &[&dir]));
        }
        if self.state.place == Place::Outside {
            for s in self.local_structures() {
                let name = self.structure_name(s);
                let kind = label(&self.site.structure(s).kind);
                out.push(Candidate::new(Target::Structure(s), &name, &[&kind]));
            }
            out.extend(self.outdoor_targets());
            out.extend(self.place_targets());
        } else {
            out.extend(self.room_targets());
        }
        // Alike things together, by their plural ("the tombs", "the jars").
        let mut groups: BTreeMap<(bool, usize), usize> = BTreeMap::new();
        // Alike buildings in one condition, when the group is mixed.
        let mut by_condition: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        if self.state.place == Place::Outside {
            for s in self.local_structures() {
                let kind = label(&self.site.structure(s).kind);
                let condition = label(&self.site.structure(s).condition);
                if let Some(k) = slots::STRUCTURES.iter().position(|x| *x == kind) {
                    *groups.entry((true, k)).or_default() += 1;
                    if let Some(c) = slots::CONDITIONS.iter().position(|x| *x == condition) {
                        *by_condition.entry((k, c)).or_default() += 1;
                    }
                }
            }
        }
        for (&(k, c), &n) in &by_condition {
            let all = groups.get(&(true, k)).copied().unwrap_or(0);
            if all < 2 || n == all {
                continue;
            }
            let target = Target::Buildings(k, Some(c));
            let name = self.target_name(target);
            let many = senses::plural(slots::STRUCTURES[k]);
            let condition = slots::CONDITIONS[c];
            out.push(Candidate::new(target, &name, &[condition, &many, "ones"]));
        }
        if !self.is_dark() {
            for t in self.here() {
                if let Some(k) = slots::kinds().iter().position(|x| *x == self.thing(t).kind) {
                    *groups.entry((false, k)).or_default() += 1;
                }
            }
        }
        for ((building, k), n) in groups {
            if n < 2 {
                continue;
            }
            let (target, kind) = if building {
                (Target::Buildings(k, None), slots::STRUCTURES[k])
            } else {
                (Target::Things(k), slots::kinds()[k])
            };
            let name = self.target_name(target);
            let many = senses::plural(kind);
            out.push(Candidate::new(target, &name, &[&many, "ones"]));
        }
        out
    }

    /// Describes the place on arriving: a room as it's entered, or the
    /// land on stepping out or ending a journey (D02's attention model).
    fn look(&mut self) -> String {
        if let Place::Room { structure, room } = self.state.place {
            self.state.rooms_seen.insert((structure, room));
        }
        let r = match self.state.place {
            Place::Outside => attention::Response::Arrival,
            Place::Room { .. } => attention::Response::Room,
        };
        self.describe(r)
    }

    fn summary(&mut self) -> Summary {
        let place = match self.state.place {
            Place::Outside => "outside".to_string(),
            Place::Room { structure, room } => format!("structure {structure} room {room}"), // DEBUG-TEXT: machine-readable id
        };
        let things = self
            .here()
            .into_iter()
            .map(|t| self.thing_name(t))
            .collect();
        let carried = self
            .state
            .carried
            .clone()
            .into_iter()
            .map(|t| self.thing_name(t))
            .collect();
        let exits = match self.state.place {
            Place::Outside => self
                .local_structures()
                .into_iter()
                .map(|s| self.structure_name(s))
                .collect(),
            // Ways that can be gone through from here: not windows, and not
            // the far side of a one-way way (D04).
            // Several ways one way are each given by name ("the western
            // door north"), so each can be told apart (S01).
            _ => {
                let ways: Vec<Way> = self
                    .ways()
                    .into_iter()
                    .filter(|w| w.passage != Some(Passage::Window) && !w.against)
                    .collect();
                let mut v: Vec<String> = Vec::new();
                for w in &ways {
                    let dir = label(&w.exit);
                    let alike = ways.iter().filter(|o| o.exit == w.exit).count();
                    let e = if alike > 1 { self.way_name(w) } else { dir };
                    if !v.contains(&e) {
                        v.push(e);
                    }
                }
                v
            }
        };
        let (landmarks, edges, weather) = if self.state.place == Place::Outside {
            self.outdoor_summary()
        } else {
            (Vec::new(), Vec::new(), None)
        };
        let light = self
            .env()
            .local(self.spot(), self.state.minutes, self.carried_light())
            .light
            .to_string();
        let mechanisms = if self.is_dark() {
            Vec::new()
        } else {
            self.mechanisms_here()
                .into_iter()
                .map(|m| self.mech_name(m))
                .collect()
        };
        Summary {
            place,
            things,
            carried,
            exits,
            mechanisms,
            minutes: self.state.minutes,
            landmarks,
            edges,
            weather,
            travelled: self.last_travel.take(),
            wrote: self.last_write.take(),
            scraped: self.last_scrape.take().map(|_| self.scrape_felt),
            body: self
                .state
                .body
                .states()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            light,
            load: (self.weight(), scraped_sim::items::CARRY),
            dead: self.state.dead.as_ref().map(|d| d.cause.clone()),
        }
    }

    fn output(&mut self, parts: Vec<String>, truth: Option<Json>) -> Output {
        let mut parts = parts;
        self.run_storylets();
        parts.extend(self.take_notes());
        if let Place::Room { structure, .. } = self.state.place {
            self.state.visited.insert(structure);
        }
        let mut truth = truth;
        if self.state.dead.is_some() && !self.summarised {
            self.summarised = true;
            let ending = self.ending().unwrap_or("death").to_string();
            parts.extend(self.hook("ending", "", &ending));
            parts.extend(self.summary_parts());
            truth = Some(self.end_truth());
        }
        let text = parts
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let state = self.summary();
        let truth = truth.or_else(|| {
            Some(json!({
                "pos": [self.state.pos.x, self.state.pos.y],
                "cell": self.state.pos.cell(),
                "town": self.site.land.town(&self.site.world, self.state.pos),
                "claims": self.claims_here(),
                "understanding": self.understanding(),
                "regions": self.regions_truth(),
                "storylets": self.storylets_truth(),
            }))
        });
        let truth = truth.map(|mut t| {
            if !self.attended.is_empty() {
                t["attention"] = Json::from(std::mem::take(&mut self.attended));
            }
            t
        });
        if let Some(cmd) = self.log.last() {
            if self.transcript.len() < self.log.len() {
                self.transcript.push((cmd.clone(), text.clone()));
            }
        }
        Output {
            text,
            state,
            truth: if self.spoiling() { truth } else { None },
            renders: if self.trace && cfg!(feature = "spoilers") {
                self.renders.clone()
            } else {
                Vec::new()
            },
        }
    }

    // ---------- commands ----------

    /// Runs one command and describes the result.
    pub fn step(&mut self, input: &str) -> Output {
        if !self.keep_renders {
            self.renders.clear();
        }
        self.attended.clear();
        self.log.push(input.to_string());
        self.sync_made();
        self.sync_written();
        self.state.noise = 0;
        if self.state.dead.is_some() {
            return self.ended();
        }
        let parsed = parser::parse(input);
        // While a "which one?" question is open, anything that is not a
        // command is taken as the answer.
        if let Some(pending) = self.state.pending.take() {
            if matches!(parsed, Err(ParseError::UnknownVerb(_))) {
                let words: Vec<String> = input
                    .to_lowercase()
                    .split_whitespace()
                    .map(str::to_string)
                    .collect();
                let cands: Vec<Candidate<Target>> = self
                    .visible_targets()
                    .into_iter()
                    .filter(|c| pending.options.contains(&c.target))
                    .collect();
                if let Resolution::One(t) = resolve(&words, &cands, None) {
                    return self.act(&pending.verb, t);
                }
            }
        }
        let cmd = match parsed {
            Ok(c) => c,
            Err(ParseError::Empty) => {
                let t = self.say("say.empty", Context::new());
                return self.output(vec![t], None);
            }
            Err(ParseError::UnknownVerb(w)) => {
                let t = self.say("say.unknown_verb", ctx(&[("word", Value::from(w))]));
                return self.output(vec![t], None);
            }
        };
        self.command(cmd)
    }

    fn pass(&mut self, minutes: u32) {
        self.advance(minutes, Activity::Resting);
    }

    fn command(&mut self, mut cmd: Command) -> Output {
        // "enter" alone is "go in" (S04).
        if cmd.verb == "go"
            && cmd.words.is_empty()
            && self
                .log
                .last()
                .is_some_and(|l| l.trim().eq_ignore_ascii_case("enter"))
        {
            cmd.words = vec!["in".to_string()];
        }
        self.breathe();
        let was_reading = std::mem::take(&mut self.reading_now);
        self.attentive = std::mem::take(&mut self.listening);
        let rule = parser::verbs()
            .iter()
            .find(|v| v.id == cmd.verb)
            .map(|v| v.object);
        if rule == Some(parser::ObjectRule::Required) && cmd.words.is_empty() {
            let t = self.say(
                "say.need_object",
                ctx(&[("verb", Value::from(cmd.verb.as_str()))]),
            );
            return self.output(vec![t], None);
        }
        match cmd.verb.as_str() {
            "look" => {
                self.pass(1);
                let t = self.describe(attention::Response::Look);
                self.output(vec![t], None)
            }
            "inventory" => {
                let items: Vec<Value> = self
                    .state
                    .carried
                    .clone()
                    .into_iter()
                    .map(|t| Value::from(self.thing_name(t)))
                    .collect();
                let n = items.len() as i64;
                let t = self.say(
                    "say.inventory",
                    ctx(&[("items", Value::List(items)), ("count", Value::Number(n))]),
                );
                self.output(vec![t], None)
            }
            "take" if cmd.words.first().map(String::as_str) == Some("all") => {
                self.take_all(&cmd.words[1..])
            }
            "drop" if cmd.words.first().map(String::as_str) == Some("all") => {
                let carried = self.state.carried.clone();
                if carried.is_empty() {
                    let t = self.say(
                        "say.inventory",
                        ctx(&[
                            ("items", Value::List(Vec::new())),
                            ("count", Value::Number(0)),
                        ]),
                    );
                    return self.output(vec![t], None);
                }
                let mut parts = Vec::new();
                for t in carried {
                    let o = self.act("drop", Target::Thing(t));
                    if !o.text.trim().is_empty() {
                        parts.push(o.text);
                    }
                }
                self.output(parts, None)
            }
            "wait" => self.wait(&cmd.words),
            "manual" => {
                let section = cmd
                    .words
                    .iter()
                    .find(|w| slots::MANUAL.contains(&w.as_str()))
                    .map_or("contents", String::as_str)
                    .to_string();
                let t = self.say("manual.page", ctx(&[("section", Value::from(section))]));
                self.output(vec![t], None)
            }
            "help" => {
                let t = self.say("say.help", Context::new());
                self.output(vec![t], None)
            }
            "more" => self.read_closely(&cmd.words),
            "out" => self.go_out(),
            "north" | "south" | "east" | "west" | "up" | "down" | "northeast" | "northwest"
            | "southeast" | "southwest" => self.go_dir(&cmd.verb),
            "head" => self.head(&cmd.words),
            // Inside, `follow the passage` runs on through it (D04).
            "follow" if self.state.place != Place::Outside => self.follow_passage(),
            "follow" => self.follow(&cmd.words),
            "mark" => self.mark_here(),
            "dig" => self.dig(),
            "back" => self.go_back(),
            "name" => self.name_place(&cmd.words),
            "light" => self.light(&cmd.words),
            "extinguish" => self.extinguish(&cmd.words),
            "drink" => self.drink(&cmd.words),
            "eat" => self.eat(&cmd.words),
            "sleep" => self.sleep(),
            "forage" => self.forage(),
            "fish" => self.fish(),
            "snare" => self.snare(),
            "gather" => self.gather(),
            "make" => self.make(&cmd.words),
            "feed" => self.feed_fire(),
            "shout" => self.shout(),
            "cross" => self.cross(&cmd.words),
            "status" => self.status(),
            "beyond" => self.go_beyond(),
            "look_around" => {
                self.pass(2);
                let r = if self.state.place == Place::Outside {
                    attention::Response::Around
                } else {
                    attention::Response::Closer
                };
                let t = self.describe(r);
                self.output(vec![t], None)
            }
            // While reading, looking closer reads on (S01).
            "look_closer" if was_reading && self.state.reading.is_some() => self.page(),
            "look_closer" => {
                self.pass(3);
                let t = self.describe(attention::Response::Closer);
                self.output(vec![t], None)
            }
            "look_up" => self.look_up(),
            "look_down" => self.look_down(),
            "listen" => self.listen(),
            "smell" => self.smell(),
            "taste" => self.taste(),
            "count" => self.count(&cmd.words),
            // The ground, the floor or the air: what's all around.
            "touch"
                if cmd.words.is_empty()
                    || cmd.words.iter().all(|w| {
                        matches!(
                            w.as_str(),
                            "the" | "ground" | "floor" | "earth" | "air" | "soil" | "dirt"
                        )
                    }) =>
            {
                self.touch(None)
            }
            "write" => self.write(&cmd.words),
            "go" => {
                // "go north" is a direction; anything else names a place.
                if let [d] = cmd.words.as_slice() {
                    if let Some(v) = parser::parse(d).ok().filter(|c| {
                        c.words.is_empty() && DIRECTION_VERBS.contains(&c.verb.as_str())
                    }) {
                        return self.go_dir(&v.verb);
                    }
                }
                if matches!(cmd.words.as_slice(), [w] if w == "out" || w == "outside") {
                    return self.go_out();
                }
                self.with_target("go", &cmd.words)
            }
            verb => self.with_target(verb, &cmd.words),
        }
    }

    fn with_target(&mut self, verb: &str, words: &[String]) -> Output {
        let mut cands = self.visible_targets();
        // You can only drop what you carry, and only take what you don't.
        let carried = self.state.carried.clone();
        let held =
            |c: &Candidate<Target>| matches!(c.target, Target::Thing(t) if carried.contains(&t));
        match verb {
            "drop" if cands.iter().any(held) => cands.retain(held),
            "take" if cands.iter().any(|c| !held(c)) => cands.retain(|c| !held(c)),
            _ => {}
        }
        // "another tomb", "the nearest tomb": the choosing word isn't part
        // of any name, so resolve without it (and any one will do, below).
        const CHOOSING: [&str; 6] = ["a", "an", "another", "any", "nearest", "closest"];
        let any_one = words.iter().any(|w| CHOOSING.contains(&w.as_str()));
        let bare: Vec<String> = words
            .iter()
            .filter(|w| !CHOOSING.contains(&w.as_str()))
            .cloned()
            .collect();
        // "go in", "enter" alone, out in the open (S04): the building here,
        // or which of them.
        let bare_in = verb == "go"
            && self.state.place == Place::Outside
            && matches!(
                words
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .as_slice(),
                ["in"] | ["inside"]
            );
        let buildings: Vec<Target> = cands
            .iter()
            .map(|c| c.target)
            .filter(|t| matches!(t, Target::Structure(_)))
            .collect();
        // A plural ("look at the walls", "take the jars", S04): each of them.
        if let Some(last) = words.last().filter(|w| w.len() > 3 && w.ends_with('s')) {
            if matches!(verb, "examine" | "take" | "read")
                && matches!(
                    resolve(words, &cands, self.state.it.as_ref()),
                    Resolution::None
                )
            {
                let stem = last
                    .strip_suffix("es")
                    .filter(|s| s.ends_with(['s', 'x', 'h']))
                    .or_else(|| last.strip_suffix('s'))
                    .unwrap_or(last);
                let mut single = words.to_vec();
                *single.last_mut().expect("a word") = stem.to_string();
                let found = match resolve(&single, &cands, None) {
                    Resolution::One(t) => vec![t],
                    Resolution::Many(ts) => ts,
                    Resolution::None => Vec::new(),
                };
                if !found.is_empty() {
                    let mut parts = Vec::new();
                    for t in found.into_iter().take(4) {
                        let o = self.act(verb, t);
                        if !o.text.trim().is_empty() {
                            parts.push(o.text);
                        }
                    }
                    return self.output(parts, None);
                }
            }
        }
        let resolved = match resolve(words, &cands, self.state.it.as_ref()) {
            _ if bare_in && buildings.len() == 1 => Resolution::One(buildings[0]),
            // The building just looked at (S05).
            _ if bare_in && self.state.it.is_some_and(|it| buildings.contains(&it)) => {
                Resolution::One(self.state.it.expect("checked"))
            }
            // The one stood before: come to by name, its door at hand.
            _ if bare_in && self.building_at_hand(&buildings).is_some() => {
                Resolution::One(self.building_at_hand(&buildings).expect("checked"))
            }
            _ if bare_in && buildings.len() > 1 => Resolution::Many(buildings.clone()),
            Resolution::None if any_one && !bare.is_empty() => {
                resolve(&bare, &cands, self.state.it.as_ref())
            }
            r => r,
        };
        match resolved {
            Resolution::One(t) => self.act(verb, t),
            Resolution::None if verb == "go" && self.state.place == Place::Outside => {
                let t = self.say(
                    "travel.unseen",
                    ctx(&[("words", Value::from(words.join(" ")))]),
                );
                self.output(vec![t], None)
            }
            // Indoors, going to something out in the open (S04): from the
            // entrance, out and on; from further in, say so.
            Resolution::None if verb == "go" && self.state.place != Place::Outside => {
                let here = self.state.place;
                self.state.place = Place::Outside;
                let outside = self.visible_targets();
                self.state.place = here;
                let seen = matches!(
                    resolve(words, &outside, None),
                    Resolution::One(_) | Resolution::Many(_)
                );
                if seen && matches!(here, Place::Room { room: 0, .. }) {
                    let out = self.go_out();
                    let on = self.with_target("go", words);
                    let text = [out.text, on.text]
                        .into_iter()
                        .filter(|t| !t.trim().is_empty())
                        .collect();
                    return self.output(text, None);
                }
                let slot = if seen { "say.indoors" } else { "say.not_here" };
                let t = self.say(slot, ctx(&[("words", Value::from(words.join(" ")))]));
                self.output(vec![t], None)
            }
            // In the dark, what can't be seen isn't "not here" (S04).
            Resolution::None if self.is_dark() => {
                let t = self.say(
                    "say.too_dark",
                    ctx(&[("words", Value::from(words.join(" ")))]),
                );
                self.output(vec![t], None)
            }
            Resolution::None => {
                let t = self.say(
                    "say.not_here",
                    ctx(&[("words", Value::from(words.join(" ")))]),
                );
                self.output(vec![t], None)
            }
            // "a tomb", "another tomb", "the nearest tomb": any one will do,
            // so take the nearest not yet been into.
            Resolution::Many(options) if any_one => {
                let p = self.state.pos;
                let pick = options
                    .iter()
                    .copied()
                    .min_by_key(|o| match *o {
                        Target::Structure(s) => (
                            self.state.visited.contains(&s),
                            self.site.land.structure_pos[s].dist2(p),
                        ),
                        Target::Thing(t) => {
                            (self.state.told.contains_key(&format!("examined:{t}")), 0)
                        }
                        _ => (true, i64::MAX),
                    })
                    .expect("several options");
                self.act(verb, pick)
            }
            Resolution::Many(options) => {
                // Things that look the same are as good as each other: one
                // of each look is offered, never two identical names (S01).
                let mut names: Vec<Value> = Vec::new();
                let mut kept: Vec<Target> = Vec::new();
                for o in options {
                    let name = match o {
                        Target::Landmark(i) => self.landmark_choice(i),
                        t => self.target_name(t),
                    };
                    let v = Value::from(name);
                    if !names.contains(&v) {
                        names.push(v);
                        kept.push(o);
                    }
                }
                if let [only] = kept.as_slice() {
                    return self.act(verb, *only);
                }
                let options = kept;
                self.state.pending = Some(Pending {
                    verb: verb.to_string(),
                    options,
                });
                let t = self.say("say.which", ctx(&[("options", Value::List(names))]));
                self.output(vec![t], None)
            }
        }
    }

    fn target_name(&mut self, t: Target) -> String {
        match t {
            Target::Thing(i) => self.thing_name(i),
            Target::Structure(i) => self.structure_name(i),
            Target::Way(l) => match self.ways().into_iter().find(|w| w.link == l) {
                Some(w) => self.way_name(&w),
                None => String::new(),
            },
            Target::Landmark(i) => self.landmark_name(i),
            Target::Named(i) => self.state.names[i].0.clone(),
            Target::Edge(e) => self.edge_name(e),
            Target::Mechanism(m) => self.mech_name(m),
            Target::Fire => self.fire_name(),
            Target::District(d) => match self.district_here() {
                Some((t, _)) => self.district_name(t, d),
                None => String::new(),
            },
            Target::Feature(f) => self.feature_name(f),
            Target::Room(r) => match self.state.place {
                Place::Room { structure, .. } => self.room_name(structure, r),
                Place::Outside => String::new(),
            },
            Target::Sign(n) => self.stable(
                "sign.name",
                ctx(&[("number", Value::Number(n as i64))]),
                4_000_000 + n as u64,
            ),
            Target::Buildings(k, c) => {
                let kind = slots::STRUCTURES.get(k).copied().unwrap_or("house");
                let condition = c
                    .and_then(|c| slots::CONDITIONS.get(c))
                    .copied()
                    .unwrap_or("");
                self.say(
                    "place.group_name",
                    ctx(&[
                        ("kind", Value::from(kind)),
                        ("condition", Value::from(condition)),
                    ]),
                )
            }
            Target::Things(k) => {
                let kind = slots::kinds().get(k).copied().unwrap_or("jar");
                self.say(
                    "place.group_name",
                    ctx(&[("kind", Value::from(kind)), ("condition", Value::from(""))]),
                )
            }
        }
    }

    /// `take all` and `take all <kind>` (S04): every loose thing here that
    /// could be carried, one take each.
    fn take_all(&mut self, kind: &[String]) -> Output {
        let want: Vec<String> = kind
            .iter()
            .filter(|w| !matches!(w.as_str(), "the" | "of" | "a"))
            .map(|w| w.trim_end_matches('s').to_string())
            .collect();
        let things: Vec<usize> = if self.is_dark() {
            Vec::new()
        } else {
            self.here()
                .into_iter()
                .filter(|&t| {
                    !self.state.carried.contains(&t)
                        && (self.thing(t).portable
                            || scraped_sim::items::kind(self.thing(t).kind).is_some())
                        && (want.is_empty() || {
                            let name = self.thing_name(t);
                            want.iter().all(|w| name.contains(w.as_str()))
                        })
                })
                .collect()
        };
        if things.is_empty() {
            let t = self.say("say.take_none", Context::new());
            return self.output(vec![t], None);
        }
        let mut parts = Vec::new();
        for t in things {
            let o = self.act("take", Target::Thing(t));
            if !o.text.trim().is_empty() {
                parts.push(o.text);
            }
            if self.state.dead.is_some() {
                break;
            }
        }
        self.output(parts, None)
    }

    fn act(&mut self, verb: &str, target: Target) -> Output {
        // Groups: looking at them lists their members; going to one goes to
        // the nearest member not yet entered.
        if let Target::Buildings(k, _) | Target::Things(k) = target {
            if verb == "go" {
                if let Target::Buildings(_, c) = target {
                    if let Some(s) = self.pick_member(k, c) {
                        return self.act("go", Target::Structure(s));
                    }
                }
            }
            if matches!(verb, "examine" | "count" | "go" | "look") {
                return if verb == "count" {
                    let kind = match target {
                        Target::Buildings(..) => slots::STRUCTURES.get(k).copied(),
                        _ => slots::kinds().get(k).copied(),
                    }
                    .unwrap_or("house");
                    self.count(&[kind.to_string()])
                } else {
                    self.examine_group(target)
                };
            }
        }
        // Places (D03): parts of town and features close by.
        match (verb, target) {
            ("go" | "examine" | "look", Target::District(d)) => return self.go_district(d),
            ("go", Target::Feature(f)) => return self.go_feature(f),
            ("examine" | "look", Target::Feature(f)) => return self.examine_feature(f),
            ("touch", Target::Feature(_)) => return self.touch(None),
            ("go" | "examine" | "look", Target::Room(r)) => return self.go_room(r),
            _ => {}
        }
        if verb == "touch" {
            return self.touch(Some(target));
        }
        if let ("examine", Target::Thing(i)) = (verb, target) {
            if self.thing(i).kind == "calendar stone" {
                if let Some(o) = self.examine_calendar(i) {
                    return o;
                }
            }
            if self.thing(i).object.is_some() {
                // The first look, then closer ones (D05).
                let key = format!("examined:{i}");
                let closer = self.state.told.contains_key(&key);
                self.state.told.insert(
                    key,
                    attention::Told {
                        minutes: self.state.minutes,
                        signature: 0,
                        said: true,
                    },
                );
                self.state.it = Some(target);
                return self.examine_object(i, closer);
            }
            if let Some(o) = self.examine_closer(i) {
                return o;
            }
        }
        self.state.it = Some(target);
        let name = self.target_name(target);
        let named = ctx(&[("thing", Value::from(name.as_str()))]);
        match (verb, target) {
            ("examine", Target::Thing(i)) => {
                if self.thing(i).texts.is_empty()
                    && scraped_sim::items::kind(self.thing(i).kind).is_some()
                {
                    return self.examine_item(i);
                }
                self.pass(2);
                let c = self.site.thing_vars(self.thing(i));
                let t = self.say("thing.examine", c);
                self.output(vec![t], None)
            }
            ("examine", Target::Mechanism(m)) => self.examine_mech(m),
            ("open" | "close" | "operate", Target::Mechanism(m)) => self.operate(m, verb),
            ("light", Target::Mechanism(m)) | ("extinguish", Target::Mechanism(m)) => {
                self.operate(m, if verb == "light" { "open" } else { "close" })
            }
            ("drink", Target::Mechanism(_)) => self.drink(&[]),
            ("fill", Target::Thing(i)) => self.fill(i),
            ("eat", Target::Thing(i)) => self.eat_thing(i),
            ("light", Target::Thing(i)) => {
                if self.thing(i).kind == "hearth" {
                    self.light_fire()
                } else {
                    self.light_item(i)
                }
            }
            ("light", Target::Fire) | ("feed", Target::Fire) => self.feed_fire(),
            ("extinguish", Target::Fire) => self.extinguish_fire(),
            ("extinguish", Target::Thing(i)) => self.douse_item(i),
            ("wear", Target::Thing(i)) => self.wear(i, true),
            ("remove", Target::Thing(i)) => self.wear(i, false),
            ("use", Target::Thing(i)) => self.use_thing(i),
            ("use", Target::Mechanism(m)) => self.operate(m, "operate"),
            ("pry", t) => self.pry(t),
            ("scrape", Target::Thing(i)) => self.scrape(i),
            ("clean", Target::Thing(i)) => self.clean(i),
            // A building looked at from outside: never a way in (S04).
            ("examine", Target::Structure(s)) if self.state.place == Place::Outside => {
                self.pass(1);
                let name = self.structure_name(s);
                let st = &self.site.world.structures[s];
                let vars = ctx(&[
                    ("name", Value::from(name)),
                    ("kind", Value::from(label(&st.kind))),
                    ("condition", Value::from(label(&st.condition))),
                ]);
                let t = self.say("place.building", vars);
                self.output(vec![t], None)
            }
            ("examine", Target::Structure(s)) | ("go", Target::Structure(s)) => {
                if verb == "examine" {
                    self.pass(1);
                }
                if !self.site.enterable(s) {
                    let t = self.say("say.buried", named);
                    return self.output(vec![t], None);
                }
                // A sealed place (D11): its way in won't give while its
                // ward holds.
                if self.sealed_shut(s)
                    && !self
                        .site
                        .writing
                        .sealed
                        .iter()
                        .any(|x| x.structure == s && x.inner)
                {
                    self.pass(1);
                    let noun = self
                        .site
                        .writing
                        .sealed
                        .iter()
                        .find(|x| x.structure == s)
                        .map_or("door", |x| x.noun);
                    let t = self.say(
                        "effect.held",
                        ctx(&[("thing", Value::from(format!("the {noun}")))]),
                    );
                    return self.output(vec![t], None);
                }
                self.pass(1);
                self.state.place = Place::Room {
                    structure: s,
                    room: 0,
                };
                let t = self.look();
                self.output(vec![t], None)
            }
            ("take", Target::Thing(i)) => {
                if self.state.carried.contains(&i) {
                    let t = self.say("say.take_held", named);
                    return self.output(vec![t], None);
                }
                if !self.thing(i).portable {
                    let mut c = named.clone();
                    c.insert("cause".into(), Value::from(""));
                    let t = self.say("say.take_fixed", c);
                    return self.output(vec![t], None);
                }
                if let Some(how) = self.held_by_writing(i) {
                    let mut c = named.clone();
                    c.insert("how".into(), Value::from(how));
                    let t = self.say("effect.fast", c);
                    return self.output(vec![t], None);
                }
                if self.overloaded_by(i) {
                    return self.too_heavy_thing(i);
                }
                self.pass(1);
                self.state.carried.push(i);
                self.state.moved.remove(&i);
                self.state.dropped.remove(&i);
                let t = self.say("say.take", named);
                let kind = self.thing(i).kind;
                let found = self.tool_found(kind);
                if found.is_some() {
                    self.hook("tool_found", kind, "");
                }
                self.output(std::iter::once(t).chain(found).collect(), None)
            }
            ("drop", Target::Thing(i)) => {
                if !self.state.carried.contains(&i) {
                    let t = self.say("say.drop_unheld", named);
                    return self.output(vec![t], None);
                }
                self.pass(1);
                self.state.carried.retain(|&c| c != i);
                self.state.dropped.remove(&i);
                if self.state.place == Place::Outside {
                    self.state.dropped.insert(i, self.state.pos);
                }
                self.state.worn.remove(&i);
                if self.thing(i).home != self.state.place {
                    self.state.moved.insert(i, self.state.place);
                } else {
                    self.state.moved.remove(&i);
                }
                let t = self.say("say.drop", named);
                self.output(vec![t], None)
            }
            ("read", Target::Thing(i))
                if self
                    .thing(i)
                    .object
                    .is_some_and(|o| self.site.world.objects[o].map.is_some()) =>
            {
                self.read_map(i)
            }
            ("read", Target::Thing(i)) => {
                if self.thing(i).texts.is_empty() {
                    let t = self.say("read.nothing", named);
                    return self.output(vec![t], None);
                }
                if self.is_dark() {
                    let t = self.say("read.dark", named);
                    return self.output(vec![t], None);
                }
                self.read_whole(i)
            }
            ("examine" | "look" | "read", Target::Sign(n)) => self.examine_sign(n),
            ("trace", t) => self.trace(t),
            ("go", Target::Way(l)) => self.go_way(l),
            ("go", Target::Landmark(i)) => self.go_landmark(i),
            ("go", Target::Named(i)) => {
                let pos = self.state.names[i].1;
                self.travel_to(pos, Some(target), "walk")
            }
            ("go", Target::Edge(e)) | ("follow", Target::Edge(e)) => {
                self.follow_edge(e, outdoors::Way::Onward)
            }
            ("examine", Target::Landmark(_))
            | ("examine", Target::Edge(_))
            | ("examine", Target::Named(_)) => {
                let t = self.describe_far(target);
                self.output(vec![t], None)
            }
            ("open", Target::Way(l)) | ("close", Target::Way(l)) => self.door(verb == "open", l),
            ("open", Target::Thing(t)) => self.open_object(t),

            ("examine", Target::Way(l)) => {
                let w = self
                    .ways()
                    .into_iter()
                    .find(|w| w.link == l)
                    .expect("resolved from ways");
                let t = self.way_name(&w);
                self.output(vec![t], None)
            }
            ("go", Target::Thing(_)) | (_, Target::Structure(_)) => {
                let t = self.say("say.not_here", ctx(&[("words", Value::from(name))]));
                self.output(vec![t], None)
            }
            _ => {
                let t = self.say("say.not_here", ctx(&[("words", Value::from(name))]));
                self.output(vec![t], None)
            }
        }
    }

    /// Opens a door, for bots that move without commands (the mapper).
    pub(crate) fn step_quiet_door(&mut self, link: usize) {
        self.door(true, link);
    }

    /// Opens or closes a door. Blocked and collapsed ways stay as they are.
    fn door(&mut self, open: bool, link: usize) -> Output {
        let Place::Room { structure, .. } = self.state.place else {
            unreachable!("ways exist only in rooms")
        };
        let w = self
            .ways()
            .into_iter()
            .find(|w| w.link == link)
            .expect("resolved from ways");
        let named = ctx(&[("thing", Value::from(self.way_name(&w)))]);
        // Sealed by works or by its day (D05): nothing the hand can do.
        if open {
            if let Some(cause) = self.sealed(structure, w.link) {
                let mut c = named.clone();
                c.insert("cause".into(), Value::from(cause));
                let t = self.say("say.door_stuck", c);
                return self.output(vec![t], None);
            }
        }
        let movable = !w.collapsed
            && w.state != PassageState::Blocked
            && w.passage == Some(scraped_world::structures::Passage::Door);
        let already = (w.state == PassageState::Open) == open;
        if movable && !already {
            if let Some(h) = self.doors_held() {
                if (h > 0) != open {
                    let t = self.say("effect.held", named);
                    return self.output(vec![t], None);
                }
            }
        }
        // A locked door: its key opens it; without, a keyhole (D05).
        let mut unlocked = None;
        if movable && open && !already {
            let was = self.state.unlocked.contains(&(structure, w.link));
            match self.locked_door(structure, w.link) {
                Some(true) => {
                    let mut c = named.clone();
                    c.insert("cause".into(), Value::from("locked"));
                    let t = self.say("say.door_stuck", c);
                    return self.output(vec![t], None);
                }
                Some(false) if !was => {
                    unlocked = Some(self.say("door.unlocked", named.clone()));
                }
                _ => {}
            }
        }
        if movable && open && !already && self.barred(structure, w.link) {
            if self.carrying_pry_bar() {
                self.pass(10);
                self.state.sim.unbarred.insert((structure, w.link));
                self.state
                    .doors
                    .insert(format!("{structure}:{}", w.link), true);
                let t = self.say("door.pried", named);
                return self.output(vec![t], None);
            }
            let t = self.say("hazard.barred", named);
            return self.output(vec![t], None);
        }
        let slot = match (movable, already, open) {
            (false, _, _) => "say.door_stuck",
            (true, true, true) => "say.door_already_open",
            (true, true, false) => "say.door_already_closed",
            (true, false, true) => "say.door_open",
            (true, false, false) => "say.door_close",
        };
        if movable && !already {
            self.pass(1);
            self.state
                .doors
                .insert(format!("{structure}:{}", w.link), open);
        }
        // Why it won't move, so the player has something to notice (S01).
        // DESIGN-Q: locked, stuck and swollen doors come with D05's keys.
        let mut named = named;
        if !movable {
            let cause = if w.passage != Some(scraped_world::structures::Passage::Door) {
                "not_door"
            } else if w.state == PassageState::Blocked {
                "rubble"
            } else {
                "fallen"
            };
            named.insert("cause".into(), Value::from(cause));
        }
        let t = self.say(slot, named);
        self.output(unlocked.into_iter().chain([t]).collect(), None)
    }

    fn go_out(&mut self) -> Output {
        match self.state.place {
            Place::Outside => {
                let t = self.say("say.outside_already", Context::new());
                self.output(vec![t], None)
            }
            Place::Room { structure, room }
                if room == 0 || self.site.structure(structure).interior.rooms[room].outside =>
            {
                self.pass(1);
                self.state.place = Place::Outside;
                if room != 0 {
                    // Out by another mouth: where it lies (D04).
                    let (x, y) = self.site.structure(structure).interior.rooms[room].centre();
                    let p = self.site.land.structure_pos[structure];
                    let out = Pos::new(p.x + x.round() as i32, p.y + y.round() as i32);
                    let (cx, cy) = out.cell();
                    if self.site.land.passable(&self.site.world, cx, cy) {
                        self.state.pos = out;
                    }
                }
                let t = self.look();
                self.output(vec![t], None)
            }
            Place::Room { .. } => {
                let t = self.say("say.not_entrance", Context::new());
                self.output(vec![t], None)
            }
        }
    }

    fn go_dir(&mut self, dir: &str) -> Output {
        if self.state.place == Place::Outside {
            if let Some(b) = outdoors::parse_bearing(dir) {
                return self.head_toward(b);
            }
        }
        let exit = match dir {
            "northeast" | "northwest" | "southeast" | "southwest" => {
                let t = self.say("say.no_exit", ctx(&[("direction", Value::from(dir))]));
                return self.output(vec![t], None);
            }
            "north" => Exit::North,
            "south" => Exit::South,
            "east" => Exit::East,
            "west" => Exit::West,
            "up" => Exit::Up,
            _ => Exit::Down,
        };
        let ways: Vec<Way> = self
            .ways()
            .into_iter()
            .filter(|w| w.exit == exit && w.passage != Some(Passage::Window))
            .collect();
        match ways.len() {
            0 => {
                let t = self.say("say.no_exit", ctx(&[("direction", Value::from(dir))]));
                self.output(vec![t], None)
            }
            1 => self.go_way(ways[0].link),
            // Several ways that way (a great hall's doors): which?
            _ => self.with_target("go", &[dir.to_string()]),
        }
    }

    /// Whether a room lies under water no one can wade (D04: a sump, a
    /// flooded working).
    pub(crate) fn under_water(&self, p: Place) -> bool {
        match p {
            Place::Room { structure, room } => {
                matches!(
                    self.site.structure(structure).interior.rooms[room].water,
                    "sump" | "flooded"
                )
            }
            Place::Outside => false,
        }
    }

    /// Which way and how far the player went, honestly, so a map can be
    /// drawn (D04): only the direction by feel in the dark.
    fn move_report(&mut self, way: &Way) -> Option<String> {
        let metres = (way.metres / 2.0).round() as i64 * 2;
        // The move's two components, so a map can be drawn true.
        let (north, east) = match (self.state.place, way.to) {
            (Place::Room { structure, room: a }, Place::Room { room: b, .. }) => {
                let rooms = &self.site.structure(structure).interior.rooms;
                let (ax, ay) = rooms[a].centre();
                let (bx, by) = rooms[b].centre();
                ((ay - by).round() as i64, (bx - ax).round() as i64)
            }
            _ => (0, 0),
        };
        let c = ctx(&[
            ("north", Value::Number(north)),
            ("east", Value::Number(east)),
            ("direction", Value::from(label(&way.exit))),
            (
                "passage",
                Value::from(way.passage.map_or("opening".to_string(), |p| label(&p))),
            ),
            ("metres", Value::Number(metres)),
            ("dark", Value::Bool(self.is_dark())),
        ]);
        Some(self.say("move.report", c))
    }

    /// Goes through one way out of a room.
    pub(crate) fn go_way(&mut self, link: usize) -> Output {
        let Some(way) = self.ways().into_iter().find(|w| w.link == link) else {
            let t = self.say("say.not_here", ctx(&[("words", Value::from(""))]));
            return self.output(vec![t], None);
        };
        if way.passage == Some(Passage::Window) {
            let named = ctx(&[("thing", Value::from(self.way_name(&way)))]);
            let t = self.say("move.window", named);
            return self.output(vec![t], None);
        }
        if way.collapsed || way.state != PassageState::Open {
            let c = self.site.way_vars(&way);
            let t = self.say("say.blocked", c);
            return self.output(vec![t], None);
        }
        // Sealed from within (D11): every way on from the entrance room
        // holds while the ward on its wall does.
        if let Place::Room { structure, room: 0 } = self.state.place {
            let inner = self
                .site
                .writing
                .sealed
                .iter()
                .any(|x| x.structure == structure && x.inner);
            if inner && way.to != self.state.place && self.sealed_shut(structure) {
                let named = ctx(&[("thing", Value::from(self.way_name(&way)))]);
                let t = self.say("effect.held", named);
                return self.output(vec![t], None);
            }
        }
        // A rope climbs back up a drop (D05).
        let climb = way.against
            && way.exit == Exit::Up
            && matches!(way.passage, Some(Passage::Hole | Passage::Shaft))
            && self
                .state
                .carried
                .iter()
                .any(|&t| self.thing(t).kind == "rope");
        if way.against && !climb {
            let named = ctx(&[("thing", Value::from(self.way_name(&way)))]);
            let t = self.say("move.against", named);
            return self.output(vec![t], None);
        }
        if self.flooded(way.to) || self.under_water(way.to) {
            let named = ctx(&[("thing", Value::from(self.way_name(&way)))]);
            let t = self.say("hazard.flooded", named);
            return self.output(vec![t], None);
        }
        let fall = self.dark_stair(way.passage);
        // Time by distance: a slow walk, slower by feel (D04).
        let pace = if self.is_dark() { 30.0 } else { 70.0 };
        let minutes = (way.metres / pace).ceil().max(1.0) as u32;
        self.pass(minutes);
        if self.state.dead.is_some() {
            return self.output(fall.into_iter().collect(), None);
        }
        let report = self.move_report(&way);
        let roped = climb.then(|| {
            self.pass(10);
            self.say("move.rope", Context::new())
        });
        // A tight crawl with a heavy load: slow going (D04).
        let squeeze = if way.passage == Some(Passage::Crawlway)
            && self.weight() * 3 > scraped_sim::items::CARRY * 2
        {
            self.pass(10);
            Some(self.say("move.squeeze", Context::new()))
        } else {
            None
        };
        self.last_link = Some(way.link);
        self.state.place = way.to;
        self.said_already = usize::from(report.is_some());
        let t = self.look();
        self.output(
            fall.into_iter()
                .chain(roped)
                .chain(report)
                .chain(squeeze)
                .chain([t])
                .collect(),
            None,
        )
    }

    // ---------- reading ----------

    /// The glyphs of everything written on a thing, in reading order.
    fn marks(&self, thing: usize) -> Vec<Mark> {
        let mut out = Vec::new();
        let deep = self.deep_layers(thing);
        for (n, (tid, partial)) in self.layers_seen(thing).into_iter().enumerate() {
            if n > 0 {
                out.push(Mark::Break);
            }
            let text = self.text(tid);
            let r = self.site.world.renderer(text.era);
            let rendered = r.render(&text.meaning);
            let script = &self.site.world.languages[text.era as usize].script;
            for (g, k) in r.glyphs(&rendered).into_iter().enumerate() {
                out.push(match k {
                    Some(k) => Mark::Glyph {
                        era: text.era,
                        index: script.index(&k),
                        lost: (partial && self.lost(tid, g, deep.contains(&tid)))
                            || self.grimed(thing, tid, g),
                    },
                    None => Mark::Gap,
                });
            }
        }
        out
    }
}

impl Game {
    fn carrying_pry_bar(&self) -> bool {
        self.state
            .carried
            .iter()
            .any(|&t| self.thing(t).kind == "pry_bar")
    }

    pub(crate) fn fire_here_pub(&self) -> bool {
        self.env().fire_at(self.spot()).is_some()
    }
}
