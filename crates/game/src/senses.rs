//! Digging in (D02): the senses as actions, and looking further or closer.
//!
//! Sound and smell are real channels: each has sources in the world (water,
//! weather, fire, creatures, the season's life, a room's damp), and they
//! carry by distance and through walls; smell also drifts with the wind.
//! Touch reports what a thing is made of and how cold and damp it is.
//! Looking up shows the sky and so the time of day; looking down, the
//! ground. Nothing here is said unless the player asks.

use scraped_content::Value;
use scraped_sim::outdoors::{bearing, hash, Pos, BEARINGS, EDGES};
use scraped_world::structures::{Condition, Material, StructureKind};
use scraped_world::terrain::Biome;

use crate::attention::{temperature_band, Fact, Response};
use crate::site::{ctx, label, time_of_day, Place};
use crate::{Game, Output, Target};

/// A sound or smell, with how strongly it arrives (0–1) and from where.
pub(crate) struct Source {
    what: &'static str,
    name: String,
    pub(crate) strength: f64,
    from: Option<usize>,
    /// For a call: (call, role, size) of what makes it.
    call: Option<(&'static str, &'static str, &'static str, usize)>,
}

/// Whether it is quiet enough here to hear faint sounds: nothing louder
/// than faint.
pub(crate) fn quiet(sounds: &[f64]) -> bool {
    sounds.iter().all(|&s| s < 0.25)
}

fn strength_band(s: f64) -> &'static str {
    if s >= 0.6 {
        "strong"
    } else if s >= 0.25 {
        "clear"
    } else {
        "faint"
    }
}

/// How a sound or smell falls off with distance: full at `near` metres,
/// a quarter at twice that.
fn carry(strength: f64, d: f64, near: f64) -> f64 {
    let r = d / near;
    strength / (1.0 + r * r)
}

impl Game {
    /// The wind's bearing this hour (where it blows from), for smells.
    // DESIGN-Q: the wind turns every six hours, at random.
    fn wind_from(&self) -> usize {
        (hash(&[self.seed(), 0x3172, u64::from(self.state.minutes / 360)]) % 8) as usize
    }

    fn indoors(&self) -> bool {
        !matches!(self.state.place, Place::Outside)
    }

    /// Water near the player: (edge kind, metres, bearing).
    pub(crate) fn water_near(&self, radius_cells: i64) -> Vec<(&'static str, f64, Option<usize>)> {
        let p = self.state.pos;
        let mut out = Vec::new();
        for (e, kind) in EDGES.iter().enumerate() {
            if matches!(*kind, "road" | "treeline") {
                continue;
            }
            if let Some((x, y)) = self.site.land.nearest_edge(p, e, radius_cells) {
                let at = Pos::of_cell(x, y);
                let d = p.dist(at);
                out.push((*kind, d, bearing(p, at).filter(|_| d > 150.0)));
            }
        }
        out
    }

    pub(crate) fn sounds(&mut self) -> Vec<Source> {
        let mut out = Vec::new();
        let indoors = self.indoors();
        let wall = if indoors { 0.3 } else { 1.0 };
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        for (kind, d, from) in self.water_near(4) {
            let (base, near) = match kind {
                "river" => (1.0, 400.0),
                "coast" => (1.0, 600.0),
                "stream" => (0.6, 200.0),
                _ => (0.3, 200.0),
            };
            let what = match kind {
                "coast" => "sea",
                "lakeshore" => "lake",
                "river" => "river",
                _ => "stream",
            };
            out.push(Source {
                what,
                name: String::new(),
                strength: carry(base, d, near) * wall,
                from,
                call: None,
            });
        }
        let storm = local.weather == "storm"
            || self.env().weather(self.state.pos, self.state.minutes) == "storm";
        if storm {
            out.push(Source {
                what: "thunder",
                name: String::new(),
                strength: if indoors { 0.5 } else { 0.8 },
                from: None,
                call: None,
            });
        }
        if storm
            || local.weather == "rain"
            || self.env().weather(self.state.pos, self.state.minutes) == "rain"
        {
            out.push(Source {
                what: "rain",
                name: String::new(),
                strength: if indoors { 0.3 } else { 0.7 },
                from: None,
                call: None,
            });
        }
        if local.air == "windy" {
            out.push(Source {
                what: "wind",
                name: String::new(),
                strength: 0.5,
                from: Some(self.wind_from()),
                call: None,
            });
            if !indoors && self.wooded() {
                out.push(Source {
                    what: "rustling",
                    name: String::new(),
                    strength: 0.4,
                    from: None,
                    call: None,
                });
            }
        }
        if local.fire {
            out.push(Source {
                what: "fire",
                name: String::new(),
                strength: 0.4,
                from: None,
                call: None,
            });
        }
        // Creatures, by distance.
        let p = self.state.pos;
        for i in 0..self.state.creatures.len() {
            let at = self.state.creatures[i].pos;
            let d = p.dist(at);
            if d > 1500.0 {
                continue;
            }
            let name = self.creature_name(i);
            out.push(Source {
                what: "creature",
                name,
                strength: carry(0.6, d, 250.0) * wall,
                from: bearing(p, at).filter(|_| d > 100.0),
                call: None,
            });
        }
        // Calls of the living things about (D06).
        if !indoors {
            for (call, role, size, strength, from, sp) in self.calls() {
                out.push(Source {
                    what: "call",
                    name: String::new(),
                    strength,
                    from,
                    call: Some((call, role, size, sp)),
                });
            }
        }
        // Inside: water dripping, timber creaking.
        if let Place::Room { structure, room } = self.state.place {
            let st = self.site.structure(structure);
            let below = st.interior.rooms[room].level < 0;
            if below || matches!(local.wetness, "damp" | "wet" | "flooded") {
                out.push(Source {
                    what: "dripping",
                    name: String::new(),
                    strength: 0.3,
                    from: None,
                    call: None,
                });
            }
            if st.condition >= Condition::Damaged && local.air != "still" {
                out.push(Source {
                    what: "creaking",
                    name: String::new(),
                    strength: 0.25,
                    from: None,
                    call: None,
                });
            }
        }
        out.retain(|s| s.strength >= 0.08);
        out.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.what.cmp(b.what))
        });
        out
    }

    fn smells(&mut self) -> Vec<Source> {
        let mut out = Vec::new();
        let indoors = self.indoors();
        let p = self.state.pos;
        let wind = self.wind_from();
        // Upwind smells carry further; downwind ones hardly at all.
        let drift = |from: Option<usize>| match from {
            Some(b) if b == wind => 2.0,
            Some(b) if (b + 4) % 8 == wind => 0.4,
            _ => 1.0,
        };
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        if !indoors {
            for (kind, d, from) in self.water_near(3) {
                let what = if kind == "coast" { "salt" } else { "water" };
                out.push(Source {
                    what,
                    name: String::new(),
                    strength: carry(0.6, d, 300.0 * drift(from)),
                    from,
                    call: None,
                });
            }
        }
        if local.fire {
            out.push(Source {
                what: "smoke",
                name: String::new(),
                strength: 0.8,
                from: None,
                call: None,
            });
        }
        let (x, y) = p.cell();
        let biome = *self.site.world.terrain.biome.get(x, y);
        let season = scraped_sim::region::season(self.state.minutes);
        if !indoors {
            match biome {
                Biome::Pine => out.push(Source {
                    what: "resin",
                    name: String::new(),
                    strength: 0.4,
                    from: None,
                    call: None,
                }),
                Biome::Grassland | Biome::Scrub | Biome::Forest | Biome::Marsh if season <= 1 => {
                    out.push(Source {
                        what: "flowers",
                        name: String::new(),
                        strength: 0.3,
                        from: None,
                        call: None,
                    })
                }
                Biome::Desert => out.push(Source {
                    what: "dust",
                    name: String::new(),
                    strength: 0.3,
                    from: None,
                    call: None,
                }),
                _ => {}
            }
            if matches!(local.weather, "rain" | "storm") {
                out.push(Source {
                    what: "earth",
                    name: String::new(),
                    strength: 0.5,
                    from: None,
                    call: None,
                });
            }
        }
        // Creatures, on the wind.
        for i in 0..self.state.creatures.len() {
            let at = self.state.creatures[i].pos;
            let d = p.dist(at);
            if d > 800.0 || indoors {
                continue;
            }
            let from = bearing(p, at).filter(|_| d > 100.0);
            out.push(Source {
                what: "animal",
                name: String::new(),
                strength: carry(0.5, d, 150.0 * drift(from)),
                from,
                call: None,
            });
        }
        // Old stone and the dead.
        if let Place::Room { structure, room } = self.state.place {
            let st = self.site.structure(structure);
            if matches!(local.wetness, "damp" | "wet" | "flooded")
                || st.interior.rooms[room].level < 0
            {
                out.push(Source {
                    what: "damp_stone",
                    name: String::new(),
                    strength: 0.4,
                    from: None,
                    call: None,
                });
            }
            if matches!(st.kind, StructureKind::Tomb | StructureKind::Cemetery) {
                out.push(Source {
                    what: "rot",
                    name: String::new(),
                    strength: 0.25,
                    from: None,
                    call: None,
                });
            }
            if st.condition >= Condition::Ruined {
                out.push(Source {
                    what: "dust",
                    name: String::new(),
                    strength: 0.3,
                    from: None,
                    call: None,
                });
            }
        }
        out.retain(|s| s.strength >= 0.08);
        out.sort_by(|a, b| {
            b.strength
                .partial_cmp(&a.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.what.cmp(b.what))
        });
        out
    }

    /// How many facts digging here could turn up: what `look closer` and
    /// `look around` would weigh, what can be heard and smelt, the ground
    /// and sky, and writing to read. Measured for D02 (depth on demand),
    /// leaving the game as it was.
    pub fn on_demand(&mut self) -> usize {
        let saved = self.state.clone();
        let renders = self.renders.len();
        let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let outside = matches!(self.state.place, crate::site::Place::Outside);
        let responses: &[Response] = if outside {
            &[Response::Closer, Response::Around]
        } else {
            &[Response::Closer]
        };
        for &r in responses {
            let facts = if outside {
                self.outdoor_facts(r)
            } else {
                self.room_facts(r)
            };
            keys.extend(facts.into_iter().map(|f| f.key));
        }
        let mut n = keys.len();
        n += self.sounds().len().min(3) + self.smells().len().min(2);
        let mut ground = Vec::new();
        self.ground_evidence(&mut ground);
        n += ground.len();
        n += self
            .here()
            .into_iter()
            .filter(|&t| !self.thing(t).texts.is_empty())
            .count();
        self.state = saved;
        self.renders.truncate(renders);
        n
    }

    /// `listen`: up to three sounds, loudest first.
    pub(crate) fn listen(&mut self) -> Output {
        self.pass(1);
        self.listening = true;
        let indoors = self.indoors();
        let heard = self.sounds();
        let mut parts = Vec::new();
        for s in heard.into_iter().take(3) {
            let c = ctx(&[
                ("source", Value::from(s.what)),
                ("name", Value::from(s.name)),
                ("strength", Value::from(strength_band(s.strength))),
                (
                    "bearing",
                    Value::from(s.from.map_or("here", |b| BEARINGS[b])),
                ),
                ("indoors", Value::Bool(indoors)),
                ("call", Value::from(s.call.map_or("", |c| c.0))),
                ("role", Value::from(s.call.map_or("", |c| c.1))),
                ("size", Value::from(s.call.map_or("", |c| c.2))),
            ]);
            if let Some((_, _, _, sp)) = s.call {
                self.note_life(&format!("life-call:{sp}"));
            }
            let t = self.say("sense.sound", c);
            parts.push(crate::attention::sentence(&t));
        }
        if parts.is_empty() {
            parts.push(self.say("sense.silence", ctx(&[("indoors", Value::Bool(indoors))])));
        }
        self.output(vec![parts.join(" ")], None)
    }

    /// `smell`: up to two smells, strongest first.
    pub(crate) fn smell(&mut self) -> Output {
        self.pass(1);
        let indoors = self.indoors();
        let smelt = self.smells();
        let mut parts = Vec::new();
        for s in smelt.into_iter().take(2) {
            let c = ctx(&[
                ("source", Value::from(s.what)),
                ("strength", Value::from(strength_band(s.strength))),
                (
                    "bearing",
                    Value::from(s.from.map_or("here", |b| BEARINGS[b])),
                ),
                ("indoors", Value::Bool(indoors)),
            ]);
            let t = self.say("sense.smell", c);
            parts.push(crate::attention::sentence(&t));
        }
        if parts.is_empty() {
            parts.push(self.say("sense.no_smell", ctx(&[("indoors", Value::Bool(indoors))])));
        }
        self.output(vec![parts.join(" ")], None)
    }

    /// How a thing feels under the hand.
    pub(crate) fn touch(&mut self, target: Option<Target>) -> Output {
        self.pass(1);
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        let temp = temperature_band(local.temperature);
        let Some(Target::Thing(t)) = target else {
            let c = ctx(&[
                ("temperature", Value::from(temp)),
                ("wetness", Value::from(local.wetness)),
                ("air", Value::from(local.air)),
                ("indoors", Value::Bool(self.indoors())),
            ]);
            let tx = self.say("sense.touch_air", c);
            return self.output(vec![tx], None);
        };
        let th = self.thing(t);
        let material = th.material;
        let marks = !th.texts.is_empty();
        let condition = match th.home {
            Place::Room { structure, .. } => self.site.structure(structure).condition,
            Place::Outside => Condition::Worn,
        };
        let texture = if condition >= Condition::Ruined && material != Material::Metal {
            "crumbling"
        } else {
            match material {
                Material::Stone if condition == Condition::Intact => "smooth",
                Material::Stone => "rough",
                Material::Clay => "gritty",
                Material::Wood => "grained",
                Material::Metal => "cold and smooth",
                Material::Plaster => "chalky",
                Material::Vellum => "soft",
            }
        };
        // Metal and stone feel colder than the air.
        let felt = if matches!(material, Material::Metal | Material::Stone) {
            temperature_band(local.temperature - 4.0)
        } else {
            temp
        };
        let name = self.thing_name(t);
        let c = ctx(&[
            ("thing", Value::from(name)),
            ("material", Value::from(label(&material))),
            ("texture", Value::from(texture)),
            ("temperature", Value::from(felt)),
            (
                "wetness",
                Value::from(if local.wetness == "flooded" {
                    "wet"
                } else {
                    local.wetness
                }),
            ),
            ("marks", Value::Bool(marks)),
        ]);
        let tx = self.say("sense.touch", c);
        self.output(vec![tx], None)
    }

    /// `taste`: water nearby, salt by the sea, dust otherwise.
    pub(crate) fn taste(&mut self) -> Output {
        self.pass(1);
        let water = if self.indoors() {
            Vec::new()
        } else {
            self.water_near(1)
        };
        let taste = if water.iter().any(|(k, _, _)| *k == "coast") {
            "salt"
        } else if water
            .iter()
            .any(|(k, _, _)| matches!(*k, "river" | "stream" | "lakeshore"))
        {
            let (x, y) = self.state.pos.cell();
            if matches!(
                self.site.world.terrain.biome.get(x, y),
                Biome::Marsh | Biome::Shore
            ) {
                "brackish water"
            } else {
                "fresh water"
            }
        } else if self.biome_here() == Biome::Desert {
            "dust"
        } else {
            "nothing"
        };
        let tx = self.say("sense.taste", ctx(&[("taste", Value::from(taste))]));
        self.output(vec![tx], None)
    }

    /// `look up`: the sky outdoors (and so the time of day), the ceiling
    /// inside.
    pub(crate) fn look_up(&mut self) -> Output {
        self.pass(1);
        if self.indoors() {
            let t = self.describe(Response::Closer);
            return self.output(vec![t], None);
        }
        let (weather, light, _) = self.conditions();
        let time = time_of_day(self.state.minutes);
        let sky = match (weather, light, time) {
            (_, "dark", _) if weather == "clear" => "stars",
            (_, "dark", _) => "overcast night",
            ("rain" | "fog" | "storm" | "snow", _, _) => "grey",
            (_, _, "dawn" | "morning") => "sun low in the east",
            (_, _, "evening") => "sun low in the west",
            _ => "sun high",
        };
        let season = scraped_sim::region::season(self.state.minutes);
        let birds = light != "dark" && season != 3 && self.biome_here() != Biome::Snow;
        let c = ctx(&[
            ("sky", Value::from(sky)),
            ("weather", Value::from(weather)),
            ("time", Value::from(time)),
            ("birds", Value::Bool(birds)),
        ]);
        let tx = self.say("sense.sky", c);
        self.output(vec![tx], None)
    }

    /// `look down`: the ground or floor, and what the season or the land's
    /// state leaves on it.
    pub(crate) fn look_down(&mut self) -> Output {
        self.pass(1);
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        let pos = self.state.pos;
        let mut facts = vec![Fact::new(
            "sense.ground",
            "look-down",
            90.0,
            ctx(&[
                ("biome", Value::from(self.site.biome_at(pos))),
                ("terrain", Value::from(self.site.land.terrain_here(pos))),
                ("wetness", Value::from(local.wetness)),
                ("indoors", Value::Bool(self.indoors())),
            ]),
        )
        .anchored()];
        if !self.indoors() {
            let mut evidence = Vec::new();
            self.ground_evidence(&mut evidence);
            facts.extend(evidence);
        }
        let parts = self.attend(facts, Response::Arrival);
        self.output(vec![parts.join(" ")], None)
    }

    /// Evidence on the ground: the region's state and the season's signs
    /// that lie underfoot.
    fn ground_evidence(&mut self, out: &mut Vec<Fact>) {
        let mut all = Vec::new();
        self.felt_facts(&mut all);
        let mut evidence = Vec::new();
        self.outdoor_evidence(&mut evidence);
        let underfoot = [
            "frost",
            "snow cover",
            "falling leaves",
            "meltwater",
            "green shoots",
            "dust",
            "browning grass",
            "first frost",
            "seed heads",
            "ice at the edges",
        ];
        for f in evidence {
            let ev = f.vars.get("evidence").map(Value::text).unwrap_or_default();
            if f.slot == "evidence.region" || underfoot.contains(&ev.as_str()) {
                out.push(f);
            }
        }
        // What animals left underfoot (D06).
        let mut life = Vec::new();
        self.life_facts(Response::Closer, &mut life);
        out.extend(
            life.into_iter()
                .filter(|f| matches!(f.slot, "life.sign" | "life.home")),
        );
    }

    /// `count the tombs`: the one exact number in the game.
    pub(crate) fn count(&mut self, words: &[String]) -> Output {
        self.pass(2);
        let noun = words
            .iter()
            .rev()
            .find(|w| !matches!(w.as_str(), "the" | "a" | "an" | "all"))
            .cloned()
            .unwrap_or_default();
        let singular = singular(&noun);
        let structures = self
            .local_structures()
            .into_iter()
            .filter(|&s| label(&self.site.world.structures[s].kind) == singular)
            .count();
        let things = self
            .here()
            .into_iter()
            .filter(|&t| self.thing(t).kind == singular)
            .count();
        let n = structures.max(things);
        if n == 0 {
            let t = self.say(
                "say.not_here",
                ctx(&[("words", Value::from(words.join(" ")))]),
            );
            return self.output(vec![t], None);
        }
        let c = ctx(&[
            ("kind", Value::from(singular)),
            ("count", Value::Number(n as i64)),
        ]);
        let t = self.say("place.count", c);
        self.output(vec![t], None)
    }

    /// A group the player is looking at: its members, a few at a time,
    /// those not yet named first.
    pub(crate) fn examine_group(&mut self, target: Target) -> Output {
        self.pass(2);
        const BATCH: usize = 3;
        let mut parts = Vec::new();
        match target {
            Target::Buildings(k, c) => {
                let kind = crate::slots::STRUCTURES.get(k).copied().unwrap_or("house");
                let condition = c.and_then(|c| crate::slots::CONDITIONS.get(c)).copied();
                let mut all: Vec<usize> = self
                    .local_structures()
                    .into_iter()
                    .filter(|&s| label(&self.site.world.structures[s].kind) == kind)
                    .filter(|&s| {
                        condition
                            .is_none_or(|c| label(&self.site.world.structures[s].condition) == c)
                    })
                    .collect();
                all.sort_by_key(|&s| (self.site.world.structures[s].condition, s));
                let (shown, left) = self.next_members("member", &all, BATCH);
                for (i, &s) in shown.iter().enumerate() {
                    let name = self.structure_name(s);
                    let at = self.site.land.structure_pos[s];
                    let p = self.state.pos;
                    let way = bearing(p, at)
                        .filter(|_| p.dist(at) > 40.0)
                        .map_or("here", |b| BEARINGS[b]);
                    let st = &self.site.world.structures[s];
                    let c = ctx(&[
                        ("bearing", Value::from(way)),
                        ("name", Value::from(name)),
                        ("kind", Value::from(label(&st.kind))),
                        ("condition", Value::from(label(&st.condition))),
                        ("rank", Value::Number(i as i64 + 1)),
                        (
                            "more",
                            Value::Number(if i + 1 == shown.len() { left as i64 } else { 0 }),
                        ),
                    ]);
                    parts.push(self.say("place.member", c));
                }
            }
            Target::Things(k) => {
                let kind = crate::slots::kinds().get(k).copied().unwrap_or("jar");
                let all: Vec<usize> = self
                    .here()
                    .into_iter()
                    .filter(|&t| self.thing(t).kind == kind)
                    .collect();
                let (shown, left) = self.next_members("tmember", &all, BATCH);
                for (i, &t) in shown.iter().enumerate() {
                    let name = self.thing_name(t);
                    let c = ctx(&[
                        ("bearing", Value::from("here")),
                        ("name", Value::from(name)),
                        ("kind", Value::from(kind)),
                        ("condition", Value::from("worn")),
                        ("rank", Value::Number(i as i64 + 1)),
                        (
                            "more",
                            Value::Number(if i + 1 == shown.len() { left as i64 } else { 0 }),
                        ),
                    ]);
                    parts.push(self.say("place.member", c));
                }
            }
            _ => {}
        }
        self.output(vec![parts.join(" ")], None)
    }

    /// The next few members of a group not yet named (in the given order),
    /// remembered as named, and how many are left after them. Once all
    /// have been named the round starts again, so every member is reached.
    fn next_members(&mut self, prefix: &str, all: &[usize], batch: usize) -> (Vec<usize>, usize) {
        let key = |m: usize| format!("{prefix}:{m}");
        let mut fresh: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&m| !self.state.told.contains_key(&key(m)))
            .collect();
        if fresh.is_empty() {
            for &m in all {
                self.state.told.remove(&key(m));
            }
            fresh = all.to_vec();
        }
        let shown: Vec<usize> = fresh.iter().copied().take(batch).collect();
        for &m in &shown {
            self.state.told.insert(
                key(m),
                crate::attention::Told {
                    minutes: self.state.minutes,
                    signature: 0,
                    said: true,
                },
            );
        }
        let left = fresh.len() - shown.len();
        (shown, left)
    }

    /// A second look at a thing finds what the first glance missed.
    pub(crate) fn examine_closer(&mut self, t: usize) -> Option<Output> {
        let key = format!("examined:{t}");
        let before = self.state.told.contains_key(&key);
        self.state.told.insert(
            key,
            crate::attention::Told {
                minutes: self.state.minutes,
                signature: 0,
                said: true,
            },
        );
        if !before {
            return None;
        }
        self.pass(2);
        let th = self.thing(t);
        let size = match th.kind {
            "jar" | "tablet" | "scroll" | "torch" | "lamp" | "oil" | "firesteel" | "lens"
            | "stylus" | "scraper" => "small",
            "shelf" | "basin" | "chest" | "table" | "lintel" | "niche" | "gravestone"
            | "milestone" => "middling",
            "inscription" | "gate" | "wall" => "huge",
            _ => "large",
        };
        let condition = match th.home {
            Place::Room { structure, .. } => label(&self.site.structure(structure).condition),
            Place::Outside => "worn".to_string(),
        };
        let item = th.portable || scraped_sim::items::kind(th.kind).is_some();
        let c = ctx(&[
            ("kind", Value::from(th.kind)),
            ("material", Value::from(label(&th.material))),
            ("condition", Value::from(condition)),
            ("size", Value::from(size)),
            ("written", Value::Bool(!th.texts.is_empty())),
            ("item", Value::Bool(item)),
        ]);
        let tx = self.say("thing.closer", c);
        Some(self.output(vec![tx], None))
    }
}

impl Game {
    /// For "go to a tomb": the nearest building of a kind here that the
    /// player hasn't been into, or the nearest at all.
    pub(crate) fn pick_member(&self, k: usize, condition: Option<usize>) -> Option<usize> {
        let kind = crate::slots::STRUCTURES.get(k)?;
        let condition = condition
            .and_then(|c| crate::slots::CONDITIONS.get(c))
            .copied();
        let p = self.state.pos;
        let mut all: Vec<usize> = self
            .local_structures()
            .into_iter()
            .filter(|&s| label(&self.site.world.structures[s].kind) == *kind)
            .filter(|&s| {
                condition.is_none_or(|c| label(&self.site.world.structures[s].condition) == c)
            })
            .collect();
        all.sort_by_key(|&s| {
            (
                self.state.visited.contains(&s),
                self.site.land.structure_pos[s].dist2(p),
                s,
            )
        });
        all.first().copied()
    }
}

/// A plural noun's singular, for counting and groups ("tombs" → "tomb").
pub(crate) fn singular(w: &str) -> String {
    for (pl, sg) in [
        ("ies", "y"),
        ("ches", "ch"),
        ("shes", "sh"),
        ("ses", "s"),
        ("s", ""),
    ] {
        if let Some(stem) = w.strip_suffix(pl) {
            if !stem.is_empty() {
                return format!("{stem}{sg}");
            }
        }
    }
    w.to_string()
}

/// A noun's plural, for naming groups ("tomb" → "tombs").
pub(crate) fn plural(w: &str) -> String {
    if w.ends_with(['s', 'x']) || w.ends_with("ch") || w.ends_with("sh") {
        format!("{w}es")
    } else if let Some(stem) = w.strip_suffix('y') {
        format!("{stem}ies")
    } else {
        format!("{w}s")
    }
}
