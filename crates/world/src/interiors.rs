//! Interiors as real places (D04): every space has a position, a
//! footprint, a height and a level, so directions and distances inside are
//! honest and a paper map can be checked against the truth.
//!
//! Three things happen here:
//!
//! - **Layout.** The small interiors of ordinary buildings, generated as
//!   room graphs, are laid out on a 2 m grid: each room beside the one it
//!   opens from, on the side its exit says.
//! - **Growth.** A few buildings in each world become great interiors,
//!   grown the way real ones come to exist: a core in the founding era's
//!   manner, then wings, floors and cellars added era by era in each era's
//!   own manner (its module, its symmetry, its courtyards), doors blocked
//!   and new ones cut, so the plan is a palimpsest with causes. Mirrored
//!   wings leave gaps where a space is reached only by a hidden way, which
//!   a careful map shows.
//! - **Caves.** Cave mouths (D03) open into cave systems grown from the
//!   rock and the water: passages along the rock's fractures, chambers
//!   where they meet, pits down to wetter levels, formations, sumps, and
//!   other mouths.
//!
//! Each great interior grows from its own random stream, keyed by the
//! building, so it is the same whenever it is generated.

use std::collections::HashMap;

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

use crate::features::Feature as LandFeature;
use crate::geology::{Geology, Rock};
use crate::history::{Cell, History};
use crate::structures::{
    Condition, Exit, Feature, Interior, Link, Material, Passage, PassageState, Room, Structure,
    StructureKind,
};
use crate::terrain::Terrain;
use crate::towns::{Town, TownRole};

/// Every shape of space. Ids for content.
pub const SPACES: &[&str] = &[
    "room",
    "hall",
    "corridor",
    "stair",
    "courtyard",
    "gallery",
    "chamber",
    "passage",
    "crawl",
    "pit",
    "cell",
    "tunnel",
    "shaft",
];

/// Building styles, oldest first: the manner of each era. Ids for content.
pub const STYLES: &[&str] = &[
    "rough-hewn",
    "dressed stone",
    "vaulted",
    "brick and plaster",
    "fine ashlar",
    "painted",
];

/// The style of caves and of natural passages people broke into.
pub const NATURAL: &str = "natural";

/// Purposes the great interiors and caves give their spaces (beyond the
/// ordinary buildings' own). Ids for content.
pub const PURPOSES: &[&str] = &[
    "audience hall",
    "kitchen",
    "dormitory",
    "refectory",
    "cloister",
    "colonnade",
    "storeroom",
    "workshop",
    "shrine",
    "bath",
    "lightwell",
    "landing",
    "undercroft",
    "cistern-hall",
    "tunnel",
    "ossuary",
    "burial gallery",
    "adit",
    "working",
    "sump",
    "cavern",
    "grotto",
    "squeeze",
    "chimney",
    "stream passage",
    "mouth",
    "dwelling",
    "maze",
    "guardroom",
    "cells",
    "stacks",
    "reading",
    "treasury",
    "residence",
    "chapel",
    "crypt",
    "gallery",
    "stairwell",
    "corridor",
];

/// Furnishings and formations the great interiors and caves add. Ids for
/// content (thing kinds).
pub const FURNISHINGS: &[&str] = &[
    "stalactites",
    "flowstone",
    "crystals",
    "column",
    "fossil",
    "rubble",
    "trough",
    "cot",
];

// ---------- layout of ordinary interiors ----------

/// The footprint of a room by its purpose: (w, d, height) in 2 m cells
/// and metres.
fn footprint(purpose: &str) -> (u8, u8, u8) {
    match purpose {
        "hall" | "council-chamber" | "throne-room" | "market-floor" | "arena" | "court" => {
            (6, 5, 6)
        }
        "forecourt" | "gate-court" | "yard" | "garden" | "orchard" | "quay" | "tanning-yard"
        | "dye-yard" => (5, 5, 0),
        "sanctum" | "reading" | "brewhouse" | "millroom" | "warm-room" | "dormitory" | "mess" => {
            (4, 4, 4)
        }
        "passage" | "gallery" | "adit" | "channel" | "way" | "turning" | "gate-passage" => {
            (2, 5, 3)
        }
        "stairwell" | "watch" | "lookout" | "beacon-platform" | "lamp-room" | "platform" => {
            (3, 3, 3)
        }
        "stacks" | "graves" | "burial" | "cells" | "bone-hall" => (4, 3, 3),
        _ => (3, 3, 3),
    }
}

/// The shape of space a room is, by its purpose.
fn space_of(purpose: &str) -> &'static str {
    match purpose {
        "hall" | "throne-room" | "council-chamber" | "market-floor" | "arena" | "court"
        | "audience hall" | "refectory" => "hall",
        "passage" | "gate-passage" | "turning" | "corridor" => "corridor",
        "stairwell" | "landing" => "stair",
        "forecourt" | "gate-court" | "yard" | "garden" | "orchard" | "quay" | "cloister"
        | "lightwell" => "courtyard",
        "gallery" | "adit" | "working" | "burial gallery" | "stacks" => "gallery",
        "cell" | "cells" | "holding cell" => "cell",
        "channel" | "way" | "tunnel" | "colonnade" => "tunnel",
        _ => "room",
    }
}

/// Lays out an interior built as a room graph: room 0 at the origin, each
/// other room beside the one it opens from (on the side the link says,
/// or above or below for stairs), nowhere overlapping.
pub fn lay_out(i: &mut Interior, era: u32) {
    if i.rooms.is_empty() {
        return;
    }
    for r in &mut i.rooms {
        let (w, d, h) = footprint(r.purpose);
        r.w = w;
        r.d = d;
        r.h = if h == 0 { 0 } else { h };
        r.space = space_of(r.purpose);
        r.era = era;
    }
    i.rooms[0].outside = true;
    let mut placed = vec![false; i.rooms.len()];
    placed[0] = true;
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(a) = queue.pop_front() {
        let links: Vec<(usize, Exit, Passage)> = i
            .links
            .iter()
            .filter_map(|l| {
                if l.a == a {
                    Some((l.b, l.exit, l.passage))
                } else if l.b == a {
                    Some((l.a, l.exit.opposite(), l.passage))
                } else {
                    None
                }
            })
            .collect();
        for (b, exit, _) in links {
            if placed[b] {
                continue;
            }
            let ra = i.rooms[a].clone();
            let mut rb = i.rooms[b].clone();
            let free = |rooms: &[Room], placed: &[bool], r: &Room| {
                !rooms
                    .iter()
                    .enumerate()
                    .any(|(k, o)| placed[k] && o.overlaps(r))
            };
            let ok = match exit {
                Exit::Up | Exit::Down => {
                    // Above or below, over some of the same ground: the
                    // full size if it fits, else smaller.
                    rb.level = ra.level + if exit == Exit::Up { 1 } else { -1 };
                    let mut found = false;
                    'size: for (w, d) in [
                        (rb.w, rb.d),
                        (rb.w.min(ra.w), rb.d.min(ra.d)),
                        (2, 2),
                        (1, 1),
                    ] {
                        rb.w = w;
                        rb.d = d;
                        for y in (ra.y - i16::from(d) + 1)..(ra.y + i16::from(ra.d)) {
                            for x in (ra.x - i16::from(w) + 1)..(ra.x + i16::from(ra.w)) {
                                rb.x = x;
                                rb.y = y;
                                if free(&i.rooms, &placed, &rb) {
                                    found = true;
                                    break 'size;
                                }
                            }
                        }
                    }
                    found
                }
                side => {
                    // Beside it on its side, else on another side (the
                    // geometry then decides the way's direction), at full
                    // size or smaller.
                    rb.level = ra.level;
                    let sides = [side, side.opposite(), turn(side), turn(side).opposite()];
                    let mut found = false;
                    'outer: for (w, d) in [(rb.w, rb.d), (2, 2), (1, 1)] {
                        for s in sides {
                            rb.w = w;
                            rb.d = d;
                            if place_beside(&i.rooms, &placed, &ra, &mut rb, s) {
                                found = true;
                                break 'outer;
                            }
                        }
                    }
                    found
                }
            };
            debug_assert!(ok, "no room for a room");
            let _ = ok;
            i.rooms[b] = rb;
            placed[b] = true;
            queue.push_back(b);
        }
    }
    // Links say what the geometry says.
    for l in &mut i.links {
        let (ra, rb) = (&i.rooms[l.a], &i.rooms[l.b]);
        if ra.level == rb.level {
            if let Some(e) = ra.touches(rb) {
                l.exit = e;
            }
        } else {
            l.exit = if rb.level > ra.level {
                Exit::Up
            } else {
                Exit::Down
            };
        }
    }
}

/// A quarter turn clockwise.
fn turn(e: Exit) -> Exit {
    match e {
        Exit::North => Exit::East,
        Exit::East => Exit::South,
        Exit::South => Exit::West,
        Exit::West => Exit::North,
        other => other,
    }
}

/// Puts `rb` beside `ra` on `side`, sliding along that side until it
/// overlaps nothing placed. Returns whether a spot was found.
fn place_beside(rooms: &[Room], placed: &[bool], ra: &Room, rb: &mut Room, side: Exit) -> bool {
    let (wa, da) = (i16::from(ra.w), i16::from(ra.d));
    let (wb, db) = (i16::from(rb.w), i16::from(rb.d));
    let offsets: Vec<i16> = match side {
        Exit::North | Exit::South => (-(wb - 1)..wa).collect(),
        _ => (-(db - 1)..da).collect(),
    };
    // Centred first, then outwards.
    let centre = match side {
        Exit::North | Exit::South => (wa - wb) / 2,
        _ => (da - db) / 2,
    };
    let mut order = offsets.clone();
    order.sort_by_key(|o| ((o - centre).abs(), *o));
    for o in order {
        let (x, y) = match side {
            Exit::North => (ra.x + o, ra.y - db),
            Exit::South => (ra.x + o, ra.y + da),
            Exit::East => (ra.x + wa, ra.y + o),
            _ => (ra.x - wb, ra.y + o),
        };
        rb.x = x;
        rb.y = y;
        if !rooms
            .iter()
            .enumerate()
            .any(|(k, other)| placed[k] && other.overlaps(rb))
        {
            return true;
        }
    }
    false
}

// ---------- growth of great interiors ----------

/// What kind of great interior a building grows into. Ids for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GreatKind {
    Palace,
    Precinct,
    Fortress,
    Library,
    Necropolis,
    Mine,
    Cistern,
    Refuge,
    Labyrinth,
    Cave,
}

impl GreatKind {
    pub fn id(self) -> &'static str {
        match self {
            GreatKind::Palace => "palace complex",
            GreatKind::Precinct => "temple precinct",
            GreatKind::Fortress => "fortress",
            GreatKind::Library => "library complex",
            GreatKind::Necropolis => "necropolis",
            GreatKind::Mine => "mine",
            GreatKind::Cistern => "cistern tunnels",
            GreatKind::Refuge => "underground city",
            GreatKind::Labyrinth => "labyrinth",
            GreatKind::Cave => "cave system",
        }
    }
}

/// Every kind of great interior, by id.
pub const GREAT_KINDS: [&str; 10] = [
    "palace complex",
    "temple precinct",
    "fortress",
    "library complex",
    "necropolis",
    "mine",
    "cistern tunnels",
    "underground city",
    "labyrinth",
    "cave system",
];

/// One era's manner of building: a culture's grammar.
#[derive(Debug, Clone, Copy)]
struct Grammar {
    era: u32,
    style: &'static str,
    /// Bay size (cells): rooms along a corridor are module × module+1.
    module: u8,
    /// Wings are mirrored about their corridor.
    symmetric: bool,
    /// Wings end in courtyards.
    courtyards: bool,
    /// Corridor width (cells).
    corridor: u8,
}

impl Grammar {
    fn of(seed: u64, era: u32) -> Self {
        let mut r = Rng::new(seed ^ (u64::from(era) << 32), Stream::World(10));
        Grammar {
            era,
            style: STYLES[(era as usize).min(STYLES.len() - 1)],
            module: r.range(2, 4) as u8,
            symmetric: r.chance(65),
            courtyards: r.chance(50),
            corridor: r.range(1, 2) as u8,
        }
    }
}

/// What a great interior's spaces are for, by kind and depth.
fn purposes_for(kind: GreatKind, level: i8) -> &'static [&'static str] {
    use GreatKind as G;
    match (kind, level) {
        (G::Palace, l) if l < 0 => &["storeroom", "undercroft", "treasury", "cells", "kitchen"],
        (G::Palace, l) if l > 0 => &["residence", "chapel", "gallery", "dormitory", "bath"],
        (G::Palace, _) => &[
            "audience hall",
            "kitchen",
            "guardroom",
            "storeroom",
            "shrine",
            "refectory",
            "bath",
        ],
        (G::Precinct, l) if l < 0 => &["crypt", "ossuary", "storeroom", "undercroft"],
        (G::Precinct, _) => &[
            "shrine",
            "cells",
            "refectory",
            "chapel",
            "dormitory",
            "kitchen",
            "workshop",
        ],
        (G::Fortress, l) if l < 0 => &["tunnel", "storeroom", "cells", "cistern-hall"],
        (G::Fortress, l) if l > 0 => &["guardroom", "gallery", "dormitory"],
        (G::Fortress, _) => &[
            "guardroom",
            "dormitory",
            "kitchen",
            "storeroom",
            "workshop",
            "refectory",
        ],
        (G::Library, l) if l < 0 => &["stacks", "storeroom", "treasury"],
        (G::Library, _) => &["stacks", "reading", "workshop", "cells", "stacks"],
        (G::Necropolis, _) => &[
            "burial gallery",
            "ossuary",
            "crypt",
            "burial gallery",
            "shrine",
        ],
        (G::Mine, l) if l < -3 => &["working", "sump", "working"],
        (G::Mine, _) => &["working", "adit", "storeroom", "working"],
        (G::Cistern, _) => &["cistern-hall", "tunnel", "cistern-hall"],
        (G::Refuge, _) => &[
            "dwelling",
            "storeroom",
            "shrine",
            "dwelling",
            "workshop",
            "kitchen",
        ],
        (G::Labyrinth, _) => &["maze", "maze", "maze", "shrine"],
        (G::Cave, _) => &["cavern", "grotto", "stream passage"],
    }
}

/// What a space holds, by its purpose.
fn furnish(purpose: &str, m: Material) -> Vec<(&'static str, Material)> {
    match purpose {
        "kitchen" => vec![("hearth", Material::Stone), ("table", Material::Wood)],
        "dormitory" | "cells" | "dwelling" => vec![("cot", Material::Wood)],
        "refectory" => vec![("table", Material::Wood), ("bench", Material::Wood)],
        "storeroom" | "undercroft" => vec![("shelf", Material::Wood), ("jar", Material::Clay)],
        "treasury" => vec![("chest", Material::Metal)],
        "shrine" | "chapel" => vec![("niche", Material::Stone), ("altar", Material::Stone)],
        "crypt" => vec![("sarcophagus", Material::Stone)],
        "ossuary" | "burial gallery" => vec![("niche", Material::Stone)],
        "audience hall" => vec![("throne", Material::Stone), ("pillar", Material::Stone)],
        "stacks" => vec![("rack", Material::Wood), ("shelf", Material::Wood)],
        "reading" => vec![("table", Material::Wood)],
        "workshop" => vec![("table", Material::Wood), ("rack", Material::Wood)],
        "bath" => vec![("pool", Material::Stone)],
        "guardroom" => vec![("bench", Material::Wood)],
        "cistern-hall" => vec![("pillar", Material::Stone)],
        "working" | "adit" => vec![("beam", Material::Wood)],
        "residence" => vec![("hearth", Material::Stone), ("chest", Material::Wood)],
        "maze" | "corridor" | "landing" | "stairwell" | "tunnel" | "sump" => Vec::new(),
        _ => vec![("wall", m)],
    }
}

/// Grows interiors: a set of occupied cells per level, and the interior
/// being grown.
struct Grower<'a> {
    i: &'a mut Interior,
    occ: HashMap<(i8, i16, i16), usize>,
    rng: Rng,
    kind: GreatKind,
    material: Material,
    /// Deepest and highest levels allowed.
    low: i8,
    high: i8,
    /// How many spaces to grow to.
    target: usize,
}

impl Grower<'_> {
    fn free(&self, level: i8, x: i16, y: i16, w: u8, d: u8) -> bool {
        (0..i16::from(w))
            .all(|dx| (0..i16::from(d)).all(|dy| !self.occ.contains_key(&(level, x + dx, y + dy))))
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        level: i8,
        x: i16,
        y: i16,
        w: u8,
        d: u8,
        purpose: &'static str,
        space: &'static str,
        g: Grammar,
    ) -> Option<usize> {
        if !self.free(level, x, y, w, d) {
            return None;
        }
        let features = furnish(purpose, self.material);
        let r = self.i.room(purpose, level, features);
        let room = &mut self.i.rooms[r];
        room.x = x;
        room.y = y;
        room.w = w;
        room.d = d;
        room.h = match space {
            "hall" => 7,
            "courtyard" => 0,
            "stair" | "shaft" | "pit" => 4,
            "crawl" => 1,
            _ => 3,
        };
        room.space = space;
        room.era = g.era;
        room.style = g.style;
        for dx in 0..i16::from(w) {
            for dy in 0..i16::from(d) {
                self.occ.insert((level, x + dx, y + dy), r);
            }
        }
        Some(r)
    }

    fn mark(&mut self, r: usize) {
        let room = self.i.rooms[r].clone();
        for dx in 0..i16::from(room.w) {
            for dy in 0..i16::from(room.d) {
                self.occ.insert((room.level, room.x + dx, room.y + dy), r);
            }
        }
    }

    fn link(&mut self, a: usize, b: usize, passage: Passage) -> usize {
        let (ra, rb) = (&self.i.rooms[a], &self.i.rooms[b]);
        let exit = if ra.level != rb.level {
            if rb.level > ra.level {
                Exit::Up
            } else {
                Exit::Down
            }
        } else {
            ra.touches(rb).unwrap_or(Exit::North)
        };
        self.i.link(a, b, exit, passage);
        self.i.links.len() - 1
    }

    /// A new space touching `from` on `side`, its near edge centred on
    /// `from`'s side when it can be, else slid along. Linked by `passage`.
    #[allow(clippy::too_many_arguments)]
    fn beside(
        &mut self,
        from: usize,
        side: Exit,
        w: u8,
        d: u8,
        purpose: &'static str,
        space: &'static str,
        passage: Passage,
        g: Grammar,
    ) -> Option<usize> {
        let ra = self.i.rooms[from].clone();
        let (wa, da) = (i16::from(ra.w), i16::from(ra.d));
        let (wb, db) = (i16::from(w), i16::from(d));
        let centre = match side {
            Exit::North | Exit::South => (wa - wb) / 2,
            _ => (da - db) / 2,
        };
        let span: Vec<i16> = match side {
            Exit::North | Exit::South => (-(wb - 1)..wa).collect(),
            _ => (-(db - 1)..da).collect(),
        };
        let mut order = span;
        order.sort_by_key(|o| ((o - centre).abs(), *o));
        for o in order.into_iter().take(5) {
            let (x, y) = match side {
                Exit::North => (ra.x + o, ra.y - db),
                Exit::South => (ra.x + o, ra.y + da),
                Exit::East => (ra.x + wa, ra.y + o),
                _ => (ra.x - wb, ra.y + o),
            };
            if let Some(r) = self.add(ra.level, x, y, w, d, purpose, space, g) {
                self.link(from, r, passage);
                return Some(r);
            }
        }
        None
    }

    /// A stair from `from` to a new level: a stairwell beside it, and one
    /// stacked over or under that. Returns the far stairwell.
    fn stair(&mut self, from: usize, up: bool, g: Grammar) -> Option<usize> {
        let level = self.i.rooms[from].level + if up { 1 } else { -1 };
        if level > self.high || level < self.low {
            return None;
        }
        let sides = [Exit::North, Exit::East, Exit::South, Exit::West];
        let start = self.rng.index(4);
        for k in 0..4 {
            let side = sides[(start + k) % 4];
            let probe = self.i.rooms[from].clone();
            // Room for the stairwell here and above or below.
            let Some(s) = self.beside(from, side, 2, 2, "stairwell", "stair", Passage::Arch, g)
            else {
                continue;
            };
            let (x, y) = (self.i.rooms[s].x, self.i.rooms[s].y);
            let passage = match self.kind {
                GreatKind::Mine | GreatKind::Cave => {
                    if self.rng.chance(50) {
                        Passage::Ladder
                    } else {
                        Passage::Shaft
                    }
                }
                _ => Passage::Stair,
            };
            if let Some(t) = self.add(level, x, y, 2, 2, "landing", "stair", g) {
                self.link(s, t, passage);
                return Some(t);
            }
            // No room on the other level: undo the stairwell.
            let _ = probe;
            self.unadd(s);
        }
        None
    }

    /// Takes back the last space added (it led nowhere).
    fn unadd(&mut self, r: usize) {
        if r + 1 != self.i.rooms.len() {
            return;
        }
        let room = self.i.rooms.pop().expect("a room");
        for dx in 0..i16::from(room.w) {
            for dy in 0..i16::from(room.d) {
                self.occ.remove(&(room.level, room.x + dx, room.y + dy));
            }
        }
        self.i.links.retain(|l| l.a != r && l.b != r);
    }

    /// A wing: a corridor out from `from`, bays along it (mirrored in a
    /// symmetric manner, with a gap now and then holding a hidden space),
    /// and at its end a hall, a courtyard or a stair on.
    fn wing(&mut self, from: usize, side: Exit, g: Grammar) -> Vec<usize> {
        let mut made = Vec::new();
        let len = self.rng.range(5, 10) as u8;
        let (cw, cd) = match side {
            Exit::North | Exit::South => (g.corridor, len),
            _ => (len, g.corridor),
        };
        let space = if matches!(
            self.kind,
            GreatKind::Mine | GreatKind::Cistern | GreatKind::Refuge
        ) {
            "tunnel"
        } else {
            "corridor"
        };
        let Some(c) = self.beside(from, side, cw, cd, "corridor", space, Passage::Arch, g) else {
            return made;
        };
        made.push(c);
        let level = self.i.rooms[c].level;
        let purposes = purposes_for(self.kind, level);
        // Bays on both sides, step by step along the corridor.
        let (left, right) = match side {
            Exit::North | Exit::South => (Exit::West, Exit::East),
            _ => (Exit::North, Exit::South),
        };
        let m = g.module;
        let steps = (len / m).max(1);
        for step in 0..steps {
            let purpose = purposes[self.rng.index(purposes.len())];
            let (w, d) = (m, m + 1);
            let mut pair = Vec::new();
            for bay_side in [left, right] {
                if !g.symmetric && bay_side == right && self.rng.chance(50) {
                    continue;
                }
                let r = self.bay(c, bay_side, step, m, w, d, purpose, g);
                pair.push(r);
                if let Some(r) = r {
                    made.push(r);
                }
            }
            // A gap in a mirrored pair hides a space: same size, reached
            // only through a loose panel from its neighbour.
            if g.symmetric
                && pair.len() == 2
                && pair[0].is_some()
                && pair[1].is_some()
                && self.rng.chance(12)
            {
                let hidden = pair[1].expect("bay");
                let n = self.i.links.iter().position(|l| l.b == hidden && l.a == c);
                if let Some(n) = n {
                    // Swap the door off the corridor for a panel from the
                    // other bay, through the corridor's far wall... or the
                    // simplest honest way: the corridor door becomes a
                    // hidden panel.
                    self.i.links[n].passage = Passage::Panel;
                    self.i.links[n].hidden = true;
                    self.i.rooms[hidden].hidden = true;
                }
            }
        }
        // At the end: a hall, a courtyard, or a stair on.
        let roll = self.rng.below(100);
        let end = if roll < 30 && g.courtyards && level == 0 {
            self.beside(c, side, 6, 6, "cloister", "courtyard", Passage::Arch, g)
        } else if roll < 55 {
            let p = purposes[self.rng.index(purposes.len())];
            self.beside(c, side, 5, 4, p, "hall", Passage::Door, g)
        } else {
            None
        };
        if let Some(e) = end {
            made.push(e);
        }
        made
    }

    /// One bay off a corridor at step `step`, on `side`.
    #[allow(clippy::too_many_arguments)]
    fn bay(
        &mut self,
        c: usize,
        side: Exit,
        step: u8,
        m: u8,
        w: u8,
        d: u8,
        purpose: &'static str,
        g: Grammar,
    ) -> Option<usize> {
        let rc = self.i.rooms[c].clone();
        let along = i16::from(step) * i16::from(m);
        let (x, y) = match side {
            Exit::West => (rc.x - i16::from(w), rc.y + along),
            Exit::East => (rc.x + i16::from(rc.w), rc.y + along),
            Exit::North => (rc.x + along, rc.y - i16::from(d)),
            _ => (rc.x + along, rc.y + i16::from(rc.d)),
        };
        let (w, d) = match side {
            Exit::West | Exit::East => (d, w),
            _ => (w, d),
        };
        let space = if purpose.contains("cell") {
            "cell"
        } else if purpose.contains("gallery") || purpose == "working" {
            "gallery"
        } else {
            "room"
        };
        let r = self.add(rc.level, x, y, w, d, purpose, space, g)?;
        if self.i.rooms[r].touches(&rc).is_none() {
            self.unadd(r);
            return None;
        }
        self.link(c, r, Passage::Door);
        Some(r)
    }

    /// Spaces with a side free to grow from.
    fn open_sides(&self, r: usize) -> Vec<Exit> {
        let room = &self.i.rooms[r];
        let mut out = Vec::new();
        for side in [Exit::North, Exit::East, Exit::South, Exit::West] {
            let (x, y) = match side {
                Exit::North => (room.x + i16::from(room.w) / 2, room.y - 1),
                Exit::South => (room.x + i16::from(room.w) / 2, room.y + i16::from(room.d)),
                Exit::East => (room.x + i16::from(room.w), room.y + i16::from(room.d) / 2),
                _ => (room.x - 1, room.y + i16::from(room.d) / 2),
            };
            if !self.occ.contains_key(&(room.level, x, y)) {
                out.push(side);
            }
        }
        out
    }

    /// Joins touching spaces not yet joined, making loops: doors in the
    /// manner of the later space; some barred from one side, some only
    /// windows or gratings to look through.
    fn loops(&mut self, want: usize) -> usize {
        let mut made = 0;
        let n = self.i.rooms.len();
        let mut joined: std::collections::BTreeSet<(usize, usize)> = self
            .i
            .links
            .iter()
            .map(|l| (l.a.min(l.b), l.a.max(l.b)))
            .collect();
        let mut pairs = Vec::new();
        for a in 0..n {
            let ra = &self.i.rooms[a];
            if ra.hidden {
                continue;
            }
            // Neighbours by the cells just outside each side.
            let mut seen = std::collections::BTreeSet::new();
            for dx in -1..=i16::from(ra.w) {
                for dy in -1..=i16::from(ra.d) {
                    let inside_x = (0..i16::from(ra.w)).contains(&dx);
                    let inside_y = (0..i16::from(ra.d)).contains(&dy);
                    if inside_x == inside_y {
                        continue; // corners and the inside
                    }
                    if let Some(&b) = self.occ.get(&(ra.level, ra.x + dx, ra.y + dy)) {
                        if b > a && seen.insert(b) && !self.i.rooms[b].hidden {
                            pairs.push((a, b));
                        }
                    }
                }
            }
        }
        self.rng.shuffle(&mut pairs);
        for (a, b) in pairs {
            if made >= want {
                break;
            }
            if joined.contains(&(a, b)) || self.i.rooms[a].touches(&self.i.rooms[b]).is_none() {
                continue;
            }
            joined.insert((a, b));
            let roll = self.rng.below(100);
            let passage = if roll < 12 {
                Passage::Window
            } else if roll < 30 {
                Passage::Arch
            } else {
                Passage::Door
            };
            let l = self.link(a, b, passage);
            if passage == Passage::Door && self.rng.chance(20) {
                // Barred on one side: a shortcut once reached from behind.
                self.i.links[l].one_way = true;
                if !self.i.escapable() {
                    self.i.links[l].one_way = false;
                }
            }
            if passage != Passage::Window {
                made += 1;
            }
        }
        made
    }

    /// Holes in floors over spaces below: drops that can't be climbed back.
    fn drops(&mut self, want: usize) {
        let n = self.i.rooms.len();
        let mut made = 0;
        for a in 0..n {
            if made >= want {
                break;
            }
            let ra = self.i.rooms[a].clone();
            if ra.hidden || ra.space == "stair" || !self.rng.chance(15) {
                continue;
            }
            let below = (0..n).find(|&b| {
                let rb = &self.i.rooms[b];
                rb.level == ra.level - 1 && rb.stacked(&ra) && !rb.hidden && rb.space != "stair"
            });
            if let Some(b) = below {
                let l = self.link(a, b, Passage::Hole);
                self.i.links[l].one_way = true;
                // No drop into a room with no way back out.
                if self.i.escapable() {
                    made += 1;
                } else {
                    self.i.links.pop();
                }
            }
        }
    }
}

/// Which buildings become great interiors, and as what.
fn great_kind(st: &Structure, towns: &[Town]) -> Option<GreatKind> {
    use StructureKind as K;
    let role = st.settlement.and_then(|s| towns.get(s)).map(|t| t.role);
    Some(match st.kind {
        K::Palace => GreatKind::Palace,
        K::Temple if matches!(role, Some(TownRole::HolyCity)) => GreatKind::Precinct,
        K::Barracks if matches!(role, Some(TownRole::Fortress | TownRole::Capital)) => {
            GreatKind::Fortress
        }
        K::Library | K::Archive => GreatKind::Library,
        K::Catacombs => GreatKind::Necropolis,
        K::Mine => GreatKind::Mine,
        K::Cistern if matches!(role, Some(TownRole::Refuge)) => GreatKind::Refuge,
        K::Cistern => GreatKind::Cistern,
        K::Labyrinth => GreatKind::Labyrinth,
        _ => return None,
    })
}

/// Levels and size for each kind of great interior.
fn shape_of(kind: GreatKind, rng: &mut Rng) -> (i8, i8, usize) {
    use GreatKind as G;
    let (low, high, lo, hi) = match kind {
        G::Palace => (-2, 3, 380, 560),
        G::Precinct => (-2, 1, 240, 360),
        G::Fortress => (-3, 3, 240, 380),
        G::Library => (-2, 2, 220, 340),
        G::Necropolis => (-5, 0, 280, 420),
        G::Mine => (-7, 0, 220, 360),
        G::Cistern => (-3, 0, 200, 280),
        G::Refuge => (-6, 0, 300, 460),
        G::Labyrinth => (0, 0, 200, 260),
        G::Cave => (-6, 0, 220, 420),
    };
    (low, high, rng.range(lo as u32, hi as u32) as usize)
}

/// Grows one great interior from its existing core.
fn grow_building(seed: u64, st: &mut Structure, kind: GreatKind, h: &History) {
    let mut rng = Rng::new(
        seed ^ (st.id as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15),
        Stream::World(9),
    );
    let (low, high, target) = shape_of(kind, &mut rng);
    let material = st
        .interior
        .rooms
        .first()
        .and_then(|r| r.features.first())
        .map_or(Material::Stone, |f| f.material);
    let mut g = Grower {
        i: &mut st.interior,
        occ: HashMap::new(),
        rng,
        kind,
        material,
        low,
        high,
        target,
    };
    for r in 0..g.i.rooms.len() {
        g.mark(r);
    }
    // Grow only from the part of the core that can be reached: a wing
    // behind a fallen room would be lost.
    let core = g.i.rooms.len();
    let reach = g.i.reachable_all();
    // Era by era from its founding: each era's manner. The last eras of
    // an abandoned town add nothing.
    let last = h.eras.len() as u32 - 1;
    let eras: Vec<u32> = (st.era..=last).collect();
    let per_era = (g.target / eras.len().max(1)).max(20);
    for (n, &era) in eras.iter().enumerate() {
        let gr = Grammar::of(seed, era);
        let goal = if n + 1 == eras.len() {
            g.target
        } else {
            (n + 1) * per_era
        };
        let mut stalls = 0;
        while g.i.rooms.len() < goal && stalls < 200 {
            // Grow from a space with an open side, preferring recent ones
            // for depth and old ones now and then for breadth.
            let count = g.i.rooms.len();
            let deep = matches!(
                kind,
                GreatKind::Mine | GreatKind::Necropolis | GreatKind::Refuge | GreatKind::Cave
            );
            let lowest = g.i.rooms.iter().map(|r| r.level).min().unwrap_or(0);
            let pick = if deep && lowest > low && g.rng.chance(25) {
                // Deep places reach down from their lowest level.
                let at: Vec<usize> = (0..count)
                    .filter(|&r| g.i.rooms[r].level == lowest && !g.i.rooms[r].hidden)
                    .collect();
                at[g.rng.index(at.len())]
            } else if g.rng.chance(70) {
                count - 1 - g.rng.index(count.min(12))
            } else {
                g.rng.index(count)
            };
            if g.i.rooms[pick].hidden || (pick < core && !reach[pick]) {
                stalls += 1;
                continue;
            }
            let level = g.i.rooms[pick].level;
            // Go up or down now and then, more so for deep kinds.
            let vertical = g.rng.chance(if deep { 18 } else { 10 });
            if vertical {
                let up = if level <= low {
                    true
                } else if level >= high {
                    false
                } else if deep {
                    g.rng.chance(20)
                } else {
                    g.rng.chance(55)
                };
                if let Some(t) = g.stair(pick, up, gr) {
                    let sides = g.open_sides(t);
                    if let Some(&side) = sides.first() {
                        g.wing(t, side, gr);
                    }
                    continue;
                }
            }
            let sides = g.open_sides(pick);
            if sides.is_empty() {
                stalls += 1;
                continue;
            }
            let side = sides[g.rng.index(sides.len())];
            if g.wing(pick, side, gr).is_empty() {
                stalls += 1;
            } else {
                stalls = 0;
            }
        }
        // Later eras block old doors and cut new ones.
        if n > 0 {
            let doors: Vec<usize> = (0..g.i.links.len())
                .filter(|&l| g.i.links[l].passage == Passage::Door && !g.i.links[l].hidden)
                .collect();
            for _ in 0..3 {
                if doors.is_empty() {
                    break;
                }
                let l = doors[g.rng.index(doors.len())];
                g.i.links[l].state = PassageState::Blocked;
            }
            g.loops(4);
        }
    }
    let want_loops = 18 + g.i.rooms.len() / 40;
    g.loops(want_loops);
    g.drops(3);
    hide(&mut g, 6);
    // Where people met caves: deep workings break into natural passages.
    if matches!(
        kind,
        GreatKind::Mine | GreatKind::Necropolis | GreatKind::Refuge
    ) {
        let deepest = (0..g.i.rooms.len())
            .filter(|&r| !g.i.rooms[r].hidden)
            .min_by_key(|&r| (g.i.rooms[r].level, r));
        if let Some(d) = deepest {
            natural_section(&mut g, d, 12);
        }
    }
    // The lowest levels of mines and cisterns lie under water.
    if matches!(kind, GreatKind::Mine | GreatKind::Cistern) {
        for r in g.i.rooms.iter_mut() {
            if r.level <= low + 1 && r.water.is_empty() {
                r.water = if kind == GreatKind::Mine {
                    "flooded"
                } else {
                    "pool"
                };
            }
        }
    }
    fix_reachable(g.i);
    make_escapable(g.i);
    one_way_doors(g.i, 2);
}

/// Opens ways until no reachable space is a trap: blocked ways from a
/// trapped space reopen, one-way ways out of it work both ways.
fn make_escapable(i: &mut Interior) {
    for _ in 0..1000 {
        let Some(r) = i.trapped() else { return };
        // A way out of r that is shut, or one-way inward.
        let l = i.links.iter().position(|l| {
            (l.a == r || l.b == r)
                && l.passage != Passage::Window
                && (l.state == PassageState::Blocked || (l.one_way && l.b == r))
        });
        match l {
            Some(l) => {
                i.links[l].state = PassageState::Open;
                i.links[l].one_way = false;
            }
            None => {
                // Its only ways lead to other trapped spaces: open the
                // first shut way anywhere among them.
                match i
                    .links
                    .iter()
                    .position(|l| l.one_way || l.state == PassageState::Blocked)
                {
                    Some(l) => {
                        i.links[l].state = PassageState::Open;
                        i.links[l].one_way = false;
                    }
                    None => return,
                }
            }
        }
    }
}

/// Bars doors on one side (shortcuts once reached from behind) until
/// `want` ways are one-way, keeping every space reachable. An arch gets a
/// door to bar.
fn one_way_doors(i: &mut Interior, want: usize) {
    let mut have = i.links.iter().filter(|l| l.one_way).count();
    let before = i.reachable_all().iter().filter(|&&s| s).count();
    for l in 0..i.links.len() {
        if have >= want {
            break;
        }
        let link = &i.links[l];
        if !matches!(link.passage, Passage::Door | Passage::Arch)
            || link.hidden
            || link.state != PassageState::Open
            || link.one_way
        {
            continue;
        }
        let was = link.passage;
        i.links[l].passage = Passage::Door;
        i.links[l].one_way = true;
        if i.reachable_all().iter().filter(|&&s| s).count() == before && i.escapable() {
            have += 1;
        } else {
            i.links[l].one_way = false;
            i.links[l].passage = was;
        }
    }
}

/// Whether `o` stands across `hall` from `me` at the mirrored place,
/// the same size and in plain view (D04: what makes a gap in a plan).
pub fn mirrors(hall: &Room, me: &Room, o: &Room, side: Option<Exit>) -> bool {
    let along_same = match side {
        Some(Exit::North | Exit::South) => o.x == me.x,
        Some(Exit::East | Exit::West) => o.y == me.y,
        _ => false,
    };
    !o.hidden
        && o.w == me.w
        && o.d == me.d
        && along_same
        && hall.touches(o).is_some_and(|s| Some(s.opposite()) == side)
}

/// Tops up hidden spaces to `want`: bays off a corridor reached by one
/// door, whose door becomes a hidden panel. Each leaves a gap in the
/// visible plan, the more telling where the wing is mirrored.
fn hide(g: &mut Grower, want: usize) {
    let mut have = g.i.rooms.iter().filter(|r| r.hidden).count();
    let n = g.i.rooms.len();
    for r in (1..n).rev() {
        if have >= want {
            break;
        }
        let room = &g.i.rooms[r];
        if room.hidden || !matches!(room.space, "room" | "cell" | "gallery") || room.outside {
            continue;
        }
        let links: Vec<usize> = (0..g.i.links.len())
            .filter(|&l| g.i.links[l].a == r || g.i.links[l].b == r)
            .collect();
        if links.len() != 1 {
            continue;
        }
        let l = links[0];
        if g.i.links[l].passage != Passage::Door {
            continue;
        }
        // Only where a twin of the same size stands across the same
        // space: the plan's symmetry then shows the gap.
        let c = if g.i.links[l].a == r {
            g.i.links[l].b
        } else {
            g.i.links[l].a
        };
        let me = g.i.rooms[r].clone();
        let hall = g.i.rooms[c].clone();
        let side = hall.touches(&me);
        let twin =
            g.i.rooms
                .iter()
                .enumerate()
                .any(|(k, o)| k != r && mirrors(&hall, &me, o, side));
        // Nor where it is itself the visible twin of a hidden space across
        // some other space: that gap would lose its witness.
        let witness = g.i.rooms.iter().any(|n| {
            let mine = n.touches(&me);
            !n.hidden
                && mine.is_some()
                && g.i.rooms.iter().any(|h| {
                    h.hidden
                        && h.w == me.w
                        && h.d == me.d
                        && (h.x == me.x || h.y == me.y)
                        && n.touches(h).is_some_and(|sd| Some(sd.opposite()) == mine)
                })
        });
        if !twin || witness {
            continue;
        }
        g.i.links[l].passage = Passage::Panel;
        g.i.links[l].hidden = true;
        g.i.rooms[r].hidden = true;
        have += 1;
    }
}

/// Natural passages and chambers grown from a space: a cave section.
fn natural_section(g: &mut Grower, from: usize, n: usize) {
    let gr = Grammar {
        era: 0,
        style: NATURAL,
        module: 3,
        symmetric: false,
        courtyards: false,
        corridor: 1,
    };
    let mut at = from;
    for k in 0..n {
        let sides = g.open_sides(at);
        if sides.is_empty() {
            break;
        }
        let side = sides[g.rng.index(sides.len())];
        let chamber = k % 4 == 3;
        let (w, d) = if chamber {
            (5, 4)
        } else if matches!(side, Exit::North | Exit::South) {
            (1, 4)
        } else {
            (4, 1)
        };
        let (purpose, space) = if chamber {
            ("cavern", "chamber")
        } else {
            ("stream passage", "passage")
        };
        let passage = if k == 0 {
            Passage::Crawlway
        } else {
            Passage::Opening
        };
        if let Some(r) = g.beside(at, side, w, d, purpose, space, passage, gr) {
            if chamber {
                g.i.rooms[r].features = vec![
                    Feature {
                        kind: "stalactites",
                        material: Material::Stone,
                        texts: Vec::new(),
                    },
                    Feature {
                        kind: "flowstone",
                        material: Material::Stone,
                        texts: Vec::new(),
                    },
                ];
            }
            at = r;
        }
    }
}

/// Every space must be reachable from the entrance, or be hidden behind
/// a recorded way in: spaces cut off by blocked doors get an opening.
fn fix_reachable(i: &mut Interior) {
    loop {
        let seen = i.reachable_all();
        let Some(r) = (0..i.rooms.len()).find(|&r| !seen[r] && !i.rooms[r].collapsed) else {
            break;
        };
        // Reopen a link to it from a space that is reached.
        let l = i
            .links
            .iter()
            .position(|l| (l.a == r && seen[l.b]) || (l.b == r && seen[l.a]));
        match l {
            Some(l) => {
                let link = &mut i.links[l];
                link.state = PassageState::Open;
                if link.passage == Passage::Window {
                    link.passage = Passage::Opening;
                }
                // A drop leads down only: make it climbable when it is the
                // only way.
                link.one_way = false;
            }
            None => {
                // Joined only to other unreachable spaces: open one of
                // theirs that is blocked, or seal it.
                let any = i.links.iter().position(|l| {
                    (l.a == r || l.b == r)
                        && (l.state == PassageState::Blocked
                            || l.one_way
                            || l.passage == Passage::Window)
                });
                match any {
                    Some(l) => {
                        let link = &mut i.links[l];
                        link.state = PassageState::Open;
                        link.one_way = false;
                        if link.passage == Passage::Window {
                            link.passage = Passage::Opening;
                        }
                    }
                    None => i.rooms[r].collapsed = true,
                }
            }
        }
    }
}

impl Interior {
    /// Whether every space that can be reached can also be left: from
    /// each, some way leads back to a way outside (no drop or barred door
    /// shuts the player in).
    pub fn escapable(&self) -> bool {
        self.trapped().is_none()
    }

    /// A reachable space with no way back out, if there is one.
    pub fn trapped(&self) -> Option<usize> {
        let n = self.rooms.len();
        let reach = self.reachable_all();
        // Backwards from the ways outside, along passable ways.
        let mut out = vec![false; n];
        let mut stack: Vec<usize> = (0..n)
            .filter(|&r| (r == 0 || self.rooms[r].outside) && !self.rooms[r].collapsed)
            .collect();
        for &r in &stack {
            out[r] = true;
        }
        while let Some(r) = stack.pop() {
            for l in &self.links {
                if l.state == PassageState::Blocked || l.passage == Passage::Window {
                    continue;
                }
                // Someone at `from` can step to `r`.
                let froms: &[usize] = if l.one_way {
                    if l.b == r {
                        &[l.a][..]
                    } else {
                        &[][..]
                    }
                } else if l.a == r {
                    &[l.b][..]
                } else if l.b == r {
                    &[l.a][..]
                } else {
                    &[][..]
                };
                for &f in froms {
                    if !out[f] && !self.rooms[f].collapsed {
                        out[f] = true;
                        stack.push(f);
                    }
                }
            }
        }
        (0..n).find(|&r| reach[r] && !out[r])
    }

    /// Spaces reachable from the entrance or from any way outside, through
    /// passable ways, hidden ones included; collapsed spaces are sealed and
    /// never reached.
    pub fn reachable_all(&self) -> Vec<bool> {
        let mut seen = vec![false; self.rooms.len()];
        let mut stack: Vec<usize> = (0..self.rooms.len())
            .filter(|&r| (r == 0 || self.rooms[r].outside) && !self.rooms[r].collapsed)
            .collect();
        for &s in &stack {
            seen[s] = true;
        }
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

// ---------- caves ----------

/// Grows a cave system from a cave mouth: passages along the rock's two
/// fracture directions, chambers where passages meet, pits down toward
/// the water, formations in soluble rock, sumps at the bottom, and other
/// mouths to the outside.
pub fn grow_cave(seed: u64, st: &mut Structure, f: &LandFeature, rock: Rock, sea: bool) {
    let mut rng = Rng::new(
        seed ^ (st.id as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f),
        Stream::World(11),
    );
    let (low, _, mut target) = shape_of(GreatKind::Cave, &mut rng);
    let size = f.inside.as_ref().map_or(1, |i| i.size);
    // Small caves stay small; the largest are systems.
    if size < 4 {
        target = 20 + usize::from(size) * 25;
    }
    let low = if size < 4 {
        -i8::try_from(size).unwrap_or(1)
    } else {
        low
    };
    st.interior = Interior::default();
    let gr = Grammar {
        era: 0,
        style: NATURAL,
        module: 3,
        symmetric: false,
        courtyards: false,
        corridor: 1,
    };
    let mut g = Grower {
        i: &mut st.interior,
        occ: HashMap::new(),
        rng,
        kind: GreatKind::Cave,
        material: Material::Stone,
        low,
        high: 0,
        target,
    };
    let mouth = g
        .add(0, 0, 0, 4, 3, "mouth", "chamber", gr)
        .expect("an empty grid");
    g.i.rooms[mouth].outside = true;
    // The fracture directions: one main, one across it.
    let main = if g.rng.chance(50) {
        [Exit::North, Exit::South]
    } else {
        [Exit::East, Exit::West]
    };
    let across = if main[0] == Exit::North {
        [Exit::East, Exit::West]
    } else {
        [Exit::North, Exit::South]
    };
    let soluble = rock.soluble();
    let mut tips = vec![mouth];
    let mut stalls = 0;
    let mut mouths = 1;
    while g.i.rooms.len() < g.target && stalls < 400 {
        let from = tips[g.rng.index(tips.len())];
        let level = g.i.rooms[from].level;
        // Down a pit now and then, toward the water.
        if level > low && g.rng.chance(9) {
            if let Some(t) = g.stair(from, false, gr) {
                g.i.rooms[t].purpose = "chimney";
                g.i.rooms[t].space = "pit";
                tips.push(t);
                continue;
            }
        }
        let side = if g.rng.chance(70) {
            main[g.rng.index(2)]
        } else {
            across[g.rng.index(2)]
        };
        let len = g.rng.range(3, 8) as u8;
        let chamber = g.rng.chance(18);
        let squeeze = !chamber && g.rng.chance(8);
        let (w, d) = if chamber {
            (g.rng.range(4, 8) as u8, g.rng.range(4, 7) as u8)
        } else if matches!(side, Exit::North | Exit::South) {
            (1, len)
        } else {
            (len, 1)
        };
        let deep = level <= low + 1;
        let (purpose, space) = if chamber {
            (if deep { "grotto" } else { "cavern" }, "chamber")
        } else if squeeze {
            ("squeeze", "crawl")
        } else if deep {
            ("stream passage", "passage")
        } else {
            ("cavern", "passage")
        };
        let passage = if squeeze {
            Passage::Crawlway
        } else {
            Passage::Opening
        };
        match g.beside(from, side, w, d, purpose, space, passage, gr) {
            Some(r) => {
                stalls = 0;
                if chamber {
                    let mut fs = Vec::new();
                    if soluble {
                        fs.push(("stalactites", Material::Stone));
                        if g.rng.chance(50) {
                            fs.push(("flowstone", Material::Stone));
                        }
                        if g.rng.chance(25) {
                            fs.push(("column", Material::Stone));
                        }
                    } else if g.rng.chance(40) {
                        fs.push(("crystals", Material::Stone));
                    }
                    if g.rng.chance(20) {
                        fs.push(("fossil", Material::Stone));
                    }
                    g.i.rooms[r].features = fs
                        .into_iter()
                        .map(|(kind, material)| Feature {
                            kind,
                            material,
                            texts: Vec::new(),
                        })
                        .collect();
                    tips.push(r);
                } else {
                    // A passage's far end goes on.
                    tips.push(r);
                }
                if deep {
                    g.i.rooms[r].water = if g.rng.chance(15) {
                        "sump"
                    } else if chamber {
                        "pool"
                    } else {
                        "stream"
                    };
                }
                if sea && level == 0 && g.rng.chance(20) {
                    g.i.rooms[r].water = "pool";
                }
                // Another mouth, out near the edge of the system.
                if level == 0 && mouths < 3 && g.i.rooms.len() > 30 * mouths && g.rng.chance(10) {
                    g.i.rooms[r].outside = true;
                    g.i.rooms[r].purpose = "mouth";
                    mouths += 1;
                }
                if tips.len() > 24 {
                    let k = g.rng.index(tips.len());
                    tips.swap_remove(k);
                }
            }
            None => {
                stalls += 1;
                if stalls % 20 == 0 {
                    tips.push(g.rng.index(g.i.rooms.len()));
                }
            }
        }
    }
    let want = 15 + g.i.rooms.len() / 40;
    // Caves loop where passages meet; no windows or doors in rock.
    let made = g.loops(want);
    let _ = made;
    for l in g.i.links.iter_mut() {
        if matches!(l.passage, Passage::Door | Passage::Arch | Passage::Window) {
            l.passage = Passage::Opening;
            l.one_way = false;
        }
    }
    // Hidden passages: crawls behind rubble from a chamber.
    let mut hidden = 0;
    let chambers: Vec<usize> = (0..g.i.rooms.len())
        .filter(|&r| g.i.rooms[r].space == "chamber")
        .collect();
    for &c in chambers.iter().rev() {
        if hidden >= 6 {
            break;
        }
        let sides = g.open_sides(c);
        let Some(&side) = sides.first() else { continue };
        let (w, d) = if matches!(side, Exit::North | Exit::South) {
            (1, 3)
        } else {
            (3, 1)
        };
        if let Some(cr) = g.beside(c, side, w, d, "squeeze", "crawl", Passage::Crawlway, gr) {
            let l = g.i.links.len() - 1;
            g.i.links[l].hidden = true;
            g.i.rooms[cr].hidden = true;
            if let Some(gro) = g.beside(cr, side, 4, 4, "grotto", "chamber", Passage::Opening, gr) {
                g.i.rooms[gro].hidden = true;
                g.i.rooms[gro].features = vec![Feature {
                    kind: "crystals",
                    material: Material::Stone,
                    texts: Vec::new(),
                }];
            }
            hidden += 1;
        }
    }
    g.drops(4);
    fix_reachable(g.i);
    make_escapable(g.i);
}

/// Grows every great interior in a world and makes caves of its cave
/// mouths. Returns the great interiors' structures and kinds.
pub fn grow(
    seed: u64,
    t: &Terrain,
    g: &Geology,
    h: &History,
    towns: &[Town],
    features: &[LandFeature],
    structures: &mut Vec<Structure>,
) -> Vec<(usize, GreatKind)> {
    // Every ordinary interior gets its geometry.
    for st in structures.iter_mut() {
        lay_out(&mut st.interior, st.era);
    }
    // Great buildings: each kind once per town at most, buildings that
    // stand (not buried), largest towns first.
    let mut greats: Vec<(usize, GreatKind)> = Vec::new();
    let mut taken: std::collections::BTreeSet<(Option<usize>, GreatKind)> = Default::default();
    let mut order: Vec<usize> = (0..structures.len()).collect();
    order.sort_by_key(|&i| {
        let s = structures[i].settlement;
        (std::cmp::Reverse(s.map_or(0, |s| h.settlements[s].size)), i)
    });
    for i in order {
        let st = &structures[i];
        if st.condition == Condition::Buried {
            continue;
        }
        let Some(kind) = great_kind(st, towns) else {
            continue;
        };
        // Mines and archives are many; only the largest few grow.
        let same = greats.iter().filter(|(_, k)| *k == kind).count();
        let cap = match kind {
            GreatKind::Mine | GreatKind::Library => 2,
            GreatKind::Palace => 1,
            _ => 2,
        };
        if same >= cap || !taken.insert((st.settlement, kind)) {
            continue;
        }
        greats.push((i, kind));
    }
    for &(i, kind) in &greats {
        grow_building(seed, &mut structures[i], kind, h);
    }
    // Caves: a structure at every cave mouth and sea cave.
    for f in features.iter().filter(|f| f.inside.is_some()) {
        let id = structures.len();
        let sea = f.kind == "sea cave";
        let mut st = Structure {
            id,
            kind: if sea {
                StructureKind::SeaCave
            } else {
                StructureKind::Cave
            },
            cell: f.cell,
            settlement: None,
            era: 0,
            built: h.eras.first().map_or(0, |e| e.start),
            event: None,
            person: None,
            condition: Condition::Intact,
            interior: Interior::default(),
            outside: Vec::new(),
            district: None,
        };
        grow_cave(seed, &mut st, f, g.at(f.cell.ux(), f.cell.uy()), sea);
        let big = st.interior.rooms.len() >= 200;
        structures.push(st);
        if big {
            greats.push((id, GreatKind::Cave));
        }
    }
    let _ = t;
    greats
}

/// Cells of the world a space's way outside opens onto: the structure's
/// own cell, moved by the space's position (2 m cells; a world cell is
/// 300 m).
pub fn outside_cell(st: &Structure, room: &Room) -> Cell {
    let (cx, cy) = room.centre();
    let dx = (cx / 300.0).round() as i64;
    let dy = (cy / 300.0).round() as i64;
    let x = (i64::from(st.cell.x) + dx).clamp(0, crate::terrain::SIZE as i64 - 1);
    let y = (i64::from(st.cell.y) + dy).clamp(0, crate::terrain::SIZE as i64 - 1);
    Cell::new(x as usize, y as usize)
}

/// Counts of an interior's shape, for measures: spaces, levels,
/// independent loops, dead ends, hidden spaces, one-way ways, and the
/// longest shortest path from the entrance (in ways).
#[derive(Debug, Clone, Default, Serialize)]
pub struct Shape {
    pub spaces: usize,
    pub levels: usize,
    pub loops: usize,
    pub dead_ends: usize,
    pub hidden: usize,
    pub one_way: usize,
    pub deepest_path: usize,
}

pub fn shape(i: &Interior) -> Shape {
    let n = i.rooms.len();
    let passable: Vec<&Link> = i
        .links
        .iter()
        .filter(|l| l.passage != Passage::Window && l.state != PassageState::Blocked)
        .collect();
    let levels: std::collections::BTreeSet<i8> = i.rooms.iter().map(|r| r.level).collect();
    // Loops: edges − vertices + components, over the passable graph.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            r = p[r];
        }
        let mut y = x;
        while p[y] != r {
            let nx = p[y];
            p[y] = r;
            y = nx;
        }
        r
    }
    let mut cycles = 0;
    for l in &passable {
        let (a, b) = (find(&mut parent, l.a), find(&mut parent, l.b));
        if a == b {
            cycles += 1;
        } else {
            parent[a] = b;
        }
    }
    let mut degree = vec![0usize; n];
    for l in &passable {
        degree[l.a] += 1;
        degree[l.b] += 1;
    }
    // Breadth-first from the entrance.
    let mut dist = vec![usize::MAX; n];
    let mut q = std::collections::VecDeque::new();
    if n > 0 {
        dist[0] = 0;
        q.push_back(0);
    }
    while let Some(r) = q.pop_front() {
        for l in &passable {
            let next = if l.a == r {
                Some(l.b)
            } else if l.b == r && !l.one_way {
                Some(l.a)
            } else {
                None
            };
            if let Some(nx) = next {
                if dist[nx] == usize::MAX {
                    dist[nx] = dist[r] + 1;
                    q.push_back(nx);
                }
            }
        }
    }
    Shape {
        spaces: n,
        levels: levels.len(),
        loops: cycles,
        dead_ends: degree.iter().filter(|&&d| d == 1).count(),
        hidden: i.rooms.iter().filter(|r| r.hidden).count(),
        one_way: i.links.iter().filter(|l| l.one_way).count(),
        deepest_path: dist
            .iter()
            .filter(|&&d| d != usize::MAX)
            .copied()
            .max()
            .unwrap_or(0),
    }
}
