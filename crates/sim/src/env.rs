//! Local properties of a place at a moment: temperature, light, water,
//! wetness, air, stability. Computed from the world, the time, and what the
//! player has changed (`SimState`).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use scraped_world::structures::Condition;
use scraped_world::terrain::Biome;
use scraped_world::water::RIVER_FLOW;
use scraped_world::World;

use crate::fixtures::{Fixtures, MechKind, Spot};
use crate::outdoors::{hash, outdoor_light, weather, Land, Pos, LOCAL};
use crate::region::{season_offset, RegionState, Regions, CLIMATE, LIFE, STABILITY, WATER};
use crate::rules::{ice_cm, Effect, Props, Rules};
use crate::writing::{resolve, Claim, Class, Property};

/// A fire burning somewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fire {
    pub at: Spot,
    /// Minutes of fuel left.
    pub fuel: u32,
}

/// What the player has changed in the physical world.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SimState {
    pub fires: Vec<Fire>,
    /// Mechanisms moved from how they were found (by index): sluices
    /// opened, levers pulled, bridges lowered.
    pub moved: BTreeSet<usize>,
    /// When each flooded room's drain was opened: (flooded index, minute).
    pub drained: Vec<(usize, u32)>,
    /// Barred doors forced open: (structure, link).
    pub unbarred: BTreeSet<(usize, usize)>,
    /// Ways closed by falling stone: (structure, link).
    pub fallen: BTreeSet<(usize, usize)>,
    /// Ways opened by falling stone: (structure, link).
    pub opened: BTreeSet<(usize, usize)>,
    /// Unstable rooms whose stone has already come down.
    pub settled: BTreeSet<(usize, usize)>,
    /// Shelters the player has built.
    pub shelters: Vec<Pos>,
}

/// How long a drained room takes to empty, in minutes.
pub const DRAIN_MINUTES: u32 = 60;

/// Hourly swing of air temperature around the day's mean, from midnight.
// DESIGN-Q: a fixed daily swing of 13 degrees and a day-to-day spell of
// ±3 degrees; no seasons yet.
const DIURNAL: [f64; 24] = [
    -5.0, -6.0, -6.0, -7.0, -7.0, -6.0, -5.0, -3.0, -1.0, 1.0, 3.0, 4.0, 5.0, 6.0, 6.0, 5.0, 4.0,
    2.0, 1.0, -1.0, -2.0, -3.0, -4.0, -4.0,
];

/// The physical world at a moment, for one game.
pub struct Env<'a> {
    pub world: &'a World,
    pub land: &'a Land,
    pub fixtures: &'a Fixtures,
    pub state: &'a SimState,
    /// Debug and tests: fixed weather and light outdoors.
    pub forced: Option<(&'static str, &'static str)>,
    /// Claims of live writing acting now.
    pub claims: &'a [Claim],
    /// The regions and their state, when the regional simulation runs.
    pub regional: Option<(&'a Regions, &'a RegionState)>,
}

/// What a spot is like, in coarse terms for descriptions and the body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Local {
    pub temperature: f64,
    pub weather: &'static str,
    /// "daylight", "dim" or "dark".
    pub light: &'static str,
    /// "dry", "damp", "wet" or "flooded".
    pub wetness: &'static str,
    /// "still", "draughty" or "windy".
    pub air: &'static str,
    pub unstable: bool,
    /// A fire burning here.
    pub fire: bool,
    /// Built or found shelter from wind and rain.
    pub sheltered: bool,
}

impl<'a> Env<'a> {
    pub fn weather(&self, pos: Pos, minutes: u32) -> &'static str {
        match self.forced {
            Some((w, _)) => w,
            None => weather(self.world, pos, minutes),
        }
    }

    pub fn outdoor_light(&self, minutes: u32) -> &'static str {
        match self.forced {
            Some((_, l)) => l,
            None => outdoor_light(minutes),
        }
    }

    /// Air temperature outdoors at a cell.
    pub fn air(&self, x: usize, y: usize, minutes: u32) -> f64 {
        let mean = *self.world.terrain.temperature.get(x, y);
        let hour = ((minutes / 60) % 24) as usize;
        let day = u64::from(minutes / 1440);
        let spell = (hash(&[self.world.seed, 0x7e3d, day]) % 7) as f64 - 3.0;
        let pos = Pos::of_cell(x, y);
        let wet = match self.weather(pos, minutes) {
            "rain" => -2.0,
            "fog" => -1.0,
            _ => 0.0,
        };
        let swing = match self.forced {
            Some(_) => 0.0,
            None => DIURNAL[hour],
        };
        let seasonal = match self.forced {
            Some(_) => 0.0,
            None => season_offset(minutes),
        };
        let regional = self
            .region_var(pos, CLIMATE)
            .map_or(0.0, |c| f64::from(c) / 1000.0);
        mean + swing
            + spell
            + wet
            + seasonal
            + regional
            + f64::from(
                self.claimed(Property::Heat, &[Class::Land], pos)
                    .unwrap_or(0),
            )
    }

    /// A regional variable where a point lies.
    pub fn region_var(&self, p: Pos, v: usize) -> Option<i32> {
        let (regions, st) = self.regional?;
        let r = regions.at(p)?;
        st.vars.get(r).map(|x| x[v])
    }

    /// A regional variable against where it stood when play began (1000:
    /// unchanged): the land as generated already shows that state.
    pub fn region_ratio(&self, p: Pos, v: usize) -> Option<i32> {
        let (regions, st) = self.regional?;
        let r = regions.at(p)?;
        let start = regions.initial.vars.get(r)?[v].max(1);
        // Both floored at 1, so a variable that starts at nothing reads as
        // unchanged until it changes.
        Some(st.vars[r][v].max(1) * 1000 / start)
    }

    /// Coarse regional cues at a point: (life, water, stability, climate).
    pub fn region_bands(&self, p: Pos) -> Option<[&'static str; 4]> {
        let (regions, st) = self.regional?;
        let r = regions.at(p)?;
        let x = st.vars[r];
        Some([
            crate::region::band(LIFE, x[LIFE]),
            crate::region::band(WATER, x[WATER]),
            crate::region::band(STABILITY, x[STABILITY]),
            crate::region::band(CLIMATE, x[CLIMATE]),
        ])
    }

    /// What live writing makes of a property at a point, for claims on
    /// things of the given classes.
    pub fn claimed(&self, property: Property, classes: &[Class], at: Pos) -> Option<i32> {
        if self.claims.is_empty() {
            return None;
        }
        let near: Vec<&Claim> = self
            .claims
            .iter()
            .filter(|c| c.property == property && classes.contains(&c.class))
            .collect();
        resolve(&near, at)
    }

    /// Whether writing holds a building's doors open (+1) or shut (-1).
    pub fn held(&self, structure: usize) -> Option<i32> {
        self.claimed(
            Property::Openness,
            &[Class::Passage],
            self.land.structure_pos[structure],
        )
    }

    /// Heat writing adds in a building's rooms.
    pub fn room_heat(&self, structure: usize) -> i32 {
        self.claimed(
            Property::Heat,
            &[Class::Room, Class::Passage, Class::Structure],
            self.land.structure_pos[structure],
        )
        .unwrap_or(0)
    }

    /// Whether writing makes a building's stone sound (+1) or crumbling (-1).
    pub fn stability(&self, structure: usize) -> Option<i32> {
        self.claimed(
            Property::Stability,
            &[Class::Structure],
            self.land.structure_pos[structure],
        )
    }

    /// The fire burning at a spot (outdoors: within a few hundred metres).
    pub fn fire_at(&self, spot: Spot) -> Option<usize> {
        self.state.fires.iter().position(|f| {
            f.fuel > 0
                && match (f.at, spot) {
                    (Spot::Out { x, y }, Spot::Out { x: px, y: py }) => {
                        Pos::new(x, y).dist(Pos::new(px, py)) <= f64::from(LOCAL) / 3.0
                    }
                    (a, b) => a == b,
                }
        })
    }

    /// Water in a room: 0 dry, up to 100 for a room full to the roof.
    pub fn flood(&self, structure: usize, room: usize, minutes: u32) -> u32 {
        let Some(i) = self
            .fixtures
            .flooded
            .iter()
            .position(|f| f.structure == structure && f.room == room)
        else {
            return 0;
        };
        match self.state.drained.iter().find(|(f, _)| *f == i) {
            None => 100,
            Some((_, at)) => {
                let gone = minutes.saturating_sub(*at).min(DRAIN_MINUTES);
                100 - gone * 100 / DRAIN_MINUTES
            }
        }
    }

    pub fn sluice_open(&self, i: usize) -> bool {
        self.fixtures.mechanisms.iter().enumerate().any(|(m, x)| {
            x.kind == MechKind::Sluice && x.controls == Some(i) && self.state.moved.contains(&m)
        })
    }

    /// River flow at a cell after any open sluices.
    pub fn flow(&self, x: usize, y: usize) -> u32 {
        let f = self
            .fixtures
            .flow(self.world, x, y, &|i| self.sluice_open(i));
        // Rivers rise and shrink with their region's water.
        // DESIGN-Q: river flow scales with the region's water against its
        // natural level, between a fifth and double.
        match self.region_ratio(Pos::of_cell(x, y), WATER) {
            Some(r) => (u64::from(f) * r.clamp(200, 2000) as u64 / 1000) as u32,
            None => f,
        }
    }

    pub fn is_river(&self, x: usize, y: usize) -> bool {
        self.world.terrain.is_land(x, y)
            && !*self.world.water.pooled.get(x, y)
            && self.flow(x, y) >= RIVER_FLOW
    }

    /// Whether a bridge at a cell stands raised.
    pub fn bridge_raised(&self, x: usize, y: usize) -> bool {
        self.world.structures.iter().any(|st| {
            st.cell.ux() == x
                && st.cell.uy() == y
                && self.fixtures.raised.contains(&st.id)
                && !self.fixtures.mechanisms.iter().enumerate().any(|(m, k)| {
                    k.kind == MechKind::BridgeLever
                        && k.controls == Some(st.id)
                        && self.state.moved.contains(&m)
                })
        })
    }

    /// Whether a cell can be walked into without wading, swimming or ice.
    pub fn passable(&self, x: usize, y: usize) -> bool {
        if *self.land.bridges.get(x, y) {
            return !self.bridge_raised(x, y);
        }
        if self.world.terrain.biome.get(x, y).is_water() {
            return false;
        }
        !(self.is_river(x, y) && !*self.world.water.ford.get(x, y))
    }

    /// What bars a cell: "sea", "lake", "river", "bridge" (raised), or None.
    pub fn obstacle(&self, x: usize, y: usize) -> Option<&'static str> {
        if self.passable(x, y) {
            return None;
        }
        Some(match self.world.terrain.biome.get(x, y) {
            Biome::Sea => "sea",
            Biome::Lake => "lake",
            _ if *self.land.bridges.get(x, y) => "bridge",
            _ => "river",
        })
    }

    /// Ice on the water at a cell, in cm, from the last day's air.
    pub fn ice(&self, x: usize, y: usize, minutes: u32) -> f64 {
        let water = match self.world.terrain.biome.get(x, y) {
            Biome::Lake => "still",
            Biome::Sea => return 0.0,
            _ if self.is_river(x, y) => "flowing",
            _ => return 0.0,
        };
        let hourly: Vec<f64> = (0..24u32)
            .rev()
            .map(|h| self.air(x, y, minutes.saturating_sub(h * 60)))
            .collect();
        ice_cm(water, &hourly)
    }

    /// The properties rules see at a spot. `carried_light`: the player
    /// holds a lit torch or lamp; `noise`: how loud the player is being.
    pub fn props(&self, spot: Spot, minutes: u32, carried_light: bool, noise: u8) -> Props {
        let local = self.local(spot, minutes, carried_light);
        let fire = self.fire_at(spot);
        let (water, flow, stability) = match spot {
            Spot::Out { x, y } => {
                let (cx, cy) = Pos::new(x, y).cell();
                let flow = f64::from(self.flow(cx, cy));
                let water = if self.world.terrain.biome.get(cx, cy) == &Biome::Lake {
                    "still"
                } else if self.is_river(cx, cy) {
                    "flowing"
                } else {
                    "none"
                };
                (water, flow, 100.0)
            }
            Spot::Room { structure, room } => {
                let water = if self.flood(structure, room, minutes) > 0 {
                    "still"
                } else {
                    "none"
                };
                let stability = if local.unstable { 20.0 } else { 100.0 };
                (water, 0.0, stability)
            }
        };
        Props {
            temperature: local.temperature,
            water,
            wetness: match local.wetness {
                "flooded" => 100.0,
                "wet" => 70.0,
                "damp" => 40.0,
                _ => 0.0,
            },
            fire: fire.is_some(),
            fuel: fire.map_or(0.0, |i| f64::from(self.state.fires[i].fuel)),
            flow,
            wheel: false,
            light: match local.light {
                "daylight" => 2,
                "dim" => 1,
                _ => 0,
            },
            noise,
            stability,
            creatures: 0,
        }
    }

    /// Whether a wheel at a mechanism turns now.
    pub fn wheel_turns(&self, mechanism: usize) -> bool {
        let m = &self.fixtures.mechanisms[mechanism];
        let Some(s) = m.controls else { return false };
        let cell = self.fixtures.sluices[s].below[0];
        let p = Props {
            flow: f64::from(self.flow(cell.0, cell.1)),
            wheel: true,
            ..Props::default()
        };
        Rules::get().yields(&p, Effect::TurnWheel)
    }

    /// Coarse local conditions at a spot.
    pub fn local(&self, spot: Spot, minutes: u32, carried_light: bool) -> Local {
        let fire = self.fire_at(spot).is_some();
        match spot {
            Spot::Out { x, y } => {
                let p = Pos::new(x, y);
                let (cx, cy) = p.cell();
                let weather = self.weather(p, minutes);
                let sky = self.outdoor_light(minutes);
                let light = if sky == "dark" && (fire || carried_light) {
                    "dim"
                } else {
                    sky
                };
                let sheltered = self.state.shelters.iter().any(|s| s.dist(p) <= 100.0);
                let biome = *self.world.terrain.biome.get(cx, cy);
                let wooded = matches!(biome, Biome::Forest | Biome::Pine);
                let high = self.land.high(p);
                let channel = self
                    .fixtures
                    .channel_water(cx, cy, &|i| self.sluice_open(i))
                    > 0;
                let wetness = if channel || (weather == "rain" && !sheltered) {
                    "wet"
                } else if matches!(biome, Biome::Marsh) || weather == "fog" {
                    "damp"
                } else {
                    "dry"
                };
                Local {
                    temperature: self.air(cx, cy, minutes),
                    weather,
                    light,
                    wetness,
                    air: if sheltered || wooded {
                        "still"
                    } else if high || matches!(biome, Biome::Shore | Biome::Snow | Biome::Tundra) {
                        "windy"
                    } else {
                        "draughty"
                    },
                    unstable: false,
                    fire,
                    sheltered,
                }
            }
            Spot::Room { structure, room } => {
                let st = &self.world.structures[structure];
                let r = &st.interior.rooms[room];
                let (cx, cy) = (st.cell.ux(), st.cell.uy());
                let mean = *self.world.terrain.temperature.get(cx, cy);
                let outside = self.air(cx, cy, minutes);
                // How much of the weather gets in.
                let open = match st.condition {
                    Condition::Intact => 0.3,
                    Condition::Worn => 0.5,
                    Condition::Damaged => 0.8,
                    _ => 1.0,
                };
                // DESIGN-Q: underground rooms sit near the yearly mean less a
                // degree; buildings are 2 degrees warmer than the mean, with
                // part of the outside swing getting in.
                let mut temperature = if r.level < 0 {
                    mean - 1.0
                } else {
                    mean + 2.0 + (outside - mean) * open
                };
                if fire {
                    temperature += 12.0;
                }
                temperature += f64::from(self.room_heat(structure));
                let sky = self.outdoor_light(minutes);
                let base = if r.level < 0 || sky == "dark" {
                    "dark"
                } else if sky == "daylight" && open >= 0.8 {
                    "daylight"
                } else {
                    "dim"
                };
                let light = if (fire && base != "daylight") || (carried_light && base == "dark") {
                    "dim"
                } else {
                    base
                };
                let flood = self.flood(structure, room, minutes);
                let weather = self.weather(Pos::of_cell(cx, cy), minutes);
                let wetness = if flood >= 50 {
                    "flooded"
                } else if flood > 0 {
                    "wet"
                } else if r.level < 0 || (weather == "rain" && open >= 0.8) {
                    "damp"
                } else {
                    "dry"
                };
                Local {
                    temperature,
                    weather,
                    light,
                    wetness,
                    air: if open >= 0.8 { "draughty" } else { "still" },
                    unstable: room > 0
                        && !self.state.settled.contains(&(structure, room))
                        && match self.stability(structure) {
                            Some(s) => s < 0,
                            None => self.fixtures.unstable.contains(&(structure, room)),
                        },
                    fire,
                    sheltered: open < 0.8,
                }
            }
        }
    }
}
