//! Living things in play (D06): the signs animals leave, the plants of
//! the season, homes, the calls that carry, and the animals glimpsed by a
//! player who stops and watches.
//!
//! Everything goes through the attention model at low salience, so a
//! traveller hears birds and sees trees, and one who looks closer, looks
//! down, listens or waits finds the tracks, the droppings, the den. Most
//! animals are met by their signs long before they are seen.

use scraped_content::Value;
use scraped_sim::outdoors::{bearing, hash, Pos, BEARINGS};
use scraped_sim::region::{season, LIFE, SEASON_DAYS};
use scraped_world::life::{Cycle, Habitat, Kingdom, Species};
use scraped_world::terrain::Biome;

use scraped_sim::outdoors::distance_band;

use crate::attention::{Fact, Response};
use crate::site::{ctx, time_of_day, Place};
use crate::{Game, Output};

/// A species where the player is, with how plentiful it is now (0–1.5).
pub(crate) struct Living {
    pub(crate) species: usize,
    pub(crate) plenty: f64,
}

/// A call carried to the player: (call, role, size, strength, bearing,
/// species).
pub(crate) type Call = (
    &'static str,
    &'static str,
    &'static str,
    f64,
    Option<usize>,
    usize,
);

/// Whether a species is about at this time of day.
fn about(active: &str, time: &str) -> bool {
    match active {
        "night" => matches!(time, "night" | "evening"),
        "dusk" => matches!(time, "dawn" | "evening" | "night"),
        _ => !matches!(time, "night"),
    }
}

/// How readily a role is seen by someone standing still, out of 1000.
// DESIGN-Q: how shy each role is. Birds and insects show themselves;
// hunters almost never do; large grazers now and then, at a distance.
fn boldness(role: &str) -> u64 {
    match role {
        "bird" | "migrant" => 300,
        "insect" => 250,
        "grazer" => 120,
        "small" | "fish" => 80,
        "burrower" => 60,
        "browser" => 45,
        "scavenger" => 30,
        "predator" => 10,
        _ => 0,
    }
}

/// What an animal is doing when seen.
fn doing(s: &Species, time: &str, h: u64) -> &'static str {
    let opts: &[&str] = match s.role {
        "grazer" => &["grazing", "standing watchful", "moving off"],
        "browser" => &["browsing", "standing watchful", "moving off"],
        "predator" => &["watching", "slipping away"],
        "scavenger" => &["nosing about", "trotting off"],
        "burrower" => &["sitting up", "diving into a hole"],
        "bird" | "migrant" if time == "dawn" => &["singing", "calling"],
        "bird" | "migrant" => &["flying over", "feeding", "perched"],
        "insect" => &["on the wing", "at the flowers"],
        "fish" => &["rising", "holding in the current"],
        _ => &["basking", "still"],
    };
    opts[(h % opts.len() as u64) as usize]
}

impl Game {
    /// The habitats around the player: the land here, and the water
    /// beside it.
    pub(crate) fn habitats_here(&self) -> Vec<Habitat> {
        let (x, y) = self.state.pos.cell();
        let w = &self.site.world;
        let mut out = vec![Habitat::of(&w.terrain, &w.water, x, y)];
        for (kind, d, _) in self.water_near(1) {
            if d > 400.0 {
                continue;
            }
            let h = if kind == "coast" {
                Habitat::Sea
            } else {
                Habitat::Fresh
            };
            if !out.contains(&h) {
                out.push(h);
            }
        }
        out
    }

    /// The seasons since play began, for the cycle of hunters and prey.
    fn seasons_gone(&self) -> u32 {
        self.state.minutes / 1440 / SEASON_DAYS
    }

    /// What writing does to a species at a point (D09): +1 drawn here or
    /// thriving, -1 kept off or withering, 0 nothing.
    fn spelled_for(&self, s: &scraped_world::life::Species, at: scraped_sim::outdoors::Pos) -> i32 {
        use scraped_sim::writing::{resolve, Class, Property};
        let id = scraped_world::life::concept_id(s.form);
        let (class, q) = match s.kingdom {
            scraped_world::life::Kingdom::Animal => (Class::Animal, Property::Lure),
            _ => (Class::Plant, Property::Growth),
        };
        let env = self.env();
        let on: Vec<&scraped_sim::writing::Claim> = self
            .claims
            .iter()
            .filter(|c| c.class == class && c.property == q && c.subject == id && c.acts(&env.when))
            .collect();
        if on.is_empty() {
            return 0;
        }
        resolve(&on, at).map_or(0, i32::signum)
    }

    /// The living things here now: in these habitats, here this season,
    /// not gone from this region, and found in this patch of it.
    pub(crate) fn life_here(&self) -> Vec<Living> {
        if !matches!(self.state.place, Place::Outside) {
            return Vec::new();
        }
        let pos = self.state.pos;
        let life_ratio = self
            .env()
            .region_ratio(pos, LIFE)
            .unwrap_or(1000)
            .clamp(0, 1600);
        let capacity = life_ratio as f64 / 1000.0;
        let season = season(self.state.minutes);
        let (x, y) = pos.cell();
        let seed = self.seed();
        let mut out = Vec::new();
        for hab in self.habitats_here() {
            let cycle = Cycle::after(seed, hab, self.seasons_gone(), capacity);
            for s in self.site.world.life.of(hab) {
                if !s.here_in(season) || life_ratio < s.tolerance {
                    continue;
                }
                // Writing may draw a kind of beast or make a plant thrive
                // here, or keep it off and wither it (D09).
                let spelled = self.spelled_for(s, pos);
                if spelled < 0 {
                    continue;
                }
                // Patches of about a kilometre: the commoner a species,
                // the more patches hold it.
                let patch = hash(&[seed, 0x11fe, s.id as u64, (x / 3) as u64, (y / 3) as u64]) % 10;
                if patch > u64::from(s.abundance) && spelled == 0 {
                    continue;
                }
                let plenty = (s.abundance as f64 / 10.0)
                    * cycle.factor(s.role).clamp(0.1, 2.0)
                    * capacity.min(1.5)
                    * if spelled > 0 { 1.5 } else { 1.0 };
                out.push(Living {
                    species: s.id,
                    plenty,
                });
            }
        }
        out
    }

    /// The nearest home of a living species within reach, with metres.
    fn home_near(&self, living: &[Living]) -> Option<(usize, f64, Pos)> {
        let p = self.state.pos;
        self.site
            .world
            .life
            .homes
            .iter()
            .filter(|h| living.iter().any(|l| l.species == h.species))
            .map(|h| {
                let at = Pos::of_cell(h.cell.ux(), h.cell.uy());
                (h.species, p.dist(at), at)
            })
            .filter(|(_, d, _)| *d <= 350.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// The ground for tracks: "mud", "snow", "sand", or "" when it is too
    /// hard to hold them.
    fn soft_ground(&self) -> &'static str {
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        let (x, y) = self.state.pos.cell();
        let biome = *self.site.world.terrain.biome.get(x, y);
        use scraped_world::terrain::Biome::*;
        let winter = season(self.state.minutes) == 3;
        if biome == Snow || (winter && matches!(biome, Tundra | Rock | Pine | Forest)) {
            "snow"
        } else if matches!(local.wetness, "damp" | "wet" | "flooded") || matches!(biome, Marsh) {
            "mud"
        } else if matches!(biome, Desert | Shore) {
            "sand"
        } else {
            ""
        }
    }

    /// Everything living that could be said here.
    pub(crate) fn life_facts(&mut self, response: Response, out: &mut Vec<Fact>) {
        let living = self.life_here();
        if living.is_empty() {
            return;
        }
        let seed = self.seed();
        let now = self.state.minutes;
        let time = time_of_day(now);
        let season_now = season(now);
        let (x, y) = self.state.pos.cell();
        let dark = self.conditions().1 == "dark";
        let ground = self.soft_ground();
        let home = self.home_near(&living);
        let in_town = self.town_here().is_some();
        let closer = matches!(response, Response::Closer);
        // Signs: looked for, not seen in passing.
        if closer {
            let mut signs: Vec<(u64, Fact)> = Vec::new();
            for l in &living {
                let s = &self.site.world.life.species[l.species];
                if s.kingdom != Kingdom::Animal || s.role == "deep" {
                    continue;
                }
                let h = hash(&[
                    seed,
                    0x5195,
                    s.id as u64,
                    x as u64,
                    y as u64,
                    u64::from(now / 720),
                ]);
                let at_home = home.is_some_and(|(sp, _, _)| sp == s.id);
                if !at_home && (h % 1000) as f64 > l.plenty * 700.0 {
                    continue;
                }
                let tracks = !ground.is_empty() && s.foot != "none";
                let sign = if tracks && h >> 12 & 1 == 0 {
                    "tracks"
                } else {
                    s.signs[((h >> 13) % s.signs.len() as u64) as usize]
                };
                // Insects' signs are everywhere and small: half are passed by.
                if s.role == "insect" && h >> 30 & 1 == 0 {
                    continue;
                }
                let lasting = sign.starts_with("an old")
                    || matches!(sign, "burrows" | "galls" | "a scratched tree");
                let fresh = !lasting && (about(s.active, time) || (h >> 20).is_multiple_of(3));
                let sal = 20.0 + l.plenty * 8.0 + if at_home { 14.0 } else { 0.0 };
                signs.push((
                    h,
                    Fact::new(
                        "life.sign",
                        format!("life-sign:{}:{sign}", s.id),
                        sal,
                        ctx(&[
                            ("sign", Value::from(sign)),
                            ("foot", Value::from(s.foot)),
                            ("size", Value::from(s.size)),
                            ("ground", Value::from(if tracks { ground } else { "" })),
                            ("fresh", Value::Bool(fresh)),
                            ("role", Value::from(s.role)),
                        ]),
                    ),
                ));
            }
            signs.sort_by_key(|(h, _)| *h);
            out.extend(signs.into_iter().take(4).map(|(_, f)| f));
        }
        // The animals actually roaming near leave fresh signs: a warning
        // before they are seen, noticed in passing where the ground holds
        // tracks, and always by looking closer.
        let p = self.state.pos;
        for i in 0..self.state.creatures.len() {
            let home = &self.site.fixtures.creatures[i];
            let Some(sp) = home.species else { continue };
            if !matches!(home.home, scraped_sim::fixtures::Spot::Out { .. }) {
                continue;
            }
            // Its range: about 1.5 km round where it lives and where it is.
            let d = p.dist(self.state.creatures[i].pos).min(p.dist(home.pos));
            if d > 1500.0 || (!closer && ground.is_empty()) {
                continue;
            }
            let s = &self.site.world.life.species[sp];
            // Only where the species lives.
            if !living.iter().any(|l| l.species == sp) && !self.habitats_here().contains(&s.habitat)
            {
                continue;
            }
            let tracks = !ground.is_empty() && s.foot != "none";
            let sign = if tracks { "tracks" } else { s.signs[0] };
            out.push(Fact::new(
                "life.sign",
                format!("life-sign:{sp}:{sign}"),
                if closer { 34.0 } else { 24.0 },
                ctx(&[
                    ("sign", Value::from(sign)),
                    ("foot", Value::from(s.foot)),
                    ("size", Value::from(s.size)),
                    ("ground", Value::from(if tracks { ground } else { "" })),
                    ("fresh", Value::Bool(true)),
                    ("role", Value::from(s.role)),
                ]),
            ));
        }
        // A home close by.
        if let Some((sp, d, at)) = home {
            if matches!(response, Response::Closer | Response::Around) || d < 100.0 {
                let s = &self.site.world.life.species[sp];
                out.push(Fact::new(
                    "life.home",
                    format!("life-home:{sp}"),
                    if closer { 30.0 } else { 20.0 },
                    ctx(&[
                        ("home", Value::from(s.home.unwrap_or("den"))),
                        ("role", Value::from(s.role)),
                        ("size", Value::from(s.size)),
                        ("foot", Value::from(s.foot)),
                        ("occupied", Value::Bool(!about(s.active, time))),
                        (
                            "bearing",
                            Value::from(
                                bearing(self.state.pos, at)
                                    .filter(|_| d > 60.0)
                                    .map_or("here", |b| BEARINGS[b]),
                            ),
                        ),
                    ]),
                ));
            }
        }
        // Plants: the tall ones around, the small ones closer; what is in
        // flower or fruit stands out.
        if !dark {
            let mut plants: Vec<(f64, Fact)> = Vec::new();
            for l in &living {
                let s = &self.site.world.life.species[l.species];
                if s.kingdom != Kingdom::Plant {
                    continue;
                }
                let tall = matches!(s.role, "tree" | "shrub");
                let passing = matches!(
                    response,
                    Response::Look | Response::Arrival | Response::Travel
                );
                if tall == closer && !passing {
                    continue;
                }
                let state = s.state(season_now);
                let showy = matches!(state, "in flower" | "in fruit" | "turning");
                // In passing, only what is in flower or fruit, and not in towns.
                if passing && (!showy || in_town) {
                    continue;
                }
                let h = hash(&[seed, 0x9147, s.id as u64, x as u64, y as u64]) % 100;
                let sal = 6.0 + l.plenty * 6.0 + if showy { 10.0 } else { 0.0 } + h as f64 / 50.0;
                plants.push((
                    sal,
                    Fact::new(
                        "life.plant",
                        format!("life-plant:{}", s.id),
                        sal,
                        ctx(&[
                            ("form", Value::from(s.form)),
                            ("role", Value::from(s.role)),
                            ("state", Value::from(state)),
                            ("colour", Value::from(s.colour)),
                            ("mark", Value::from(s.mark)),
                            ("fruit", Value::from(s.fruit)),
                        ]),
                    ),
                ));
            }
            plants.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let many = if matches!(response, Response::Closer | Response::Around) {
                2
            } else {
                1
            };
            out.extend(plants.into_iter().take(many).map(|(_, f)| f));
        }
        // Animals glimpsed: only by someone looking about, and more by one
        // who has stood still.
        if matches!(response, Response::Around | Response::Closer)
            || (response == Response::Look && self.attentive)
        {
            let still = if self.attentive { 2 } else { 1 };
            let mut seen: Vec<(u64, Fact)> = Vec::new();
            for l in &living {
                let s = &self.site.world.life.species[l.species];
                if s.kingdom != Kingdom::Animal || !about(s.active, time) || s.role == "deep" {
                    continue;
                }
                if dark && !matches!(s.role, "small" | "insect") {
                    continue;
                }
                if s.role == "insect" && season_now >= 2 {
                    continue;
                }
                let h = hash(&[
                    seed,
                    0x5ee4,
                    s.id as u64,
                    x as u64,
                    y as u64,
                    u64::from(now / 20),
                ]);
                let chance = (boldness(s.role) as f64 * l.plenty) as u64 * still;
                if h % 1000 >= chance {
                    continue;
                }
                let far = matches!(s.role, "grazer" | "browser" | "predator" | "scavenger");
                let dist = if far {
                    [120.0, 400.0, 900.0][((h >> 12) % 3) as usize]
                } else {
                    30.0
                };
                let b = BEARINGS[((h >> 16) % 8) as usize];
                let many = match s.role {
                    "grazer" | "insect" | "fish" => {
                        ["one", "a few", "many"][((h >> 20) % 3) as usize]
                    }
                    "bird" | "migrant" => ["one", "a pair", "a few"][((h >> 20) % 3) as usize],
                    _ => "one",
                };
                seen.push((
                    h,
                    Fact::new(
                        "life.seen",
                        format!("life-seen:{}", s.id),
                        if far { 30.0 } else { 16.0 },
                        ctx(&[
                            ("form", Value::from(s.form)),
                            ("role", Value::from(s.role)),
                            ("colour", Value::from(s.colour)),
                            ("mark", Value::from(s.mark)),
                            ("size", Value::from(s.size)),
                            ("doing", Value::from(doing(s, time, h >> 24))),
                            ("count", Value::from(many)),
                            ("bearing", Value::from(b)),
                            ("distance", Value::from(distance_band(dist))),
                        ]),
                    ),
                ));
            }
            seen.sort_by_key(|(h, _)| *h);
            out.extend(seen.into_iter().take(1).map(|(_, f)| f));
        }
    }

    /// Calls carried to the player now: (call, role, size, strength,
    /// bearing, species).
    pub(crate) fn calls(&self) -> Vec<Call> {
        let mut out = Vec::new();
        let now = self.state.minutes;
        let time = time_of_day(now);
        let season_now = season(now);
        let seed = self.seed();
        for l in self.life_here() {
            let s = &self.site.world.life.species[l.species];
            let Some(call) = s.call else { continue };
            if !about(s.active, time) {
                continue;
            }
            // When each calls most: birds at dawn, the deer kind in their
            // autumn rut, frogs on spring nights, insects in summer.
            let base = match s.role {
                "bird" | "migrant" if time == "dawn" => 0.55,
                "bird" | "migrant" => 0.25,
                "grazer" | "browser" if season_now == 2 => 0.6,
                "grazer" | "browser" => 0.12,
                "predator" | "scavenger" if time == "night" => 0.45,
                "small" if season_now <= 1 => 0.4,
                "insect" if season_now == 1 => 0.3,
                "burrower" => 0.15,
                _ => 0.0,
            };
            let h = hash(&[seed, 0xca11, s.id as u64, u64::from(now / 30)]);
            if base == 0.0 || (h % 1000) as f64 > l.plenty * 900.0 {
                continue;
            }
            let from = (!matches!(s.role, "insect" | "small")).then_some(((h >> 16) % 8) as usize);
            out.push((
                call,
                s.role,
                s.size,
                base * (0.7 + l.plenty * 0.3),
                from,
                s.id,
            ));
        }
        out
    }

    /// Records, for the depth metrics, that a species' sign or the animal
    /// itself was put before the player (first time only).
    pub(crate) fn note_life(&mut self, key: &str) {
        let mut parts = key.split(':');
        let (Some(kind), Some(id)) = (parts.next(), parts.next()) else {
            return;
        };
        let idx = match kind {
            "life-sign" | "life-home" | "life-call" => 0,
            "life-seen" => 1,
            "creature" => {
                let Some(sp) = id
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| self.site.fixtures.creatures.get(i))
                    .and_then(|c| c.species)
                else {
                    return;
                };
                let turn = self.state.minutes;
                self.life_log.entry(sp).or_default()[1].get_or_insert(turn);
                return;
            }
            _ => return,
        };
        let Ok(sp) = id.parse::<usize>() else { return };
        let turn = self.state.minutes;
        self.life_log.entry(sp).or_default()[idx].get_or_insert(turn);
    }
}

/// What a forager takes from a plant: the item it makes.
fn forage_item(s: &Species) -> &'static str {
    match (s.role, s.fruit) {
        ("fungus", _) => "fungi",
        (_, "nuts") => "nuts",
        (_, "berries" | "hips") => "berries",
        _ => "greens",
    }
}

impl Game {
    /// `forage`: an hour's search for what is safe to eat here this
    /// season, from the plants that grow here.
    // DESIGN-Q: foraging only ever takes what is safe; harmful plants are
    // left alone. Leaves and shoots ("greens") can be had in spring and
    // summer from any food plant.
    pub(crate) fn forage(&mut self) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("forage");
        }
        let now = self.state.minutes;
        let season_now = season(now);
        let living = self.life_here();
        let mut best: Option<(f64, usize)> = None;
        for l in &living {
            let s = &self.site.world.life.species[l.species];
            if s.kingdom != Kingdom::Plant || s.use_ != "food" {
                continue;
            }
            let ripe = s.fruits == Some(season_now);
            let leafy = season_now <= 1 && s.role != "fungus";
            if !ripe && !leafy {
                continue;
            }
            let score = l.plenty * if ripe { 1.0 } else { 0.4 };
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, s.id));
            }
        }
        // How much there is to find goes by the land, as before D06, and
        // falls with the region's life; the plants say what it is.
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
        let life = self
            .env()
            .region_ratio(self.state.pos, LIFE)
            .unwrap_or(1000)
            .clamp(0, 1500);
        let chance = chance * life as u64 / 1000;
        let roll = hash(&[self.seed(), 0xf0a6, u64::from(now), self.state.pos.x as u64]) % 100;
        self.advance(60, scraped_sim::body::Activity::Walking);
        let biome = crate::site::label(&self.biome_here());
        if roll < chance && self.state.dead.is_none() {
            let (kind, form, role) = match best {
                Some((_, sp)) => {
                    let s = &self.site.world.life.species[sp];
                    let ripe = s.fruits == Some(season_now);
                    (if ripe { forage_item(s) } else { "greens" }, s.form, s.role)
                }
                None => ("berries", "", ""),
            };
            self.make_item(kind, true);
            let t = self.say(
                "forage.found",
                ctx(&[
                    ("kind", Value::from(kind)),
                    ("biome", Value::from(biome)),
                    ("form", Value::from(form)),
                    ("role", Value::from(role)),
                ]),
            );
            self.output(vec![t], None)
        } else {
            let t = self.say("forage.none", ctx(&[("biome", Value::from(biome))]));
            self.output(vec![t], None)
        }
    }

    /// `fish`: an hour and a half at the water's edge with a line of
    /// twisted grass and a bent pin.
    // DESIGN-Q: fishing needs no gear and takes 90 minutes; the catch
    // depends on the fish living in that water and the season (few in
    // winter).
    pub(crate) fn fish(&mut self) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("fish");
        }
        let water = self
            .water_near(1)
            .into_iter()
            .filter(|(_, d, _)| *d <= 300.0)
            .map(|(k, _, _)| k)
            .next();
        let Some(water) = water else {
            let t = self.say("fish.no_water", ctx(&[]));
            return self.output(vec![t], None);
        };
        let now = self.state.minutes;
        let fish: Vec<(f64, usize)> = self
            .life_here()
            .into_iter()
            .filter(|l| self.site.world.life.species[l.species].role == "fish")
            .map(|l| (l.plenty, l.species))
            .collect();
        let winter = season(now) == 3;
        let chance: f64 =
            fish.iter().map(|(p, _)| p * 40.0).sum::<f64>() * if winter { 0.4 } else { 1.0 };
        let roll = hash(&[self.seed(), 0xf15_u64, u64::from(now)]) % 100;
        self.advance(90, scraped_sim::body::Activity::Resting);
        let water_id = match water {
            "coast" => "sea",
            "lakeshore" => "lake",
            other => other,
        };
        if (roll as f64) < chance.min(85.0) && self.state.dead.is_none() && !fish.is_empty() {
            let (_, sp) = fish[(roll as usize) % fish.len()];
            let s = &self.site.world.life.species[sp];
            let vars = ctx(&[
                ("form", Value::from(s.form)),
                ("colour", Value::from(s.colour)),
                ("mark", Value::from(s.mark)),
                ("water", Value::from(water_id)),
            ]);
            self.note_life(&format!("life-seen:{sp}"));
            self.make_item("fish", true);
            let t = self.say("fish.caught", vars);
            self.output(vec![t], None)
        } else {
            let t = self.say("fish.none", ctx(&[("water", Value::from(water_id))]));
            self.output(vec![t], None)
        }
    }

    /// `snare`: sets a snare here, or checks one set close by.
    // DESIGN-Q: a snare needs no gear, takes 20 minutes to set, and can
    // hold something after six hours; the catch is a small animal that
    // lives there (burrowers, hares and the like).
    pub(crate) fn snare(&mut self) -> Output {
        if self.state.place != Place::Outside {
            return self.outdoors_needed("snare");
        }
        let now = self.state.minutes;
        let p = self.state.pos;
        let near = self
            .state
            .snares
            .iter()
            .position(|(at, _)| at.dist(p) <= 200.0);
        let Some(i) = near else {
            self.advance(20, scraped_sim::body::Activity::Walking);
            self.state.snares.push((p, now));
            let t = self.say("snare.set", ctx(&[]));
            return self.output(vec![t], None);
        };
        self.advance(5, scraped_sim::body::Activity::Walking);
        let (at, set) = self.state.snares[i];
        let hours = now.saturating_sub(set) / 60;
        if hours < 6 {
            let t = self.say("snare.waiting", ctx(&[]));
            return self.output(vec![t], None);
        }
        let saved = self.state.pos;
        self.state.pos = at;
        let prey: Vec<(f64, usize)> = self
            .life_here()
            .into_iter()
            .filter(|l| {
                let s = &self.site.world.life.species[l.species];
                s.role == "burrower" || (s.role == "browser" && s.size != "large")
            })
            .map(|l| (l.plenty, l.species))
            .collect();
        self.state.pos = saved;
        let chance: f64 =
            prey.iter().map(|(p, _)| p * 50.0).sum::<f64>() * (hours.min(24) as f64 / 12.0);
        let roll = hash(&[self.seed(), 0x5a4e, u64::from(set), u64::from(now / 60)]) % 100;
        self.state.snares[i].1 = now;
        if (roll as f64) < chance.min(80.0) && !prey.is_empty() {
            let (_, sp) = prey[(roll as usize) % prey.len()];
            let s = &self.site.world.life.species[sp];
            let vars = ctx(&[
                ("form", Value::from(s.form)),
                ("colour", Value::from(s.colour)),
                ("mark", Value::from(s.mark)),
            ]);
            self.note_life(&format!("life-seen:{sp}"));
            self.make_item("game", true);
            let t = self.say("snare.caught", vars);
            self.output(vec![t], None)
        } else {
            let t = self.say("snare.empty", ctx(&[]));
            self.output(vec![t], None)
        }
    }
}
