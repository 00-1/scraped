//! Buildings placed by history, and their interiors.
//!
//! Room purposes and feature kinds are data ids; what they look like is
//! content for M05–M06. Interiors come from a few generators with authored
//! shapes (house, temple, archive, tomb, tower), varied by seed.

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

use crate::history::{Cell, EventKind, History, Role};
use crate::terrain::{Biome, Terrain, SIZE};
use crate::water::Water;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Passage {
    Door,
    Arch,
    Stair,
    Opening,
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

#[derive(Debug, Clone, Serialize)]
pub struct Room {
    pub purpose: &'static str,
    pub level: i8,
    pub features: Vec<Feature>,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Link {
    pub a: usize,
    pub b: usize,
    /// Direction from a to b.
    pub exit: Exit,
    pub passage: Passage,
    pub state: PassageState,
}

/// Rooms and how they connect. Room 0 is the entrance.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Interior {
    pub rooms: Vec<Room>,
    pub links: Vec<Link>,
}

impl Interior {
    fn room(
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
        });
        self.rooms.len() - 1
    }

    fn link(&mut self, a: usize, b: usize, exit: Exit, passage: Passage) {
        self.links.push(Link {
            a,
            b,
            exit,
            passage,
            state: PassageState::Open,
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
                if l.state == PassageState::Blocked {
                    continue;
                }
                for (from, to) in [(l.a, l.b), (l.b, l.a)] {
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
    }
    i
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
