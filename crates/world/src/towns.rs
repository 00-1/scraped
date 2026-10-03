//! What each settlement was for, and how it was laid out (D03).
//!
//! A settlement's role comes from its history and its ground: the capital,
//! a port on the coast, a fortress that saw war, a mining camp by rock, a
//! holy city where priests and potent writing gathered, a market town
//! where roads meet, a refuge founded by people fleeing, or a farming
//! village. The role decides its districts and buildings; the ground
//! decides its shape (a river crossing, a hilltop, a harbour). Streets
//! join the districts, so moving through a town has places in it.

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

use crate::geology::Geology;
use crate::history::{Cell, EventKind, History, Role as PersonRole};
use crate::terrain::{Biome, Terrain, SIZE};
use crate::water::Water;

/// What a settlement was for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TownRole {
    Capital,
    Port,
    HolyCity,
    MiningCamp,
    Fortress,
    MarketTown,
    FarmingVillage,
    Refuge,
}

impl TownRole {
    pub const ALL: [TownRole; 8] = [
        TownRole::Capital,
        TownRole::Port,
        TownRole::HolyCity,
        TownRole::MiningCamp,
        TownRole::Fortress,
        TownRole::MarketTown,
        TownRole::FarmingVillage,
        TownRole::Refuge,
    ];

    /// The id for content.
    pub fn id(self) -> &'static str {
        match self {
            TownRole::Capital => "capital",
            TownRole::Port => "port",
            TownRole::HolyCity => "holy city",
            TownRole::MiningCamp => "mining camp",
            TownRole::Fortress => "fortress",
            TownRole::MarketTown => "market town",
            TownRole::FarmingVillage => "farming village",
            TownRole::Refuge => "refuge",
        }
    }
}

/// Why a town has its shape. Ids for content.
pub const REASONS: [&str; 7] = [
    "river crossing",
    "harbour",
    "hilltop",
    "crossroads",
    "valley floor",
    "spring",
    "open plain",
];

/// How a town's streets run. Ids for content.
pub const PLANS: [&str; 7] = [
    "grid",
    "ring",
    "strung along the road",
    "terraced",
    "clustered",
    "radial",
    "along the shore",
];

/// Kinds of district. Ids for content.
pub const DISTRICTS: [&str; 15] = [
    "square",
    "temple quarter",
    "market",
    "harbour",
    "workshops",
    "palace quarter",
    "garrison",
    "gardens",
    "graves",
    "mines",
    "farmyards",
    "scholars' quarter",
    "sacred way",
    "upper town",
    "old town",
];

/// Kinds of street. Ids for content.
pub const STREETS: [&str; 7] = [
    "main street",
    "lane",
    "stair street",
    "quay",
    "causeway",
    "colonnade",
    "bridge street",
];

/// A part of a town, around a cell.
#[derive(Debug, Clone, Serialize)]
pub struct District {
    pub kind: &'static str,
    pub cell: Cell,
}

/// A street joining two districts (indices into `Town::districts`).
#[derive(Debug, Clone, Serialize)]
pub struct Street {
    pub kind: &'static str,
    pub from: usize,
    pub to: usize,
}

/// A settlement's role and layout.
#[derive(Debug, Clone, Serialize)]
pub struct Town {
    pub settlement: usize,
    pub role: TownRole,
    pub reason: &'static str,
    pub plan: &'static str,
    /// District 0 is always the square at the town's heart.
    pub districts: Vec<District>,
    pub streets: Vec<Street>,
    pub walled: bool,
    /// Gates in the walls (0 when unwalled).
    pub gates: usize,
}

impl Town {
    /// What makes a layout: towns of the same role and size never share it.
    pub fn signature(&self) -> String {
        let mut d: Vec<&str> = self.districts.iter().map(|d| d.kind).collect();
        d.sort_unstable();
        format!(
            "{}|{}|{}|{}",
            self.plan,
            self.reason,
            self.walled,
            d.join(",")
        )
    }

    /// The district nearest a cell.
    pub fn district_at(&self, c: Cell) -> usize {
        self.districts
            .iter()
            .enumerate()
            .min_by_key(|(i, d)| (d.cell.dist2(c), *i))
            .map_or(0, |(i, _)| i)
    }
}

/// Counts cells within `r` of `(x, y)` matching `f`.
fn count_near<F: Fn(usize, usize) -> bool>(x: usize, y: usize, r: i64, f: F) -> usize {
    let mut n = 0;
    for dy in -r..=r {
        for dx in -r..=r {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if (dx, dy) != (0, 0)
                && nx >= 0
                && ny >= 0
                && nx < SIZE as i64
                && ny < SIZE as i64
                && f(nx as usize, ny as usize)
            {
                n += 1;
            }
        }
    }
    n
}

/// Whether rock that can be mined lies within a few cells.
pub fn rocky(t: &Terrain, c: Cell) -> bool {
    count_near(c.ux(), c.uy(), 6, |x, y| {
        matches!(t.biome.get(x, y), Biome::Rock | Biome::Pine | Biome::Tundra)
    }) > 3
}

/// Whether the sea is within reach of the town.
pub fn coastal(t: &Terrain, c: Cell) -> bool {
    count_near(c.ux(), c.uy(), 4, |x, y| *t.biome.get(x, y) == Biome::Sea) > 0
}

/// Whether a river runs by the town.
pub fn riverside(t: &Terrain, w: &Water, c: Cell) -> bool {
    count_near(c.ux(), c.uy(), 1, |x, y| w.is_river(t, x, y)) > 0
}

/// Plans every settlement's role and layout.
// DESIGN-Q: roles in order of precedence: the capital; a port within 4
// cells of the sea; a fortress where war came to a town of size 2+; a
// mining camp by rock (60%); the two holiest towns; up to three refuges
// (founded by migration, or high and remote); a market town where three
// roads meet; else a farming village.
pub fn plan(seed: u64, t: &Terrain, w: &Water, _g: &Geology, h: &History) -> Vec<Town> {
    let mut rng = Rng::new(seed, Stream::World(7));
    let mut towns: Vec<Town> = Vec::new();
    // Holiness: priests who lived there, and potent writing made there.
    let holiness = |id: usize| {
        h.people
            .iter()
            .filter(|p| p.settlement == id && p.role == PersonRole::Priest)
            .count()
            + 2 * h
                .events
                .iter()
                .filter(|e| matches!(&e.kind, EventKind::Writing { author, .. } if h.people[*author].settlement == id))
                .count()
    };
    // Only the two holiest places become holy cities.
    let mut ranked: Vec<(usize, usize)> = h
        .settlements
        .iter()
        .filter(|s| !s.capital)
        .map(|s| (holiness(s.id), s.id))
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let holiest: Vec<usize> = ranked
        .iter()
        .take(2)
        .filter(|(n, _)| *n >= 3)
        .map(|(_, id)| *id)
        .collect();
    let mut refuges = 0;
    for s in &h.settlements {
        let (x, y) = (s.cell.ux(), s.cell.uy());
        let wars = h
            .events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::War { settlement, .. } if settlement == s.id))
            .count();
        let migrated = h
            .events
            .iter()
            .any(|e| matches!(e.kind, EventKind::Migration { to, .. } if to == s.id));
        let roads = h
            .roads
            .iter()
            .filter(|r| r.from == s.id || r.to == s.id)
            .count();
        let here = *t.height.get(x, y);
        let mut sum = 0.0;
        let mut n = 0.0;
        for dy in -3i64..=3 {
            for dx in -3i64..=3 {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0 && ny >= 0 && nx < SIZE as i64 && ny < SIZE as i64 {
                    sum += *t.height.get(nx as usize, ny as usize);
                    n += 1.0;
                }
            }
        }
        let rise = here - sum / n;
        let sea = coastal(t, s.cell);
        let river = riverside(t, w, s.cell);
        let role = if s.capital {
            TownRole::Capital
        } else if sea {
            TownRole::Port
        } else if wars > 0 && s.size >= 2 {
            TownRole::Fortress
        } else if rocky(t, s.cell) && s.size <= 2 && rng.chance(60) {
            TownRole::MiningCamp
        } else if holiest.contains(&s.id) {
            TownRole::HolyCity
        } else if refuges < 3
            && s.size <= 2
            && roads <= 2
            && (migrated || (rise > 25.0 && here > 500.0))
        {
            refuges += 1;
            TownRole::Refuge
        } else if roads >= 3 {
            TownRole::MarketTown
        } else if wars > 0 {
            TownRole::Fortress
        } else {
            TownRole::FarmingVillage
        };
        let reason = if sea {
            "harbour"
        } else if river && roads > 0 {
            "river crossing"
        } else if rise > 25.0 {
            "hilltop"
        } else if roads >= 3 {
            "crossroads"
        } else if rise < -20.0 {
            "valley floor"
        } else if *t.moisture.get(x, y) > 0.55 {
            "spring"
        } else {
            "open plain"
        };
        let plans: &[&str] = match reason {
            "harbour" => &["along the shore", "clustered", "grid"],
            "river crossing" => &["strung along the road", "grid", "clustered"],
            "hilltop" => &["ring", "terraced", "clustered"],
            "crossroads" => &["radial", "grid", "strung along the road"],
            "valley floor" => &["strung along the road", "clustered", "terraced"],
            _ => &["grid", "clustered", "radial"],
        };
        let plan = *rng.pick(plans);
        // Districts: the square, then what the role needs.
        let mut kinds: Vec<&'static str> = vec!["square"];
        let extra: &[&str] = match role {
            TownRole::Capital => &[
                "palace quarter",
                "temple quarter",
                "market",
                "workshops",
                "gardens",
                "scholars' quarter",
            ],
            TownRole::Port => &["harbour", "market", "workshops", "temple quarter"],
            TownRole::HolyCity => &[
                "temple quarter",
                "sacred way",
                "scholars' quarter",
                "graves",
            ],
            TownRole::MiningCamp => &["mines", "workshops"],
            TownRole::Fortress => &["garrison", "upper town", "workshops"],
            TownRole::MarketTown => &["market", "workshops", "temple quarter"],
            TownRole::FarmingVillage => &["farmyards"],
            TownRole::Refuge => &["upper town", "farmyards"],
        };
        let room = (usize::from(s.size) + 1).min(extra.len());
        kinds.extend(extra.iter().take(room));
        if wars > 0 && !kinds.contains(&"garrison") && s.size >= 2 {
            kinds.push("garrison");
        }
        if s.size >= 2 && !kinds.contains(&"graves") {
            kinds.push("graves");
        }
        let eras = h
            .eras
            .iter()
            .filter(|e| e.start >= s.founded && s.abandoned.is_none_or(|a| a > e.start))
            .count();
        if eras >= 2 && s.size >= 2 {
            kinds.push("old town");
        }
        let walled = wars > 0;
        let gates = if walled { roads.max(1) } else { 0 };
        let districts = layout(&mut rng, t, s.cell, &kinds, sea);
        let streets = streets(&districts, reason, plan, role, t);
        towns.push(Town {
            settlement: s.id,
            role,
            reason,
            plan,
            districts,
            streets,
            walled,
            gates,
        });
    }
    // No two towns of the same role and size share a layout: change the
    // plan of a later one, or failing that give it an old town.
    for i in 0..towns.len() {
        for attempt in 0..PLANS.len() + 1 {
            let clash = (0..i).any(|j| {
                towns[j].role == towns[i].role
                    && h.settlements[j].size == h.settlements[i].size
                    && towns[j].signature() == towns[i].signature()
            });
            if !clash {
                break;
            }
            if attempt < PLANS.len() {
                // The next plan, but only a harbour lies along the shore.
                let mut at = PLANS.iter().position(|p| *p == towns[i].plan).unwrap_or(0);
                loop {
                    at = (at + 1) % PLANS.len();
                    if PLANS[at] != "along the shore" || towns[i].reason == "harbour" {
                        break;
                    }
                }
                towns[i].plan = PLANS[at];
            } else {
                let c = towns[i].districts[0].cell;
                towns[i].districts.push(District {
                    kind: "old town",
                    cell: c,
                });
            }
        }
    }
    towns
}

/// Where each district lies: the square at the centre, the harbour toward
/// the sea, the mines toward rock, the rest around the square.
fn layout(
    rng: &mut Rng,
    t: &Terrain,
    centre: Cell,
    kinds: &[&'static str],
    _sea: bool,
) -> Vec<District> {
    const RING: [(i64, i64); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    let mut used: Vec<Cell> = vec![centre];
    let mut out = vec![District {
        kind: "square",
        cell: centre,
    }];
    let start = rng.index(8);
    for (n, &kind) in kinds.iter().enumerate().skip(1) {
        // Ground the district wants.
        let want = |x: usize, y: usize| -> i64 {
            let b = *t.biome.get(x, y);
            match kind {
                "harbour" => {
                    i64::from(count_near(x, y, 1, |a, c| *t.biome.get(a, c) == Biome::Sea) as u32)
                        * 10
                }
                "mines" => i64::from(count_near(x, y, 3, |a, c| {
                    matches!(t.biome.get(a, c), Biome::Rock | Biome::Pine | Biome::Tundra)
                }) as u32),
                "upper town" | "palace quarter" | "temple quarter" => {
                    (*t.height.get(x, y) / 10.0) as i64
                }
                "farmyards" | "gardens" => {
                    i64::from(matches!(b, Biome::Grassland | Biome::Forest)) * 5
                }
                _ => 0,
            }
        };
        let mut best: Option<(i64, Cell)> = None;
        for r in 1..=2i64 {
            for i in 0..8 {
                let (dx, dy) = RING[(start + i + n) % 8];
                let (x, y) = (i64::from(centre.x) + dx * r, i64::from(centre.y) + dy * r);
                if x < 1 || y < 1 || x >= SIZE as i64 - 1 || y >= SIZE as i64 - 1 {
                    continue;
                }
                let (x, y) = (x as usize, y as usize);
                let c = Cell::new(x, y);
                if !t.is_land(x, y) || used.contains(&c) {
                    continue;
                }
                let score = want(x, y) * 10 - r;
                if best.is_none_or(|(b, _)| score > b) {
                    best = Some((score, c));
                }
            }
        }
        let cell = best.map_or(centre, |(_, c)| c);
        used.push(cell);
        out.push(District { kind, cell });
    }
    out
}

/// Streets from the square to every district, of a kind the ground and
/// plan suggest.
fn streets(d: &[District], reason: &str, plan: &str, role: TownRole, t: &Terrain) -> Vec<Street> {
    let mut out = Vec::new();
    for (i, dist) in d.iter().enumerate().skip(1) {
        let rise = (*t.height.get(dist.cell.ux(), dist.cell.uy())
            - *t.height.get(d[0].cell.ux(), d[0].cell.uy()))
        .abs();
        let kind = match dist.kind {
            "harbour" => "quay",
            "sacred way" => "colonnade",
            _ if rise > 30.0 || plan == "terraced" => "stair street",
            _ if *t.biome.get(dist.cell.ux(), dist.cell.uy()) == Biome::Marsh => "causeway",
            _ if reason == "river crossing" && i == 1 => "bridge street",
            _ if i == 1 || (role == TownRole::Capital && dist.kind == "palace quarter") => {
                "main street"
            }
            _ => "lane",
        };
        out.push(Street {
            kind,
            from: 0,
            to: i,
        });
    }
    // In a ring or radial town, neighbouring districts are joined too.
    if matches!(plan, "ring" | "radial") && d.len() > 3 {
        for i in 1..d.len() - 1 {
            out.push(Street {
                kind: "lane",
                from: i,
                to: i + 1,
            });
        }
    }
    out
}
