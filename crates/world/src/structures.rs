//! Buildings placed by history, and their interiors.
//!
//! Room purposes and feature kinds are data ids; what they look like is
//! content for M05–M06. Interiors come from a few generators with authored
//! shapes (house, temple, archive, tomb, tower), varied by seed, and (D03)
//! a table of room plans for the many newer kinds of building, which each
//! town's role calls for.

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

use crate::history::{Cell, EventKind, History, Role};
use crate::terrain::{Biome, Terrain, SIZE};
use crate::towns::{Town, TownRole};
use crate::water::Water;

/// What a building is. The first eleven are the original kinds with their
/// own interior generators; the rest (D03) share a table-driven placeholder
/// layout until D04 replaces every interior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StructureKind {
    House,
    Temple,
    Storehouse,
    Archive,
    Tomb,
    Cemetery,
    Tower,
    Wall,
    Waystation,
    Bridge,
    Mine,
    Mill,
    Granary,
    Bakehouse,
    Brewery,
    Bathhouse,
    Cistern,
    Aqueduct,
    #[serde(rename = "fountain house")]
    FountainHouse,
    #[serde(rename = "market hall")]
    MarketHall,
    Warehouse,
    Harbour,
    Lighthouse,
    Smithy,
    Kiln,
    Tannery,
    #[serde(rename = "dye works")]
    DyeWorks,
    #[serde(rename = "weaving house")]
    WeavingHouse,
    Scriptorium,
    School,
    Library,
    Observatory,
    Palace,
    #[serde(rename = "council hall")]
    CouncilHall,
    Courthouse,
    Prison,
    Barracks,
    Armoury,
    Gatehouse,
    #[serde(rename = "signal station")]
    SignalStation,
    Hermitage,
    #[serde(rename = "wayside shrine")]
    WaysideShrine,
    Ossuary,
    Catacombs,
    Garden,
    Orchard,
    Amphitheatre,
    Mausoleum,
    Labyrinth,
    #[serde(rename = "processional way")]
    ProcessionalWay,
    /// A cave system behind a cave mouth (D04).
    Cave,
    #[serde(rename = "sea cave")]
    SeaCave,
}

/// What a kind of building was for, broadly: decides what is found in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Dwelling,
    Holy,
    Burial,
    Store,
    Food,
    Craft,
    Learning,
    Water,
    Rule,
    Defence,
    Travel,
    Trade,
    Leisure,
    Garden,
    Mining,
    /// Not built: caves.
    Natural,
}

/// One room of a placeholder layout: purpose, level, features (`None`
/// material: the local building stuff) and whether it may be left out.
pub struct RoomPlan {
    pub purpose: &'static str,
    pub level: i8,
    pub features: &'static [(&'static str, Option<Material>)],
    pub optional: bool,
}

/// Facts about a kind of building: its id (for content), how tall it
/// stands and how much it draws the eye from afar, its family, and the
/// rooms of its placeholder layout (empty for the original kinds, which
/// have their own generators).
pub struct KindInfo {
    pub id: &'static str,
    pub height: f64,
    pub weight: f64,
    pub family: Family,
    pub rooms: &'static [RoomPlan],
}

const fn r(
    purpose: &'static str,
    level: i8,
    features: &'static [(&'static str, Option<Material>)],
) -> RoomPlan {
    RoomPlan {
        purpose,
        level,
        features,
        optional: false,
    }
}

const fn opt(
    purpose: &'static str,
    level: i8,
    features: &'static [(&'static str, Option<Material>)],
) -> RoomPlan {
    RoomPlan {
        purpose,
        level,
        features,
        optional: true,
    }
}

const STONE: Option<Material> = Some(Material::Stone);
const WOOD: Option<Material> = Some(Material::Wood);
const CLAY: Option<Material> = Some(Material::Clay);
const METAL: Option<Material> = Some(Material::Metal);
const PLASTER: Option<Material> = Some(Material::Plaster);
const LOCAL: Option<Material> = None;

const fn info(
    id: &'static str,
    height: f64,
    weight: f64,
    family: Family,
    rooms: &'static [RoomPlan],
) -> KindInfo {
    KindInfo {
        id,
        height,
        weight,
        family,
        rooms,
    }
}

/// Every kind's id, in the order of `StructureKind::ALL`.
pub const IDS: [&str; 52] = [
    "house",
    "temple",
    "storehouse",
    "archive",
    "tomb",
    "cemetery",
    "tower",
    "wall",
    "waystation",
    "bridge",
    "mine",
    "mill",
    "granary",
    "bakehouse",
    "brewery",
    "bathhouse",
    "cistern",
    "aqueduct",
    "fountain house",
    "market hall",
    "warehouse",
    "harbour",
    "lighthouse",
    "smithy",
    "kiln",
    "tannery",
    "dye works",
    "weaving house",
    "scriptorium",
    "school",
    "library",
    "observatory",
    "palace",
    "council hall",
    "courthouse",
    "prison",
    "barracks",
    "armoury",
    "gatehouse",
    "signal station",
    "hermitage",
    "wayside shrine",
    "ossuary",
    "catacombs",
    "garden",
    "orchard",
    "amphitheatre",
    "mausoleum",
    "labyrinth",
    "processional way",
    "cave",
    "sea cave",
];

impl StructureKind {
    /// Every kind, in order.
    pub const ALL: [StructureKind; 52] = {
        use StructureKind::*;
        [
            House,
            Temple,
            Storehouse,
            Archive,
            Tomb,
            Cemetery,
            Tower,
            Wall,
            Waystation,
            Bridge,
            Mine,
            Mill,
            Granary,
            Bakehouse,
            Brewery,
            Bathhouse,
            Cistern,
            Aqueduct,
            FountainHouse,
            MarketHall,
            Warehouse,
            Harbour,
            Lighthouse,
            Smithy,
            Kiln,
            Tannery,
            DyeWorks,
            WeavingHouse,
            Scriptorium,
            School,
            Library,
            Observatory,
            Palace,
            CouncilHall,
            Courthouse,
            Prison,
            Barracks,
            Armoury,
            Gatehouse,
            SignalStation,
            Hermitage,
            WaysideShrine,
            Ossuary,
            Catacombs,
            Garden,
            Orchard,
            Amphitheatre,
            Mausoleum,
            Labyrinth,
            ProcessionalWay,
            Cave,
            SeaCave,
        ]
    };

    pub fn id(self) -> &'static str {
        self.info().id
    }

    pub fn info(self) -> &'static KindInfo {
        use Family::*;
        use StructureKind as K;
        match self {
            K::House => {
                const I: KindInfo = info("house", 4.0, 4.0, Dwelling, &[]);
                &I
            }
            K::Temple => {
                const I: KindInfo = info("temple", 10.0, 15.0, Holy, &[]);
                &I
            }
            K::Storehouse => {
                const I: KindInfo = info("storehouse", 5.0, 6.0, Store, &[]);
                &I
            }
            K::Archive => {
                const I: KindInfo = info("archive", 6.0, 8.0, Learning, &[]);
                &I
            }
            K::Tomb => {
                const I: KindInfo = info("tomb", 3.0, 8.0, Burial, &[]);
                &I
            }
            K::Cemetery => {
                const I: KindInfo = info("cemetery", 1.0, 4.0, Burial, &[]);
                &I
            }
            K::Tower => {
                const I: KindInfo = info("tower", 15.0, 25.0, Defence, &[]);
                &I
            }
            K::Wall => {
                const I: KindInfo = info("wall", 5.0, 10.0, Defence, &[]);
                &I
            }
            K::Waystation => {
                const I: KindInfo = info("waystation", 4.0, 12.0, Travel, &[]);
                &I
            }
            K::Bridge => {
                const I: KindInfo = info("bridge", 3.0, 10.0, Travel, &[]);
                &I
            }
            K::Mine => {
                const I: KindInfo = info("mine", 2.0, 6.0, Mining, &[]);
                &I
            }
            K::Mill => {
                const I: KindInfo = info(
                    "mill",
                    8.0,
                    10.0,
                    Food,
                    &[
                        r("millroom", 0, &[("millstone", STONE), ("waterwheel", WOOD)]),
                        opt("loft", 1, &[("bin", WOOD)]),
                    ],
                );
                &I
            }
            K::Granary => {
                const I: KindInfo = info(
                    "granary",
                    7.0,
                    8.0,
                    Store,
                    &[
                        r("grain-floor", 0, &[("bin", WOOD), ("jar", CLAY)]),
                        opt("loft", 1, &[("bin", WOOD)]),
                    ],
                );
                &I
            }
            K::Bakehouse => {
                const I: KindInfo = info(
                    "bakehouse",
                    4.0,
                    4.0,
                    Food,
                    &[
                        r("bakery", 0, &[("oven", CLAY), ("table", WOOD)]),
                        opt("store", 0, &[("shelf", WOOD), ("jar", CLAY)]),
                    ],
                );
                &I
            }
            K::Brewery => {
                const I: KindInfo = info(
                    "brewery",
                    5.0,
                    5.0,
                    Food,
                    &[
                        r("brewhouse", 0, &[("vat", WOOD), ("jar", CLAY)]),
                        opt("cellar", -1, &[("cask", WOOD)]),
                    ],
                );
                &I
            }
            K::Bathhouse => {
                const I: KindInfo = info(
                    "bathhouse",
                    6.0,
                    8.0,
                    Water,
                    &[
                        r("changing-room", 0, &[("bench", STONE), ("niche", LOCAL)]),
                        r("warm-room", 0, &[("pool", STONE)]),
                        opt("cold-room", 0, &[("pool", STONE)]),
                        opt("furnace", -1, &[("hearth", STONE)]),
                    ],
                );
                &I
            }
            K::Cistern => {
                const I: KindInfo = info(
                    "cistern",
                    2.0,
                    4.0,
                    Water,
                    &[
                        r("cistern-head", 0, &[("basin", STONE)]),
                        r("cistern-hall", -1, &[("pillar", STONE)]),
                    ],
                );
                &I
            }
            K::Aqueduct => {
                const I: KindInfo = info(
                    "aqueduct",
                    12.0,
                    18.0,
                    Water,
                    &[r("channel", 0, &[("pier", STONE), ("stele", STONE)])],
                );
                &I
            }
            K::FountainHouse => {
                const I: KindInfo = info(
                    "fountain house",
                    5.0,
                    6.0,
                    Water,
                    &[r(
                        "fountain-room",
                        0,
                        &[("fountain", STONE), ("basin", STONE)],
                    )],
                );
                &I
            }
            K::MarketHall => {
                const I: KindInfo = info(
                    "market hall",
                    7.0,
                    10.0,
                    Trade,
                    &[
                        r("market-floor", 0, &[("stall", WOOD), ("table", WOOD)]),
                        opt("weighhouse", 0, &[("scales", METAL), ("tablet", CLAY)]),
                    ],
                );
                &I
            }
            K::Warehouse => {
                const I: KindInfo = info(
                    "warehouse",
                    6.0,
                    6.0,
                    Store,
                    &[
                        r("warehouse-floor", 0, &[("crate", WOOD), ("rack", WOOD)]),
                        opt("counting-room", 0, &[("table", WOOD), ("tablet", CLAY)]),
                    ],
                );
                &I
            }
            K::Harbour => {
                const I: KindInfo = info(
                    "harbour",
                    3.0,
                    12.0,
                    Trade,
                    &[
                        r("quay", 0, &[("bollard", STONE), ("crate", WOOD)]),
                        opt("harbour-office", 0, &[("table", WOOD), ("tablet", CLAY)]),
                    ],
                );
                &I
            }
            K::Lighthouse => {
                const I: KindInfo = info(
                    "lighthouse",
                    25.0,
                    30.0,
                    Travel,
                    &[
                        r("keeper's room", 0, &[("hearth", STONE)]),
                        r("stairwell", 1, &[("wall", LOCAL)]),
                        r("lamp-room", 2, &[("beacon", METAL)]),
                    ],
                );
                &I
            }
            K::Smithy => {
                const I: KindInfo = info(
                    "smithy",
                    4.0,
                    5.0,
                    Craft,
                    &[
                        r("forge", 0, &[("anvil", METAL), ("hearth", STONE)]),
                        opt("store", 0, &[("rack", WOOD)]),
                    ],
                );
                &I
            }
            K::Kiln => {
                const I: KindInfo = info(
                    "kiln",
                    5.0,
                    6.0,
                    Craft,
                    &[
                        r(
                            "potter's workshop",
                            0,
                            &[("potter's wheel", WOOD), ("shelf", WOOD)],
                        ),
                        r("kiln-room", 0, &[("kiln", CLAY), ("jar", CLAY)]),
                    ],
                );
                &I
            }
            K::Tannery => {
                const I: KindInfo = info(
                    "tannery",
                    4.0,
                    4.0,
                    Craft,
                    &[
                        r("tanning-yard", 0, &[("vat", STONE)]),
                        opt("drying-room", 0, &[("rack", WOOD)]),
                    ],
                );
                &I
            }
            K::DyeWorks => {
                const I: KindInfo = info(
                    "dye works",
                    4.0,
                    4.0,
                    Craft,
                    &[
                        r("dye-yard", 0, &[("vat", CLAY), ("basin", STONE)]),
                        opt("drying-room", 0, &[("rack", WOOD)]),
                    ],
                );
                &I
            }
            K::WeavingHouse => {
                const I: KindInfo = info(
                    "weaving house",
                    5.0,
                    5.0,
                    Craft,
                    &[
                        r("loom-room", 0, &[("loom", WOOD), ("shelf", WOOD)]),
                        opt("store", 0, &[("chest", WOOD)]),
                    ],
                );
                &I
            }
            K::Scriptorium => {
                const I: KindInfo = info(
                    "scriptorium",
                    6.0,
                    6.0,
                    Learning,
                    &[
                        r("writing-room", 0, &[("desk", WOOD), ("shelf", WOOD)]),
                        opt("store", 0, &[("chest", WOOD), ("jar", CLAY)]),
                    ],
                );
                &I
            }
            K::School => {
                const I: KindInfo = info(
                    "school",
                    5.0,
                    5.0,
                    Learning,
                    &[
                        r("schoolroom", 0, &[("bench", WOOD), ("wall", PLASTER)]),
                        opt("yard", 0, &[("basin", STONE)]),
                    ],
                );
                &I
            }
            K::Library => {
                const I: KindInfo = info(
                    "library",
                    8.0,
                    10.0,
                    Learning,
                    &[
                        r("entrance", 0, &[("lintel", STONE)]),
                        r("reading", 0, &[("table", WOOD), ("wall", PLASTER)]),
                        r("stacks", 0, &[("rack", WOOD), ("shelf", WOOD)]),
                        opt("stacks", 1, &[("rack", WOOD)]),
                    ],
                );
                &I
            }
            K::Observatory => {
                const I: KindInfo = info(
                    "observatory",
                    14.0,
                    20.0,
                    Learning,
                    &[
                        r("lower hall", 0, &[("table", WOOD), ("wall", LOCAL)]),
                        r("stairwell", 1, &[("wall", LOCAL)]),
                        r("platform", 2, &[("dial", STONE)]),
                    ],
                );
                &I
            }
            K::Palace => {
                const I: KindInfo = info(
                    "palace",
                    12.0,
                    25.0,
                    Rule,
                    &[
                        r("gate-court", 0, &[("statue", STONE), ("basin", STONE)]),
                        r("throne-room", 0, &[("throne", STONE), ("wall", PLASTER)]),
                        opt("residence", 1, &[("hearth", STONE), ("chest", WOOD)]),
                        opt("treasury", -1, &[("chest", METAL)]),
                    ],
                );
                &I
            }
            K::CouncilHall => {
                const I: KindInfo = info(
                    "council hall",
                    8.0,
                    10.0,
                    Rule,
                    &[
                        r("council-chamber", 0, &[("bench", STONE), ("table", WOOD)]),
                        opt("store", 0, &[("chest", WOOD)]),
                    ],
                );
                &I
            }
            K::Courthouse => {
                const I: KindInfo = info(
                    "courthouse",
                    7.0,
                    8.0,
                    Rule,
                    &[
                        r("court", 0, &[("bench", WOOD), ("table", WOOD)]),
                        opt("holding cell", 0, &[("bars", METAL)]),
                    ],
                );
                &I
            }
            K::Prison => {
                const I: KindInfo = info(
                    "prison",
                    6.0,
                    7.0,
                    Rule,
                    &[
                        r("guardroom", 0, &[("table", WOOD)]),
                        r("cells", 0, &[("bars", METAL)]),
                        opt("pit", -1, &[("wall", STONE)]),
                    ],
                );
                &I
            }
            K::Barracks => {
                const I: KindInfo = info(
                    "barracks",
                    6.0,
                    7.0,
                    Defence,
                    &[
                        r("dormitory", 0, &[("bench", WOOD)]),
                        r("mess", 0, &[("hearth", STONE), ("table", WOOD)]),
                        opt("yard", 0, &[("rack", WOOD)]),
                    ],
                );
                &I
            }
            K::Armoury => {
                const I: KindInfo = info(
                    "armoury",
                    5.0,
                    5.0,
                    Defence,
                    &[r("armoury-store", 0, &[("rack", WOOD), ("chest", METAL)])],
                );
                &I
            }
            K::Gatehouse => {
                const I: KindInfo = info(
                    "gatehouse",
                    10.0,
                    15.0,
                    Defence,
                    &[
                        r("gate-passage", 0, &[("gate", WOOD), ("wall", STONE)]),
                        opt("guardroom", 1, &[("wall", STONE)]),
                    ],
                );
                &I
            }
            K::SignalStation => {
                const I: KindInfo = info(
                    "signal station",
                    10.0,
                    18.0,
                    Defence,
                    &[
                        r("watch", 0, &[("hearth", STONE)]),
                        r("beacon-platform", 1, &[("beacon", STONE)]),
                    ],
                );
                &I
            }
            K::Hermitage => {
                const I: KindInfo = info(
                    "hermitage",
                    3.0,
                    5.0,
                    Holy,
                    &[r("cell", 0, &[("bench", STONE), ("niche", STONE)])],
                );
                &I
            }
            K::WaysideShrine => {
                const I: KindInfo = info(
                    "wayside shrine",
                    2.0,
                    6.0,
                    Holy,
                    &[r("shrine", 0, &[("niche", STONE), ("altar", STONE)])],
                );
                &I
            }
            K::Ossuary => {
                const I: KindInfo = info(
                    "ossuary",
                    5.0,
                    6.0,
                    Burial,
                    &[r("bone-hall", 0, &[("niche", STONE), ("wall", STONE)])],
                );
                &I
            }
            K::Catacombs => {
                const I: KindInfo = info(
                    "catacombs",
                    2.0,
                    6.0,
                    Burial,
                    &[
                        r("catacomb-entrance", 0, &[("door-slab", STONE)]),
                        r("gallery", -1, &[("niche", STONE)]),
                        opt("burial", -1, &[("sarcophagus", STONE)]),
                    ],
                );
                &I
            }
            K::Garden => {
                const I: KindInfo = info(
                    "garden",
                    1.0,
                    4.0,
                    Garden,
                    &[r("garden", 0, &[("bench", STONE), ("basin", STONE)])],
                );
                &I
            }
            K::Orchard => {
                const I: KindInfo = info(
                    "orchard",
                    3.0,
                    5.0,
                    Garden,
                    &[r("orchard", 0, &[("tree", WOOD), ("wall", LOCAL)])],
                );
                &I
            }
            K::Amphitheatre => {
                const I: KindInfo = info(
                    "amphitheatre",
                    12.0,
                    22.0,
                    Leisure,
                    &[
                        r("arena", 0, &[("seat", STONE), ("statue", STONE)]),
                        opt("passage", -1, &[("wall", STONE)]),
                    ],
                );
                &I
            }
            K::Mausoleum => {
                const I: KindInfo = info(
                    "mausoleum",
                    8.0,
                    14.0,
                    Burial,
                    &[
                        r("mausoleum-hall", 0, &[("statue", STONE), ("wall", STONE)]),
                        r("burial", -1, &[("sarcophagus", STONE)]),
                    ],
                );
                &I
            }
            K::Labyrinth => {
                const I: KindInfo = info(
                    "labyrinth",
                    4.0,
                    10.0,
                    Holy,
                    &[
                        r("maze-entrance", 0, &[("lintel", STONE)]),
                        r("turning", 0, &[("wall", STONE)]),
                        r("turning", 0, &[("wall", STONE)]),
                        r("heart", 0, &[("altar", STONE)]),
                    ],
                );
                &I
            }
            K::ProcessionalWay => {
                const I: KindInfo = info(
                    "processional way",
                    2.0,
                    8.0,
                    Holy,
                    &[r("way", 0, &[("statue", STONE), ("stele", STONE)])],
                );
                &I
            }
            K::Cave => {
                const I: KindInfo = info("cave", 0.0, 0.0, Natural, &[]);
                &I
            }
            K::SeaCave => {
                const I: KindInfo = info("sea cave", 0.0, 0.0, Natural, &[]);
                &I
            }
        }
    }
}

/// What became of a building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Condition {
    Intact,
    Worn,
    Damaged,
    Ruined,
    Buried,
}

/// What a surface is made of; decides what writing it takes (M08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    Stone,
    Clay,
    Wood,
    Metal,
    Plaster,
    Vellum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exit {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Exit {
    pub fn opposite(self) -> Exit {
        match self {
            Exit::North => Exit::South,
            Exit::South => Exit::North,
            Exit::East => Exit::West,
            Exit::West => Exit::East,
            Exit::Up => Exit::Down,
            Exit::Down => Exit::Up,
        }
    }
}

/// How two rooms connect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Passage {
    Door,
    Arch,
    Stair,
    Opening,
    // D04: great interiors and caves.
    Ramp,
    Ladder,
    /// A vertical shaft or pit: climbable with care, or a drop.
    Shaft,
    /// Low and tight: crawled through.
    Crawlway,
    /// Seen through, never passed: a window, a grating, a gallery rail.
    Window,
    /// A hole in a floor: a drop down that can't be climbed back.
    Hole,
    /// A loose panel or a stone that turns: hidden until found.
    Panel,
}

impl Passage {
    /// Whether it changes level.
    pub fn vertical(self) -> bool {
        matches!(
            self,
            Passage::Stair | Passage::Ramp | Passage::Ladder | Passage::Shaft | Passage::Hole
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PassageState {
    Open,
    Closed,
    Blocked,
}

/// Something in a room; may carry writing.
#[derive(Debug, Clone, Serialize)]
pub struct Feature {
    pub kind: &'static str,
    pub material: Material,
    /// Indices into `World::texts`.
    pub texts: Vec<usize>,
}

/// One space of an interior: a room, hall, corridor, stair, chamber or
/// passage, with real geometry (D04) so maps can be checked against it.
#[derive(Debug, Clone, Serialize)]
pub struct Room {
    pub purpose: &'static str,
    pub level: i8,
    pub features: Vec<Feature>,
    pub collapsed: bool,
    /// Where it lies: its north-west corner, in 2 m cells east and south
    /// of the entrance's corner (D04).
    pub x: i16,
    pub y: i16,
    /// Its footprint, in 2 m cells east-west and north-south.
    pub w: u8,
    pub d: u8,
    /// Its height in metres.
    pub h: u8,
    /// What shape of space it is: an id such as "room", "hall",
    /// "corridor", "stair", "courtyard", "gallery", "chamber", "passage",
    /// "crawl", "pit".
    pub space: &'static str,
    /// The era it was built in (caves: 0), and its building style.
    pub era: u32,
    pub style: &'static str,
    /// Leads straight outdoors (an entrance, or a cave's other mouth).
    pub outside: bool,
    /// Found only by a hidden way in (D04).
    pub hidden: bool,
    /// Water in it: "", "pool", "stream", "river", "flooded", "sump".
    pub water: &'static str,
    /// What makes it a landmark inside a great interior (D04): "vast" (one
    /// of its largest spaces), "lofty" (one of its highest) or "lone" (the
    /// only space of its purpose); "" for most.
    pub landmark: &'static str,
    /// "foul" where the air is bad (deep mines, caves, catacombs), else "".
    pub air: &'static str,
}

impl Room {
    /// Its area in square metres.
    pub fn area(&self) -> u32 {
        u32::from(self.w) * u32::from(self.d) * 4
    }

    /// Whether two spaces on the same level overlap.
    pub fn overlaps(&self, o: &Room) -> bool {
        self.level == o.level
            && self.x < o.x + i16::from(o.w)
            && o.x < self.x + i16::from(self.w)
            && self.y < o.y + i16::from(o.d)
            && o.y < self.y + i16::from(self.d)
    }

    /// Whether two spaces on the same level share a stretch of wall, and
    /// on which side of this one.
    pub fn touches(&self, o: &Room) -> Option<Exit> {
        if self.level != o.level {
            return None;
        }
        let along_x = self.x < o.x + i16::from(o.w) && o.x < self.x + i16::from(self.w);
        let along_y = self.y < o.y + i16::from(o.d) && o.y < self.y + i16::from(self.d);
        if along_x && o.y + i16::from(o.d) == self.y {
            Some(Exit::North)
        } else if along_x && self.y + i16::from(self.d) == o.y {
            Some(Exit::South)
        } else if along_y && self.x + i16::from(self.w) == o.x {
            Some(Exit::East)
        } else if along_y && o.x + i16::from(o.w) == self.x {
            Some(Exit::West)
        } else {
            None
        }
    }

    /// Whether two spaces on neighbouring levels lie one over the other.
    pub fn stacked(&self, o: &Room) -> bool {
        (i16::from(self.level) - i16::from(o.level)).abs() == 1
            && self.x < o.x + i16::from(o.w)
            && o.x < self.x + i16::from(self.w)
            && self.y < o.y + i16::from(o.d)
            && o.y < self.y + i16::from(self.d)
    }

    /// Its centre, in metres east and south of the entrance's corner.
    pub fn centre(&self) -> (f64, f64) {
        (
            (f64::from(self.x) + f64::from(self.w) / 2.0) * 2.0,
            (f64::from(self.y) + f64::from(self.d) / 2.0) * 2.0,
        )
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub a: usize,
    pub b: usize,
    /// Direction from a to b.
    pub exit: Exit,
    pub passage: Passage,
    pub state: PassageState,
    /// Passable only from a to b (a drop, a door barred on a's side) (D04).
    pub one_way: bool,
    /// Not noticed until found (a loose panel, a crawlway behind rubble).
    pub hidden: bool,
}

/// Rooms and how they connect. Room 0 is the entrance.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Interior {
    pub rooms: Vec<Room>,
    pub links: Vec<Link>,
}

impl Interior {
    pub(crate) fn room(
        &mut self,
        purpose: &'static str,
        level: i8,
        features: Vec<(&'static str, Material)>,
    ) -> usize {
        self.rooms.push(Room {
            purpose,
            level,
            features: features
                .into_iter()
                .map(|(kind, material)| Feature {
                    kind,
                    material,
                    texts: Vec::new(),
                })
                .collect(),
            collapsed: false,
            x: 0,
            y: 0,
            w: 0,
            d: 0,
            h: 3,
            space: "room",
            era: 0,
            style: "",
            outside: false,
            hidden: false,
            water: "",
            landmark: "",
            air: "",
        });
        self.rooms.len() - 1
    }

    pub(crate) fn link(&mut self, a: usize, b: usize, exit: Exit, passage: Passage) {
        self.links.push(Link {
            a,
            b,
            exit,
            passage,
            state: PassageState::Open,
            one_way: false,
            hidden: false,
        });
    }

    /// Rooms reachable from the entrance through passable links.
    pub fn reachable(&self) -> Vec<bool> {
        let mut seen = vec![false; self.rooms.len()];
        if seen.is_empty() {
            return seen;
        }
        let mut stack = vec![0];
        seen[0] = true;
        while let Some(r) = stack.pop() {
            for l in &self.links {
                if l.state == PassageState::Blocked || l.passage == Passage::Window {
                    continue;
                }
                let ways: &[(usize, usize)] = if l.one_way {
                    &[(l.a, l.b)][..]
                } else {
                    &[(l.a, l.b), (l.b, l.a)][..]
                };
                for &(from, to) in ways {
                    if from == r && !seen[to] && !self.rooms[to].collapsed {
                        seen[to] = true;
                        stack.push(to);
                    }
                }
            }
        }
        seen
    }
}

/// A building or monument.
#[derive(Debug, Clone, Serialize)]
pub struct Structure {
    pub id: usize,
    pub kind: StructureKind,
    pub cell: Cell,
    pub settlement: Option<usize>,
    pub era: u32,
    pub built: i32,
    /// The history event this structure is evidence of, if any.
    pub event: Option<usize>,
    /// For tombs: who lies there.
    pub person: Option<usize>,
    pub condition: Condition,
    pub interior: Interior,
    /// Writing on the outside (stele, lintel, milestone): indices into
    /// `World::texts`.
    pub outside: Vec<usize>,
    /// The district of its settlement it stands in (index into the
    /// town's districts).
    pub district: Option<usize>,
}

/// Materials by what a culture builds with nearby.
fn local_stone(t: &Terrain, c: Cell) -> Material {
    match t.biome.get(c.ux(), c.uy()) {
        Biome::Forest | Biome::Pine | Biome::Marsh => Material::Wood,
        Biome::Desert | Biome::Scrub | Biome::Grassland => Material::Clay,
        _ => Material::Stone,
    }
}

fn house(i: &mut Interior, rng: &mut Rng, m: Material) {
    let main = i.room(
        "hall",
        0,
        vec![("hearth", Material::Stone), ("wall", Material::Plaster)],
    );
    if rng.chance(70) {
        let back = i.room(
            "store",
            0,
            vec![("shelf", Material::Wood), ("jar", Material::Clay)],
        );
        i.link(main, back, Exit::North, Passage::Door);
    }
    if rng.chance(40) {
        let up = i.room("sleeping", 1, vec![("wall", m)]);
        i.link(main, up, Exit::Up, Passage::Stair);
    }
}

fn temple(i: &mut Interior, rng: &mut Rng, m: Material) {
    let court = i.room(
        "forecourt",
        0,
        vec![("stele", Material::Stone), ("basin", Material::Stone)],
    );
    let hall = i.room("hall", 0, vec![("wall", m), ("statue", Material::Stone)]);
    let sanctum = i.room("sanctum", 0, vec![("altar", Material::Stone), ("wall", m)]);
    i.link(court, hall, Exit::North, Passage::Door);
    i.link(hall, sanctum, Exit::North, Passage::Arch);
    for exit in [Exit::East, Exit::West] {
        if rng.chance(50) {
            let side = i.room("chapel", 0, vec![("niche", Material::Stone), ("wall", m)]);
            i.link(hall, side, exit, Passage::Arch);
        }
    }
    if rng.chance(45) {
        let crypt = i.room("crypt", -1, vec![("sarcophagus", Material::Stone)]);
        i.link(sanctum, crypt, Exit::Down, Passage::Stair);
    }
}

fn archive(i: &mut Interior, rng: &mut Rng, _m: Material) {
    let entry = i.room("entrance", 0, vec![("lintel", Material::Stone)]);
    let reading = i.room(
        "reading",
        0,
        vec![("table", Material::Wood), ("wall", Material::Plaster)],
    );
    i.link(entry, reading, Exit::North, Passage::Door);
    let mut last = reading;
    for n in 0..rng.range(2, 4) {
        let stacks = i.room(
            "stacks",
            0,
            vec![
                ("rack", Material::Wood),
                ("tablet", Material::Clay),
                ("scroll", Material::Vellum),
            ],
        );
        i.link(
            last,
            stacks,
            if n % 2 == 0 { Exit::East } else { Exit::North },
            Passage::Opening,
        );
        last = stacks;
    }
    let vault = i.room(
        "vault",
        -1,
        vec![("chest", Material::Metal), ("tablet", Material::Clay)],
    );
    i.link(last, vault, Exit::Down, Passage::Stair);
}

fn tomb(i: &mut Interior, rng: &mut Rng, _m: Material) {
    let entry = i.room("tomb-entrance", 0, vec![("door-slab", Material::Stone)]);
    let passage = i.room("passage", -1, vec![("wall", Material::Stone)]);
    let chamber = i.room(
        "burial",
        -1,
        vec![("sarcophagus", Material::Stone), ("wall", Material::Stone)],
    );
    i.link(entry, passage, Exit::Down, Passage::Stair);
    i.link(passage, chamber, Exit::North, Passage::Door);
    if rng.chance(50) {
        let side = i.room(
            "offerings",
            -1,
            vec![("jar", Material::Clay), ("shelf", Material::Stone)],
        );
        i.link(chamber, side, Exit::East, Passage::Opening);
    }
}

fn tower(i: &mut Interior, rng: &mut Rng, m: Material) {
    let mut below = i.room("guardroom", 0, vec![("wall", m)]);
    for level in 1..=rng.range(2, 4) as i8 {
        let r = i.room(
            if level == 1 { "watch" } else { "lookout" },
            level,
            vec![("wall", m)],
        );
        i.link(below, r, Exit::Up, Passage::Stair);
        below = r;
    }
}

fn simple(i: &mut Interior, purpose: &'static str, features: Vec<(&'static str, Material)>) {
    i.room(purpose, 0, features);
}

fn interior(kind: StructureKind, rng: &mut Rng, m: Material) -> Interior {
    let mut i = Interior::default();
    match kind {
        StructureKind::House => house(&mut i, rng, m),
        StructureKind::Temple => temple(&mut i, rng, m),
        StructureKind::Archive => archive(&mut i, rng, m),
        StructureKind::Tomb => tomb(&mut i, rng, m),
        StructureKind::Tower => tower(&mut i, rng, m),
        StructureKind::Storehouse => {
            let a = i.room("storeroom", 0, vec![("jar", Material::Clay), ("wall", m)]);
            let b = i.room(
                "tally-room",
                0,
                vec![("table", Material::Wood), ("tablet", Material::Clay)],
            );
            i.link(a, b, Exit::East, Passage::Door);
        }
        StructureKind::Cemetery => simple(
            &mut i,
            "graves",
            vec![
                ("gravestone", Material::Stone),
                ("gravestone", Material::Stone),
            ],
        ),
        StructureKind::Wall => simple(
            &mut i,
            "gatehouse",
            vec![("gate", Material::Wood), ("wall", Material::Stone)],
        ),
        StructureKind::Waystation => simple(
            &mut i,
            "shelter",
            vec![("milestone", Material::Stone), ("hearth", Material::Stone)],
        ),
        StructureKind::Bridge => simple(&mut i, "span", vec![("parapet", Material::Stone)]),
        StructureKind::Mine => {
            let a = i.room("adit", 0, vec![("beam", Material::Wood)]);
            let b = i.room("gallery", -1, vec![("wall", Material::Stone)]);
            i.link(a, b, Exit::Down, Passage::Opening);
        }
        k => planned(&mut i, rng, m, k.info().rooms),
    }
    i
}

/// A placeholder layout from a kind's room plan (D03; D04 replaces it):
/// rooms in a line, each through a door from the last, stairs between
/// levels; optional rooms are left out a third of the time.
fn planned(i: &mut Interior, rng: &mut Rng, m: Material, plan: &[RoomPlan]) {
    let mut last: Option<(usize, i8)> = None;
    let mut turn = 0;
    for p in plan {
        if p.optional && rng.chance(33) {
            continue;
        }
        let features = p
            .features
            .iter()
            .map(|&(f, mat)| (f, mat.unwrap_or(m)))
            .collect();
        let room = i.room(p.purpose, p.level, features);
        if let Some((prev, level)) = last {
            let (exit, passage) = match p.level.cmp(&level) {
                std::cmp::Ordering::Greater => (Exit::Up, Passage::Stair),
                std::cmp::Ordering::Less => (Exit::Down, Passage::Stair),
                std::cmp::Ordering::Equal => {
                    turn += 1;
                    (
                        if turn % 2 == 1 {
                            Exit::North
                        } else {
                            Exit::East
                        },
                        if turn % 3 == 0 {
                            Passage::Arch
                        } else {
                            Passage::Door
                        },
                    )
                }
            };
            i.link(prev, room, exit, passage);
        }
        last = Some((room, p.level));
    }
}

/// Places every structure history implies.
pub fn place(seed: u64, t: &Terrain, w: &Water, h: &History) -> Vec<Structure> {
    let mut rng = Rng::new(seed, Stream::World(3));
    let mut out: Vec<Structure> = Vec::new();
    #[allow(clippy::too_many_arguments)]
    fn add(
        out: &mut Vec<Structure>,
        t: &Terrain,
        rng: &mut Rng,
        kind: StructureKind,
        cell: Cell,
        settlement: Option<usize>,
        era: u32,
        built: i32,
        event: Option<usize>,
        person: Option<usize>,
    ) {
        let m = local_stone(t, cell);
        let id = out.len();
        let interior = interior(kind, rng, m);
        out.push(Structure {
            id,
            kind,
            cell,
            settlement,
            era,
            built,
            event,
            person,
            condition: Condition::Intact,
            interior,
            outside: Vec::new(),
            district: None,
        });
    }
    let near = |rng: &mut Rng, c: Cell, r: u32| -> Cell {
        for _ in 0..30 {
            let x = (i32::from(c.x) + rng.range(0, 2 * r) as i32 - r as i32)
                .clamp(1, SIZE as i32 - 2) as usize;
            let y = (i32::from(c.y) + rng.range(0, 2 * r) as i32 - r as i32)
                .clamp(1, SIZE as i32 - 2) as usize;
            if t.is_land(x, y) && !w.is_river(t, x, y) {
                return Cell::new(x, y);
            }
        }
        c
    };

    for s in &h.settlements {
        let founding = h
            .events
            .iter()
            .find(|e| matches!(e.kind, EventKind::Founding { settlement } if settlement == s.id))
            .map(|e| e.id);
        add(
            &mut out,
            t,
            &mut rng,
            StructureKind::Temple,
            s.cell,
            Some(s.id),
            s.era,
            s.founded,
            founding,
            None,
        );
        // Houses are rebuilt while a town lives: they date from the last
        // era it was standing in.
        let last_era = h
            .eras
            .iter()
            .rev()
            .find(|e| e.start >= s.founded && s.abandoned.is_none_or(|a| a > e.start + 20))
            .map(|e| e.index)
            .unwrap_or(s.era);
        for _ in 0..s.size + 1 {
            let c = near(&mut rng, s.cell, 1);
            let start = h.eras[last_era as usize].start.max(s.founded);
            let built = start + rng.below(60) as i32;
            add(
                &mut out,
                t,
                &mut rng,
                StructureKind::House,
                c,
                Some(s.id),
                last_era,
                built,
                None,
                None,
            );
        }
        let c = near(&mut rng, s.cell, 1);
        add(
            &mut out,
            t,
            &mut rng,
            StructureKind::Storehouse,
            c,
            Some(s.id),
            s.era,
            s.founded + 10,
            None,
            None,
        );
        let c = near(&mut rng, s.cell, 2);
        add(
            &mut out,
            t,
            &mut rng,
            StructureKind::Cemetery,
            c,
            Some(s.id),
            s.era,
            s.founded + 5,
            None,
            None,
        );
        if s.capital {
            add(
                &mut out,
                t,
                &mut rng,
                StructureKind::Archive,
                s.cell,
                Some(s.id),
                s.era,
                s.founded + 30,
                None,
                None,
            );
        }
        // Mines where rock is close.
        let rocky = (0..12).any(|_| {
            let c = near(&mut rng, s.cell, 6);
            matches!(
                t.biome.get(c.ux(), c.uy()),
                Biome::Rock | Biome::Pine | Biome::Tundra
            )
        });
        if rocky && rng.chance(50) {
            let c = near(&mut rng, s.cell, 5);
            add(
                &mut out,
                t,
                &mut rng,
                StructureKind::Mine,
                c,
                Some(s.id),
                s.era,
                s.founded + 40,
                None,
                None,
            );
        }
    }

    for e in &h.events {
        match &e.kind {
            EventKind::Death { person, .. } => {
                let p = &h.people[*person];
                let s = &h.settlements[p.settlement];
                // Rulers get their own tomb; others lie in the cemetery.
                if p.role == Role::Ruler {
                    let c = near(&mut rng, s.cell, 3);
                    add(
                        &mut out,
                        t,
                        &mut rng,
                        StructureKind::Tomb,
                        c,
                        Some(s.id),
                        e.era,
                        e.year,
                        Some(e.id),
                        Some(*person),
                    );
                }
            }
            EventKind::War { settlement, .. } => {
                let s = &h.settlements[*settlement];
                let c = near(&mut rng, s.cell, 1);
                add(
                    &mut out,
                    t,
                    &mut rng,
                    StructureKind::Wall,
                    c,
                    Some(s.id),
                    e.era,
                    e.year,
                    Some(e.id),
                    None,
                );
                let c = near(&mut rng, s.cell, 2);
                add(
                    &mut out,
                    t,
                    &mut rng,
                    StructureKind::Tower,
                    c,
                    Some(s.id),
                    e.era,
                    e.year,
                    Some(e.id),
                    None,
                );
            }
            _ => {}
        }
    }

    for r in &h.roads {
        // Bridges where the road crosses a river it cannot ford.
        for c in &r.path {
            if w.needs_crossing(t, c.ux(), c.uy())
                && !out
                    .iter()
                    .any(|s| s.kind == StructureKind::Bridge && s.cell == *c)
            {
                add(
                    &mut out,
                    t,
                    &mut rng,
                    StructureKind::Bridge,
                    *c,
                    None,
                    r.era,
                    h.eras[r.era as usize].start + 40,
                    None,
                    None,
                );
            }
        }
        if r.path.len() > 30 {
            let c = r.path[r.path.len() / 2];
            add(
                &mut out,
                t,
                &mut rng,
                StructureKind::Waystation,
                c,
                None,
                r.era,
                h.eras[r.era as usize].start + 60,
                None,
                None,
            );
        }
    }
    out
}

// DESIGN-Q: which buildings each role calls for, and how many (size + 1,
// four more in the capital).
/// Buildings a town of a role may have: the first is always built (when
/// the ground allows), the rest are picked by size.
fn role_kinds(role: TownRole) -> &'static [StructureKind] {
    use StructureKind::*;
    match role {
        TownRole::Capital => &[
            Palace,
            CouncilHall,
            Courthouse,
            Library,
            MarketHall,
            School,
            Bathhouse,
            Amphitheatre,
            ProcessionalWay,
            FountainHouse,
            Barracks,
            Mausoleum,
            Observatory,
            Scriptorium,
            Garden,
            Granary,
            Prison,
            Aqueduct,
        ],
        TownRole::Port => &[
            Harbour,
            Lighthouse,
            Warehouse,
            MarketHall,
            Smithy,
            Tannery,
            DyeWorks,
            FountainHouse,
            Bathhouse,
            Brewery,
            Granary,
        ],
        TownRole::HolyCity => &[
            ProcessionalWay,
            Ossuary,
            Catacombs,
            Scriptorium,
            Mausoleum,
            Garden,
            Observatory,
            School,
            Labyrinth,
            FountainHouse,
            Library,
        ],
        TownRole::MiningCamp => &[
            Smithy, Kiln, Barracks, Granary, Bakehouse, Brewery, Warehouse,
        ],
        TownRole::Fortress => &[
            Gatehouse,
            Barracks,
            Armoury,
            Prison,
            Cistern,
            Smithy,
            Granary,
            CouncilHall,
        ],
        TownRole::MarketTown => &[
            MarketHall,
            Warehouse,
            WeavingHouse,
            DyeWorks,
            Brewery,
            Bakehouse,
            Smithy,
            Courthouse,
            Bathhouse,
            School,
            Tannery,
        ],
        TownRole::FarmingVillage => &[
            Mill,
            Granary,
            Bakehouse,
            Orchard,
            Brewery,
            Kiln,
            WaysideShrine,
        ],
        TownRole::Refuge => &[
            Cistern,
            Granary,
            WeavingHouse,
            Kiln,
            Hermitage,
            Orchard,
            Garden,
        ],
    }
}

/// The districts a kind of building prefers, best first.
fn districts_for(k: StructureKind) -> &'static [&'static str] {
    use StructureKind::*;
    match k {
        Temple | ProcessionalWay | Labyrinth => &["sacred way", "temple quarter", "square"],
        Palace | CouncilHall | Courthouse => &["palace quarter", "square", "upper town"],
        Library | Scriptorium | School | Observatory | Archive => {
            &["scholars' quarter", "temple quarter", "square"]
        }
        MarketHall | Warehouse | Storehouse => &["market", "harbour", "square"],
        Harbour | Lighthouse => &["harbour"],
        Smithy | Kiln | Tannery | DyeWorks | WeavingHouse | Bakehouse | Brewery => {
            &["workshops", "market", "mines", "farmyards"]
        }
        Mill | Granary | Orchard => &["farmyards", "workshops"],
        Garden | Bathhouse | FountainHouse | Amphitheatre => {
            &["gardens", "palace quarter", "square"]
        }
        Barracks | Armoury | Prison | Gatehouse | Tower | Wall => &["garrison", "upper town"],
        Cemetery | Tomb | Ossuary | Catacombs | Mausoleum => {
            &["graves", "sacred way", "temple quarter"]
        }
        Mine => &["mines"],
        Cistern | Hermitage => &["upper town", "square"],
        House => &[],
        _ => &["square"],
    }
}

/// Whether the ground allows a kind of building at a town.
fn allowed(k: StructureKind, t: &Terrain, w: &Water, c: Cell) -> bool {
    use StructureKind::*;
    match k {
        Harbour | Lighthouse => crate::towns::coastal(t, c),
        Mill => {
            let mut wet = false;
            for dy in -2i64..=2 {
                for dx in -2i64..=2 {
                    let (x, y) = (i64::from(c.x) + dx, i64::from(c.y) + dy);
                    if x >= 0
                        && y >= 0
                        && x < SIZE as i64
                        && y < SIZE as i64
                        && *w.flow.get(x as usize, y as usize) >= crate::water::RIVER_FLOW / 4
                    {
                        wet = true;
                    }
                }
            }
            wet
        }
        Aqueduct => source(t, c).is_some(),
        _ => true,
    }
}

/// High ground an aqueduct could bring water from: the highest land cell
/// within 10 cells, if well above the town.
fn source(t: &Terrain, c: Cell) -> Option<Cell> {
    let here = *t.height.get(c.ux(), c.uy());
    let mut best: Option<(f64, Cell)> = None;
    for dy in -10i64..=10 {
        for dx in -10i64..=10 {
            let (x, y) = (i64::from(c.x) + dx, i64::from(c.y) + dy);
            if x < 1
                || y < 1
                || x >= SIZE as i64 - 1
                || y >= SIZE as i64 - 1
                || dx * dx + dy * dy < 25
            {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            let h = *t.height.get(x, y);
            if t.is_land(x, y) && best.is_none_or(|(b, _)| h > b) {
                best = Some((h, Cell::new(x, y)));
            }
        }
    }
    best.filter(|(h, _)| *h > here + 80.0).map(|(_, c)| c)
}

/// The era a year falls in.
fn era_of(h: &History, year: i32) -> u32 {
    h.eras
        .iter()
        .rev()
        .find(|e| year >= e.start)
        .map_or(0, |e| e.index)
}

/// D03: the buildings each town's role calls for, and those history put
/// out on the land (signal stations, wayside shrines, hermitages, an
/// aqueduct). Appended after the original buildings, from their own
/// random stream, so the original ones are unchanged. Also gives every
/// town building its district.
pub fn place_more(
    seed: u64,
    t: &Terrain,
    w: &Water,
    h: &History,
    towns: &[Town],
    out: &mut Vec<Structure>,
) {
    let mut rng = Rng::new(seed, Stream::World(8));
    let new = |out: &mut Vec<Structure>,
               rng: &mut Rng,
               kind: StructureKind,
               cell: Cell,
               settlement: Option<usize>,
               built: i32,
               event: Option<usize>,
               district: Option<usize>| {
        let m = local_stone(t, cell);
        let id = out.len();
        let interior = interior(kind, rng, m);
        out.push(Structure {
            id,
            kind,
            cell,
            settlement,
            era: era_of(h, built),
            built,
            event,
            person: None,
            condition: Condition::Intact,
            interior,
            outside: Vec::new(),
            district,
        });
    };
    let near = |rng: &mut Rng, c: Cell| -> Cell {
        for _ in 0..20 {
            let x =
                (i32::from(c.x) + rng.range(0, 2) as i32 - 1).clamp(1, SIZE as i32 - 2) as usize;
            let y =
                (i32::from(c.y) + rng.range(0, 2) as i32 - 1).clamp(1, SIZE as i32 - 2) as usize;
            if t.is_land(x, y) && !w.is_river(t, x, y) {
                return Cell::new(x, y);
            }
        }
        c
    };
    // The original buildings: each in the district nearest it, or the
    // one its kind prefers when that lies close by.
    for st in out.iter_mut() {
        let Some(s) = st.settlement else { continue };
        let town = &towns[s];
        let pref = districts_for(st.kind).iter().find_map(|k| {
            town.districts
                .iter()
                .position(|d| d.kind == *k && d.cell.dist2(st.cell) <= 8)
        });
        st.district = Some(pref.unwrap_or_else(|| town.district_at(st.cell)));
    }
    for town in towns {
        let s = &h.settlements[town.settlement];
        let founding = h
            .events
            .iter()
            .find(|e| matches!(e.kind, EventKind::Founding { settlement } if settlement == s.id))
            .map(|e| e.id);
        let end = s.abandoned.unwrap_or(h.present);
        let pool = role_kinds(town.role);
        let mut chosen: Vec<StructureKind> = Vec::new();
        if let Some(&first) = pool.iter().find(|&&k| allowed(k, t, w, s.cell)) {
            chosen.push(first);
        }
        let mut rest: Vec<StructureKind> = pool
            .iter()
            .copied()
            .filter(|k| !chosen.contains(k) && allowed(*k, t, w, s.cell))
            .collect();
        rng.shuffle(&mut rest);
        let more = usize::from(s.size) + 1 + if town.role == TownRole::Capital { 4 } else { 0 };
        chosen.extend(rest.into_iter().take(more));
        // A gatehouse at each gate of a walled town.
        if town.walled {
            for _ in 0..town.gates.min(3) {
                chosen.push(StructureKind::Gatehouse);
            }
        }
        for kind in chosen {
            let district = districts_for(kind)
                .iter()
                .find_map(|k| town.districts.iter().position(|d| d.kind == *k))
                .unwrap_or(0);
            let base = town.districts[district].cell;
            let span = (end - s.founded).max(20) as u32;
            let built = s.founded + 10 + rng.below(span) as i32;
            let built = built.min(end - 5).max(s.founded);
            match kind {
                StructureKind::Aqueduct => {
                    // Out on the land, between the town and its source.
                    let Some(src) = source(t, s.cell) else {
                        continue;
                    };
                    let mid = Cell::new(
                        ((src.ux() + 2 * s.cell.ux()) / 3).clamp(1, SIZE - 2),
                        ((src.uy() + 2 * s.cell.uy()) / 3).clamp(1, SIZE - 2),
                    );
                    // On dry land, as near the line as can be.
                    let mut best: Option<(i64, Cell)> = None;
                    for dy in -3i64..=3 {
                        for dx in -3i64..=3 {
                            let (x, y) = (i64::from(mid.x) + dx, i64::from(mid.y) + dy);
                            if x < 1 || y < 1 || x >= SIZE as i64 - 1 || y >= SIZE as i64 - 1 {
                                continue;
                            }
                            let (x, y) = (x as usize, y as usize);
                            let d = dx * dx + dy * dy;
                            if t.is_land(x, y)
                                && !w.is_river(t, x, y)
                                && best.is_none_or(|(b, _)| d < b)
                            {
                                best = Some((d, Cell::new(x, y)));
                            }
                        }
                    }
                    let Some((_, c)) = best else { continue };
                    new(out, &mut rng, kind, c, None, built, founding, None);
                }
                _ => {
                    let c = near(&mut rng, base);
                    new(
                        out,
                        &mut rng,
                        kind,
                        c,
                        Some(s.id),
                        built,
                        founding,
                        Some(district),
                    );
                }
            }
        }
    }
    // Out on the land.
    for e in &h.events {
        if let EventKind::War { settlement, .. } = e.kind {
            // A signal station on the highest ground in sight of the town.
            let s = &h.settlements[settlement];
            let mut best: Option<(f64, Cell)> = None;
            for dy in -12i64..=12 {
                for dx in -12i64..=12 {
                    let d2 = dx * dx + dy * dy;
                    if !(36..=144).contains(&d2) {
                        continue;
                    }
                    let (x, y) = (i64::from(s.cell.x) + dx, i64::from(s.cell.y) + dy);
                    if x < 1 || y < 1 || x >= SIZE as i64 - 1 || y >= SIZE as i64 - 1 {
                        continue;
                    }
                    let (x, y) = (x as usize, y as usize);
                    let hh = *t.height.get(x, y);
                    if t.is_land(x, y) && !w.is_river(t, x, y) && best.is_none_or(|(b, _)| hh > b) {
                        best = Some((hh, Cell::new(x, y)));
                    }
                }
            }
            if let Some((_, c)) = best {
                if !out
                    .iter()
                    .any(|st| st.kind == StructureKind::SignalStation && st.cell.dist2(c) < 16)
                {
                    new(
                        out,
                        &mut rng,
                        StructureKind::SignalStation,
                        c,
                        None,
                        e.year,
                        Some(e.id),
                        None,
                    );
                }
            }
        }
    }
    for r in &h.roads {
        if r.path.len() > 16 {
            let c = r.path[r.path.len() / 4];
            if t.is_land(c.ux(), c.uy()) && !w.is_river(t, c.ux(), c.uy()) {
                let built = h.eras[r.era as usize].start + 30;
                new(
                    out,
                    &mut rng,
                    StructureKind::WaysideShrine,
                    c,
                    None,
                    built,
                    None,
                    None,
                );
            }
        }
    }
    for town in towns.iter().filter(|tw| {
        matches!(
            tw.role,
            TownRole::HolyCity | TownRole::Refuge | TownRole::Capital
        )
    }) {
        // A hermitage far from roads, on high ground some way off.
        let s = &h.settlements[town.settlement];
        let mut best: Option<(i64, Cell)> = None;
        for _ in 0..60 {
            let x = (i32::from(s.cell.x) + rng.range(0, 20) as i32 - 10).clamp(1, SIZE as i32 - 2)
                as usize;
            let y = (i32::from(s.cell.y) + rng.range(0, 20) as i32 - 10).clamp(1, SIZE as i32 - 2)
                as usize;
            let c = Cell::new(x, y);
            if !t.is_land(x, y) || w.is_river(t, x, y) || c.dist2(s.cell) < 25 {
                continue;
            }
            let road = h
                .roads
                .iter()
                .flat_map(|r| r.path.iter())
                .map(|p| p.dist2(c))
                .min()
                .unwrap_or(i64::MAX)
                .min(400);
            let score = road * 10 + (*t.height.get(x, y) / 10.0) as i64;
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, c));
            }
        }
        if let Some((_, c)) = best {
            let built = s.founded + 40;
            new(
                out,
                &mut rng,
                StructureKind::Hermitage,
                c,
                None,
                built,
                None,
                None,
            );
        }
    }
}
