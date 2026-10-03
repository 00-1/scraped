//! The physical game: time passing on the body, fire, light, water and
//! food, mechanisms, creatures, hazards and death.

use scraped_content::{Context, Value};
use scraped_sim::body::{Activity, Exposure};
use scraped_sim::creatures::{self, Event, Senses};
use scraped_sim::env::{Env, Fire};
use scraped_sim::fixtures::{MechKind, Spot};
use scraped_sim::items::{self, CARRY};
use scraped_sim::outdoors::{bearing, distance_band, hash, Pos, BEARINGS, EYE};
use scraped_sim::rules::{Effect, Props, Rules, ICE_BEARS};
use scraped_world::structures::{Passage, PassageState};
use scraped_world::terrain::Biome;
use scraped_world::water::RIVER_FLOW;

use crate::site::{ctx, item_material, label, Place, Thing};
use crate::{Death, Game, Output, Target};

/// How long a chunk of simulated time is, at most, in minutes.
const CHUNK: u32 = 15;
/// How far off a mechanism outdoors can be worked from, in metres.
const REACH: f64 = 400.0;
/// Fuel a fire can hold, in minutes.
const FIRE_MAX: u32 = 600;

impl Game {
    pub(crate) fn env(&self) -> Env<'_> {
        Env {
            world: &self.site.world,
            land: &self.site.land,
            fixtures: &self.site.fixtures,
            state: &self.state.sim,
            forced: self.forced,
            claims: &self.claims,
            regional: Some((&self.site.regions, &self.state.regions)),
        }
    }

    pub(crate) fn spot(&self) -> Spot {
        match self.state.place {
            Place::Outside => Spot::out(self.state.pos),
            Place::Room { structure, room } => Spot::Room { structure, room },
        }
    }

    /// The world's things and those made during play.
    pub(crate) fn thing(&self, id: usize) -> &Thing {
        let n = self.site.things.len();
        if id < n {
            &self.site.things[id]
        } else {
            &self.extra[id - n]
        }
    }

    pub(crate) fn thing_count(&self) -> usize {
        self.site.things.len() + self.extra.len()
    }

    /// Makes a new item where the player is (or in their hands).
    pub(crate) fn make_item(&mut self, kind: &str, carry: bool) -> Option<usize> {
        let k = items::kind(kind)?;
        let id = self.thing_count();
        self.state.made.push(k.id.to_string());
        self.extra.push(Thing {
            id,
            kind: k.id,
            material: item_material(k.id),
            home: self.state.place,
            portable: true,
            texts: Vec::new(),
            pos: self.state.pos,
            surface: None,
            object: None,
        });
        if carry {
            self.state.carried.push(id);
        } else if self.state.place == Place::Outside {
            self.state.dropped.insert(id, self.state.pos);
        }
        Some(id)
    }

    /// Rebuilds things made during play from the state (after cloning a
    /// state in tests, or replay).
    pub(crate) fn sync_made(&mut self) {
        while self.extra.len() < self.state.made.len() {
            let kind = self.state.made[self.extra.len()].clone();
            let id = self.thing_count();
            let k = items::kind(&kind).expect("made items are known kinds");
            self.extra.push(Thing {
                id,
                kind: k.id,
                material: item_material(k.id),
                home: Place::Outside,
                portable: true,
                texts: Vec::new(),
                pos: self.state.pos,
                surface: None,
                object: None,
            });
        }
        self.extra.truncate(self.state.made.len());
    }

    fn kind_of(&self, t: usize) -> &'static str {
        self.thing(t).kind
    }

    fn carrying(&self, kind: &str) -> Option<usize> {
        self.state
            .carried
            .iter()
            .copied()
            .find(|&t| self.kind_of(t) == kind && !self.state.gone.contains(&t))
    }

    pub(crate) fn carried_light(&self) -> bool {
        self.forced_light
            || self
                .state
                .lit
                .iter()
                .any(|t| self.state.carried.contains(t))
    }

    /// Whether it is too dark here to see.
    pub(crate) fn is_dark(&self) -> bool {
        let local = self
            .env()
            .local(self.spot(), self.state.minutes, self.carried_light());
        local.light == "dark"
    }

    pub(crate) fn weight(&self) -> u32 {
        self.state
            .carried
            .iter()
            .map(|&t| {
                let kind = self.kind_of(t);
                items::kind(kind).map_or_else(
                    || scraped_world::objects::kind(kind).map_or(1, |k| k.weight),
                    |k| k.weight,
                )
            })
            .sum()
    }

    fn clothing(&self) -> i32 {
        self.state
            .worn
            .iter()
            .filter(|t| self.state.carried.contains(t))
            .map(|&t| items::kind(self.kind_of(t)).map_or(0, |k| k.warmth))
            .sum()
    }

    pub(crate) fn fire_here(&self) -> Option<usize> {
        self.env().fire_at(self.spot())
    }

    /// Lets time pass on the body and the world. Messages for the player
    /// gather in `notes`; anything that should stop a journey sets
    /// `interrupted`.
    pub(crate) fn advance(&mut self, minutes: u32, activity: Activity) {
        let mut left = minutes;
        while left > 0 && self.state.dead.is_none() {
            let dt = left.min(CHUNK);
            let spot = self.spot();
            let light = self.carried_light();
            let local = self.env().local(spot, self.state.minutes, light);
            let outdoors = matches!(spot, Spot::Out { .. });
            let exposure = Exposure {
                temperature: local.temperature,
                clothing: self.clothing(),
                fire: local.fire,
                sheltered: local.sheltered,
                raining: outdoors && local.weather == "rain",
                windy: local.air == "windy",
                in_water: local.wetness == "flooded",
                activity,
            };
            self.state.body.age = self.age();
            if self.sustain {
                // A measuring bot's body is kept well (never in play).
                let b = &mut self.state.body;
                (b.thirst, b.hunger, b.fatigue, b.chill, b.wet, b.injury) = (0, 0, 0, 0, 0, 0);
            } else if let Some(cause) = self.state.body.pass(dt, &exposure) {
                self.die(cause);
            }
            // Fires burn their fuel, by the rules.
            let burn = Rules::get().rate(
                &Props {
                    fire: true,
                    fuel: 1.0,
                    ..Props::default()
                },
                Effect::ConsumeFuel,
            );
            let used = (f64::from(dt) * burn / 60.0) as u32;
            let mut out = Vec::new();
            for (i, f) in self.state.sim.fires.iter_mut().enumerate() {
                f.fuel = f.fuel.saturating_sub(used);
                if f.fuel == 0 {
                    out.push(i);
                }
            }
            for i in out.into_iter().rev() {
                let f = self.state.sim.fires.remove(i);
                if self.env().fire_at(spot).is_none() && fire_near(f.at, spot) {
                    let where_ = self.fire_kind(f.at);
                    let t = self.say("fire.out", ctx(&[("where", Value::from(where_))]));
                    self.notes.push(t);
                }
            }
            // Torches and lamps burn down.
            let lit: Vec<usize> = self.state.lit.iter().copied().collect();
            for t in lit {
                let kind = self.kind_of(t);
                let fuel = self
                    .state
                    .fuel
                    .entry(t)
                    .or_insert(items::kind(kind).map_or(0, |k| k.burns));
                *fuel = fuel.saturating_sub(dt);
                if *fuel == 0 {
                    self.state.lit.remove(&t);
                    if kind == "torch" {
                        self.state.gone.insert(t);
                        self.state.carried.retain(|&c| c != t);
                    }
                    if self.state.carried.contains(&t) || kind == "torch" {
                        let tx = self.say("item.out", ctx(&[("kind", Value::from(kind))]));
                        self.notes.push(tx);
                    }
                }
            }
            self.creatures_tick(dt);
            self.state.minutes += dt;
            self.step_regions();
            self.check_time_endings();
            left -= dt;
            if activity != Activity::Sleeping
                && self.state.body.collapses()
                && self.state.dead.is_none()
            {
                let t = self.say("body.collapse", Context::new());
                self.notes.push(t);
                self.interrupted = true;
                self.advance(8 * 60, Activity::Sleeping);
            }
        }
        self.announce_body();
    }

    /// Tells the player when a need changes state.
    fn announce_body(&mut self) {
        let states = self.state.body.states();
        let now: Vec<usize> = vec![
            self.state.body.warmth_state(),
            self.state.body.thirst_state(),
            self.state.body.hunger_state(),
            self.state.body.rest_state(),
            self.state.body.injury_state(),
            self.state.body.wet_state(),
        ];
        if self.state.felt.len() != now.len() {
            self.state.felt = vec![0; now.len()];
        }
        for (i, (&n, &(need, state))) in now.iter().zip(states.iter()).enumerate() {
            let before = self.state.felt[i];
            if n != before && self.state.dead.is_none() {
                let c = ctx(&[
                    ("need", Value::from(need)),
                    ("state", Value::from(state)),
                    ("worse", Value::Bool(n > before)),
                ]);
                let t = self.say("body.change", c);
                self.notes.push(t);
            }
        }
        self.state.felt = now;
    }

    pub(crate) fn die(&mut self, cause: &str) {
        if self.state.dead.is_some() || (self.sustain && cause != "old_age") {
            return;
        }
        let doing = self.log.last().cloned().unwrap_or_default();
        let indoors = self.state.place != Place::Outside;
        self.state.dead = Some(Death {
            cause: cause.to_string(),
            doing: doing.clone(),
            minutes: self.state.minutes,
            place: match self.state.place {
                Place::Outside => format!("outside {} {}", self.state.pos.x, self.state.pos.y), // DEBUG-TEXT: machine-readable
                Place::Room { structure, room } => format!("structure {structure} room {room}"), // DEBUG-TEXT: machine-readable
            },
        });
        let c = ctx(&[
            ("cause", Value::from(cause)),
            ("doing", Value::from(doing)),
            (
                "day",
                Value::Number(i64::from(self.state.minutes / 1440) + 1),
            ),
            ("indoors", Value::Bool(indoors)),
        ]);
        let t = self.say("death.narrate", c);
        self.notes.push(t);
        self.interrupted = true;
    }

    /// Injury from something: a fall, falling stone, a creature.
    pub(crate) fn hurt(&mut self, levels: u32, cause: &str) {
        if self.sustain {
            return;
        }
        if self.state.body.hurt(levels).is_some() {
            self.die(cause);
        }
    }

    fn creatures_tick(&mut self, dt: u32) {
        let spot = self.spot();
        let player = self.state.pos;
        let light = self.carried_light() || self.fire_here().is_some();
        let night = self.env().outdoor_light(self.state.minutes) == "dark";
        let range = self.conditions().2;
        let food = self
            .state
            .carried
            .iter()
            .any(|&t| matches!(self.kind_of(t), "provisions" | "berries"));
        let n = self.site.fixtures.creatures.len();
        if self.state.creatures.len() != n {
            self.state.creatures = self
                .site
                .fixtures
                .creatures
                .iter()
                .map(creatures::Creature::new)
                .collect();
        }
        let mut events = Vec::new();
        for i in 0..n {
            let home = &self.site.fixtures.creatures[i];
            let near = match home.home {
                Spot::Room { .. } => home.home == spot,
                Spot::Out { .. } => self.state.creatures[i].pos.dist(player) <= 3000.0,
            };
            if !near && self.state.creatures[i].mode == creatures::Mode::Wander {
                continue;
            }
            let land = &self.site.land;
            let world = &self.site.world;
            let outdoors = matches!(spot, Spot::Out { .. });
            let visible = |p: Pos| outdoors && land.sight(world, player, EYE, p, 1.0, range);
            let senses = Senses {
                seed: self.site.world.seed,
                id: i,
                minutes: self.state.minutes,
                dt,
                player,
                player_spot: spot,
                night,
                light,
                noise: self.state.noise,
                carrying_food: food,
                visible: &visible,
            };
            let mut c = self.state.creatures[i].clone();
            if let Some(e) = creatures::update(&mut c, home, &senses) {
                events.push((i, e, c.pos));
            }
            self.state.creatures[i] = c;
        }
        for (i, e, at) in events {
            let name = self.creature_name(i);
            let arch = self.site.fixtures.creatures[i].archetype;
            match e {
                Event::Sighted => {
                    let c = ctx(&[
                        ("name", Value::from(name)),
                        ("archetype", Value::from(arch)),
                        (
                            "bearing",
                            Value::from(bearing(player, at).map_or("north", |b| BEARINGS[b])),
                        ),
                        ("distance", Value::from(distance_band(player.dist(at)))),
                    ]);
                    let t = self.say("creature.sighted", c);
                    self.notes.push(t);
                    self.interrupted = true;
                }
                Event::Struck { harm, .. } => {
                    let c = ctx(&[
                        ("name", Value::from(name)),
                        ("archetype", Value::from(arch)),
                        ("harm", Value::Number(i64::from(harm))),
                    ]);
                    let t = self.say("creature.struck", c);
                    self.notes.push(t);
                    self.interrupted = true;
                    self.hurt(harm, "creature");
                }
                Event::Stole => {
                    if let Some(f) = self
                        .carrying("provisions")
                        .or_else(|| self.carrying("berries"))
                    {
                        let thing = self.thing_name(f);
                        self.state.carried.retain(|&c| c != f);
                        self.state.gone.insert(f);
                        let c = ctx(&[("name", Value::from(name)), ("thing", Value::from(thing))]);
                        let t = self.say("creature.stole", c);
                        self.notes.push(t);
                        self.interrupted = true;
                    }
                }
                Event::Fled => {
                    let t = self.say("creature.fled", ctx(&[("name", Value::from(name))]));
                    self.notes.push(t);
                }
            }
        }
    }

    pub(crate) fn creature_name(&mut self, i: usize) -> String {
        let s = &self.site.fixtures.creatures[i];
        let biome = match s.home {
            Spot::Room { .. } => "underground".to_string(),
            Spot::Out { .. } => self.site.biome_at(s.pos),
        };
        let c = ctx(&[
            ("archetype", Value::from(s.archetype)),
            ("biome", Value::from(biome)),
        ]);
        self.stable("creature.name", c, 6_000_000 + i as u64)
    }

    /// Creatures the player can see from here.
    pub(crate) fn creatures_in_view(&self) -> Vec<usize> {
        let spot = self.spot();
        let range = self.conditions().2;
        (0..self.state.creatures.len())
            .filter(|&i| {
                let home = &self.site.fixtures.creatures[i];
                let c = &self.state.creatures[i];
                match (home.home, spot) {
                    (Spot::Room { .. }, _) => false,
                    (_, Spot::Out { .. }) => {
                        c.pos.dist(self.state.pos) <= 2000.0
                            && self.site.land.sight(
                                &self.site.world,
                                self.state.pos,
                                EYE,
                                c.pos,
                                1.0,
                                range,
                            )
                    }
                    _ => false,
                }
            })
            .collect()
    }

    // ---------- mechanisms ----------

    /// Mechanisms the player can reach from here.
    pub(crate) fn mechanisms_here(&self) -> Vec<usize> {
        let spot = self.spot();
        let dark = self.is_dark();
        self.site
            .fixtures
            .mechanisms
            .iter()
            .enumerate()
            .filter(|(_, m)| match (m.at, spot) {
                (Spot::Room { .. }, Spot::Room { .. }) => m.at == spot && !dark,
                (Spot::Out { .. }, Spot::Out { .. }) => m.pos.dist(self.state.pos) <= REACH,
                _ => false,
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub(crate) fn mech_name(&mut self, i: usize) -> String {
        let kind = label(&self.site.fixtures.mechanisms[i].kind);
        self.stable(
            "mech.name",
            ctx(&[("kind", Value::from(kind))]),
            5_000_000 + i as u64,
        )
    }

    fn mech_state(&self, i: usize) -> &'static str {
        let m = &self.site.fixtures.mechanisms[i];
        let moved = self.state.sim.moved.contains(&i);
        match m.kind {
            MechKind::Well => {
                if m.works {
                    "water"
                } else {
                    "dry"
                }
            }
            MechKind::Sluice => {
                if moved {
                    "open"
                } else {
                    "shut"
                }
            }
            MechKind::Wheel => {
                if self.env().wheel_turns(i) {
                    "turning"
                } else {
                    "still"
                }
            }
            MechKind::DrainLever => {
                if moved {
                    "pulled"
                } else {
                    "up"
                }
            }
            MechKind::BridgeLever => {
                if moved {
                    "lowered"
                } else {
                    "raised"
                }
            }
            MechKind::Brazier => {
                if self.env().fire_at(m.at).is_some() {
                    "lit"
                } else {
                    "cold"
                }
            }
        }
    }

    pub(crate) fn examine_mech(&mut self, i: usize) -> Output {
        self.advance(1, Activity::Resting);
        let c = ctx(&[
            (
                "kind",
                Value::from(label(&self.site.fixtures.mechanisms[i].kind)),
            ),
            ("state", Value::from(self.mech_state(i))),
        ]);
        let t = self.say("mech.examine", c);
        self.output(vec![t], None)
    }

    /// Pulls, pushes, opens or closes a mechanism.
    pub(crate) fn operate(&mut self, i: usize, verb: &str) -> Output {
        let kind = self.site.fixtures.mechanisms[i].kind;
        let moved = self.state.sim.moved.contains(&i);
        let wants = match verb {
            "open" => Some(true),
            "close" => Some(false),
            _ => None,
        };
        match kind {
            MechKind::Sluice | MechKind::BridgeLever => {
                let to = wants.unwrap_or(!moved);
                if to == moved {
                    return self.mech_reply("mech.already", i);
                }
                self.advance(10, Activity::Resting);
                if to {
                    self.state.sim.moved.insert(i);
                } else {
                    self.state.sim.moved.remove(&i);
                }
                self.mech_reply("mech.operate", i)
            }
            MechKind::DrainLever => {
                if moved {
                    return self.mech_reply("mech.already", i);
                }
                self.advance(2, Activity::Resting);
                self.state.sim.moved.insert(i);
                let flooded = self.site.fixtures.mechanisms[i]
                    .controls
                    .expect("drains a room");
                self.state.sim.drained.push((flooded, self.state.minutes));
                self.mech_reply("mech.operate", i)
            }
            MechKind::Well => self.drink(&[]),
            MechKind::Brazier => {
                if verb == "close" {
                    self.extinguish_fire()
                } else {
                    self.light_fire()
                }
            }
            MechKind::Wheel => self.mech_reply("mech.cannot", i),
        }
    }

    fn mech_reply(&mut self, slot: &str, i: usize) -> Output {
        let c = ctx(&[
            (
                "kind",
                Value::from(label(&self.site.fixtures.mechanisms[i].kind)),
            ),
            ("state", Value::from(self.mech_state(i))),
        ]);
        let t = self.say(slot, c);
        self.output(vec![t], None)
    }

    // ---------- fire and light ----------

    fn fire_kind(&self, at: Spot) -> &'static str {
        match at {
            Spot::Out { .. } => "campfire",
            Spot::Room { .. } => {
                if self
                    .site
                    .fixtures
                    .mechanisms
                    .iter()
                    .any(|m| m.kind == MechKind::Brazier && m.at == at)
                {
                    "brazier"
                } else {
                    "hearth"
                }
            }
        }
    }

    pub(crate) fn fire_name(&mut self) -> String {
        let at = self.spot();
        let kind = self.fire_kind(at);
        self.stable("fire.name", ctx(&[("where", Value::from(kind))]), 7_000_001)
    }

    fn has_flame(&self) -> bool {
        self.carrying("firesteel").is_some() || self.carried_light() || self.fire_here().is_some()
    }

    fn fire_fail(&mut self, reason: &str) -> Output {
        let t = self.say("fire.fail", ctx(&[("reason", Value::from(reason))]));
        self.output(vec![t], None)
    }

    /// Lights a fire here: outdoors anywhere, indoors at a hearth or brazier.
    pub(crate) fn light_fire(&mut self) -> Output {
        let spot = self.spot();
        if let Some(i) = self.fire_here() {
            let _ = i;
            return self.fire_fail("burning");
        }
        let place_ok = match spot {
            Spot::Out { .. } => true,
            Spot::Room { .. } => {
                self.here().iter().any(|&t| self.kind_of(t) == "hearth")
                    || self
                        .site
                        .fixtures
                        .mechanisms
                        .iter()
                        .any(|m| m.kind == MechKind::Brazier && m.at == spot)
            }
        };
        if !place_ok {
            return self.fire_fail("no_place");
        }
        if !self.has_flame() {
            return self.fire_fail("no_flame");
        }
        let Some(wood) = self.carrying("wood") else {
            return self.fire_fail("no_fuel");
        };
        let local = self.env().local(spot, self.state.minutes, false);
        if matches!(spot, Spot::Out { .. }) && local.weather == "rain" && !local.sheltered {
            self.advance(5, Activity::Resting);
            return self.fire_fail("rain");
        }
        // Wood is as wet as the one carrying it.
        let p = Props {
            wetness: f64::from(self.state.body.wet),
            ..Props::default()
        };
        if Rules::get().yields(&p, Effect::ResistIgnition) {
            self.advance(5, Activity::Resting);
            return self.fire_fail("wet");
        }
        self.advance(10, Activity::Resting);
        self.state.carried.retain(|&c| c != wood);
        self.state.gone.insert(wood);
        let fuel = items::kind("wood").map_or(120, |k| k.burns);
        self.state.sim.fires.push(Fire { at: spot, fuel });
        let where_ = self.fire_kind(spot);
        let t = self.say(
            "fire.lit",
            ctx(&[
                ("where", Value::from(where_)),
                ("fuel", Value::Number(i64::from(fuel))),
            ]),
        );
        self.output(vec![t], None)
    }

    pub(crate) fn feed_fire(&mut self) -> Output {
        let Some(i) = self.fire_here() else {
            return self.fire_fail("no_fire");
        };
        let Some(wood) = self.carrying("wood") else {
            return self.fire_fail("no_fuel");
        };
        self.advance(2, Activity::Resting);
        self.state.carried.retain(|&c| c != wood);
        self.state.gone.insert(wood);
        let add = items::kind("wood").map_or(120, |k| k.burns);
        let f = &mut self.state.sim.fires[i];
        f.fuel = (f.fuel + add).min(FIRE_MAX);
        let fuel = f.fuel;
        let t = self.say("fire.fed", ctx(&[("fuel", Value::Number(i64::from(fuel)))]));
        self.output(vec![t], None)
    }

    pub(crate) fn extinguish_fire(&mut self) -> Output {
        let Some(i) = self.fire_here() else {
            return self.fire_fail("no_fire");
        };
        self.advance(2, Activity::Resting);
        let f = self.state.sim.fires.remove(i);
        let where_ = self.fire_kind(f.at);
        let t = self.say("fire.doused", ctx(&[("where", Value::from(where_))]));
        self.output(vec![t], None)
    }

    /// Lights a torch or lamp.
    pub(crate) fn light_item(&mut self, t: usize) -> Output {
        let kind = self.kind_of(t);
        let named = ctx(&[("kind", Value::from(kind))]);
        if !items::kind(kind).is_some_and(|k| k.light) {
            return self.fire_fail("not_lightable");
        }
        if self.state.lit.contains(&t) {
            let tx = self.say("item.lit", named);
            return self.output(vec![tx], None);
        }
        if !self.has_flame() {
            return self.fire_fail("no_flame");
        }
        let fuel = *self
            .state
            .fuel
            .entry(t)
            .or_insert(items::kind(kind).map_or(0, |k| k.burns));
        if fuel == 0 {
            return self.fire_fail("no_fuel");
        }
        self.advance(2, Activity::Resting);
        self.state.lit.insert(t);
        let tx = self.say("item.lit", named);
        self.output(vec![tx], None)
    }

    pub(crate) fn douse_item(&mut self, t: usize) -> Output {
        let kind = self.kind_of(t);
        if !self.state.lit.remove(&t) {
            return self.fire_fail("not_lit");
        }
        let tx = self.say("item.doused", ctx(&[("kind", Value::from(kind))]));
        self.output(vec![tx], None)
    }

    /// `light X`: a torch or lamp, a brazier, or a fire.
    pub(crate) fn light(&mut self, words: &[String]) -> Output {
        if words.is_empty()
            || words
                .iter()
                .any(|w| matches!(w.as_str(), "fire" | "campfire" | "hearth"))
        {
            return self.light_fire();
        }
        self.with_target("light", words)
    }

    pub(crate) fn extinguish(&mut self, words: &[String]) -> Output {
        if words
            .iter()
            .any(|w| matches!(w.as_str(), "fire" | "campfire" | "hearth"))
        {
            return self.extinguish_fire();
        }
        self.with_target("extinguish", words)
    }

    // ---------- water, food, rest ----------

    /// Where water can be had here: "well", "river", "stream", "lake", or None.
    fn water_source(&self) -> Option<&'static str> {
        for i in self.mechanisms_here() {
            let m = &self.site.fixtures.mechanisms[i];
            if m.kind == MechKind::Well && m.works {
                return Some("well");
            }
        }
        if let Spot::Room { structure, room } = self.spot() {
            return (self.env().flood(structure, room, self.state.minutes) > 0).then_some("flood");
        }
        let near = self.site.land.edges_near(self.state.pos);
        for (e, id) in [(0, "river"), (1, "stream"), (4, "lake")] {
            if near.iter().any(|(k, _)| *k == e) {
                return Some(id);
            }
        }
        None
    }

    pub(crate) fn drink(&mut self, words: &[String]) -> Output {
        let container = self
            .state
            .carried
            .iter()
            .copied()
            .find(|t| self.state.water.get(t).is_some_and(|&w| w > 0));
        let from_container = words
            .iter()
            .any(|w| matches!(w.as_str(), "skin" | "waterskin" | "jar"));
        match (self.water_source(), container) {
            (Some(src), _) if !from_container => {
                self.advance(5, Activity::Resting);
                self.state.body.thirst = 0;
                let t = self.say(
                    "drink.done",
                    ctx(&[("source", Value::from(src)), ("full", Value::Bool(true))]),
                );
                self.output(vec![t], None)
            }
            (_, Some(c)) => {
                self.advance(2, Activity::Resting);
                *self.state.water.get_mut(&c).expect("has water") -= 1;
                self.state.body.drink(8 * 60);
                let t = self.say(
                    "drink.done",
                    ctx(&[
                        ("source", Value::from("container")),
                        ("full", Value::Bool(false)),
                    ]),
                );
                self.output(vec![t], None)
            }
            _ => {
                let t = self.say("drink.none", Context::new());
                self.output(vec![t], None)
            }
        }
    }

    pub(crate) fn fill(&mut self, t: usize) -> Output {
        let named = self.thing_name(t);
        let holds = items::kind(self.kind_of(t)).map_or(0, |k| k.holds);
        if holds == 0 || !self.state.carried.contains(&t) {
            let tx = self.say(
                "fill.none",
                ctx(&[
                    ("thing", Value::from(named)),
                    ("reason", Value::from("container")),
                ]),
            );
            return self.output(vec![tx], None);
        }
        if self.water_source().is_none() {
            let tx = self.say(
                "fill.none",
                ctx(&[
                    ("thing", Value::from(named)),
                    ("reason", Value::from("water")),
                ]),
            );
            return self.output(vec![tx], None);
        }
        self.advance(3, Activity::Resting);
        self.state.water.insert(t, holds);
        let tx = self.say(
            "fill.done",
            ctx(&[
                ("thing", Value::from(named)),
                ("drinks", Value::Number(i64::from(holds))),
            ]),
        );
        self.output(vec![tx], None)
    }

    pub(crate) fn eat(&mut self, words: &[String]) -> Output {
        if !words.is_empty() {
            return self.with_target("eat", words);
        }
        match self
            .carrying("provisions")
            .or_else(|| self.carrying("berries"))
        {
            Some(t) => self.eat_thing(t),
            None => {
                let t = self.say("eat.none", Context::new());
                self.output(vec![t], None)
            }
        }
    }

    pub(crate) fn eat_thing(&mut self, t: usize) -> Output {
        let meal = items::kind(self.kind_of(t)).map_or(0, |k| k.meal);
        let named = self.thing_name(t);
        if meal == 0 {
            let tx = self.say("eat.none", Context::new());
            return self.output(vec![tx], None);
        }
        self.advance(10, Activity::Resting);
        self.state.carried.retain(|&c| c != t);
        self.state.moved.remove(&t);
        self.state.gone.insert(t);
        self.state.body.eat(meal);
        let tx = self.say(
            "eat.done",
            ctx(&[
                ("thing", Value::from(named)),
                ("hours", Value::Number(i64::from(meal))),
            ]),
        );
        self.output(vec![tx], None)
    }

    /// Sleeps until rested (at most ten hours), unless woken.
    pub(crate) fn sleep(&mut self) -> Output {
        let hours = self.state.body.fatigue.div_ceil(180).clamp(1, 10);
        self.interrupted = false;
        let mut slept = 0;
        while slept < hours && !self.interrupted && self.state.dead.is_none() {
            self.advance(60, Activity::Sleeping);
            slept += 1;
        }
        let c = ctx(&[
            ("hours", Value::Number(i64::from(slept))),
            ("woken", Value::Bool(slept < hours)),
        ]);
        let t = if self.state.dead.is_some() {
            String::new()
        } else {
            self.say("sleep.done", c)
        };
        self.output(vec![t], None)
    }

    pub(crate) fn biome_here(&self) -> Biome {
        let (x, y) = self.state.pos.cell();
        *self.site.world.terrain.biome.get(x, y)
    }

    /// An hour searching for food.
    // DESIGN-Q: foraging odds by biome (forest 60%, shore 50%, grassland
    // 45%, marsh 40%, scrub 35%, pine 30%, tundra 15%, desert and rock 10%).
    pub(crate) fn forage(&mut self) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("forage");
        }
        let chance = match self.biome_here() {
            Biome::Forest => 60,
            Biome::Shore => 50,
            Biome::Grassland => 45,
            Biome::Marsh => 40,
            Biome::Scrub => 35,
            Biome::Pine => 30,
            Biome::Tundra => 15,
            Biome::Desert | Biome::Rock => 10,
            _ => 0,
        };
        let roll = hash(&[
            self.seed(),
            0xf0a6,
            u64::from(self.state.minutes),
            self.state.pos.x as u64,
        ]) % 100;
        // Less to find where the region's life has fallen.
        let life = self
            .env()
            .region_ratio(self.state.pos, scraped_sim::region::LIFE)
            .unwrap_or(1000)
            .clamp(0, 1500);
        let chance = chance * life as u64 / 1000;
        self.advance(60, Activity::Walking);
        let biome = label(&self.biome_here());
        if roll < chance && self.state.dead.is_none() {
            self.make_item("berries", true);
            let t = self.say(
                "forage.found",
                ctx(&[
                    ("kind", Value::from("berries")),
                    ("biome", Value::from(biome)),
                ]),
            );
            self.output(vec![t], None)
        } else {
            let t = self.say("forage.none", ctx(&[("biome", Value::from(biome))]));
            self.output(vec![t], None)
        }
    }

    pub(crate) fn wooded(&self) -> bool {
        matches!(
            self.biome_here(),
            Biome::Forest | Biome::Pine | Biome::Scrub | Biome::Shore
        )
    }

    pub(crate) fn gather(&mut self) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("gather");
        }
        let biome = label(&self.biome_here());
        if !self.wooded() {
            let t = self.say("gather.none", ctx(&[("biome", Value::from(biome))]));
            return self.output(vec![t], None);
        }
        if self.weight() + 3 > CARRY {
            return self.too_heavy("wood");
        }
        self.advance(30, Activity::Walking);
        self.make_item("wood", true);
        let t = self.say("gather.found", ctx(&[("biome", Value::from(biome))]));
        self.output(vec![t], None)
    }

    fn outdoors_needed(&mut self, verb: &str) -> Output {
        let t = self.say("travel.indoors", ctx(&[("verb", Value::from(verb))]));
        self.output(vec![t], None)
    }

    fn too_heavy(&mut self, thing: &str) -> Output {
        let c = ctx(&[
            ("thing", Value::from(thing)),
            ("weight", Value::Number(i64::from(self.weight()))),
            ("limit", Value::Number(i64::from(CARRY))),
        ]);
        let t = self.say("item.too_heavy", c);
        self.output(vec![t], None)
    }

    /// Whether picking up a thing would be too much to carry.
    pub(crate) fn overloaded_by(&self, t: usize) -> bool {
        self.weight() + items::kind(self.kind_of(t)).map_or(1, |k| k.weight) > CARRY
    }

    pub(crate) fn too_heavy_thing(&mut self, t: usize) -> Output {
        let name = self.thing_name(t);
        self.too_heavy(&name)
    }

    /// `make torch`, `make fire`, `make shelter`.
    pub(crate) fn make(&mut self, words: &[String]) -> Output {
        let what = words
            .iter()
            .find(|w| matches!(w.as_str(), "torch" | "fire" | "campfire" | "shelter"));
        match what.map(String::as_str) {
            Some("fire") | Some("campfire") => self.light_fire(),
            Some("torch") => {
                let Some(wood) = self.carrying("wood") else {
                    return self.make_fail("torch", "no_wood");
                };
                self.advance(15, Activity::Resting);
                self.state.carried.retain(|&c| c != wood);
                self.state.gone.insert(wood);
                self.make_item("torch", true);
                let t = self.say("item.made", ctx(&[("kind", Value::from("torch"))]));
                self.output(vec![t], None)
            }
            Some("shelter") => {
                if self.state.place != Place::Outside {
                    return self.make_fail("shelter", "indoors");
                }
                let wood = self.carrying("wood");
                if !self.wooded() && wood.is_none() {
                    return self.make_fail("shelter", "no_wood");
                }
                if let (false, Some(w)) = (self.wooded(), wood) {
                    self.state.carried.retain(|&c| c != w);
                    self.state.gone.insert(w);
                }
                self.advance(60, Activity::Walking);
                self.state.sim.shelters.push(self.state.pos);
                let t = self.say("item.made", ctx(&[("kind", Value::from("shelter"))]));
                self.output(vec![t], None)
            }
            _ => self.make_fail("unknown", "unknown"),
        }
    }

    fn make_fail(&mut self, what: &str, reason: &str) -> Output {
        let t = self.say(
            "item.make_fail",
            ctx(&[("what", Value::from(what)), ("reason", Value::from(reason))]),
        );
        self.output(vec![t], None)
    }

    pub(crate) fn wear(&mut self, t: usize, on: bool) -> Output {
        let named = self.thing_name(t);
        let warm = items::kind(self.kind_of(t)).is_some_and(|k| k.warmth > 0);
        if !warm || !self.state.carried.contains(&t) {
            let tx = self.say("item.cannot_use", ctx(&[("thing", Value::from(named))]));
            return self.output(vec![tx], None);
        }
        self.advance(1, Activity::Resting);
        if on {
            self.state.worn.insert(t);
        } else {
            self.state.worn.remove(&t);
        }
        let slot = if on { "item.wear" } else { "item.remove" };
        let tx = self.say(slot, ctx(&[("thing", Value::from(named))]));
        self.output(vec![tx], None)
    }

    /// `use X`: whatever the thing is for.
    pub(crate) fn use_thing(&mut self, t: usize) -> Output {
        let kind = self.kind_of(t);
        match kind {
            "torch" | "lamp" => self.light_item(t),
            "provisions" | "berries" => self.eat_thing(t),
            "waterskin" | "jar" => {
                if self.state.water.get(&t).is_some_and(|&w| w > 0) {
                    self.drink(&["skin".to_string()])
                } else {
                    self.fill(t)
                }
            }
            "cloak" => self.wear(t, true),
            "firesteel" | "wood" => self.light_fire(),
            "oil" => {
                let Some(lamp) = self.carrying("lamp") else {
                    let named = self.thing_name(t);
                    let tx = self.say("item.cannot_use", ctx(&[("thing", Value::from(named))]));
                    return self.output(vec![tx], None);
                };
                self.advance(2, Activity::Resting);
                self.state.carried.retain(|&c| c != t);
                self.state.gone.insert(t);
                self.state
                    .fuel
                    .insert(lamp, items::kind("lamp").map_or(240, |k| k.burns));
                let tx = self.say("item.made", ctx(&[("kind", Value::from("lamp_filled"))]));
                self.output(vec![tx], None)
            }
            _ => {
                let named = self.thing_name(t);
                let tx = self.say("item.cannot_use", ctx(&[("thing", Value::from(named))]));
                self.output(vec![tx], None)
            }
        }
    }

    pub(crate) fn examine_item(&mut self, t: usize) -> Output {
        self.advance(1, Activity::Resting);
        let kind = self.kind_of(t);
        let k = items::kind(kind);
        let c = ctx(&[
            ("kind", Value::from(kind)),
            ("lit", Value::Bool(self.state.lit.contains(&t))),
            (
                "fuel",
                Value::Number(i64::from(
                    *self.state.fuel.get(&t).unwrap_or(&k.map_or(0, |k| k.burns)),
                )),
            ),
            (
                "water",
                Value::Number(i64::from(*self.state.water.get(&t).unwrap_or(&0))),
            ),
            ("holds", Value::Number(i64::from(k.map_or(0, |k| k.holds)))),
            ("worn", Value::Bool(self.state.worn.contains(&t))),
        ]);
        let tx = self.say("item.examine", c);
        self.output(vec![tx], None)
    }

    /// `check myself` (and `status`): how the body feels, need by need,
    /// as sensations; and the years, as the body feels them (D02: there is
    /// no status line, and no levels or numbers).
    pub(crate) fn status(&mut self) -> Output {
        self.pass(1);
        let mut parts = Vec::new();
        for (need, state) in self.state.body.states() {
            let mildest = crate::slots::need_states()
                .iter()
                .find(|(n, _)| *n == need)
                .is_some_and(|(_, states)| states.first() == Some(&state));
            if !mildest {
                let c = ctx(&[("need", Value::from(need)), ("state", Value::from(state))]);
                parts.push(self.say("body.felt", c));
            }
        }
        if parts.is_empty() {
            parts.push(self.say("body.well", Context::new()));
        }
        let age = self.age_band();
        if age != "young" {
            parts.push(self.say("body.age", ctx(&[("age", Value::from(age))])));
        }
        self.output(vec![parts.join(" ")], None)
    }

    // ---------- noise and hazards ----------

    /// Shouting: drives some creatures off, and brings down loose stone.
    pub(crate) fn shout(&mut self) -> Output {
        self.state.noise = 3;
        self.advance(1, Activity::Resting);
        let mut parts = vec![self.say("shout.done", Context::new())];
        if let Some(t) = self.disturb_stone() {
            parts.push(t);
        }
        self.state.noise = 0;
        self.output(parts, None)
    }

    /// Noise in an unstable room brings stone down: one way closes, another
    /// may open, and the player may be hurt.
    fn disturb_stone(&mut self) -> Option<String> {
        let Place::Room { structure, room } = self.state.place else {
            return None;
        };
        let local = self.env().local(self.spot(), self.state.minutes, false);
        let p = Props {
            noise: self.state.noise,
            stability: if local.unstable { 20.0 } else { 100.0 },
            ..Props::default()
        };
        if !Rules::get().yields(&p, Effect::Collapse) {
            return None;
        }
        self.state.sim.settled.insert((structure, room));
        let links = &self.site.structure(structure).interior.links;
        if let Some(l) = links
            .iter()
            .enumerate()
            .find(|(i, l)| {
                (l.a == room || l.b == room)
                    && l.state == PassageState::Open
                    && !self.state.sim.fallen.contains(&(structure, *i))
            })
            .map(|(i, _)| i)
        {
            self.state.sim.fallen.insert((structure, l));
        }
        if let Some(l) = links
            .iter()
            .enumerate()
            .find(|(i, l)| {
                l.state == PassageState::Blocked
                    && !self.state.sim.opened.contains(&(structure, *i))
            })
            .map(|(i, _)| i)
        {
            self.state.sim.opened.insert((structure, l));
        }
        let levels = 1 + (hash(&[self.seed(), 0xc011, u64::from(self.state.minutes)]) % 2) as u32;
        let t = self.say(
            "hazard.collapse",
            ctx(&[("hurt", Value::Number(i64::from(levels)))]),
        );
        self.hurt(levels, "collapse");
        Some(t)
    }

    /// Climbing a stair in the dark: a chance of falling.
    // DESIGN-Q: a third of dark stair climbs end in a fall (one or two
    // levels of injury).
    pub(crate) fn dark_stair(&mut self, passage: Option<Passage>) -> Option<String> {
        if passage != Some(Passage::Stair) || !self.is_dark() {
            return None;
        }
        let h = hash(&[self.seed(), 0xfa11, u64::from(self.state.minutes)]);
        if !h.is_multiple_of(3) {
            return None;
        }
        let levels = 1 + ((h >> 8) % 2) as u32;
        let t = self.say(
            "hazard.fall",
            ctx(&[("hurt", Value::Number(i64::from(levels)))]),
        );
        self.hurt(levels, "fall");
        Some(t)
    }

    /// Whether a room is too flooded to enter.
    pub(crate) fn flooded(&self, p: Place) -> bool {
        match p {
            Place::Room { structure, room } => {
                self.env().flood(structure, room, self.state.minutes) >= 50
            }
            Place::Outside => false,
        }
    }

    /// Crossing water by ice, wading or swimming.
    // DESIGN-Q: wading is possible below twice river strength (one in ten
    // is swept and hurt); swimming deeper rivers drowns one in three (more
    // if hurt or exhausted); lakes are too wide to swim.
    pub(crate) fn cross(&mut self, words: &[String]) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("cross");
        }
        let (cx, cy) = self.state.pos.cell();
        let w = &self.site.world;
        // The nearest water cell, and the land beyond it.
        let mut water = None;
        for (nx, ny) in w.terrain.height.neighbours(cx, cy) {
            if self
                .env()
                .obstacle(nx, ny)
                .is_some_and(|o| o != "sea" && o != "bridge")
                || (nx, ny) == (cx, cy) && self.env().is_river(nx, ny)
            {
                water = Some((nx, ny));
                break;
            }
        }
        if water.is_none() && self.env().is_river(cx, cy) {
            water = Some((cx, cy));
        }
        let Some((wx, wy)) = water else {
            let t = self.say("cross.fail", ctx(&[("reason", Value::from("no_water"))]));
            return self.output(vec![t], None);
        };
        let lake = self.site.world.terrain.biome.get(wx, wy) == &Biome::Lake;
        let beyond = (2 * wx as i64 - cx as i64, 2 * wy as i64 - cy as i64);
        let land_beyond = (0..scraped_world::terrain::SIZE as i64).contains(&beyond.0)
            && (0..scraped_world::terrain::SIZE as i64).contains(&beyond.1)
            && self.env().passable(beyond.0 as usize, beyond.1 as usize);
        let far = if land_beyond {
            Pos::of_cell(beyond.0 as usize, beyond.1 as usize)
        } else {
            Pos::of_cell(wx, wy)
        };
        let ice = self.env().ice(wx, wy, self.state.minutes);
        let roll = hash(&[
            self.seed(),
            0xc705,
            u64::from(self.state.minutes),
            wx as u64,
            wy as u64,
        ]) % 100;
        let by = if lake { "lake" } else { "river" };
        let _ = words;
        if ice >= ICE_BEARS {
            if !land_beyond && lake {
                // Walk out onto the lake towards its far side.
            }
            self.advance(20, Activity::Walking);
            self.state.pos = far;
            let t = self.say(
                "cross.done",
                ctx(&[("how", Value::from("ice")), ("by", Value::from(by))]),
            );
            let look = self.look();
            return self.output(vec![t, look], None);
        }
        if ice > 0.0 {
            // Thin ice gives way.
            let t = self.say("cross.fell_through", ctx(&[("by", Value::from(by))]));
            self.state.body.wet = 100;
            let tough = self.state.body.injury < 2 && self.state.body.rest_state() < 2;
            if roll < if tough { 30 } else { 60 } {
                self.notes.push(t);
                self.die("drowning");
                return self.output(Vec::new(), None);
            }
            self.advance(20, Activity::Resting);
            return self.output(vec![t], None);
        }
        if lake || !land_beyond {
            let t = self.say("cross.fail", ctx(&[("reason", Value::from("too_wide"))]));
            return self.output(vec![t], None);
        }
        let flow = self.env().flow(wx, wy);
        let wade = flow < 2 * RIVER_FLOW;
        self.state.body.wet = 100;
        if wade {
            self.advance(20, Activity::Walking);
            if roll < 10 {
                let t = self.say("cross.swept", ctx(&[("by", Value::from(by))]));
                self.notes.push(t);
                self.hurt(1, "drowning");
            }
        } else {
            let tough = self.state.body.injury < 2 && self.state.body.rest_state() < 2;
            if roll < if tough { 33 } else { 60 } {
                let t = self.say("cross.swept", ctx(&[("by", Value::from(by))]));
                self.notes.push(t);
                self.die("drowning");
                return self.output(Vec::new(), None);
            }
            self.advance(30, Activity::Walking);
        }
        if self.state.dead.is_some() {
            return self.output(Vec::new(), None);
        }
        self.state.pos = far;
        let how = if wade { "wade" } else { "swim" };
        let t = self.say(
            "cross.done",
            ctx(&[("how", Value::from(how)), ("by", Value::from(by))]),
        );
        let look = self.look();
        self.output(vec![t, look], None)
    }

    /// Forces a barred door with a pry bar.
    pub(crate) fn pry(&mut self, target: Target) -> Output {
        let Target::Way(l) = target else {
            let name = self.target_name(target);
            let t = self.say("item.cannot_use", ctx(&[("thing", Value::from(name))]));
            return self.output(vec![t], None);
        };
        self.door(true, l)
    }

    /// Whether a door is barred and not yet forced.
    pub(crate) fn barred(&self, structure: usize, link: usize) -> bool {
        self.site.fixtures.barred.contains(&(structure, link))
            && !self.state.sim.unbarred.contains(&(structure, link))
    }

    /// Notes for the player when the body or world changed during a
    /// command; also clears the interruption flag.
    pub(crate) fn take_notes(&mut self) -> Vec<String> {
        self.interrupted = false;
        std::mem::take(&mut self.notes)
    }
}

fn fire_near(a: Spot, b: Spot) -> bool {
    match (a, b) {
        (Spot::Out { x, y }, Spot::Out { x: px, y: py }) => {
            Pos::new(x, y).dist(Pos::new(px, py)) <= 300.0
        }
        _ => a == b,
    }
}
