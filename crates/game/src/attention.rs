//! The attention model (D02): what a response says, out of everything that
//! could be said.
//!
//! Every description path gathers **candidate facts**, each a single slot
//! with its variables and a **salience** (size, contrast, danger, rarity).
//! The model weighs them by **novelty** against what this player has
//! already been told ([`State::told`]), keeps the best few within the
//! response's **budget**, and lets only **interruptions** (sudden, loud,
//! new or dangerous) break through. Everything left out stays available
//! to digging: `look around`, `look closer`, `listen`, `examine`…
//!
//! Places are seen as people see them: first **the whole** (a ruined town,
//! a crowd of tombs), then **groups** (several worn houses), then
//! **individuals** that stand out (the only temple, the tallest tower).
//! Arrival and `look` start at the whole; digging moves down a level.
//!
//! [`State::told`]: crate::State::told

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use scraped_content::{Context, Value};
use scraped_sim::outdoors::{distance_band, BEARINGS};
use scraped_world::structures::{Condition, StructureKind};
use scraped_world::terrain::Biome;

use crate::site::{ctx, label, time_of_day, Place};
use crate::Game;

/// One thing a response could say.
#[derive(Debug, Clone)]
pub(crate) struct Fact {
    pub slot: &'static str,
    pub vars: Context,
    /// Identity for memory: the same thing gets the same key.
    pub key: String,
    /// How strongly it draws attention, before novelty (0–100).
    pub salience: f64,
    /// Breaks through the budget (sudden, loud, new or dangerous).
    pub interrupt: bool,
    /// Always said in this response, inside the budget (the whole of a
    /// place, which a look starts from).
    pub anchor: bool,
    /// What the rendered names in `vars` stand for, so memory notices a
    /// change in the world but never a change in the prose.
    pub meaning: String,
}

impl Fact {
    pub fn new(slot: &'static str, key: impl Into<String>, salience: f64, vars: Context) -> Self {
        Fact {
            slot,
            vars,
            key: key.into(),
            salience,
            interrupt: false,
            anchor: false,
            meaning: String::new(),
        }
    }

    pub fn anchored(mut self) -> Self {
        self.anchor = true;
        self
    }

    pub fn meaning(mut self, m: String) -> Self {
        self.meaning = m;
        self
    }

    pub fn interrupting(mut self) -> Self {
        self.interrupt = true;
        self
    }

    /// What it says, for noticing change: a hash of its variables, leaving
    /// out those rendered from the pack (names, ways), which `meaning`
    /// stands for instead.
    fn signature(&self) -> u64 {
        let plain: Context = self
            .vars
            .iter()
            .filter(|(k, _)| !RENDERED.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let s = serde_json::to_string(&plain).unwrap_or_default() + &self.meaning;
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in s.bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
    }
}

/// Variables whose values are rendered text.
const RENDERED: [&str; 3] = ["name", "ways", "exits"];

/// What the player has been told about one thing: when, what it said,
/// and whether it was said or only there to be noticed (a fact left out
/// of a response is noticed, so it isn't news the next time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Told {
    pub minutes: u32,
    pub signature: u64,
    #[serde(default)]
    pub said: bool,
}

/// The kinds of response, each with its budget of facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Response {
    /// Arriving somewhere outdoors (a journey's end, stepping outside).
    Arrival,
    /// `look`.
    Look,
    /// Entering a room.
    Room,
    /// The end of a journey, after the report of it.
    Travel,
    /// `look closer`, `search`: down a level.
    Closer,
    /// `look around`: the wider view outdoors.
    Around,
}

impl Response {
    // DESIGN-Q: budgets: arrival 3, look 4, room entry 3, the end of a
    // journey 2 (after the report and whatever stopped it), closer and
    // around 6; a look with nothing new or changed says at most 2.
    pub fn budget(self) -> usize {
        match self {
            Response::Travel => 2,
            Response::Arrival | Response::Room => 3,
            Response::Look => 4,
            Response::Closer | Response::Around => 6,
        }
    }
}

/// A count as people say it without counting.
pub(crate) fn vague(n: usize) -> &'static str {
    match n {
        0 => "none",
        1 => "one",
        2 => "two",
        3..=4 => "a few",
        5..=8 => "several",
        9..=15 => "many",
        16..=40 => "dozens",
        _ => "a crowd",
    }
}

/// Every vague count, for slot variables.
pub const AMOUNTS: [&str; 8] = [
    "none", "one", "two", "a few", "several", "many", "dozens", "a crowd",
];

/// How tall a kind of building stands, in metres (as seen from afar).
fn height_of(k: StructureKind) -> f64 {
    k.info().height
}

/// Evidence of the season, by season, with the land it can be seen in.
/// Each season shows at least three kinds in every land (tested).
// DESIGN-Q: the evidence list and where each shows.
pub const SEASON_EVIDENCE: &[(&str, &str, &[Biome])] = {
    use Biome::*;
    const ALL: &[Biome] = &[
        Shore, Marsh, Grassland, Scrub, Desert, Forest, Pine, Tundra, Rock, Snow,
    ];
    &[
        (
            "spring",
            "blossom",
            &[Forest, Grassland, Scrub, Shore, Marsh],
        ),
        ("spring", "new leaves", &[Forest, Pine, Scrub, Marsh]),
        ("spring", "meltwater", &[Tundra, Rock, Snow, Pine]),
        (
            "spring",
            "birdsong at dawn",
            &[
                Shore, Marsh, Grassland, Scrub, Desert, Forest, Pine, Tundra, Rock,
            ],
        ),
        (
            "spring",
            "young animals",
            &[Grassland, Scrub, Marsh, Shore, Forest, Tundra, Desert],
        ),
        ("spring", "lengthening days", ALL),
        (
            "spring",
            "green shoots",
            &[Grassland, Marsh, Shore, Desert, Scrub],
        ),
        ("spring", "softening snow", &[Snow, Tundra, Rock, Pine]),
        ("summer", "long days", ALL),
        ("summer", "heat shimmer", &[Desert, Scrub, Grassland, Rock]),
        (
            "summer",
            "insects",
            &[Marsh, Forest, Grassland, Shore, Scrub, Tundra, Pine],
        ),
        ("summer", "ripening seed", &[Grassland, Scrub, Forest]),
        ("summer", "dust", &[Desert, Scrub, Grassland, Rock]),
        ("summer", "lingering snow", &[Snow, Rock]),
        ("summer", "glare", &[Snow, Desert, Tundra, Rock]),
        (
            "summer",
            "warm nights",
            &[Shore, Marsh, Grassland, Scrub, Desert, Forest, Pine],
        ),
        ("autumn", "falling leaves", &[Forest, Scrub, Marsh]),
        ("autumn", "seed heads", &[Grassland, Scrub, Marsh, Shore]),
        ("autumn", "birds flying south", ALL),
        (
            "autumn",
            "morning mist",
            &[Marsh, Shore, Forest, Grassland, Pine],
        ),
        ("autumn", "shortening days", ALL),
        ("autumn", "first frost", &[Tundra, Rock, Snow, Pine]),
        (
            "autumn",
            "browning grass",
            &[Grassland, Desert, Scrub, Tundra],
        ),
        ("autumn", "cool nights", &[Desert, Scrub, Rock]),
        ("winter", "frost", ALL),
        (
            "winter",
            "bare branches",
            &[Forest, Scrub, Marsh, Shore, Grassland],
        ),
        ("winter", "snow cover", &[Snow, Tundra, Rock, Pine, Forest]),
        ("winter", "short days", ALL),
        ("winter", "ice at the edges", &[Marsh, Shore, Tundra, Pine]),
        ("winter", "still air", &[Desert, Grassland, Scrub, Rock]),
    ]
};

/// Every season evidence id, for slot variables.
pub fn season_evidence_ids() -> Vec<&'static str> {
    let mut v: Vec<&str> = SEASON_EVIDENCE.iter().map(|e| e.1).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// What a region's state shows on the ground: (aspect, band, evidence).
// DESIGN-Q: which bands show, and how.
pub const REGION_EVIDENCE: &[(&str, &str, &str)] = &[
    ("water", "very_low", "cracked mud"),
    ("water", "low", "dry stream beds"),
    ("water", "high", "sodden ground"),
    ("water", "very_high", "standing water"),
    ("life", "very_low", "bare earth"),
    ("life", "low", "withered growth"),
    ("life", "high", "thick growth"),
    ("life", "very_high", "rampant growth"),
    ("stability", "very_low", "fresh rockfalls"),
    ("stability", "low", "cracked ground"),
    ("climate", "colder", "frost out of season"),
    ("climate", "warmer", "heat haze out of season"),
];

impl Game {
    /// Weighs candidate facts by novelty against what the player has been
    /// told, keeps the best within the budget (and every interruption),
    /// renders them and remembers what was said.
    pub(crate) fn attend(&mut self, facts: Vec<Fact>, response: Response) -> Vec<String> {
        // Facts this response has already said (a move report) count.
        let budget = response
            .budget()
            .saturating_sub(std::mem::take(&mut self.said_already))
            .max(1);
        let now = self.state.minutes;
        let mut scored: Vec<(f64, bool, Fact)> = facts
            .into_iter()
            .map(|f| {
                let sig = f.signature();
                let (score, fresh) = match self.state.told.get(&f.key) {
                    None => (f.salience * 1.6 + 10.0, true),
                    Some(t) if t.signature != sig => (f.salience * 1.4 + 5.0, true),
                    // Noticed before but never said: still worth saying.
                    Some(t) if !t.said => (f.salience, false),
                    // Said before and unchanged: it fades, then returns
                    // as a reminder after a while.
                    Some(t) if now < t.minutes + 360 => (f.salience * 0.25, false),
                    Some(_) => (f.salience * 0.6, false),
                };
                (score, fresh, f)
            })
            .collect();
        // Where the player set out for is weighed first, within the budget.
        let target = |f: &Fact| self.arrival_keys.contains(&f.key);
        scored.sort_by(|a, b| {
            target(&b.2)
                .cmp(&target(&a.2))
                .then(b.2.anchor.cmp(&a.2.anchor))
                .then(b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.2.key.cmp(&b.2.key))
        });
        if self.spoil {
            for (score, fresh, f) in &scored {
                self.attended.push(serde_json::json!({
                    "slot": f.slot,
                    "key": f.key,
                    "salience": f.salience,
                    "score": score,
                    "fresh": fresh,
                    "interrupt": f.interrupt,
                    "vars": f.vars,
                }));
            }
        }
        // Nothing new or changed: say little.
        let anything_new = scored.iter().any(|(_, fresh, _)| *fresh);
        let budget = if anything_new { budget } else { budget.min(2) };
        let mut keep: Vec<Fact> = Vec::new();
        let mut taken = 0;
        let mut per_slot: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut noticed: Vec<(String, u64)> = Vec::new();
        for (_, _, f) in scored {
            // A short response says one of each kind of thing.
            let cap = match response {
                Response::Closer | Response::Around => 6,
                _ => 1,
            };
            let room = per_slot.get(f.slot).copied().unwrap_or(0) < cap;
            // Where the player set out for is always said (S01).
            let target = self.arrival_keys.contains(&f.key);
            if f.interrupt || target || (taken < budget && room) {
                *per_slot.entry(f.slot).or_default() += 1;
                if !f.interrupt {
                    taken += 1;
                }
                self.note_life(&f.key);
                keep.push(f);
            } else {
                noticed.push((f.key.clone(), f.signature()));
            }
        }
        for (key, signature) in noticed {
            let said = self.state.told.get(&key).is_some_and(|t| t.said);
            self.state.told.insert(
                key,
                Told {
                    minutes: now,
                    signature,
                    said,
                },
            );
        }
        // Said in a natural order: the place, then what's close, then what's
        // far, then the sky, the air and the rest.
        // Where the player set out for comes first.
        let keys = std::mem::take(&mut self.arrival_keys);
        keep.sort_by_key(|f| (!keys.contains(&f.key), order(f.slot)));
        let mut out = Vec::new();
        for f in keep {
            let sig = f.signature();
            self.state.told.insert(
                f.key.clone(),
                Told {
                    minutes: now,
                    signature: sig,
                    said: true,
                },
            );
            let t = self.say(f.slot, f.vars);
            let t = sentence(t.trim());
            if !t.is_empty() {
                out.push(t);
            }
        }
        out
    }

    /// Notes facts as said without saying them (already said another way).
    fn remember_said<'a>(&mut self, facts: impl Iterator<Item = &'a Fact>) {
        let now = self.state.minutes;
        let told: Vec<(String, u64)> = facts.map(|f| (f.key.clone(), f.signature())).collect();
        for (key, signature) in told {
            self.state.told.insert(
                key,
                Told {
                    minutes: now,
                    signature,
                    said: true,
                },
            );
        }
    }

    /// Describes where the player is, for a kind of response.
    pub(crate) fn describe(&mut self, response: Response) -> String {
        self.describe_except(response, None)
    }

    /// Describes where the player is, leaving out a fact just said another
    /// way (the landmark that stopped a journey).
    pub(crate) fn describe_except(&mut self, response: Response, skip: Option<&str>) -> String {
        let mut facts = match self.state.place {
            Place::Outside => self.outdoor_facts(response),
            Place::Room { .. } => self.room_facts(response),
        };
        if let Some(skip) = skip {
            self.remember_said(facts.iter().filter(|f| f.key == skip));
            facts.retain(|f| f.key != skip);
        }
        let parts = self.attend(facts, response);
        parts.join(" ")
    }

    // ---------- outdoors ----------

    /// Buildings in sight close by: those of the town here, or lone ones.
    fn whole_and_standouts(&mut self, response: Response, out: &mut Vec<Fact>) {
        let structures = self.local_structures();
        if structures.is_empty() {
            return;
        }
        let town = self.town_here();
        let w = &self.site.world;
        let mut kinds: BTreeMap<StructureKind, usize> = BTreeMap::new();
        let mut conditions: BTreeMap<Condition, usize> = BTreeMap::new();
        for &s in &structures {
            *kinds.entry(w.structures[s].kind).or_default() += 1;
            *conditions.entry(w.structures[s].condition).or_default() += 1;
        }
        let mut by_count: Vec<(usize, StructureKind)> =
            kinds.iter().map(|(k, n)| (*n, *k)).collect();
        by_count.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let condition = conditions
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
            .map_or(Condition::Intact, |(c, _)| *c);
        let role = town
            .and_then(|t| self.site.world.towns.get(t))
            .map_or("", |t| t.role.id());
        let (settlement, size, abandoned) = match town {
            Some(t) => {
                let st = &w.history.settlements[t];
                (
                    if st.abandoned.is_some() {
                        "ruins"
                    } else {
                        "town"
                    },
                    i64::from(st.size),
                    st.abandoned.is_some(),
                )
            }
            None => ("none", 0, false),
        };
        let key = match town {
            Some(t) => format!("whole:town:{t}"),
            None => format!("whole:{}", structures[0]),
        };
        let first = by_count[0];
        let second = by_count.get(1).copied();
        let vars = ctx(&[
            ("settlement", Value::from(settlement)),
            ("role", Value::from(role)),
            ("size", Value::Number(size)),
            ("abandoned", Value::Bool(abandoned)),
            ("amount", Value::from(vague(structures.len()))),
            ("main", Value::from(label(&first.1))),
            ("main_amount", Value::from(vague(first.0))),
            (
                "second",
                Value::from(second.map_or(String::new(), |s| label(&s.1))),
            ),
            (
                "second_amount",
                Value::from(second.map_or("none", |s| vague(s.0))),
            ),
            ("kinds", Value::Number(kinds.len() as i64)),
            ("condition", Value::from(label(&condition))),
            ("biome", Value::from(self.site.biome_at(self.state.pos))),
        ]);
        let whole = Fact::new("place.whole", key.clone(), 70.0, vars);
        out.push(if response == Response::Look {
            whole.anchored()
        } else {
            whole
        });
        // Standouts: tall, alone of their kind, or unlike the rest.
        let here_district = self.district_here().map(|(_, d)| d);
        let w = &self.site.world;
        let mut ranked: Vec<(f64, usize, &'static str)> = structures
            .iter()
            .map(|&s| {
                let st = &w.structures[s];
                let mut score = height_of(st.kind) * 2.0;
                let mut why = "tallest";
                // The only one of its kind stands out by how striking that
                // kind is (a palace more than a bakehouse), and buildings
                // in the part of town underfoot more than far ones (D03).
                if kinds[&st.kind] == 1 && structures.len() > 2 {
                    score += 8.0 + st.kind.info().weight;
                    why = "only";
                }
                if here_district.is_some() && st.district == here_district {
                    score += 6.0;
                }
                if st.condition != condition {
                    score += 8.0;
                    if why == "tallest" {
                        why = if st.condition < condition {
                            "best kept"
                        } else {
                            "worst kept"
                        };
                    }
                }
                if self.state.visited.contains(&s) {
                    score += 6.0;
                }
                (score, s, why)
            })
            .collect();
        ranked.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
        });
        let n = if response == Response::Closer { 4 } else { 2 };
        for (score, s, why) in ranked.into_iter().take(n) {
            if score < 20.0 && response != Response::Closer {
                continue;
            }
            let name = self.structure_name(s);
            let st = &self.site.world.structures[s];
            let vars = ctx(&[
                ("name", Value::from(name)),
                ("kind", Value::from(label(&st.kind))),
                ("condition", Value::from(label(&st.condition))),
                ("why", Value::from(why)),
            ]);
            out.push(Fact::new(
                "place.standout",
                format!("standout:{s}"),
                score.min(60.0),
                vars,
            ));
        }
        // Down a level: the groups.
        if response == Response::Closer {
            for (n, k) in &by_count {
                if *n < 2 {
                    continue;
                }
                let members: Vec<usize> = structures
                    .iter()
                    .copied()
                    .filter(|&s| self.site.world.structures[s].kind == *k)
                    .collect();
                let mut conds: BTreeMap<Condition, usize> = BTreeMap::new();
                for &s in &members {
                    *conds
                        .entry(self.site.world.structures[s].condition)
                        .or_default() += 1;
                }
                let mostly = conds
                    .iter()
                    .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                    .map_or(Condition::Intact, |(c, _)| *c);
                let vars = ctx(&[
                    ("kind", Value::from(label(k))),
                    ("amount", Value::from(vague(*n))),
                    ("condition", Value::from(label(&mostly))),
                    ("mixed", Value::Bool(conds.len() > 1)),
                ]);
                out.push(Fact::new(
                    "place.group",
                    format!("group:{}:{}", key, label(k)),
                    30.0 + *n as f64,
                    vars,
                ));
            }
        }
    }

    /// Everything outdoors that could be said.
    pub(crate) fn outdoor_facts(&mut self, response: Response) -> Vec<Fact> {
        let mut out = Vec::new();
        let (weather, light, range) = self.conditions();
        let time = time_of_day(self.state.minutes);
        let pos = self.state.pos;
        let biome = self.site.biome_at(pos);
        self.whole_and_standouts(response, &mut out);
        self.place_facts(response, &mut out);
        // Things and mechanisms standing in the open.
        if response == Response::Closer || self.town_here().is_none() {
            for t in self.here() {
                let name = self.thing_name(t);
                let vars = ctx(&[
                    ("name", Value::from(name)),
                    ("kind", Value::from(self.thing(t).kind)),
                ]);
                out.push(Fact::new("place.thing", format!("thing:{t}"), 22.0, vars));
            }
            for m in self.mechanisms_here() {
                let name = self.mech_name(m);
                let vars = ctx(&[
                    ("name", Value::from(name)),
                    (
                        "kind",
                        Value::from(label(&self.site.fixtures.mechanisms[m].kind)),
                    ),
                ]);
                out.push(Fact::new("place.thing", format!("mech:{m}"), 28.0, vars));
            }
        }
        // The ground underfoot.
        let terrain = self.site.land.terrain_here(pos);
        let high = self.site.land.high(pos) && range >= 5000.0;
        out.push(Fact::new(
            "land.ground",
            "ground",
            if response == Response::Closer {
                30.0
            } else {
                18.0
            },
            ctx(&[
                ("biome", Value::from(biome.clone())),
                ("terrain", Value::from(terrain)),
                ("high", Value::Bool(high)),
            ]),
        ));
        // Edges: roads and water, by how close.
        for (e, b) in self.site.land.edges_near(pos) {
            let name = self.edge_name(e);
            let kind = scraped_sim::outdoors::EDGES[e];
            let here = b.is_none();
            let water = !matches!(kind, "road" | "treeline");
            let sal = 14.0 + if here { 14.0 } else { 0.0 } + if water { 6.0 } else { 0.0 };
            out.push(Fact::new(
                "land.edge",
                format!("edge:{e}"),
                sal,
                ctx(&[
                    ("name", Value::from(name)),
                    ("kind", Value::from(kind)),
                    ("side", Value::from(b.map_or("here", |b| BEARINGS[b]))),
                ]),
            ));
        }
        // What stands out further off.
        let view = self.in_view();
        self.see_around();
        let many = if response == Response::Around { 6 } else { 3 };
        for (rank, v) in view.iter().take(many).enumerate() {
            let name = self.landmark_name(v.landmark);
            let kind = self.site.land.landmarks[v.landmark].kind;
            let vars = ctx(&[
                ("name", Value::from(name)),
                ("kind", Value::from(kind)),
                ("bearing", Value::from(BEARINGS[v.bearing])),
                ("distance", Value::from(distance_band(v.metres))),
            ]);
            let sal = (30.0 - rank as f64 * 8.0).max(8.0)
                + if response == Response::Around {
                    15.0
                } else {
                    0.0
                };
            out.push(Fact::new(
                "land.landmark",
                format!("landmark:{}", v.landmark),
                sal,
                vars,
            ));
        }
        if view.is_empty() && (weather != "clear" || light != "daylight") {
            out.push(Fact::new(
                "land.unseen",
                "horizon",
                12.0,
                ctx(&[
                    ("weather", Value::from(weather)),
                    ("light", Value::from(light)),
                ]),
            ));
        }
        if high && response == Response::Around {
            let (biomes, sea) = self.site.land.region(&self.site.world, pos, range);
            let main = biomes.first().copied().unwrap_or("grassland");
            out.push(Fact::new(
                "land.region",
                "region-view",
                40.0,
                ctx(&[
                    (
                        "biomes",
                        Value::List(biomes.iter().map(|b| Value::from(*b)).collect()),
                    ),
                    ("main", Value::from(main)),
                    ("shape", Value::from(label(&self.site.world.terrain.shape))),
                    ("sea", Value::Bool(sea)),
                ]),
            ));
        }
        // The sky: said when it changes or matters.
        let notable = weather != "clear";
        out.push(Fact::new(
            "sky.weather",
            "weather",
            if notable { 26.0 } else { 6.0 },
            ctx(&[
                ("weather", Value::from(weather)),
                ("light", Value::from(light)),
                ("time", Value::from(time)),
            ]),
        ));
        self.felt_facts(&mut out);
        self.outdoor_evidence(&mut out);
        self.creature_facts(&mut out);
        self.life_facts(response, &mut out);
        self.weather_facts(response, &mut out);
        out
    }

    /// What the skin and the air say: temperature, wind, wet, and
    /// strangeness.
    pub(crate) fn felt_facts(&mut self, out: &mut Vec<Fact>) {
        let spot = self.spot();
        let local = self
            .env()
            .local(spot, self.state.minutes, self.carried_light());
        let indoors = !matches!(self.state.place, Place::Outside);
        let temp = temperature_band(local.temperature);
        let extreme = matches!(temp, "freezing" | "hot");
        let sal = match temp {
            "freezing" | "hot" => 45.0,
            "cold" | "warm" => 18.0,
            _ => 4.0,
        };
        out.push(Fact::new(
            "air.felt",
            "air:temperature",
            sal,
            ctx(&[
                ("temperature", Value::from(temp)),
                ("indoors", Value::Bool(indoors)),
                ("extreme", Value::Bool(extreme)),
            ]),
        ));
        if local.air != "still" {
            out.push(Fact::new(
                "air.moving",
                "air:moving",
                if local.air == "windy" { 22.0 } else { 14.0 },
                ctx(&[
                    ("air", Value::from(local.air)),
                    ("indoors", Value::Bool(indoors)),
                ]),
            ));
        }
        if matches!(local.wetness, "wet" | "flooded") {
            out.push(Fact::new(
                "ground.wet",
                "ground:wet",
                if local.wetness == "flooded" {
                    40.0
                } else {
                    16.0
                },
                ctx(&[
                    ("wetness", Value::from(local.wetness)),
                    ("indoors", Value::Bool(indoors)),
                ]),
            ));
        }
        if local.unstable {
            out.push(
                Fact::new("danger.unstable", "danger:unstable", 70.0, Context::new())
                    .interrupting(),
            );
        }
        if local.fire {
            out.push(Fact::new(
                "fire.near",
                "fire:near",
                35.0,
                ctx(&[("indoors", Value::Bool(indoors))]),
            ));
        }
        let uncanny = self.uncanny();
        if uncanny != "none" {
            out.push(Fact::new(
                "air.uncanny",
                format!("uncanny:{uncanny}"),
                42.0,
                ctx(&[
                    ("kind", Value::from(uncanny)),
                    ("indoors", Value::Bool(indoors)),
                ]),
            ));
        }
    }

    /// Evidence of the season and of the region's state, never their
    /// names.
    pub(crate) fn outdoor_evidence(&mut self, out: &mut Vec<Fact>) {
        let pos = self.state.pos;
        let (x, y) = pos.cell();
        let biome = *self.site.world.terrain.biome.get(x, y);
        let season = scraped_sim::region::SEASONS[scraped_sim::region::season(self.state.minutes)];
        let shown: Vec<&str> = SEASON_EVIDENCE
            .iter()
            .filter(|(s, _, lands)| *s == season && lands.contains(&biome))
            .map(|(_, e, _)| *e)
            .collect();
        if !shown.is_empty() {
            let day = u64::from(self.state.minutes / 1440);
            let h = scraped_sim::outdoors::hash(&[self.seed(), 0x5ea5, day, x as u64, y as u64]);
            let e = shown[(h % shown.len() as u64) as usize];
            out.push(Fact::new(
                "evidence.season",
                format!("season:{e}"),
                20.0,
                ctx(&[
                    ("evidence", Value::from(e)),
                    ("biome", Value::from(label(&biome))),
                    ("time", Value::from(time_of_day(self.state.minutes))),
                ]),
            ));
        }
        if let Some(bands) = self.env().region_bands(pos) {
            for (i, aspect) in scraped_sim::region::VARIABLES.iter().enumerate() {
                let band = bands[i];
                let Some((_, _, e)) = REGION_EVIDENCE
                    .iter()
                    .find(|(a, b, _)| a == aspect && *b == band)
                else {
                    continue;
                };
                let extreme = band.starts_with("very") || aspect == &"climate";
                out.push(Fact::new(
                    "evidence.region",
                    format!("region:{aspect}"),
                    if extreme { 34.0 } else { 20.0 },
                    ctx(&[
                        ("evidence", Value::from(*e)),
                        ("aspect", Value::from(*aspect)),
                        ("biome", Value::from(label(&biome))),
                    ]),
                ));
            }
        }
    }

    /// Creatures in sight: closer and more dangerous draw the eye.
    fn creature_facts(&mut self, out: &mut Vec<Fact>) {
        let pos = self.state.pos;
        for i in self.creatures_in_view() {
            let name = self.creature_name(i);
            let at = self.state.creatures[i].pos;
            let archetype = self.site.fixtures.creatures[i].archetype;
            let d = pos.dist(at);
            let band = distance_band(d);
            let dangerous = matches!(archetype, "predator" | "deep");
            let mut f = Fact::new(
                "creature.seen",
                format!("creature:{i}"),
                if band == "near" { 50.0 } else { 26.0 },
                ctx(&[
                    ("name", Value::from(name)),
                    ("archetype", Value::from(archetype)),
                    (
                        "bearing",
                        Value::from(
                            scraped_sim::outdoors::bearing(pos, at)
                                .map_or("north", |b| BEARINGS[b]),
                        ),
                    ),
                    ("distance", Value::from(band)),
                ]),
            );
            if dangerous && band == "near" {
                f = f.interrupting();
            }
            out.push(f);
        }
    }

    // ---------- rooms ----------

    /// Everything in a room that could be said.
    pub(crate) fn room_facts(&mut self, response: Response) -> Vec<Fact> {
        let Place::Room { structure, room } = self.state.place else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let st = self.site.structure(structure);
        let r = st.interior.rooms[room].clone();
        let (purpose, level, kind, condition) =
            (r.purpose, r.level, label(&st.kind), label(&st.condition));
        let dark = self.is_dark();
        let all = self.ways();
        let shape = all
            .iter()
            .map(|w| format!("{}:{:?}:{}", w.link, w.state, w.collapsed))
            .collect::<Vec<_>>()
            .join(",");
        let mut ways: Vec<Value> = all.iter().map(|w| Value::from(self.way_name(w))).collect();
        if room == 0 {
            ways.push(Value::from(self.stable("place.out", Context::new(), 7)));
        }
        if dark {
            out.push(
                Fact::new(
                    "place.dark",
                    format!("dark:{structure}:{room}"),
                    90.0,
                    ctx(&[
                        ("level", Value::Number(i64::from(level))),
                        ("exits", Value::List(ways)),
                    ]),
                )
                .meaning(shape),
            );
            self.felt_facts(&mut out);
            return out;
        }
        let light = self
            .env()
            .local(self.spot(), self.state.minutes, self.carried_light())
            .light;
        let anchor = response == Response::Look;
        out.push(Fact {
            anchor,
            ..Fact::new(
                "room.whole",
                format!("room:{structure}:{room}"),
                80.0,
                ctx(&[
                    ("purpose", Value::from(purpose)),
                    ("structure", Value::from(kind)),
                    ("condition", Value::from(condition)),
                    ("level", Value::Number(i64::from(level))),
                    ("light", Value::from(light)),
                    ("time", Value::from(time_of_day(self.state.minutes))),
                    ("space", Value::from(r.space)),
                    ("size", Value::from(crate::quiet_slots::size_band(r.area()))),
                    ("height", Value::from(crate::quiet_slots::height_band(r.h))),
                    ("style", Value::from(r.style)),
                    (
                        "water",
                        Value::from(if r.water.is_empty() { "none" } else { r.water }),
                    ),
                    ("landmark", Value::from(r.landmark)),
                ]),
            )
        });
        out.push(
            Fact::new(
                "room.ways",
                format!("ways:{structure}:{room}"),
                55.0,
                ctx(&[("ways", Value::List(ways))]),
            )
            .meaning(shape),
        );
        // Things: alike ones together, single ones by how they stand out.
        let things = self.here();
        let mut by_kind: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();
        for &t in &things {
            by_kind.entry(self.thing(t).kind).or_default().push(t);
        }
        for (k, ts) in by_kind {
            if ts.len() >= 2 && response != Response::Closer {
                let vars = ctx(&[
                    ("kind", Value::from(k)),
                    ("amount", Value::from(vague(ts.len()))),
                    ("material", Value::from(label(&self.thing(ts[0]).material))),
                ]);
                out.push(Fact::new(
                    "room.group",
                    format!("rgroup:{structure}:{room}:{k}"),
                    30.0 + ts.len() as f64,
                    vars,
                ));
                continue;
            }
            for t in ts {
                let name = self.thing_name(t);
                let portable = self.thing(t).portable
                    || scraped_sim::items::kind(self.thing(t).kind).is_some();
                let sal = if portable { 16.0 } else { 34.0 };
                out.push(Fact::new(
                    "room.thing",
                    format!("thing:{t}"),
                    sal,
                    ctx(&[
                        ("name", Value::from(name)),
                        ("kind", Value::from(k)),
                        ("item", Value::Bool(portable)),
                    ]),
                ));
            }
        }
        for m in self.mechanisms_here() {
            let name = self.mech_name(m);
            out.push(Fact::new(
                "room.thing",
                format!("mech:{m}"),
                32.0,
                ctx(&[
                    ("name", Value::from(name)),
                    (
                        "kind",
                        Value::from(label(&self.site.fixtures.mechanisms[m].kind)),
                    ),
                    ("item", Value::Bool(false)),
                ]),
            ));
        }
        self.room_scene_facts(response, &mut out);
        self.interior_facts(response, &mut out);
        self.cache_facts(response == Response::Closer, &mut out);
        self.felt_facts(&mut out);
        if self.great_here() {
            self.hook("great_reached", "", "");
            out.push(Fact::new("great.site", "great", 85.0, Context::new()).interrupting());
        }
        out
    }
}

/// Where a kind of fact comes in a response.
fn order(slot: &str) -> u8 {
    match slot {
        "place.dark" | "room.whole" | "place.whole" => 0,
        "place.district" | "place.layout" => 1,
        "place.scene" => 2,
        "land.feature" => 4,
        "place.standout" | "place.group" | "room.group" | "room.thing" | "place.thing" => 1,
        "great.site" => 2,
        "room.ways" => 3,
        "land.ground" | "land.edge" | "ground.wet" => 4,
        "land.landmark" | "land.unseen" | "land.region" => 5,
        "sky.weather" => 6,
        "air.felt" | "air.moving" | "air.uncanny" | "fire.near" => 7,
        "evidence.season" | "evidence.region" => 8,
        "creature.seen" => 9,
        _ => 10,
    }
}

/// A fact as a sentence: capitalised, and ending in a stop. Phrase slots
/// (a landmark, an edge) also stand alone as facts.
pub(crate) fn sentence(t: &str) -> String {
    let t = t.trim();
    // A piece whose variant said nothing (only its stop) says nothing.
    if !t.chars().any(char::is_alphanumeric) {
        return String::new();
    }
    let mut c = t.chars();
    let first = c
        .next()
        .map(|f| f.to_uppercase().collect::<String>())
        .unwrap_or_default();
    let mut s = first + c.as_str();
    if !s.ends_with(['.', '!', '?', ':', '"', '\u{2019}']) {
        s.push('.');
    }
    s
}

/// A felt temperature band.
pub(crate) fn temperature_band(t: f64) -> &'static str {
    match t {
        t if t < 0.0 => "freezing",
        t if t < 8.0 => "cold",
        t if t < 15.0 => "cool",
        t if t < 22.0 => "mild",
        t if t < 28.0 => "warm",
        _ => "hot",
    }
}
