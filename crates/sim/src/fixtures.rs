//! What the world holds beyond its writing: items left behind, mechanisms,
//! creatures, and the obstacles ordinary means can overcome. All placed
//! deterministically from the world and its seed.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use scraped_world::structures::{Condition, Family, Passage, PassageState, StructureKind};
use scraped_world::terrain::{Biome, SIZE};
use scraped_world::water::RIVER_FLOW;
use scraped_world::World;

use crate::outdoors::{hash, Land, Pos, CELL};

/// A place: a point outdoors, or a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "at")]
pub enum Spot {
    Out { x: i32, y: i32 },
    Room { structure: usize, room: usize },
}

impl Spot {
    pub fn out(p: Pos) -> Self {
        Spot::Out { x: p.x, y: p.y }
    }
}

/// Kinds of mechanism. Ids for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MechKind {
    /// Draws water (dry wells give none).
    Well,
    /// A gate on a river; open, it diverts water so the river below drops.
    Sluice,
    /// A water wheel, turned by the river's flow.
    Wheel,
    /// Drains a flooded room when pulled.
    DrainLever,
    /// Raises or lowers a bridge.
    BridgeLever,
    /// A fire bowl that lights and warms a room.
    Brazier,
}

pub const MECHANISMS: &[&str] = &[
    "well",
    "sluice",
    "wheel",
    "drain_lever",
    "bridge_lever",
    "brazier",
];

/// A mechanism and what it works on.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Mechanism {
    pub kind: MechKind,
    pub at: Spot,
    /// Where on the land it is.
    pub pos: Pos,
    /// For a well: whether it still gives water.
    pub works: bool,
    /// The sluice, drained room or bridge it controls (index into
    /// `sluices`, `flooded` or `World::structures`).
    pub controls: Option<usize>,
}

/// A river gate and the water it moves.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Sluice {
    /// The river cell the gate stands in.
    pub cell: (usize, usize),
    /// River cells below the gate, in order, whose flow it takes.
    pub below: Vec<(usize, usize)>,
    /// Where the diverted water goes: a cell beside the river.
    pub channel: (usize, usize),
    /// Water diverted when open (river units).
    pub diverts: u32,
    /// The crossing it makes wadable.
    pub crossing: (usize, usize),
}

/// A room under water until drained.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Flooded {
    pub structure: usize,
    pub room: usize,
}

/// One creature of the world, where it lives.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Spawn {
    pub archetype: &'static str,
    pub home: Spot,
    pub pos: Pos,
}

/// An item lying somewhere at the start.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Placed {
    pub kind: &'static str,
    pub at: Spot,
    pub pos: Pos,
}

/// Everything placed in a world for the physical game.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Fixtures {
    pub items: Vec<Placed>,
    pub mechanisms: Vec<Mechanism>,
    pub sluices: Vec<Sluice>,
    pub flooded: Vec<Flooded>,
    /// Doors barred shut: (structure, link). A pry bar opens them.
    pub barred: BTreeSet<(usize, usize)>,
    /// Rooms whose stone could fall: (structure, room).
    pub unstable: BTreeSet<(usize, usize)>,
    /// Bridges standing raised: impassable until lowered.
    pub raised: BTreeSet<usize>,
    /// Rubble already dug through (structure, link): the way to each
    /// great inscription is always open.
    pub cleared: BTreeSet<(usize, usize)>,
    pub creatures: Vec<Spawn>,
}

fn roll(seed: u64, parts: &[u64], percent: u64) -> bool {
    let mut all = vec![seed];
    all.extend_from_slice(parts);
    hash(&all) % 100 < percent
}

impl Fixtures {
    /// Places everything for a world. `start` is the settlement play
    /// begins in, which is guaranteed the basics of survival.
    // DESIGN-Q: placement odds (barred doors 30%, flooded cellars 35%,
    // raised bridges 40%, unstable rooms in damaged and ruined buildings)
    // and that the starting town always holds a firesteel, a water
    // container, provisions and a pry bar somewhere.
    pub fn new(w: &World, land: &Land, start: usize) -> Self {
        let seed = w.seed ^ 0x00f1_77e5;
        let mut f = Fixtures::default();
        let great_events = crate::region::great_events(w);
        let great_texts: BTreeSet<usize> = w
            .texts
            .iter()
            .filter(|t| t.event.is_some_and(|e| great_events.contains(&e)))
            .map(|t| t.id)
            .collect();
        let great_homes: BTreeSet<usize> = w
            .texts
            .iter()
            .filter(|t| great_texts.contains(&t.id))
            .map(|t| t.structure)
            .collect();
        for st in &w.structures {
            let sid = st.id as u64;
            let pos = land.structure_pos[st.id];
            let rooms = &st.interior.rooms;
            // Items by what the building was.
            for (ri, room) in rooms.iter().enumerate() {
                if room.collapsed {
                    continue;
                }
                let at = Spot::Room {
                    structure: st.id,
                    room: ri,
                };
                let r = ri as u64;
                let mut put = |kind: &'static str, chance: u64, salt: u64| {
                    if roll(seed, &[sid, r, salt], chance) {
                        f.items.push(Placed { kind, at, pos });
                    }
                };
                match st.kind {
                    StructureKind::Storehouse => {
                        put("provisions", 80, 1);
                        put("provisions", 50, 2);
                        put("wood", 70, 3);
                        put("oil", 30, 4);
                    }
                    StructureKind::House => {
                        if ri == 0 {
                            put("firesteel", 45, 5);
                            put("wood", 50, 6);
                            put("waterskin", 30, 7);
                        }
                        put("cloak", 25, 8);
                        put("provisions", 30, 9);
                        put("torch", 20, 10);
                    }
                    StructureKind::Temple => {
                        put("lamp", 35, 11);
                        put("oil", 30, 12);
                        put("torch", 30, 13);
                    }
                    StructureKind::Mine => {
                        put("pry_bar", 50, 14);
                        put("torch", 60, 15);
                        put("lamp", 20, 16);
                    }
                    StructureKind::Waystation => {
                        put("provisions", 50, 17);
                        put("firesteel", 50, 18);
                        put("torch", 50, 19);
                        put("waterskin", 40, 20);
                        put("wood", 60, 21);
                    }
                    StructureKind::Tower | StructureKind::Wall => {
                        put("torch", 35, 22);
                        put("pry_bar", 20, 23);
                    }
                    // D03's kinds: by what the building was for.
                    // DESIGN-Q: item odds for the newer kinds of building.
                    k => match k.info().family {
                        Family::Food | Family::Store | Family::Trade => {
                            put("provisions", 45, 30);
                            put("wood", 40, 31);
                        }
                        Family::Craft | Family::Mining => {
                            put("pry_bar", 30, 32);
                            put("firesteel", 30, 33);
                            put("torch", 30, 34);
                        }
                        Family::Learning | Family::Holy | Family::Rule => {
                            put("lamp", 25, 35);
                            put("oil", 25, 36);
                            put("torch", 20, 37);
                        }
                        Family::Defence | Family::Travel => {
                            put("torch", 35, 38);
                            put("cloak", 25, 39);
                        }
                        Family::Water => put("waterskin", 40, 40),
                        _ => {}
                    },
                }
            }
            // Fire bowls in temples' halls and sanctums.
            if st.kind == StructureKind::Temple {
                for (ri, room) in rooms.iter().enumerate() {
                    if matches!(room.purpose, "sanctum" | "hall" | "chapel") && !room.collapsed {
                        f.mechanisms.push(Mechanism {
                            kind: MechKind::Brazier,
                            at: Spot::Room {
                                structure: st.id,
                                room: ri,
                            },
                            pos,
                            works: true,
                            controls: None,
                        });
                        break;
                    }
                }
            }
            // Flooded cellars, with a drain at the entrance.
            let deepest = rooms
                .iter()
                .enumerate()
                .filter(|(_, r)| r.level < 0 && !r.collapsed)
                .min_by_key(|(i, r)| (r.level, *i))
                .map(|(i, _)| i);
            if let Some(room) = deepest {
                if room != 0 && !great_homes.contains(&st.id) && roll(seed, &[sid, 30], 35) {
                    f.flooded.push(Flooded {
                        structure: st.id,
                        room,
                    });
                    f.mechanisms.push(Mechanism {
                        kind: MechKind::DrainLever,
                        at: Spot::Room {
                            structure: st.id,
                            room: 0,
                        },
                        pos,
                        works: true,
                        controls: Some(f.flooded.len() - 1),
                    });
                }
            }
            // Barred doors.
            for (li, l) in st.interior.links.iter().enumerate() {
                if l.passage == Passage::Door
                    && l.state == PassageState::Closed
                    && roll(seed, &[sid, li as u64, 40], 30)
                {
                    f.barred.insert((st.id, li));
                }
            }
            // Loose stone in broken buildings.
            let chance = match st.condition {
                Condition::Damaged => 35,
                Condition::Ruined => 60,
                _ => 0,
            };
            for (ri, room) in rooms.iter().enumerate().skip(1) {
                if !room.collapsed && roll(seed, &[sid, ri as u64, 50], chance) {
                    f.unstable.insert((st.id, ri));
                }
            }
            if st.kind == StructureKind::Bridge && roll(seed, &[sid, 60], 40) {
                f.raised.insert(st.id);
                f.mechanisms.push(Mechanism {
                    kind: MechKind::BridgeLever,
                    at: Spot::out(pos),
                    pos,
                    works: true,
                    controls: Some(st.id),
                });
            }
        }

        // Wells in settlements; old ones may be dry.
        for s in &w.history.settlements {
            let c = Pos::of_cell(s.cell.ux(), s.cell.uy());
            let pos = Pos::new(c.x + 60, c.y + 40);
            let works = s.abandoned.is_none() || roll(seed, &[s.id as u64, 70], 50);
            f.mechanisms.push(Mechanism {
                kind: MechKind::Well,
                at: Spot::out(pos),
                pos,
                works,
                controls: None,
            });
        }

        // DESIGN-Q: rubble on the way to a great inscription has been dug
        // through, so each is reachable by ordinary means.
        for t in w.texts.iter().filter(|t| great_texts.contains(&t.id)) {
            if let Some(room) = t.room {
                f.clear_way(w, t.structure, room);
            }
        }
        f.place_sluices(w, land);
        f.place_creatures(w, land, start);
        f.ensure_basics(w, land, start);
        f
    }

    /// For towns near a river too deep to wade, a sluice upstream that,
    /// opened, drops the river enough to ford it, and a mill wheel below.
    fn place_sluices(&mut self, w: &World, land: &Land) {
        let s = SIZE;
        let mut used = BTreeSet::new();
        for town in &w.history.settlements {
            if town.abandoned.is_some() {
                continue;
            }
            let home = Pos::of_cell(town.cell.ux(), town.cell.uy());
            // The nearest deep crossing within 3 km.
            let mut best: Option<(i64, (usize, usize))> = None;
            for y in 0..s {
                for x in 0..s {
                    if !w.water.needs_crossing(&w.terrain, x, y) || *land.bridges.get(x, y) {
                        continue;
                    }
                    let d = Pos::of_cell(x, y).dist2(home);
                    if d <= 3000 * 3000 && best.is_none_or(|(b, c)| (d, (y, x)) < (b, (c.1, c.0))) {
                        best = Some((d, (x, y)));
                    }
                }
            }
            let Some((_, crossing)) = best else { continue };
            // Walk upstream two to four cells along the main stream.
            let up = |c: (usize, usize)| {
                w.terrain
                    .height
                    .neighbours(c.0, c.1)
                    .filter(|&(nx, ny)| {
                        *w.water.down.get(nx, ny) == (c.1 * s + c.0) as u32
                            && w.water.is_river(&w.terrain, nx, ny)
                    })
                    .max_by_key(|&(nx, ny)| {
                        (*w.water.flow.get(nx, ny), std::cmp::Reverse((ny, nx)))
                    })
            };
            let mut gate = crossing;
            for _ in 0..3 {
                match up(gate) {
                    Some(c) => gate = c,
                    None => break,
                }
            }
            if gate == crossing || !used.insert(gate) {
                continue;
            }
            // The river below the gate, down to a little past the crossing.
            let mut below = Vec::new();
            let mut c = gate;
            let mut past = 0;
            while below.len() < 12 {
                let d = *w.water.down.get(c.0, c.1);
                if d == u32::MAX {
                    break;
                }
                c = (d as usize % s, d as usize / s);
                if !w.water.is_river(&w.terrain, c.0, c.1) {
                    break;
                }
                below.push(c);
                if c == crossing {
                    past = 1;
                } else if past > 0 {
                    past += 1;
                    if past > 2 {
                        break;
                    }
                }
            }
            if !below.contains(&crossing) {
                continue;
            }
            // A dry bank beside the gate takes the water.
            let Some(channel) = w
                .terrain
                .height
                .neighbours(gate.0, gate.1)
                .filter(|&(x, y)| w.terrain.is_land(x, y) && !w.water.is_river(&w.terrain, x, y))
                .min_by_key(|&(x, y)| ((*w.terrain.height.get(x, y) * 10.0) as i64, y, x))
            else {
                continue;
            };
            let flow = *w.water.flow.get(gate.0, gate.1);
            // Enough water leaves that the crossing falls below a river.
            let at_crossing = *w.water.flow.get(crossing.0, crossing.1);
            let diverts = (at_crossing + 1)
                .saturating_sub(RIVER_FLOW)
                .max(flow * 7 / 10)
                .min(flow);
            if at_crossing.saturating_sub(diverts) >= RIVER_FLOW {
                // Too much water joins below the gate for it to matter.
                continue;
            }
            let bank = Pos::of_cell(channel.0, channel.1);
            let i = self.sluices.len();
            self.sluices.push(Sluice {
                cell: gate,
                below: below.clone(),
                channel,
                diverts,
                crossing,
            });
            self.mechanisms.push(Mechanism {
                kind: MechKind::Sluice,
                at: Spot::out(bank),
                pos: bank,
                works: true,
                controls: Some(i),
            });
            let mill = below[0];
            if let Some(mbank) = w
                .terrain
                .height
                .neighbours(mill.0, mill.1)
                .find(|&(x, y)| w.terrain.is_land(x, y) && !w.water.is_river(&w.terrain, x, y))
            {
                let p = Pos::of_cell(mbank.0, mbank.1);
                self.mechanisms.push(Mechanism {
                    kind: MechKind::Wheel,
                    at: Spot::out(p),
                    pos: p,
                    works: true,
                    controls: Some(i),
                });
            }
        }
    }

    /// A few creatures by biome, away from the starting town, and things
    /// in the deep rooms of tombs and mines.
    // DESIGN-Q: about two dozen creatures outdoors, none within 2 km of the
    // start, and a "deep" creature in 40% of dry underground tomb and mine
    // rooms.
    fn place_creatures(&mut self, w: &World, land: &Land, start: usize) {
        let s = SIZE;
        let home = Pos::of_cell(
            w.history.settlements[start].cell.ux(),
            w.history.settlements[start].cell.uy(),
        );
        let seed = w.seed ^ 0x000c_4ea7;
        let mut tries = 0u64;
        let mut placed = 0;
        while placed < 24 && tries < 4000 {
            tries += 1;
            let h = hash(&[seed, tries]);
            let (x, y) = ((h % s as u64) as usize, ((h >> 16) % s as u64) as usize);
            let b = *w.terrain.biome.get(x, y);
            let archetype = match b {
                Biome::Grassland | Biome::Scrub | Biome::Desert | Biome::Shore => {
                    if h >> 40 & 1 == 0 {
                        "scavenger"
                    } else {
                        "grazer"
                    }
                }
                Biome::Tundra | Biome::Marsh => "grazer",
                Biome::Forest | Biome::Pine | Biome::Rock => "predator",
                _ => continue,
            };
            let pos = Pos::of_cell(x, y);
            if pos.dist(home) < 2000.0 || land.town(w, pos).is_some() {
                continue;
            }
            self.creatures.push(Spawn {
                archetype,
                home: Spot::out(pos),
                pos,
            });
            placed += 1;
        }
        let flooded: BTreeSet<(usize, usize)> =
            self.flooded.iter().map(|f| (f.structure, f.room)).collect();
        for st in &w.structures {
            if !matches!(st.kind, StructureKind::Tomb | StructureKind::Mine)
                || st.settlement == Some(start)
            {
                continue;
            }
            for (ri, r) in st.interior.rooms.iter().enumerate() {
                if r.level < 0
                    && !r.collapsed
                    && !flooded.contains(&(st.id, ri))
                    && roll(seed, &[st.id as u64, ri as u64], 40)
                {
                    self.creatures.push(Spawn {
                        archetype: "deep",
                        home: Spot::Room {
                            structure: st.id,
                            room: ri,
                        },
                        pos: land.structure_pos[st.id],
                    });
                    break;
                }
            }
        }
    }

    /// The starting town always has what a careful newcomer needs, so
    /// the early game is about finding it rather than luck.
    fn ensure_basics(&mut self, w: &World, land: &Land, start: usize) {
        let rooms: Vec<(usize, usize, i8)> = w
            .structures
            .iter()
            .filter(|st| st.settlement == Some(start) && st.condition != Condition::Buried)
            .flat_map(|st| {
                st.interior
                    .rooms
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| !r.collapsed && r.level >= 0)
                    .map(move |(ri, r)| (st.id, ri, r.level))
            })
            .collect();
        if rooms.is_empty() {
            return;
        }
        let in_town = |f: &Fixtures, kinds: &[&str]| {
            f.items.iter().any(|p| {
                kinds.contains(&p.kind)
                    && matches!(p.at, Spot::Room { structure, .. } if w.structures[structure].settlement == Some(start))
            })
        };
        let needs: [(&[&str], &'static str); 5] = [
            (&["firesteel"], "firesteel"),
            (&["waterskin"], "waterskin"),
            (&["provisions"], "provisions"),
            (&["wood"], "wood"),
            (&["pry_bar"], "pry_bar"),
        ];
        for (n, (have, kind)) in needs.iter().enumerate() {
            if !in_town(self, have) {
                let (structure, room, _) =
                    rooms[(hash(&[w.seed, n as u64, 0x6a5]) % rooms.len() as u64) as usize];
                self.items.push(Placed {
                    kind,
                    at: Spot::Room { structure, room },
                    pos: land.structure_pos[structure],
                });
            }
        }
        // And the three writing tools lie somewhere in the world, inert
        // until M08–M09.
        // DESIGN-Q: where the scraper, stylus and lens lie (an archive or
        // temple each, never the starting town).
        let homes: Vec<usize> = w
            .structures
            .iter()
            .filter(|st| {
                st.settlement != Some(start)
                    && matches!(st.kind, StructureKind::Archive | StructureKind::Temple)
                    && st.condition != Condition::Buried
            })
            .map(|st| st.id)
            .filter(|&sid| !self.reachable_rooms(w, sid).is_empty())
            .collect();
        // Reachable on foot from the start, by the land as it is.
        let from = Pos::of_cell(
            w.history.settlements[start].cell.ux(),
            w.history.settlements[start].cell.uy(),
        );
        // Routing across the land is slow: each home once.
        let on_foot: BTreeSet<usize> = homes
            .iter()
            .copied()
            .filter(|&sid| land.route(w, from, land.structure_pos[sid]).is_some())
            .collect();
        for (n, tool) in [
            "scraper",
            "stylus",
            "lens",
            "fine_scraper",
            "old_scraper",
            "first_scraper",
            "first_lens",
        ]
        .into_iter()
        .enumerate()
        {
            if homes.is_empty() {
                continue;
            }
            let pick = hash(&[w.seed, 0x7001, n as u64]) as usize;
            let reachable = |sid: usize| on_foot.contains(&sid);
            // Stronger scrapers lie farther out; the strongest with the root.
            // DESIGN-Q: the fine scraper about halfway out, the old one far,
            // the first scraper where the root inscription lies, the first
            // lens nearly as far out as anything.
            let chosen = if n < 3 {
                (0..homes.len())
                    .map(|k| homes[(pick + k) % homes.len()])
                    .find(|&sid| reachable(sid))
            } else {
                let root_home = w
                    .texts
                    .iter()
                    .find(|t| t.event == Some(w.history.root))
                    .map(|t| t.structure)
                    .filter(|&sid| {
                        reachable(sid)
                            || (!self.reachable_rooms(w, sid).is_empty()
                                && land.route(w, from, land.structure_pos[sid]).is_some())
                    });
                let mut far: Vec<usize> = homes
                    .iter()
                    .copied()
                    .filter(|&sid| reachable(sid))
                    .collect();
                far.sort_by_key(|&sid| (land.structure_pos[sid].dist2(from), sid));
                match (tool, root_home) {
                    ("first_scraper", Some(r)) => Some(r),
                    ("first_scraper", None) => far.last().copied(),
                    ("old_scraper", _) => far.get(far.len() * 4 / 5).copied(),
                    ("first_lens", _) => far.get(far.len() * 9 / 10).copied(),
                    _ => far.get(far.len() / 2).copied(),
                }
            };
            let Some(sid) = chosen else {
                continue;
            };
            let room = *self.reachable_rooms(w, sid).last().unwrap_or(&0);
            self.items.push(Placed {
                kind: tool,
                at: Spot::Room {
                    structure: sid,
                    room,
                },
                pos: land.structure_pos[sid],
            });
        }
    }

    /// Rooms of a building a player can reach from its entrance by ordinary
    /// means (no rubble, no water, barred doors only with a pry bar),
    /// nearest first.
    pub fn reachable_rooms(&self, w: &World, structure: usize) -> Vec<usize> {
        let st = &w.structures[structure];
        let rooms = &st.interior.rooms;
        if rooms.is_empty() || rooms[0].collapsed {
            return Vec::new();
        }
        let mut flooded = vec![false; rooms.len()];
        for f in self.flooded.iter().filter(|f| f.structure == structure) {
            flooded[f.room] = true;
        }
        // Ways out of each room, both ways unless one-way; windows are
        // seen through, never passed (D04).
        let mut next: Vec<Vec<(usize, usize)>> = vec![Vec::new(); rooms.len()];
        for (li, l) in st.interior.links.iter().enumerate() {
            if l.passage == Passage::Window {
                continue;
            }
            next[l.a].push((l.b, li));
            if !l.one_way {
                next[l.b].push((l.a, li));
            }
        }
        let mut seen = vec![0usize];
        let mut have = vec![false; rooms.len()];
        have[0] = true;
        let mut i = 0;
        while i < seen.len() {
            let r = seen[i];
            i += 1;
            for &(other, li) in &next[r] {
                let l = &st.interior.links[li];
                if (l.state == PassageState::Blocked && !self.cleared.contains(&(structure, li)))
                    || rooms[other].collapsed
                    || flooded[other]
                    || have[other]
                {
                    continue;
                }
                have[other] = true;
                seen.push(other);
            }
        }
        seen
    }

    /// Clears blocked links on the way from a building's entrance to a room.
    fn clear_way(&mut self, w: &World, structure: usize, target: usize) {
        let st = &w.structures[structure];
        let rooms = &st.interior.rooms;
        if rooms.is_empty() || rooms[0].collapsed || rooms[target].collapsed {
            return;
        }
        // Breadth-first over every passage, rubble included.
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; rooms.len()];
        let mut seen = vec![false; rooms.len()];
        seen[0] = true;
        let mut queue = vec![0usize];
        let mut i = 0;
        while i < queue.len() {
            let r = queue[i];
            i += 1;
            for (li, l) in st.interior.links.iter().enumerate() {
                let other = if l.a == r {
                    l.b
                } else if l.b == r {
                    l.a
                } else {
                    continue;
                };
                if seen[other] || rooms[other].collapsed {
                    continue;
                }
                seen[other] = true;
                prev[other] = Some((r, li));
                queue.push(other);
            }
        }
        let mut cur = target;
        while let Some((from, li)) = prev[cur] {
            if st.interior.links[li].state == PassageState::Blocked {
                self.cleared.insert((structure, li));
            }
            cur = from;
        }
    }

    /// River flow at a cell, given which sluices are open.
    pub fn flow(&self, w: &World, x: usize, y: usize, open: &dyn Fn(usize) -> bool) -> u32 {
        let mut f = *w.water.flow.get(x, y);
        for (i, s) in self.sluices.iter().enumerate() {
            if open(i) && s.below.contains(&(x, y)) {
                f -= s.diverts.min(f);
            }
        }
        f
    }

    /// Water standing in a sluice's channel cell, given open sluices.
    pub fn channel_water(&self, x: usize, y: usize, open: &dyn Fn(usize) -> bool) -> u32 {
        self.sluices
            .iter()
            .enumerate()
            .filter(|(i, s)| open(*i) && s.channel == (x, y))
            .map(|(_, s)| s.diverts)
            .sum()
    }
}

/// A coarse description of where a cell is, for debugging.
pub fn cell_pos(x: usize, y: usize) -> Pos {
    Pos::new(x as i32 * CELL + CELL / 2, y as i32 * CELL + CELL / 2)
}
