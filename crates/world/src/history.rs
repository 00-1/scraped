//! The civilisation's history: who lived where, what happened, and what
//! they wrote, across the language's eras.
//!
//! Every event records its place, era, actors and cause, and what evidence
//! it should leave (a tomb, a ledger, a ruin). Later stages turn that
//! evidence into structures and texts, so the world's past can be read back
//! from what is lying around in it.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::Serialize;

use scraped_lang::concepts::Pos;
use scraped_lang::phonology::Phonemes;
use scraped_lang::rng::{Rng, Stream};
use scraped_lang::Language;

use crate::terrain::{Biome, Grid, Terrain, SIZE};
use crate::water::Water;

/// A grid position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Cell {
    pub x: u16,
    pub y: u16,
}

impl Cell {
    pub fn new(x: usize, y: usize) -> Self {
        Cell {
            x: x as u16,
            y: y as u16,
        }
    }

    pub fn ux(self) -> usize {
        usize::from(self.x)
    }

    pub fn uy(self) -> usize {
        usize::from(self.y)
    }

    /// Squared distance in cells.
    pub fn dist2(self, o: Cell) -> i64 {
        let (dx, dy) = (
            i64::from(self.x) - i64::from(o.x),
            i64::from(self.y) - i64::from(o.y),
        );
        dx * dx + dy * dy
    }
}

/// One era of history, matching one era of the language.
#[derive(Debug, Clone, Serialize)]
pub struct Era {
    pub index: u32,
    pub start: i32,
    pub end: i32,
}

/// A people that holds settlements. Schisms split one into two.
#[derive(Debug, Clone, Serialize)]
pub struct Faction {
    pub id: usize,
    pub name: Phonemes,
    pub era: u32,
    pub parent: Option<usize>,
}

/// What a person did, which decides their title and where they appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Ruler,
    Priest,
    Scribe,
    Smith,
    Servant,
    Commoner,
}

impl Role {
    /// The lexicon concept naming this role, if it has one.
    pub fn title(self) -> Option<&'static str> {
        match self {
            Role::Ruler => Some("king"),
            Role::Priest => Some("priest"),
            Role::Scribe => Some("scribe"),
            Role::Smith => Some("smith"),
            Role::Servant => Some("servant"),
            Role::Commoner => None,
        }
    }
}

/// A named person of the past.
#[derive(Debug, Clone, Serialize)]
pub struct Person {
    pub id: usize,
    /// A word of their era's language.
    pub name: Phonemes,
    pub era: u32,
    pub role: Role,
    pub faction: usize,
    pub settlement: usize,
    /// Kinship to another person of the same era: ("son", 12).
    pub relation: Option<(String, usize)>,
    pub born: i32,
    pub died: i32,
}

/// A town or village.
#[derive(Debug, Clone, Serialize)]
pub struct Settlement {
    pub id: usize,
    pub name: Phonemes,
    pub cell: Cell,
    pub era: u32,
    pub founded: i32,
    pub abandoned: Option<i32>,
    pub faction: usize,
    /// 1 hamlet to 4 city.
    pub size: u8,
    pub capital: bool,
}

/// A road between two settlements, cell by cell.
#[derive(Debug, Clone, Serialize)]
pub struct Road {
    pub from: usize,
    pub to: usize,
    pub era: u32,
    pub path: Vec<Cell>,
}

/// A local physical property writing can push (made mechanical in M08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Property {
    Openness,
    Heat,
    Stability,
}

/// What a writing event meant to do, recorded for M08 to make real.
#[derive(Debug, Clone, Serialize)]
pub struct Effect {
    pub property: Property,
    /// +1 or −1.
    pub change: i8,
    pub cell: Cell,
    /// In cells.
    pub radius: u16,
}

/// A potent claim: verb and subject concept, and whether it is denied.
#[derive(Debug, Clone, Serialize)]
pub struct Claim {
    pub verb: &'static str,
    pub subject: &'static str,
    pub negative: bool,
}

/// How the world's fate is tending at the start of play (simulated in M10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Trajectory {
    Dying,
    Stagnant,
    Balanced,
    Recovering,
}

/// Why something was abandoned or someone died.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Cause {
    War,
    Plague,
    Famine,
    Age,
    Writing,
}

/// What happened.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum EventKind {
    Founding {
        settlement: usize,
    },
    Abandonment {
        settlement: usize,
        cause: Cause,
    },
    Succession {
        ruler: usize,
        predecessor: Option<usize>,
    },
    Death {
        person: usize,
        cause: Cause,
    },
    War {
        attacker: usize,
        defender: usize,
        settlement: usize,
    },
    Plague {
        settlement: usize,
    },
    Famine {
        settlement: usize,
    },
    Schism {
        faction: usize,
        new_faction: usize,
    },
    Migration {
        from: usize,
        to: usize,
    },
    Writing {
        claim: Claim,
        effect: Effect,
        author: usize,
        root: bool,
    },
}

/// Traces an event should leave in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    /// A dedication on a temple or monument.
    Dedication,
    /// A tomb with an epitaph.
    Tomb,
    /// A ledger in a storehouse.
    Ledger,
    /// A letter in a house.
    Letter,
    /// A warning or sign.
    Sign,
    /// The potent inscription itself.
    Inscription,
    /// Ruined buildings.
    Ruin,
    /// A defensive wall.
    Wall,
}

/// One event of history.
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: usize,
    pub era: u32,
    pub year: i32,
    pub cell: Cell,
    pub kind: EventKind,
    pub actors: Vec<usize>,
    /// The event that led to this one.
    pub cause: Option<usize>,
    pub evidence: Vec<Evidence>,
}

/// The whole past.
#[derive(Debug, Clone, Serialize)]
pub struct History {
    pub eras: Vec<Era>,
    /// The year play begins.
    pub present: i32,
    pub factions: Vec<Faction>,
    pub people: Vec<Person>,
    pub settlements: Vec<Settlement>,
    pub roads: Vec<Road>,
    pub events: Vec<Event>,
    /// The deepest text: the writing event that explains the world's state.
    pub root: usize,
    pub trajectory: Trajectory,
}

struct Builder<'a> {
    rng: Rng,
    t: &'a Terrain,
    w: &'a Water,
    h: History,
}

/// How good a cell is for a settlement: water, defence and land.
fn site_score(t: &Terrain, w: &Water, c: Cell) -> i64 {
    let (x, y) = (c.ux(), c.uy());
    if !t.is_land(x, y) || w.is_river(t, x, y) {
        return i64::MIN;
    }
    let b = *t.biome.get(x, y);
    let land = match b {
        Biome::Grassland | Biome::Forest => 30,
        Biome::Scrub | Biome::Shore | Biome::Pine => 15,
        Biome::Marsh | Biome::Tundra | Biome::Desert => 2,
        _ => -40,
    };
    let mut water = 0;
    let mut higher = 0;
    let here = *t.height.get(x, y);
    for dy in -3i32..=3 {
        for dx in -3i32..=3 {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= SIZE as i32 || ny >= SIZE as i32 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if w.is_river(t, nx, ny) || t.biome.get(nx, ny).is_water() {
                water = 40;
            }
            if *t.height.get(nx, ny) > here {
                higher += 1;
            }
        }
    }
    // Defensible ground stands above most of its surroundings.
    let defence = 20 - higher.min(40) / 2;
    land + water + defence
}

impl Builder<'_> {
    fn year_span(&mut self, era: &Era) -> i32 {
        era.start + self.rng.below((era.end - era.start) as u32) as i32
    }

    #[allow(clippy::too_many_arguments)]
    fn event(
        &mut self,
        era: u32,
        year: i32,
        cell: Cell,
        kind: EventKind,
        actors: Vec<usize>,
        cause: Option<usize>,
        evidence: Vec<Evidence>,
    ) -> usize {
        let id = self.h.events.len();
        self.h.events.push(Event {
            id,
            era,
            year,
            cell,
            kind,
            actors,
            cause,
            evidence,
        });
        id
    }

    /// The best free site, at least `gap` cells from every settlement.
    fn best_site(&mut self, near: Option<Cell>, gap: i64) -> Option<Cell> {
        let mut best: Option<(i64, Cell)> = None;
        for _ in 0..400 {
            let c = match near {
                Some(n) => {
                    let dx = self.rng.range(0, 40) as i32 - 20;
                    let dy = self.rng.range(0, 40) as i32 - 20;
                    let (x, y) = (i32::from(n.x) + dx, i32::from(n.y) + dy);
                    if x < 4 || y < 4 || x >= SIZE as i32 - 4 || y >= SIZE as i32 - 4 {
                        continue;
                    }
                    Cell::new(x as usize, y as usize)
                }
                None => Cell::new(
                    self.rng.range(8, SIZE as u32 - 9) as usize,
                    self.rng.range(8, SIZE as u32 - 9) as usize,
                ),
            };
            if self
                .h
                .settlements
                .iter()
                .any(|s| s.cell.dist2(c) < gap * gap)
            {
                continue;
            }
            let score = site_score(self.t, self.w, c);
            if score > 0 && best.is_none_or(|(b, _)| score > b) {
                best = Some((score, c));
            }
        }
        best.map(|(_, c)| c)
    }

    fn found(
        &mut self,
        maker: &mut scraped_lang::lexicon::WordMaker,
        era: u32,
        year: i32,
        cell: Cell,
        faction: usize,
        capital: bool,
    ) -> usize {
        let id = self.h.settlements.len();
        let name = maker.make(&mut self.rng, 2, Pos::Noun);
        let size = if capital {
            4
        } else {
            self.rng.range(1, 3) as u8
        };
        self.h.settlements.push(Settlement {
            id,
            name,
            cell,
            era,
            founded: year,
            abandoned: None,
            faction,
            size,
            capital,
        });
        self.event(
            era,
            year,
            cell,
            EventKind::Founding { settlement: id },
            vec![],
            None,
            vec![Evidence::Dedication],
        );
        id
    }

    fn person(
        &mut self,
        maker: &mut scraped_lang::lexicon::WordMaker,
        era: u32,
        born: i32,
        role: Role,
        settlement: usize,
        relation: Option<(String, usize)>,
    ) -> usize {
        let id = self.h.people.len();
        let syllables = self.rng.weighted(&[(2, 60), (3, 40)]);
        let name = maker.make(&mut self.rng, syllables, Pos::Noun);
        let faction = self.h.settlements[settlement].faction;
        let life = 35 + self.rng.below(40) as i32;
        self.h.people.push(Person {
            id,
            name,
            era,
            role,
            faction,
            settlement,
            relation,
            born,
            died: born + life,
        });
        id
    }

    fn living(&self, at: i32) -> Vec<usize> {
        self.h
            .settlements
            .iter()
            .filter(|s| s.founded <= at && s.abandoned.is_none_or(|a| a > at))
            .map(|s| s.id)
            .collect()
    }
}

impl History {
    pub fn generate(seed: u64, t: &Terrain, w: &Water, langs: &[Language]) -> Self {
        let mut rng = Rng::new(seed, Stream::World(2));
        let mut eras = Vec::new();
        let mut year = 0;
        for i in 0..langs.len() as u32 {
            let len = 220 + rng.below(100) as i32;
            eras.push(Era {
                index: i,
                start: year,
                end: year + len,
            });
            year += len;
        }
        let present = year + 100 + rng.below(200) as i32;
        let trajectory = rng.weighted(&[
            (Trajectory::Dying, 30),
            (Trajectory::Stagnant, 25),
            (Trajectory::Balanced, 25),
            (Trajectory::Recovering, 20),
        ]);
        let mut b = Builder {
            rng,
            t,
            w,
            h: History {
                eras,
                present,
                factions: Vec::new(),
                people: Vec::new(),
                settlements: Vec::new(),
                roads: Vec::new(),
                events: Vec::new(),
                root: 0,
                trajectory,
            },
        };
        for (e, lang) in langs.iter().enumerate() {
            let mut maker = lang.word_maker();
            // Names already used in earlier eras must not collide either.
            for p in &b.h.people {
                maker.reserve(&p.name, Pos::Noun);
            }
            simulate_era(&mut b, e as u32, &mut maker);
        }
        build_roads(&mut b);
        let mut h = b.h;
        h.events.sort_by_key(|e| (e.year, e.id));
        // Keep ids equal to positions after sorting, remapping references.
        let remap: Vec<usize> = {
            let mut m = vec![0; h.events.len()];
            for (new, e) in h.events.iter().enumerate() {
                m[e.id] = new;
            }
            m
        };
        for e in &mut h.events {
            e.id = remap[e.id];
            e.cause = e.cause.map(|c| remap[c]);
        }
        h.root = remap[h.root];
        h
    }

    /// Settlements standing in a given year.
    pub fn standing(&self, year: i32) -> Vec<&Settlement> {
        self.settlements
            .iter()
            .filter(|s| s.founded <= year && s.abandoned.is_none_or(|a| a > year))
            .collect()
    }
}

fn simulate_era(b: &mut Builder, e: u32, maker: &mut scraped_lang::lexicon::WordMaker) {
    let era = b.h.eras[e as usize].clone();
    if e == 0 {
        // The first people arrive and found their capital and first towns.
        let faction = b.h.factions.len();
        let name = maker.make(&mut b.rng, 2, Pos::Noun);
        b.h.factions.push(Faction {
            id: faction,
            name,
            era: 0,
            parent: None,
        });
        let count = b.rng.range(5, 8);
        for i in 0..count {
            let gap = 18;
            let Some(cell) = b.best_site(None, gap) else {
                break;
            };
            let year = era.start + i as i32 * b.rng.range(5, 30) as i32;
            b.found(maker, e, year, cell, faction, i == 0);
        }
    }

    // Rulers: a dynasty per faction, parent to child.
    let factions: Vec<usize> = b.h.factions.iter().map(|f| f.id).collect();
    for f in factions {
        let Some(seat) =
            b.h.settlements
                .iter()
                .find(|s| s.faction == f && s.abandoned.is_none())
                .map(|s| s.id)
        else {
            continue;
        };
        let mut year = era.start + b.rng.below(20) as i32;
        let mut prev: Option<usize> = None;
        while year < era.end {
            let rel = prev.map(|p| (b.rng.pick(&["son", "daughter"]).to_string(), p));
            let ruler = b.person(maker, e, year - 25, Role::Ruler, seat, rel);
            let cell = b.h.settlements[seat].cell;
            b.event(
                e,
                year,
                cell,
                EventKind::Succession {
                    ruler,
                    predecessor: prev,
                },
                vec![ruler],
                None,
                vec![Evidence::Dedication],
            );
            let reign = 20 + b.rng.below(25) as i32;
            b.h.people[ruler].died = (year + reign).max(b.h.people[ruler].born + 30);
            let died = b.h.people[ruler].died;
            if died < era.end {
                b.event(
                    e,
                    died,
                    cell,
                    EventKind::Death {
                        person: ruler,
                        cause: Cause::Age,
                    },
                    vec![ruler],
                    None,
                    vec![Evidence::Tomb],
                );
            }
            prev = Some(ruler);
            year += reign;
        }
    }

    // Ordinary notable people in every standing settlement.
    for s in b.living(era.start + 40) {
        let roles = [
            Role::Priest,
            Role::Scribe,
            Role::Smith,
            Role::Servant,
            Role::Commoner,
            Role::Commoner,
        ];
        let mut family: Vec<usize> = Vec::new();
        for _ in 0..b.rng.range(3, 6) {
            let role = *b.rng.pick(&roles);
            let rel = (!family.is_empty() && b.rng.chance(60)).then(|| {
                let kin = *b.rng.pick(&[
                    "child", "son", "daughter", "wife", "husband", "brother", "sister",
                ]);
                (kin.to_string(), *b.rng.pick(&family))
            });
            let born = b.year_span(&era) - 30;
            let p = b.person(maker, e, born, role, s, rel);
            family.push(p);
            let cell = b.h.settlements[s].cell;
            let died = b.h.people[p].died;
            b.event(
                e,
                died,
                cell,
                EventKind::Death {
                    person: p,
                    cause: Cause::Age,
                },
                vec![p],
                None,
                vec![Evidence::Tomb],
            );
        }
    }

    // The events of the era.
    let count = b.rng.range(8, 14);
    for _ in 0..count {
        let year = b.year_span(&era);
        let living = b.living(year);
        if living.is_empty() {
            break;
        }
        let s = *b.rng.pick(&living);
        let cell = b.h.settlements[s].cell;
        let kind = b
            .rng
            .weighted(&[(0, 20), (1, 12), (2, 12), (3, 10), (4, 8), (5, 18), (6, 8)]);
        match kind {
            0 => {
                // Growth: a new town near this one.
                if let Some(c) = b.best_site(Some(cell), 12) {
                    let f = b.h.settlements[s].faction;
                    let new = b.found(maker, e, year, c, f, false);
                    let mig = b.event(
                        e,
                        year,
                        c,
                        EventKind::Migration { from: s, to: new },
                        vec![],
                        None,
                        vec![Evidence::Letter],
                    );
                    b.h.events[mig].cause = None;
                }
            }
            1 => {
                let ev = b.event(
                    e,
                    year,
                    cell,
                    EventKind::Plague { settlement: s },
                    vec![],
                    None,
                    vec![Evidence::Tomb, Evidence::Sign],
                );
                if !b.h.settlements[s].capital && b.rng.chance(40) {
                    b.h.settlements[s].abandoned = Some(year + 5);
                    b.event(
                        e,
                        year + 5,
                        cell,
                        EventKind::Abandonment {
                            settlement: s,
                            cause: Cause::Plague,
                        },
                        vec![],
                        Some(ev),
                        vec![Evidence::Ruin],
                    );
                }
            }
            2 => {
                let ev = b.event(
                    e,
                    year,
                    cell,
                    EventKind::Famine { settlement: s },
                    vec![],
                    None,
                    vec![Evidence::Ledger, Evidence::Letter],
                );
                if !b.h.settlements[s].capital && b.rng.chance(25) {
                    b.h.settlements[s].abandoned = Some(year + 10);
                    b.event(
                        e,
                        year + 10,
                        cell,
                        EventKind::Abandonment {
                            settlement: s,
                            cause: Cause::Famine,
                        },
                        vec![],
                        Some(ev),
                        vec![Evidence::Ruin],
                    );
                }
            }
            3 if b.h.factions.len() > 1 => {
                let defender = b.h.settlements[s].faction;
                let others: Vec<usize> =
                    b.h.factions
                        .iter()
                        .map(|f| f.id)
                        .filter(|&f| f != defender)
                        .collect();
                let attacker = *b.rng.pick(&others);
                let ev = b.event(
                    e,
                    year,
                    cell,
                    EventKind::War {
                        attacker,
                        defender,
                        settlement: s,
                    },
                    vec![],
                    None,
                    vec![Evidence::Wall, Evidence::Sign],
                );
                if !b.h.settlements[s].capital && b.rng.chance(35) {
                    b.h.settlements[s].abandoned = Some(year + 1);
                    b.event(
                        e,
                        year + 1,
                        cell,
                        EventKind::Abandonment {
                            settlement: s,
                            cause: Cause::War,
                        },
                        vec![],
                        Some(ev),
                        vec![Evidence::Ruin],
                    );
                }
            }
            4 if e > 0 => {
                // A schism: a town and its neighbours break away.
                let old = b.h.settlements[s].faction;
                let new = b.h.factions.len();
                let name = maker.make(&mut b.rng, 2, Pos::Noun);
                b.h.factions.push(Faction {
                    id: new,
                    name,
                    era: e,
                    parent: Some(old),
                });
                let near: Vec<usize> = b
                    .living(year)
                    .into_iter()
                    .filter(|&o| b.h.settlements[o].faction == old && !b.h.settlements[o].capital)
                    .filter(|&o| b.h.settlements[o].cell.dist2(cell) < 30 * 30)
                    .collect();
                for o in near {
                    b.h.settlements[o].faction = new;
                }
                if !b.h.settlements[s].capital {
                    b.h.settlements[s].faction = new;
                }
                b.event(
                    e,
                    year,
                    cell,
                    EventKind::Schism {
                        faction: old,
                        new_faction: new,
                    },
                    vec![],
                    None,
                    vec![Evidence::Letter, Evidence::Dedication],
                );
            }
            _ => writing_event(b, e, year, s, false),
        }
    }
    // The root event: the deepest text, at the first capital in the
    // first era, explaining the state of the world.
    if e == 0 {
        let capital =
            b.h.settlements
                .iter()
                .find(|s| s.capital)
                .map(|s| s.id)
                .unwrap_or(0);
        let year = era.start + (era.end - era.start) / 2;
        writing_event(b, e, year, capital, true);
        b.h.root = b.h.events.len() - 1;
    }
}

/// A potent inscription cast at a settlement, recorded with its intended
/// effect.
// DESIGN-Q: claims draw on the potent verbs M02 can express (open, burn,
// break), each tied to one property. M08 replaces this with the full
// concept-to-property table.
fn writing_event(b: &mut Builder, e: u32, year: i32, s: usize, root: bool) {
    let cell = b.h.settlements[s].cell;
    let authors: Vec<usize> =
        b.h.people
            .iter()
            .filter(|p| {
                p.era == e
                    && p.settlement == s
                    && matches!(p.role, Role::Priest | Role::Scribe | Role::Ruler)
            })
            .map(|p| p.id)
            .collect();
    let Some(&author) = (if authors.is_empty() {
        None
    } else {
        Some(b.rng.pick(&authors))
    }) else {
        return;
    };
    let (verb, property, subjects): (&str, Property, &[&str]) = if root {
        match b.h.trajectory {
            Trajectory::Dying => ("break", Property::Stability, &["wall", "stone"]),
            Trajectory::Stagnant => ("open", Property::Openness, &["gate", "door"]),
            Trajectory::Balanced => ("burn", Property::Heat, &["field", "tree"]),
            Trajectory::Recovering => ("open", Property::Openness, &["tomb", "gate"]),
        }
    } else {
        *b.rng.pick(&[
            (
                "open",
                Property::Openness,
                &["gate", "door", "tomb", "box"][..],
            ),
            (
                "burn",
                Property::Heat,
                &["field", "tree", "house", "grain"][..],
            ),
            (
                "break",
                Property::Stability,
                &["wall", "gate", "stone", "statue"][..],
            ),
        ])
    };
    let subject = *b.rng.pick(subjects);
    let negative = if root {
        b.h.trajectory != Trajectory::Recovering
    } else {
        b.rng.chance(50)
    };
    let effect = Effect {
        property,
        change: if negative { -1 } else { 1 },
        cell,
        radius: if root {
            SIZE as u16
        } else {
            b.rng.range(2, 8) as u16
        },
    };
    let verb = ["open", "burn", "break"]
        .into_iter()
        .find(|v| *v == verb)
        .expect("known verb");
    let subject = [
        "gate", "door", "tomb", "box", "field", "tree", "house", "grain", "wall", "stone", "statue",
    ]
    .into_iter()
    .find(|v| *v == subject)
    .expect("known subject");
    b.event(
        e,
        year,
        cell,
        EventKind::Writing {
            claim: Claim {
                verb,
                subject,
                negative,
            },
            effect,
            author,
            root,
        },
        vec![author],
        None,
        vec![Evidence::Inscription],
    );
}

/// Roads join settlements: a spanning tree over all of them plus a few
/// extra links, each routed by least effort over the land.
fn build_roads(b: &mut Builder) {
    let n = b.h.settlements.len();
    if n < 2 {
        return;
    }
    let mut joined = vec![false; n];
    joined[0] = true;
    let mut pairs = Vec::new();
    for _ in 1..n {
        let mut best: Option<(i64, usize, usize)> = None;
        for a in (0..n).filter(|&a| joined[a]) {
            for c in (0..n).filter(|&c| !joined[c]) {
                let d = b.h.settlements[a].cell.dist2(b.h.settlements[c].cell);
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, a, c));
                }
            }
        }
        let (_, a, c) = best.expect("unjoined settlement");
        joined[c] = true;
        pairs.push((a, c));
    }
    for _ in 0..n / 3 {
        let a = b.rng.index(n);
        let c = b.rng.index(n);
        if a != c && !pairs.contains(&(a, c)) && !pairs.contains(&(c, a)) {
            pairs.push((a, c));
        }
    }
    for (a, c) in pairs {
        let (sa, sc) = (&b.h.settlements[a], &b.h.settlements[c]);
        let era = sa.era.max(sc.era);
        if let Some(path) = route(b.t, b.w, sa.cell, sc.cell) {
            b.h.roads.push(Road {
                from: a,
                to: c,
                era,
                path,
            });
        }
    }
}

/// Least-effort path over land (A*): slopes cost, rivers cost more unless
/// fordable, open water is impassable.
pub fn route(t: &Terrain, w: &Water, from: Cell, to: Cell) -> Option<Vec<Cell>> {
    let s = SIZE;
    let idx = |c: Cell| c.uy() * s + c.ux();
    let cost = |x: usize, y: usize, from_h: f64| -> Option<i64> {
        if t.biome.get(x, y).is_water() {
            return None;
        }
        let slope = (t.height.get(x, y) - from_h).abs();
        let river = if w.needs_crossing(t, x, y) {
            40
        } else if w.is_river(t, x, y) {
            8
        } else {
            0
        };
        let rough = match t.biome.get(x, y) {
            Biome::Marsh => 12,
            Biome::Rock | Biome::Snow => 20,
            Biome::Forest | Biome::Pine => 4,
            _ => 0,
        };
        Some(10 + (slope / 4.0) as i64 + river + rough)
    };
    let h = |c: Cell| ((c.dist2(to) as f64).sqrt() * 10.0) as i64;
    let mut best = Grid::new(s, i64::MAX);
    let mut prev = Grid::new(s, u32::MAX);
    let mut heap = BinaryHeap::new();
    best.cells[idx(from)] = 0;
    heap.push(Reverse((h(from), 0i64, idx(from) as u32)));
    while let Some(Reverse((_, g, i))) = heap.pop() {
        let c = Cell::new(i as usize % s, i as usize / s);
        if c == to {
            let mut path = vec![to];
            let mut cur = i;
            while prev.cells[cur as usize] != u32::MAX {
                cur = prev.cells[cur as usize];
                path.push(Cell::new(cur as usize % s, cur as usize / s));
            }
            path.reverse();
            return Some(path);
        }
        if g > best.cells[i as usize] {
            continue;
        }
        let here_h = *t.height.get(c.ux(), c.uy());
        let nbs: Vec<(usize, usize)> = best.neighbours(c.ux(), c.uy()).collect();
        for (nx, ny) in nbs {
            let Some(step) = cost(nx, ny, here_h) else {
                continue;
            };
            let ng = g + step;
            let ni = ny * s + nx;
            if ng < best.cells[ni] {
                best.cells[ni] = ng;
                prev.cells[ni] = i;
                heap.push(Reverse((ng + h(Cell::new(nx, ny)), ng, ni as u32)));
            }
        }
    }
    None
}
