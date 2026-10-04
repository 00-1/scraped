//! Living things (D06): the animals and plants of a world, generated from
//! its habitats and climate.
//!
//! A species is a body plan from a vocabulary (a deer, a birch) chosen to
//! suit where it lives, combined with traits that set it apart (its
//! colour, a distinguishing mark, when it is about, what it leaves
//! behind). Which body plans a world gets depends on which habitats it has
//! and how cold or warm they are, so two worlds share some species and
//! differ in others. Every species has a home habitat, a season rhythm,
//! signs it leaves, and a tolerance: how far its region's life can fall
//! before it disappears there (some only come back once a region
//! recovers past where it stood).
//!
//! Nothing here is player text: ids name vocabulary for Jb's templates.

use serde::Serialize;

use crate::history::Cell;
use crate::terrain::{Biome, Terrain, SIZE};
use crate::water::Water;

/// Where a species lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Habitat {
    /// Grassland and scrub.
    Open,
    /// Broadleaf forest.
    Woods,
    Pine,
    /// Marsh and shore.
    Wet,
    /// Desert.
    Dry,
    /// Tundra, bare rock and snow.
    Cold,
    /// Rivers and lakes.
    Fresh,
    Sea,
    /// Dark places underground.
    Deep,
}

pub const HABITATS: [Habitat; 9] = [
    Habitat::Open,
    Habitat::Woods,
    Habitat::Pine,
    Habitat::Wet,
    Habitat::Dry,
    Habitat::Cold,
    Habitat::Fresh,
    Habitat::Sea,
    Habitat::Deep,
];

impl Habitat {
    fn bit(self) -> u16 {
        1 << (self as u16)
    }

    /// The habitat of a cell, with rivers counted as fresh water.
    pub fn of(t: &Terrain, w: &Water, x: usize, y: usize) -> Habitat {
        match t.biome.get(x, y) {
            Biome::Sea => Habitat::Sea,
            Biome::Lake => Habitat::Fresh,
            _ if w.is_river(t, x, y) => Habitat::Fresh,
            b => Self::of_biome(*b),
        }
    }

    /// The land habitat of a biome (water biomes give their water).
    pub fn of_biome(b: Biome) -> Habitat {
        match b {
            Biome::Sea => Habitat::Sea,
            Biome::Lake => Habitat::Fresh,
            Biome::Grassland | Biome::Scrub => Habitat::Open,
            Biome::Forest => Habitat::Woods,
            Biome::Pine => Habitat::Pine,
            Biome::Marsh | Biome::Shore => Habitat::Wet,
            Biome::Desert => Habitat::Dry,
            Biome::Tundra | Biome::Rock | Biome::Snow => Habitat::Cold,
        }
    }
}

// Habitat bits for the vocabulary tables.
const O: u16 = 1;
const F: u16 = 2;
const P: u16 = 4;
const M: u16 = 8;
const D: u16 = 16;
const K: u16 = 32;
const R: u16 = 64;
const S: u16 = 128;
const U: u16 = 256;
const LAND: u16 = O | F | P | M | D | K;

// Climate bands: cold (mean below 6 °C), temperate, warm (above 14 °C).
const C: u8 = 1;
const T: u8 = 2;
const W: u8 = 4;
const ALL: u8 = C | T | W;

/// A body plan: role, form id, habitats and climates it suits.
struct Form {
    role: &'static str,
    form: &'static str,
    habitats: u16,
    climate: u8,
}

const fn f(role: &'static str, form: &'static str, habitats: u16, climate: u8) -> Form {
    Form {
        role,
        form,
        habitats,
        climate,
    }
}

/// Animal body plans.
// DESIGN-Q: the animal vocabulary, its habitats and climates.
const ANIMALS: &[Form] = &[
    f("grazer", "deer", F | O | P, C | T),
    f("grazer", "wild cattle", O | M, T),
    f("grazer", "wild horse", O | D, T | W),
    f("grazer", "wild sheep", K | O, C | T),
    f("grazer", "musk ox", K, C),
    f("grazer", "reindeer", K | P, C),
    f("grazer", "antelope", O | D, W),
    f("grazer", "wild ass", D, T | W),
    f("grazer", "buffalo", M | O, W | T),
    f("browser", "elk", P | F | M, C | T),
    f("browser", "roe deer", F | O, T),
    f("browser", "wild goat", K | D | P, ALL),
    f("browser", "boar", F | M, T | W),
    f("browser", "hare", O | K | D, ALL),
    f("browser", "camel", D, W),
    f("browser", "porcupine", F | D, T | W),
    f("predator", "wolf", F | P | O | K, C | T),
    f("predator", "lynx", P | F | K, C | T),
    f("predator", "bear", F | P, C | T),
    f("predator", "wildcat", F | O | D, T | W),
    f("predator", "big cat", O | D | F, W),
    f("predator", "otter", M | F, ALL),
    f("predator", "snake", D | O | M, T | W),
    f("predator", "stoat", K | O | P, C | T),
    f("scavenger", "fox", O | F | K | D | P, ALL),
    f("scavenger", "jackal", D | O, W | T),
    f("scavenger", "hyena", D | O, W),
    f("scavenger", "wolverine", K | P, C),
    f("scavenger", "badger", F | O, T),
    f("scavenger", "raccoon dog", F | M, T | C),
    f("burrower", "rabbit", O | D, T | W),
    f("burrower", "marmot", K | O, C | T),
    f("burrower", "mole", O | F, T),
    f("burrower", "vole", O | M | K | P, C | T),
    f("burrower", "gerbil", D, W),
    f("burrower", "ground squirrel", O | D, T | W),
    f("burrower", "lemming", K, C),
    f("bird", "crow", LAND, ALL),
    f("bird", "owl", F | P | O, ALL),
    f("bird", "woodpecker", F | P, C | T),
    f("bird", "grouse", K | P | O, C | T),
    f("bird", "partridge", O | D, T | W),
    f("bird", "heron", M, ALL),
    f("bird", "gull", M, ALL),
    f("bird", "hawk", O | F | D, ALL),
    f("bird", "wren", F | O | M, T),
    f("bird", "raven", K | D | P, ALL),
    f("bird", "jay", F, T),
    f("bird", "sandgrouse", D, W),
    f("bird", "ptarmigan", K, C),
    f("bird", "pigeon", O | F, T | W),
    f("bird", "finch", O | F | P, ALL),
    f("bird", "lark", O | D | K, ALL),
    f("migrant", "swallow", O | M, T | W),
    f("migrant", "cuckoo", F | O, T),
    f("migrant", "crane", M | O, C | T),
    f("migrant", "goose", M | K | O, C | T),
    f("migrant", "swift", O | D, T | W),
    f("migrant", "bee-eater", D | O, W),
    f("migrant", "plover", M | K, C | T),
    f("migrant", "stork", M | O, T | W),
    f("migrant", "warbler", F | M | P, ALL),
    f("insect", "bee", O | F, T | W),
    f("insect", "cricket", O | D, T | W),
    f("insect", "dragonfly", M, T | W),
    f("insect", "beetle", F | P | D, ALL),
    f("insect", "moth", F | O | P, ALL),
    f("insect", "midge", M | K, C | T),
    f("insect", "ant", F | O | D | P, ALL),
    f("insect", "locust", D | O, W),
    f("insect", "butterfly", O | F, T | W),
    f("insect", "cicada", D | F, W),
    f("insect", "wasp", O | F, T | W),
    f("small", "frog", M | F, T | W),
    f("small", "toad", F | O | M, T | C),
    f("small", "newt", M | F, T),
    f("small", "lizard", D | O, T | W),
    f("small", "tortoise", D, W),
    f("small", "salamander", F | P, T | C),
    f("small", "shrew", O | F | P | K, C | T),
    f("fish", "trout", R, C | T),
    f("fish", "pike", R, C | T),
    f("fish", "carp", R, T | W),
    f("fish", "eel", R | S, T | W),
    f("fish", "perch", R, T),
    f("fish", "catfish", R, W),
    f("fish", "salmon", R | S, C | T),
    f("fish", "herring", S, C | T),
    f("fish", "cod", S, C | T),
    f("fish", "flatfish", S, ALL),
    f("fish", "mullet", S, T | W),
    f("fish", "mackerel", S, T | W),
    // DESIGN-Q: the "something stranger in deep places" of M07, now one
    // species with signs; its nature is for Jb.
    f("deep", "pale crawler", U, ALL),
];

/// Plant body plans.
// DESIGN-Q: the plant vocabulary, its habitats and climates.
const PLANTS: &[Form] = &[
    f("tree", "oak", F | O, T),
    f("tree", "beech", F, T),
    f("tree", "ash", F | O, T),
    f("tree", "lime", F, T),
    f("tree", "birch", F | P | K | O, C | T),
    f("tree", "alder", M | F, C | T),
    f("tree", "willow", M, ALL),
    f("tree", "pine", P | O | D, ALL),
    f("tree", "spruce", P, C | T),
    f("tree", "fir", P, C | T),
    f("tree", "larch", P | K, C),
    f("tree", "yew", F, T),
    f("tree", "olive", O | D, W),
    f("tree", "fig", F | D, W),
    f("tree", "palm", D | M, W),
    f("tree", "acacia", D | O, W),
    f("tree", "cedar", P | F, T | W),
    f("tree", "tamarisk", D | M, W | T),
    f("tree", "rowan", F | P, C | T),
    f("shrub", "hazel", F | O, T),
    f("shrub", "bramble", F | O, T),
    f("shrub", "gorse", O, T),
    f("shrub", "heather", O | K | P, C | T),
    f("shrub", "juniper", K | P | D | O, ALL),
    f("shrub", "blackthorn", O | F, T),
    f("shrub", "elder", F | O, T),
    f("shrub", "bilberry", P | K, C | T),
    f("shrub", "crowberry", K, C),
    f("shrub", "myrtle", M | D, W | T),
    f("shrub", "saltbush", D | M, W | T),
    f("shrub", "sea buckthorn", M | O, C | T),
    f("shrub", "broom", O | D, T | W),
    f("shrub", "dwarf willow", K, C),
    f("shrub", "wild rose", O | F, T | W),
    f("shrub", "oleander", D | M, W),
    f("shrub", "thornbush", D | O, W),
    f("grass", "tussock grass", O | K, C | T),
    f("grass", "feather grass", O | D, T | W),
    f("grass", "reed", M, ALL),
    f("grass", "sedge", M | K, C | T),
    f("grass", "rush", M, ALL),
    f("grass", "bent grass", O | F, T),
    f("grass", "cotton grass", M | K, C),
    f("grass", "marram", M, ALL),
    f("grass", "millet grass", O | D, W),
    f("flower", "poppy", O | D, T | W),
    f("flower", "cowslip", O | F, T),
    f("flower", "harebell", O | K, C | T),
    f("flower", "orchid", F | M | O, T | W),
    f("flower", "thistle", O | D, ALL),
    f("flower", "foxglove", F | P, T),
    f("flower", "bluebell", F, T),
    f("flower", "saxifrage", K, C | T),
    f("flower", "gentian", K | O, C | T),
    f("flower", "iris", M, T | W),
    f("flower", "marigold", M | O, T | W),
    f("flower", "lily", M | F, T | W),
    f("flower", "sea pink", M, C | T),
    f("flower", "lavender", D | O, W | T),
    f("flower", "asphodel", D | O, W),
    f("flower", "anemone", F | O, T | W),
    f("flower", "crocus", O | K, T | W),
    f("flower", "campion", O | F | M, T | C),
    f("flower", "meadowsweet", M | O, T),
    f("flower", "mallow", O | D, W | T),
    f("flower", "mountain avens", K, C),
    f("fern", "bracken", F | O | P, T | C),
    f("fern", "tongue fern", F, T),
    f("fern", "royal fern", M | F, T),
    f("fern", "horsetail", M, ALL),
    f("fungus", "bracket fungus", F | P, ALL),
    f("fungus", "puffball", O | F, T),
    f("fungus", "toadstool", F | P, ALL),
    f("fungus", "morel", F, T),
    f("fungus", "chanterelle", F | P, C | T),
    f("fungus", "ink cap", O | F, T),
    f("fungus", "earthstar", D | P, T | W),
    f("climber", "ivy", F, T),
    f("climber", "honeysuckle", F | O, T),
    f("climber", "wild vine", F | D, W | T),
    f("climber", "hop", F | M, T),
    f("climber", "clematis", F | O, T | W),
    f("climber", "bindweed", O | D, ALL),
    f("moss", "bog moss", M | K, C | T),
    f("moss", "reindeer lichen", K | P, C),
    f("moss", "map lichen", K | D, ALL),
    f("moss", "feather moss", P | F, C | T),
    f("moss", "crust lichen", D | K, ALL),
    f("water", "water lily", R, T | W),
    f("water", "pondweed", R, ALL),
    f("water", "duckweed", R, T | W),
    f("water", "water crowfoot", R, C | T),
    f("water", "wrack", S, ALL),
    f("water", "kelp", S, C | T),
    f("water", "eelgrass", S, T | W),
    f("water", "sea lettuce", S, ALL),
];

/// How many species of each role a habitat holds (animals, then plants).
// DESIGN-Q: these counts set the size of a world's ecology: about 50
// animals and 45 plants in a world with most habitats.
/// Roles and how many of each.
type Roles = &'static [(&'static str, u8)];

fn roles(h: Habitat) -> (Roles, Roles) {
    match h {
        Habitat::Open => (
            &[
                ("grazer", 1),
                ("browser", 1),
                ("predator", 1),
                ("scavenger", 1),
                ("burrower", 1),
                ("bird", 2),
                ("migrant", 1),
                ("insect", 2),
                ("small", 1),
            ],
            &[
                ("tree", 1),
                ("shrub", 1),
                ("grass", 2),
                ("flower", 3),
                ("fungus", 1),
                ("climber", 1),
            ],
        ),
        Habitat::Woods => (
            &[
                ("grazer", 1),
                ("browser", 1),
                ("predator", 1),
                ("scavenger", 1),
                ("burrower", 1),
                ("bird", 2),
                ("migrant", 1),
                ("insect", 2),
                ("small", 1),
            ],
            &[
                ("tree", 2),
                ("shrub", 1),
                ("flower", 2),
                ("fern", 1),
                ("fungus", 2),
                ("climber", 1),
                ("moss", 1),
            ],
        ),
        Habitat::Pine => (
            &[
                ("grazer", 1),
                ("browser", 1),
                ("predator", 1),
                ("scavenger", 1),
                ("bird", 2),
                ("migrant", 1),
                ("insect", 1),
                ("small", 1),
            ],
            &[
                ("tree", 2),
                ("shrub", 1),
                ("flower", 1),
                ("fern", 1),
                ("fungus", 1),
                ("moss", 1),
            ],
        ),
        Habitat::Wet => (
            &[
                ("grazer", 1),
                ("predator", 1),
                ("bird", 2),
                ("migrant", 2),
                ("insect", 2),
                ("small", 1),
            ],
            &[
                ("tree", 1),
                ("shrub", 1),
                ("grass", 3),
                ("flower", 2),
                ("fern", 1),
                ("moss", 1),
            ],
        ),
        Habitat::Dry => (
            &[
                ("grazer", 1),
                ("browser", 1),
                ("predator", 1),
                ("scavenger", 1),
                ("burrower", 1),
                ("bird", 1),
                ("migrant", 1),
                ("insect", 2),
                ("small", 1),
            ],
            &[
                ("tree", 1),
                ("shrub", 2),
                ("grass", 1),
                ("flower", 2),
                ("moss", 1),
            ],
        ),
        Habitat::Cold => (
            &[
                ("grazer", 1),
                ("browser", 1),
                ("predator", 1),
                ("scavenger", 1),
                ("burrower", 1),
                ("bird", 2),
                ("migrant", 1),
                ("insect", 1),
            ],
            &[("shrub", 2), ("grass", 1), ("flower", 2), ("moss", 2)],
        ),
        Habitat::Fresh => (&[("fish", 3)], &[("water", 2)]),
        Habitat::Sea => (&[("fish", 3)], &[("water", 2)]),
        Habitat::Deep => (&[("deep", 1)], &[]),
    }
}

/// Animal or plant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kingdom {
    Animal,
    Plant,
}

/// One species in a world.
#[derive(Debug, Clone, Serialize)]
pub struct Species {
    pub id: usize,
    pub kingdom: Kingdom,
    /// "grazer", "predator", "tree", "fungus"...
    pub role: &'static str,
    /// The body plan: "deer", "birch".
    pub form: &'static str,
    pub habitat: Habitat,
    /// "cold", "temperate" or "warm".
    pub climate: &'static str,
    /// "small", "middling" or "large".
    pub size: &'static str,
    /// Coat, plumage, bark or flower colour.
    pub colour: &'static str,
    /// What sets it apart from others of its form.
    pub mark: &'static str,
    /// When it is about: "day", "dusk", "night". Plants: "day".
    pub active: &'static str,
    /// Animals: what its feet leave ("hoof", "paw", "claw", "bird", "none").
    pub foot: &'static str,
    /// Animals: the signs it leaves besides tracks.
    pub signs: Vec<&'static str>,
    /// Animals: its call, if it has one.
    pub call: Option<&'static str>,
    /// Animals: its home ("den", "nest"...), if it keeps one.
    pub home: Option<&'static str>,
    /// Seasons it is here (0 spring … 3 winter): migrants come and go.
    pub present: [bool; 4],
    /// Plants: the seasons it flowers and fruits, and what its fruit is.
    pub flowers: Option<usize>,
    pub fruits: Option<usize>,
    pub fruit: &'static str,
    /// Plants: whether it keeps its leaves in winter.
    pub evergreen: bool,
    /// What it is good (or bad) for: "food", "fibre", "dye", "fuel",
    /// "remedy", "harmful", "none".
    pub use_: &'static str,
    /// How common it is where it lives, 1 (rare) to 10 (everywhere).
    pub abundance: u32,
    /// The region's life (per mille of where it stood at the start) below
    /// which it is gone from there. Above 1000, it only appears once a
    /// region has recovered past its start.
    pub tolerance: i32,
}

impl Species {
    /// Whether it is here in a season.
    pub fn here_in(&self, season: usize) -> bool {
        self.present[season % 4]
    }

    /// What a plant looks like in a season: "in flower", "in fruit",
    /// "in leaf", "turning", "bare", "died back", "green".
    pub fn state(&self, season: usize) -> &'static str {
        let s = season % 4;
        if self.flowers == Some(s) {
            "in flower"
        } else if self.fruits == Some(s) {
            "in fruit"
        } else if s == 3 {
            if self.evergreen {
                "green"
            } else if matches!(self.role, "tree" | "shrub" | "climber") {
                "bare"
            } else {
                "died back"
            }
        } else if s == 2 && !self.evergreen {
            "turning"
        } else {
            "in leaf"
        }
    }
}

/// A den, nest, warren or roost: a place where a species lives.
#[derive(Debug, Clone, Serialize)]
pub struct Home {
    pub species: usize,
    pub cell: Cell,
}

/// The living things of a world.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Life {
    pub species: Vec<Species>,
    pub homes: Vec<Home>,
}

fn mix(parts: &[u64]) -> u64 {
    let mut v: u64 = 0x2545_f491_4f6c_dd1d;
    for &p in parts {
        v ^= p;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v ^= v >> 29;
        v = v.wrapping_mul(0x94d0_49bb_1331_11eb);
        v ^= v >> 32;
    }
    v
}

fn pick<X>(xs: &[X], h: u64) -> &X {
    &xs[(h % xs.len() as u64) as usize]
}

fn colours(kingdom: Kingdom, role: &str, climate: u8) -> &'static [&'static str] {
    match (kingdom, role) {
        (Kingdom::Plant, "tree" | "shrub" | "climber" | "fern" | "moss" | "grass" | "water") => {
            &["dark", "pale", "grey", "silvery", "reddish", "bright"]
        }
        (Kingdom::Plant, "fungus") => &["white", "brown", "yellow", "red", "grey", "orange"],
        (Kingdom::Plant, _) => &["white", "yellow", "blue", "red", "purple", "pink"],
        (_, "insect") => &["black", "yellow", "green", "blue", "brown", "red"],
        (_, "fish") => &["silver", "dark", "golden", "green", "speckled"],
        _ if climate == C => &["white", "grey", "pale", "dun", "brown"],
        _ if climate == W => &["tawny", "sandy", "russet", "dark", "spotted"],
        _ => &["brown", "red", "grey", "dark", "dun", "speckled"],
    }
}

fn marks(kingdom: Kingdom, role: &str) -> &'static [&'static str] {
    match (kingdom, role) {
        (Kingdom::Animal, "grazer") => &[
            "long horns",
            "curled horns",
            "a pale rump",
            "a dark mane",
            "a shaggy coat",
        ],
        (Kingdom::Animal, "browser") => &[
            "broad antlers",
            "a short tail",
            "long ears",
            "a striped back",
            "a dark muzzle",
        ],
        (Kingdom::Animal, "predator") => &[
            "tufted ears",
            "a ringed tail",
            "a pale throat",
            "a dark mask",
            "a heavy jaw",
        ],
        (Kingdom::Animal, "scavenger") => &[
            "a bushy tail",
            "big ears",
            "a black back",
            "a grizzled face",
        ],
        (Kingdom::Animal, "burrower") => &[
            "short ears",
            "a striped face",
            "long whiskers",
            "a pale belly",
        ],
        (Kingdom::Animal, "bird" | "migrant") => &[
            "a crest",
            "a red cap",
            "a long bill",
            "barred wings",
            "a forked tail",
            "a white rump",
        ],
        (Kingdom::Animal, "insect") => &["banded", "metallic", "hairy", "long-legged"],
        (Kingdom::Animal, "small") => &["spotted", "bright-bellied", "warty", "banded"],
        (Kingdom::Animal, "fish") => &["spotted", "barred", "dark-backed", "long-finned"],
        (Kingdom::Animal, _) => &["eyeless", "many-legged"],
        (Kingdom::Plant, "tree") => &[
            "deeply furrowed bark",
            "peeling bark",
            "a broad crown",
            "hanging twigs",
            "a narrow crown",
        ],
        (Kingdom::Plant, "shrub" | "climber") => &[
            "thorns",
            "sticky leaves",
            "a sweet smell",
            "woolly leaves",
            "glossy leaves",
        ],
        (Kingdom::Plant, "grass") => &["tall", "tufted", "feathery heads", "sharp edges"],
        (Kingdom::Plant, "flower") => &[
            "nodding heads",
            "tall spikes",
            "low rosettes",
            "a strong scent",
            "many small heads",
        ],
        (Kingdom::Plant, "fungus") => &[
            "a ringed stem",
            "gills",
            "a slimy cap",
            "shelves",
            "a strong smell",
        ],
        (Kingdom::Plant, _) => &["thick", "feathery", "floating", "trailing"],
    }
}

/// What a role leaves: (foot, signs, calls, home, activity, abundance,
/// tolerance).
#[allow(clippy::type_complexity)]
fn habits(
    role: &str,
) -> (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    Option<&'static str>,
    &'static str,
    u32,
    i32,
) {
    match role {
        "grazer" => (
            "hoof",
            &["droppings", "cropped grass", "a wallow", "shed hair"],
            &["bellow", "bleat", "snort"],
            Some("wallow"),
            "day",
            5,
            600,
        ),
        "browser" => (
            "hoof",
            &[
                "browsed bark",
                "droppings",
                "frayed saplings",
                "nibbled shoots",
            ],
            &["bark", "grunt", "thump"],
            Some("lying-up place"),
            "dusk",
            4,
            600,
        ),
        "predator" => (
            "paw",
            &["scat", "kill remains", "a scratched tree", "a trail of fur"],
            &["howl", "growl", "scream"],
            Some("den"),
            "night",
            2,
            750,
        ),
        "scavenger" => (
            "paw",
            &["scattered bones", "scat", "a dug-up cache"],
            &["yip", "cackle", "yelp"],
            Some("earth"),
            "dusk",
            3,
            300,
        ),
        "burrower" => (
            "small paw",
            &["burrows", "spoil heaps", "runs in the grass", "droppings"],
            &["whistle", "squeak", "chatter"],
            Some("warren"),
            "dusk",
            7,
            400,
        ),
        "bird" => (
            "bird",
            &["feathers", "white droppings", "pellets", "an old nest"],
            &[
                "trill", "whistle", "croak", "coo", "chatter", "piping", "drumming",
            ],
            Some("roost"),
            "day",
            6,
            500,
        ),
        "migrant" => (
            "bird",
            &["feathers", "an old nest", "white droppings"],
            &["twitter", "honk", "piping", "trill"],
            Some("colony"),
            "day",
            5,
            700,
        ),
        "insect" => (
            "none",
            &["a nest in the ground", "webs", "galls", "chewed leaves"],
            &["hum", "chirr", "whine", "buzz"],
            None,
            "day",
            9,
            300,
        ),
        "small" => (
            "tiny",
            &["spawn", "a shed skin", "slime trails"],
            &["croak", "chirp", "rustle"],
            None,
            "night",
            5,
            650,
        ),
        "fish" => (
            "none",
            &["rises", "scales on the stones", "small fry in the shallows"],
            &[],
            None,
            "day",
            6,
            600,
        ),
        _ => (
            "claw",
            &["scratches on the stone", "gnawed bones", "a musky smell"],
            &["scraping"],
            None,
            "night",
            1,
            0,
        ),
    }
}

/// What plants of a role bear, are used for, and when they flower.
fn plant_habits(role: &str, h: u64) -> (&'static str, &'static str, Option<usize>, Option<usize>) {
    let (fruits, uses): (&[&str], &[&str]) = match role {
        "tree" => (
            &["nuts", "berries", "cones", "seeds", "pods"],
            &["fuel", "food", "fibre", "dye", "fuel"],
        ),
        "shrub" => (
            &["berries", "hips", "nuts", "berries", "none"],
            &["food", "dye", "fuel", "remedy", "harmful"],
        ),
        "grass" => (&["seeds"], &["fibre", "food", "none", "fibre"]),
        "flower" => (
            &["seeds", "pods", "seeds"],
            &["dye", "remedy", "harmful", "none", "remedy"],
        ),
        "fern" => (&["spores"], &["none", "fuel", "harmful", "remedy"]),
        "fungus" => (&["none"], &["food", "harmful", "remedy", "food", "none"]),
        "climber" => (
            &["berries", "seeds"],
            &["food", "fibre", "harmful", "remedy"],
        ),
        "moss" => (&["none"], &["fuel", "remedy", "none"]),
        _ => (&["seeds", "none"], &["food", "none", "fibre"]),
    };
    let fruit = *pick(fruits, h);
    let use_ = *pick(uses, h >> 8);
    let (flowers, fruits) = match role {
        "fungus" => (None, Some(2)),
        "fern" | "moss" => (None, None),
        "grass" | "water" => (Some(1), Some(1 + (h >> 16) as usize % 2)),
        _ => {
            let fl = (h >> 16) as usize % 2;
            let fr = if fruit == "none" { None } else { Some(fl + 1) };
            (Some(fl), fr)
        }
    };
    (fruit, use_, flowers, fruits)
}

/// The climate bands a habitat spans: those holding at least a fifth of
/// its cells.
fn climates(temps: &[f64]) -> u8 {
    let n = temps.len().max(1);
    let count = |f: &dyn Fn(f64) -> bool| temps.iter().filter(|&&t| f(t)).count();
    let mut out = 0;
    if count(&|t| t < 6.0) * 5 >= n {
        out |= C;
    }
    if count(&|t| (6.0..=14.0).contains(&t)) * 5 >= n {
        out |= T;
    }
    if count(&|t| t > 14.0) * 5 >= n {
        out |= W;
    }
    if out == 0 {
        T
    } else {
        out
    }
}

/// The least habitat area (cells) that holds its own species.
const MIN_CELLS: usize = 25;

impl Life {
    /// The species and homes of a world.
    pub fn generate(seed: u64, t: &Terrain, w: &Water) -> Self {
        let mut cells: Vec<Vec<(usize, usize)>> = vec![Vec::new(); HABITATS.len()];
        let mut temps: Vec<Vec<f64>> = vec![Vec::new(); HABITATS.len()];
        for y in 0..SIZE {
            for x in 0..SIZE {
                let h = Habitat::of(t, w, x, y);
                cells[h as usize].push((x, y));
                temps[h as usize].push(*t.temperature.get(x, y));
            }
        }
        let land: usize = [
            Habitat::Open,
            Habitat::Woods,
            Habitat::Pine,
            Habitat::Wet,
            Habitat::Dry,
            Habitat::Cold,
        ]
        .iter()
        .map(|h| cells[*h as usize].len())
        .sum();
        let mut life = Life::default();
        let mut used: Vec<&str> = Vec::new();
        for hab in HABITATS {
            let n = cells[hab as usize].len();
            if hab != Habitat::Deep && n < MIN_CELLS {
                continue;
            }
            let band = if hab == Habitat::Deep {
                ALL
            } else {
                climates(&temps[hab as usize])
            };
            // A habitat covering more than a fifth of the land holds one
            // more of its first role and its birds.
            let big = hab != Habitat::Deep && n * 5 > land;
            let (animals, plants) = roles(hab);
            for (kingdom, table, list) in [
                (Kingdom::Animal, ANIMALS, animals),
                (Kingdom::Plant, PLANTS, plants),
            ] {
                for (ri, &(role, count)) in list.iter().enumerate() {
                    let extra = big && (ri == 0 || role == "bird" || role == "flower");
                    for k in 0..(count + u8::from(extra)) {
                        let h = mix(&[
                            seed,
                            0x11fe,
                            hab as u64,
                            ri as u64,
                            k as u64,
                            kingdom as u64,
                        ]);
                        let fits = |fm: &&Form| {
                            fm.role == role
                                && fm.habitats & hab.bit() != 0
                                && fm.climate & band != 0
                        };
                        let fresh: Vec<&Form> = table
                            .iter()
                            .filter(fits)
                            .filter(|fm| !used.contains(&fm.form))
                            .collect();
                        let any: Vec<&Form> = table.iter().filter(fits).collect();
                        let pool = if fresh.is_empty() { any } else { fresh };
                        if pool.is_empty() {
                            continue;
                        }
                        let form = *pick(&pool, h);
                        // A form already met elsewhere is told apart by its
                        // colour and mark; the same pair twice is skipped.
                        let climate = [C, T, W]
                            .into_iter()
                            .filter(|c| form.climate & band & c != 0)
                            .nth((h >> 20) as usize % (form.climate & band).count_ones() as usize)
                            .unwrap_or(T);
                        let colour = *pick(colours(kingdom, role, climate), h >> 24);
                        let mark = *pick(marks(kingdom, role), h >> 32);
                        if life
                            .species
                            .iter()
                            .any(|s| s.form == form.form && s.colour == colour && s.mark == mark)
                        {
                            continue;
                        }
                        used.push(form.form);
                        life.species.push(Self::make(
                            life.species.len(),
                            kingdom,
                            form,
                            hab,
                            climate,
                            colour,
                            mark,
                            h,
                        ));
                    }
                }
            }
        }
        // Homes: two or three per species that keeps one, in its habitat.
        for s in &life.species {
            if s.home.is_none() || s.habitat == Habitat::Deep {
                continue;
            }
            let pool = &cells[s.habitat as usize];
            let n = 2 + mix(&[seed, 0x40e5, s.id as u64]) as usize % 2;
            for k in 0..n {
                let (x, y) = *pick(pool, mix(&[seed, 0x40e6, s.id as u64, k as u64]));
                life.homes.push(Home {
                    species: s.id,
                    cell: Cell::new(x, y),
                });
            }
        }
        life
    }

    #[allow(clippy::too_many_arguments)]
    fn make(
        id: usize,
        kingdom: Kingdom,
        form: &Form,
        habitat: Habitat,
        climate: u8,
        colour: &'static str,
        mark: &'static str,
        h: u64,
    ) -> Species {
        let climate_id = match climate {
            C => "cold",
            W => "warm",
            _ => "temperate",
        };
        let mut s = Species {
            id,
            kingdom,
            role: form.role,
            form: form.form,
            habitat,
            climate: climate_id,
            size: "middling",
            colour,
            mark,
            active: "day",
            foot: "none",
            signs: Vec::new(),
            call: None,
            home: None,
            present: [true; 4],
            flowers: None,
            fruits: None,
            fruit: "none",
            evergreen: false,
            use_: "none",
            abundance: 5,
            tolerance: 500,
        };
        // About one species in eight is gone from the land at the start
        // and returns only where a region recovers.
        let returner = (h >> 44).is_multiple_of(8) && form.role != "deep";
        let spread = ((h >> 40) % 200) as i32 - 100;
        if kingdom == Kingdom::Animal {
            let (foot, signs, calls, home, active, abundance, tolerance) = habits(form.role);
            s.foot = foot;
            // Each species leaves two of its role's signs.
            let a = (h >> 48) as usize % signs.len();
            s.signs = vec![signs[a], signs[(a + 1) % signs.len()]];
            s.call = (!calls.is_empty()).then(|| *pick(calls, h >> 52));
            s.home = home;
            s.active = if form.form == "owl" { "night" } else { active };
            s.abundance = abundance;
            s.tolerance = if returner {
                1100 + spread.abs() * 2
            } else {
                tolerance + spread
            };
            s.size = match form.role {
                "grazer" | "browser" if !matches!(form.form, "hare" | "porcupine") => "large",
                "predator" if matches!(form.form, "bear" | "big cat" | "wolf") => "large",
                "insect" | "small" | "burrower" => "small",
                "bird" | "migrant"
                    if matches!(form.form, "heron" | "crane" | "stork" | "goose") =>
                {
                    "large"
                }
                "bird" | "migrant" => "small",
                _ => "middling",
            };
            if form.role == "migrant" {
                // Summer visitors in cold and temperate lands, winter
                // visitors in warm ones.
                s.present = if climate == W {
                    [false, false, true, true]
                } else {
                    [true, true, false, false]
                };
            }
        } else {
            let (fruit, use_, flowers, fruits) = plant_habits(form.role, h);
            s.fruit = fruit;
            s.use_ = use_;
            s.flowers = flowers;
            s.fruits = fruits;
            s.evergreen = matches!(
                form.form,
                "pine"
                    | "spruce"
                    | "fir"
                    | "yew"
                    | "olive"
                    | "palm"
                    | "cedar"
                    | "juniper"
                    | "ivy"
                    | "heather"
                    | "gorse"
                    | "myrtle"
                    | "crowberry"
                    | "bilberry"
                    | "oleander"
            ) || matches!(form.role, "moss" | "water");
            s.abundance = match form.role {
                "grass" | "moss" => 9,
                "tree" | "shrub" => 7,
                "flower" | "fern" => 6,
                _ => 4,
            };
            s.tolerance = if returner {
                1100 + spread.abs() * 2
            } else {
                400 + spread
            };
            s.size = match form.role {
                "tree" => "large",
                "shrub" | "fern" | "climber" => "middling",
                _ => "small",
            };
        }
        s
    }

    /// The species living in a habitat.
    pub fn of(&self, h: Habitat) -> impl Iterator<Item = &Species> {
        self.species.iter().filter(move |s| s.habitat == h)
    }

    pub fn count(&self, k: Kingdom) -> usize {
        self.species.iter().filter(|s| s.kingdom == k).count()
    }
}

// ---------- vocabulary, for content ----------

/// Every animal and plant role.
pub const ROLES: &[&str] = &[
    "grazer",
    "browser",
    "predator",
    "scavenger",
    "burrower",
    "bird",
    "migrant",
    "insect",
    "small",
    "fish",
    "deep",
    "tree",
    "shrub",
    "grass",
    "flower",
    "fern",
    "fungus",
    "climber",
    "moss",
    "water",
];
pub const SIZES: &[&str] = &["small", "middling", "large"];
pub const FEET: &[&str] = &["hoof", "paw", "small paw", "bird", "tiny", "claw", "none"];
pub const STATES: &[&str] = &[
    "in flower",
    "in fruit",
    "in leaf",
    "turning",
    "bare",
    "died back",
    "green",
];
pub const FRUITS: &[&str] = &[
    "nuts", "berries", "cones", "seeds", "pods", "hips", "spores", "none",
];
pub const USES: &[&str] = &["food", "fibre", "dye", "fuel", "remedy", "harmful", "none"];
const ANIMAL_ROLES: &[&str] = &[
    "grazer",
    "browser",
    "predator",
    "scavenger",
    "burrower",
    "bird",
    "migrant",
    "insect",
    "small",
    "fish",
    "deep",
];
const PLANT_ROLES: &[&str] = &[
    "tree", "shrub", "grass", "flower", "fern", "fungus", "climber", "moss", "water",
];

fn uniq(mut v: Vec<&'static str>) -> Vec<&'static str> {
    v.sort_unstable();
    v.dedup();
    v
}

/// Every body plan of a kingdom.
pub fn forms(k: Kingdom) -> Vec<&'static str> {
    let t = if k == Kingdom::Animal {
        ANIMALS
    } else {
        PLANTS
    };
    uniq(t.iter().map(|f| f.form).collect())
}

/// Every sign an animal can leave, tracks included.
pub fn signs() -> Vec<&'static str> {
    let mut v = vec!["tracks"];
    for r in ANIMAL_ROLES {
        v.extend(habits(r).1);
    }
    uniq(v)
}

pub fn calls() -> Vec<&'static str> {
    uniq(
        ANIMAL_ROLES
            .iter()
            .flat_map(|r| habits(r).2.iter().copied())
            .collect(),
    )
}

pub fn homes() -> Vec<&'static str> {
    uniq(ANIMAL_ROLES.iter().filter_map(|r| habits(r).3).collect())
}

pub fn colours_all() -> Vec<&'static str> {
    let mut v = Vec::new();
    for (k, roles) in [
        (Kingdom::Animal, ANIMAL_ROLES),
        (Kingdom::Plant, PLANT_ROLES),
    ] {
        for r in roles {
            for c in [C, T, W] {
                v.extend(colours(k, r, c));
            }
        }
    }
    uniq(v)
}

pub fn marks_all() -> Vec<&'static str> {
    let mut v = Vec::new();
    for (k, roles) in [
        (Kingdom::Animal, ANIMAL_ROLES),
        (Kingdom::Plant, PLANT_ROLES),
    ] {
        for r in roles {
            v.extend(marks(k, r));
        }
    }
    uniq(v)
}

// ---------- populations ----------

/// One habitat's grazing animals and the hunters that eat them, as
/// fractions of what the land can hold, season by season.
///
/// Prey grow towards the land's capacity and are eaten; hunters grow when
/// prey are many and starve when they are few. The two swing against each
/// other and settle; the land's life sets the capacity, so a dying region
/// holds fewer of both.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Cycle {
    pub prey: f64,
    pub hunters: f64,
}

// DESIGN-Q: a damped predator and prey cycle of a few years, stepped by
// season; spring and summer are the breeding seasons.
impl Cycle {
    pub fn start(seed: u64, habitat: Habitat) -> Self {
        let h = mix(&[seed, 0xc1c1, habitat as u64]);
        Cycle {
            prey: 0.5 + (h % 400) as f64 / 1000.0,
            hunters: 0.2 + ((h >> 16) % 300) as f64 / 1000.0,
        }
    }

    /// One season on, with `capacity` the land's life (1.0: as at the
    /// start).
    pub fn step(self, season: usize, capacity: f64) -> Self {
        let k = capacity.clamp(0.05, 1.6);
        let breeding = if season % 4 <= 1 { 0.5 } else { 0.05 };
        let eaten = 0.35 * self.prey * self.hunters;
        let prey = self.prey + breeding * self.prey * (1.0 - self.prey / k) - eaten;
        let hunters = self.hunters + 0.6 * eaten - 0.12 * self.hunters;
        Cycle {
            prey: prey.clamp(0.02, 2.0 * k),
            hunters: hunters.clamp(0.01, k),
        }
    }

    /// The cycle after some seasons at a steady capacity.
    pub fn after(seed: u64, habitat: Habitat, seasons: u32, capacity: f64) -> Self {
        let mut c = Self::start(seed, habitat);
        for s in 0..seasons {
            c = c.step(s as usize, capacity);
        }
        c
    }

    /// How a role fares in the cycle: 1.0 is ordinary.
    pub fn factor(&self, role: &str) -> f64 {
        match role {
            "predator" | "scavenger" => self.hunters / 0.35,
            "grazer" | "browser" | "burrower" => self.prey / 0.7,
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(seed: u64) -> (Terrain, Water) {
        let mut t = Terrain::generate(seed);
        let w = Water::generate(&mut t);
        (t, w)
    }

    #[test]
    fn worlds_hold_forty_animals_and_forty_plants() {
        for seed in [1, 2, 3, 42] {
            let (t, w) = world(seed);
            let life = Life::generate(seed, &t, &w);
            let a = life.count(Kingdom::Animal);
            let p = life.count(Kingdom::Plant);
            assert!(a >= 40 && p >= 40, "seed {seed}: {a} animals, {p} plants");
        }
    }

    #[test]
    fn species_suit_their_habitat_and_climate() {
        let (t, w) = world(7);
        let life = Life::generate(7, &t, &w);
        for s in &life.species {
            let table = if s.kingdom == Kingdom::Animal {
                ANIMALS
            } else {
                PLANTS
            };
            let form = table.iter().find(|f| f.form == s.form).unwrap();
            assert!(
                form.habitats & s.habitat.bit() != 0,
                "{} in {:?}",
                s.form,
                s.habitat
            );
        }
        for h in &life.homes {
            let s = &life.species[h.species];
            assert_eq!(Habitat::of(&t, &w, h.cell.ux(), h.cell.uy()), s.habitat);
        }
    }

    #[test]
    fn life_is_deterministic() {
        let (t, w) = world(5);
        let a = serde_json::to_string(&Life::generate(5, &t, &w)).unwrap();
        let b = serde_json::to_string(&Life::generate(5, &t, &w)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn populations_stay_within_bounds_for_ten_years() {
        for seed in 0..20 {
            for hab in HABITATS {
                let mut c = Cycle::start(seed, hab);
                for s in 0..40 {
                    c = c.step(s, 1.0);
                    assert!(c.prey > 0.02 && c.prey < 1.6, "{seed} {hab:?} {s} {c:?}");
                    assert!(
                        c.hunters > 0.01 && c.hunters < 1.0,
                        "{seed} {hab:?} {s} {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_dying_land_holds_fewer() {
        let full = Cycle::after(3, Habitat::Open, 40, 1.0);
        let poor = Cycle::after(3, Habitat::Open, 40, 0.3);
        assert!(poor.prey < full.prey && poor.hunters <= full.hunters);
    }

    #[test]
    fn plants_change_with_the_seasons() {
        let (t, w) = world(9);
        let life = Life::generate(9, &t, &w);
        let tree = life
            .species
            .iter()
            .find(|s| s.role == "tree" && !s.evergreen)
            .unwrap();
        assert_eq!(tree.state(3), "bare");
        assert!(life.species.iter().any(|s| s.state(0) == "in flower"));
    }
}
