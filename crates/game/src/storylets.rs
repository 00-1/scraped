//! Storylets: Jb's hand-written events and places, woven into generated
//! worlds (M12).
//!
//! A storylet is content (`[[storylet]]` in the pack). At world generation
//! the game places each one that needs a place, deterministically from the
//! seed and its id, and brings any writing it asks for into the world in
//! the world's own language. In play it triggers when the player is where
//! it was placed (or at a beat of the spine) and its condition holds; its
//! text is the slot `story.<id>` and its effects go through the ordinary
//! game systems.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use scraped_content::lint::{Issue, Severity};
use scraped_content::template::{expr_variables, parse_expr};
use scraped_content::{Context, Pack, SlotDef, Storylet, Value, VarType};
use scraped_lang::concepts::{self, Pos as PartOfSpeech};
use scraped_lang::corpus::Kind;
use scraped_lang::meaning::{Argument, Clause, Mood, NounPhrase, Polarity, Role, Sentence, Tense};
use scraped_sim::outdoors::{hash, Pos};
use scraped_sim::region::SEASONS;
use scraped_sim::writing::{Surface, Table};
use scraped_world::structures::{Material, PassageState};
use scraped_world::texts::Text;
use scraped_world::World;

use crate::site::{ctx, label, time_of_day, Place, Site, Thing};
use crate::slots::{BANDS, BIOMES};
use crate::Game;

/// Where a storylet can happen.
pub const AT: &[&str] = &["anywhere", "structure", "outdoors", "hook"];

/// Beats of the spine the game announces, for storylets with `at = "hook"`.
pub const HOOKS: &[&str] = &[
    "opening",
    "first_scraped_seen",
    "tool_found",
    "first_release",
    "first_write",
    "great_reached",
    "deepest_found",
    "ending",
];

/// Building kinds a storylet can be placed in (every kind, D03).
pub const STRUCTURES: &[&str] = crate::slots::STRUCTURES;

/// Every kind of scene a storylet can be placed at (D03).
pub fn scene_kinds() -> Vec<&'static str> {
    scraped_world::scenes::KINDS
        .iter()
        .map(|(k, _)| *k)
        .collect()
}

pub const ERAS: &[&str] = &["old", "middle", "new"];
pub const EFFECTS: &[&str] = &["give", "flag", "unflag", "open"];
pub const REGISTERS: &[&str] = &["potent", "everyday"];

/// How near (metres) the player must come to an outdoor storylet.
// DESIGN-Q: 300 m, about one travel step.
pub const NEAR: f64 = 300.0;

/// The variables every storylet's text and condition can use.
pub fn variables() -> Vec<(&'static str, VarType, &'static str)> {
    let e = |v: &[&str]| VarType::Enum {
        values: v.iter().map(|s| s.to_string()).collect(),
    };
    let places = [&["outside"][..], STRUCTURES].concat();
    let eras = [ERAS, &["none"]].concat();
    vec![
        ("place", e(&places), "Where the player is: outside, or the kind of building."),
        ("biome", e(BIOMES), "The land here."),
        ("era", e(&eras), "When the building here was raised (none outdoors)."),
        ("indoors", VarType::Bool, "Whether the player is indoors."),
        ("day", VarType::Number, "Day of the run, from 1."),
        ("season", e(&SEASONS), "The season."),
        ("time", e(crate::slots::TIMES), "Time of day."),
        ("age", VarType::Number, "The player's age in years."),
        ("life", e(BANDS), "How much grows in this region."),
        ("water", e(BANDS), "How much water this region has."),
        ("stability", e(BANDS), "How sound the ground is here."),
        ("climate", e(&["colder", "usual", "warmer"]), "Colder or warmer than the land should be."),
        ("carrying", VarType::List, "Kinds of thing carried (item ids: torch, scraper, lens…). Test with: carrying has 'torch'."),
        ("happened", VarType::List, "Storylets that have already happened, by id."),
        ("flags", VarType::List, "Flags set by storylets' effects."),
        ("tool", VarType::Text, "At the tool_found beat: which tool (scraper, stylus, lens, fine_scraper, old_scraper, first_scraper, first_lens); otherwise empty."),
        ("ending", VarType::Text, "At the ending beat: how the run ended (death, left, written_in, old_age, overtaken); otherwise empty."),
        ("read", VarType::Number, "Texts the player has read."),
        ("written", VarType::Number, "Texts the player has written."),
        ("released", VarType::Number, "Texts the player has scraped."),
        ("inscription", VarType::Text, "The name of the thing bearing this storylet's generated writing, if it asked for some."),
    ]
}

/// The slot holding a storylet's text, declared from the storylet itself.
pub fn slot(s: &Storylet) -> SlotDef {
    let mut d = SlotDef::new(
        &format!("story.{}", s.id),
        &format!("Storylet '{}': {}", s.id, s.about),
    )
    .min_variants(1)
    .max_len(1200)
    .sampler(sample);
    for (name, ty, about) in variables() {
        d = d.var(name, ty, about);
    }
    d
}

fn sample(_: u64) -> Vec<Context> {
    let list = |v: &[&str]| Value::List(v.iter().map(|s| Value::from(*s)).collect());
    let row = |place: &str, biome: &str, era: &str, day: i64, tool: &str, ending: &str| {
        ctx(&[
            ("place", Value::from(place)),
            ("biome", Value::from(biome)),
            ("era", Value::from(era)),
            ("indoors", Value::Bool(place != "outside")),
            ("day", Value::Number(day)),
            ("season", Value::from("spring")),
            ("time", Value::from("morning")),
            ("age", Value::Number(25)),
            ("life", Value::from("middling")),
            ("water", Value::from("low")),
            ("stability", Value::from("high")),
            ("climate", Value::from("usual")),
            ("carrying", list(&["torch", "scraper"])),
            ("happened", list(&[])),
            ("flags", list(&[])),
            ("tool", Value::from(tool)),
            ("ending", Value::from(ending)),
            ("read", Value::Number(3)),
            ("written", Value::Number(0)),
            ("released", Value::Number(1)),
            ("inscription", Value::from("a weathered stele")),
        ])
    };
    vec![
        row("temple", "grassland", "old", 2, "", ""),
        row("outside", "desert", "none", 9, "lens", ""),
        row("tower", "forest", "new", 40, "", "left"),
    ]
}

/// Where a storylet was placed in this world.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Placed {
    pub id: String,
    pub structure: usize,
    pub pos: Pos,
    /// The thing bearing its generated writing.
    pub thing: Option<usize>,
    pub text: Option<usize>,
}

/// What storylets have done in this run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoryState {
    /// Storylets that have happened, in order.
    pub happened: Vec<String>,
    pub flags: BTreeSet<String>,
}

fn era_index(w: &World, era: &str, own: u32) -> u32 {
    let last = (w.languages.len() as u32).saturating_sub(1);
    match era {
        "old" => 0,
        "new" => last,
        "middle" => last / 2,
        _ => own.min(last),
    }
}

/// Which era band a building belongs to.
pub fn era_band(w: &World, era: u32) -> &'static str {
    let last = (w.languages.len() as u32).saturating_sub(1);
    if era == 0 {
        "old"
    } else if era >= last {
        "new"
    } else {
        "middle"
    }
}

fn near_water(w: &World, cell: (usize, usize)) -> bool {
    let (cx, cy) = cell;
    let side = w.terrain.biome.size;
    (cx.saturating_sub(2)..=(cx + 2).min(side - 1)).any(|x| {
        (cy.saturating_sub(2)..=(cy + 2).min(side - 1)).any(|y| {
            let b = label(w.terrain.biome.get(x, y));
            b == "lake" || b == "marsh" || w.water.is_river(&w.terrain, x, y)
        })
    })
}

/// Buildings a storylet could be placed at, best first (deterministic).
pub fn candidates(w: &World, start: usize, s: &Storylet) -> Vec<usize> {
    let p = &s.place;
    let mut out: Vec<usize> = w
        .structures
        .iter()
        .filter(|st| st.condition != scraped_world::structures::Condition::Buried)
        .filter(|st| p.structure.is_empty() || p.structure.contains(&label(&st.kind)))
        .filter(|st| {
            p.biome.is_empty()
                || p.biome
                    .contains(&label(w.terrain.biome.get(st.cell.ux(), st.cell.uy())))
        })
        .filter(|st| p.era.as_deref().is_none_or(|e| era_band(w, st.era) == e))
        .filter(|st| !p.away || st.settlement != Some(start))
        .filter(|st| !p.near_water || near_water(w, (st.cell.ux(), st.cell.uy())))
        .filter(|st| s.at != "structure" || !st.interior.rooms.is_empty())
        .filter(|st| p.scene.is_empty() || scene_room(w, st.id, &p.scene).is_some())
        .map(|st| st.id)
        .collect();
    let key =
        s.id.bytes()
            .fold(0u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
    out.sort_by_key(|&id| (hash(&[w.seed, 0x5701, key, id as u64]), id));
    out
}

/// The room of a building holding a scene of one of these kinds (D03).
pub fn scene_room(w: &World, structure: usize, kinds: &[String]) -> Option<usize> {
    w.scenes.iter().find_map(|sc| match sc.at {
        scraped_world::scenes::SceneAt::Room { structure: s, room }
            if s == structure && kinds.iter().any(|k| k == sc.kind) =>
        {
            Some(room)
        }
        _ => None,
    })
}

/// The meaning of a storylet's generated writing.
fn inscription_meaning(seed: u64, s: &Storylet, register: &str, about: &str) -> Sentence {
    // Core nouns only: every language has them (D07's cultural words vary).
    let nouns: Vec<&concepts::Concept> = concepts::with_pos(PartOfSpeech::Noun)
        .filter(|c| !c.has_tag("departure"))
        .filter(|c| c.id == about || c.has_tag(about))
        .collect();
    let h = hash(&[
        seed,
        0x1a5c,
        s.id.len() as u64,
        s.id.bytes().map(u64::from).sum(),
    ]);
    let noun = nouns
        .get((h % nouns.len().max(1) as u64) as usize)
        .map_or("stone", |c| c.id.as_str());
    let negative = (h >> 16) & 1 == 1;
    let polarity = if negative {
        Polarity::Negative
    } else {
        Polarity::Positive
    };
    if register == "potent" {
        let verb = ["open", "burn", "break"]
            .into_iter()
            .find(|v| Table::get().effect(v, noun, false).is_some())
            .unwrap_or("burn");
        Sentence::Clause(Clause {
            predicate: verb.to_string(),
            mood: Mood::Potent,
            tense: Tense::NonPast,
            polarity,
            args: vec![Argument {
                role: Role::Subject,
                np: NounPhrase::concept(noun),
            }],
            adverbs: Vec::new(),
            aspect: Default::default(),
            subordinate: Vec::new(),
            complement: None,
        })
    } else {
        let n = concepts::get(noun);
        let verbs: Vec<&concepts::Concept> = concepts::with_pos(PartOfSpeech::Verb)
            .filter(|v| !v.objects.is_empty() && v.accepts_object(n))
            .filter(|v| !matches!(v.id.as_str(), "make" | "bring" | "give"))
            .collect();
        let verb = verbs
            .get(((h >> 8) % verbs.len().max(1) as u64) as usize)
            .map_or("touch", |v| v.id.as_str());
        Sentence::Clause(Clause {
            predicate: verb.to_string(),
            mood: Mood::Imperative,
            tense: Tense::NonPast,
            polarity,
            args: vec![Argument {
                role: Role::Object,
                np: NounPhrase::concept(noun),
            }],
            adverbs: Vec::new(),
            aspect: Default::default(),
            subordinate: Vec::new(),
            complement: None,
        })
    }
}

/// Places every storylet that needs a place, and brings their writing into
/// the world. Deterministic from the seed and the pack.
pub fn place(site: &mut Site, pack: &Pack) -> Vec<Placed> {
    let mut out = Vec::new();
    let start = site.settlement;
    let from = site.start();
    let mut taken: BTreeSet<usize> = BTreeSet::new();
    let storylets: Vec<Storylet> = pack.storylets().cloned().collect();
    for s in &storylets {
        if s.at != "structure" && s.at != "outdoors" {
            continue;
        }
        let w = &site.world;
        let reachable = |sid: usize| {
            (s.at == "outdoors"
                || !site.fixtures.reachable_rooms(w, sid).is_empty() && site.enterable(sid))
                && site
                    .land
                    .route(w, from, site.land.structure_pos[sid])
                    .is_some()
        };
        let Some(sid) = candidates(w, start, s)
            .into_iter()
            .filter(|c| !taken.contains(c))
            .take(12)
            .find(|&c| reachable(c))
        else {
            continue;
        };
        taken.insert(sid);
        let pos = site.land.structure_pos[sid];
        let mut placed = Placed {
            id: s.id.clone(),
            structure: sid,
            pos,
            thing: None,
            text: None,
        };
        if let Some(req) = &s.inscription {
            let st = &site.world.structures[sid];
            let era = era_index(&site.world, req.era.as_deref().unwrap_or(""), st.era);
            let meaning = inscription_meaning(site.world.seed, s, &req.register, &req.about);
            let (home, room) = if s.at == "structure" {
                let reachable = site.fixtures.reachable_rooms(&site.world, sid);
                // At its scene, when it asks for one and the room can be
                // reached; else the furthest room.
                let room = scene_room(&site.world, sid, &s.place.scene)
                    .filter(|r| reachable.contains(r))
                    .unwrap_or(*reachable.last().unwrap_or(&0));
                (
                    Place::Room {
                        structure: sid,
                        room,
                    },
                    Some(room),
                )
            } else {
                (Place::Outside, None)
            };
            let id = site.writing.count(&site.world);
            let year = site
                .world
                .history
                .eras
                .get(era as usize)
                .map_or(0, |e| e.end - 1);
            site.writing.extra.push(Text {
                id,
                era,
                year,
                kind: if req.register == "potent" {
                    Kind::Potent
                } else {
                    Kind::Warning
                },
                meaning,
                author: None,
                event: None,
                structure: sid,
                room,
                // A surface of its own: no feature of the building.
                feature: Some(10_000 + out.len()),
                material: Material::Stone,
            });
            site.writing.surfaces.push(Surface {
                structure: sid,
                room,
                feature: Some(10_000 + out.len()),
                layers: vec![id],
            });
            let thing = site.things.len();
            site.things.push(Thing {
                id: thing,
                kind: "inscription",
                material: Material::Stone,
                home,
                portable: false,
                texts: vec![id],
                pos,
                surface: Some(site.writing.surfaces.len() - 1),
                object: None,
            });
            placed.thing = Some(thing);
            placed.text = Some(id);
        }
        out.push(placed);
    }
    out
}

impl Game {
    /// The variables a storylet sees now.
    pub(crate) fn story_ctx(&mut self, tool: &str, ending: &str, s: &Storylet) -> Context {
        let pos = self.state.pos;
        let (place, era, indoors) = match self.state.place {
            Place::Outside => ("outside".to_string(), "none", false),
            Place::Room { structure, .. } => {
                let st = self.site.structure(structure);
                (label(&st.kind), era_band(&self.site.world, st.era), true)
            }
        };
        let bands = self
            .env()
            .region_bands(pos)
            .unwrap_or(["middling", "middling", "middling", "usual"]);
        let list = |v: Vec<String>| Value::List(v.into_iter().map(Value::from).collect());
        let carrying: Vec<String> = self
            .state
            .carried
            .iter()
            .map(|&t| self.thing(t).kind.to_string())
            .collect();
        let inscription = match self
            .placed
            .iter()
            .find(|p| p.id == s.id)
            .and_then(|p| p.thing)
        {
            Some(t) => self.thing_name(t),
            None => String::new(),
        };
        let released = self
            .state
            .scraped
            .iter()
            .filter(|t| !self.site.writing.scraped.contains(t))
            .count();
        ctx(&[
            ("place", Value::from(place)),
            ("biome", Value::from(self.site.biome_at(pos))),
            ("era", Value::from(era)),
            ("indoors", Value::Bool(indoors)),
            ("day", Value::Number(i64::from(self.day()) + 1)),
            (
                "season",
                Value::from(SEASONS[scraped_sim::region::season(self.state.minutes)]),
            ),
            ("time", Value::from(time_of_day(self.state.minutes))),
            ("age", Value::Number(i64::from(self.age()))),
            ("life", Value::from(bands[0])),
            ("water", Value::from(bands[1])),
            ("stability", Value::from(bands[2])),
            ("climate", Value::from(bands[3])),
            ("carrying", list(carrying)),
            ("happened", list(self.state.story.happened.clone())),
            (
                "flags",
                list(self.state.story.flags.iter().cloned().collect()),
            ),
            ("tool", Value::from(tool)),
            ("ending", Value::from(ending)),
            ("read", Value::Number(self.state.read.len() as i64)),
            ("written", Value::Number(self.state.written.len() as i64)),
            ("released", Value::Number(released as i64)),
            ("inscription", Value::from(inscription)),
        ])
    }

    /// Whether a storylet may happen now (ignoring where).
    fn story_ready(&self, s: &Storylet, c: &Context) -> bool {
        if !s.repeat && self.state.story.happened.contains(&s.id) {
            return false;
        }
        if s.after
            .as_ref()
            .is_some_and(|a| !self.state.story.happened.contains(a))
        {
            return false;
        }
        match &s.when {
            None => true,
            Some(w) => parse_expr(w)
                .ok()
                .and_then(|e| scraped_content::render::eval(&e, c).ok())
                .unwrap_or(false),
        }
    }

    /// Whether the player is where a placed storylet is.
    fn story_here(&self, s: &Storylet) -> bool {
        let Some(p) = self.placed.iter().find(|p| p.id == s.id) else {
            return false;
        };
        match (s.at.as_str(), self.state.place) {
            ("structure", Place::Room { structure, .. }) => structure == p.structure,
            ("outdoors", Place::Outside) => self.state.pos.dist(p.pos) <= NEAR,
            _ => false,
        }
    }

    /// Runs storylets that can happen now: placed ones where the player is,
    /// and those that can happen anywhere. Their text joins the notes.
    pub(crate) fn run_storylets(&mut self) {
        if self.state.dead.is_some() {
            return;
        }
        let all: Vec<Storylet> = self.storylets.clone();
        for s in all {
            let fits = match s.at.as_str() {
                "anywhere" => true,
                "structure" | "outdoors" => self.story_here(&s),
                _ => false,
            };
            if fits {
                self.try_storylet(&s, "", "");
            }
        }
    }

    /// Announces a beat of the spine; storylets waiting on it may happen.
    /// Returns their texts.
    pub(crate) fn hook(&mut self, hook: &str, tool: &str, ending: &str) -> Vec<String> {
        let all: Vec<Storylet> = self
            .storylets
            .iter()
            .filter(|s| s.at == "hook" && s.hook.as_deref() == Some(hook))
            .cloned()
            .collect();
        // "first_…" beats happen once a run.
        if hook.starts_with("first_") && self.hooks_seen.contains(hook) {
            return Vec::new();
        }
        self.hooks_seen.insert(hook.to_string());
        let texts: Vec<String> = all
            .iter()
            .filter_map(|s| self.try_storylet(s, tool, ending))
            .collect();
        // The opening is placed by `start`; the ending by the summary.
        if hook != "opening" && hook != "ending" {
            self.notes.extend(texts.iter().cloned());
        }
        texts
    }

    fn try_storylet(&mut self, s: &Storylet, tool: &str, ending: &str) -> Option<String> {
        let c = self.story_ctx(tool, ending, s);
        if !self.story_ready(s, &c) {
            return None;
        }
        self.state.story.happened.push(s.id.clone());
        let text = self.say(&format!("story.{}", s.id), c);
        self.apply_effects(s);
        if s.at != "hook" {
            self.notes.push(text.clone());
        }
        Some(text)
    }

    fn apply_effects(&mut self, s: &Storylet) {
        for e in &s.effects {
            let mut words = e.split_whitespace();
            match (words.next(), words.next()) {
                (Some("give"), Some(kind)) => {
                    self.make_item(kind, true);
                }
                (Some("flag"), Some(f)) => {
                    self.state.story.flags.insert(f.to_string());
                }
                (Some("unflag"), Some(f)) => {
                    self.state.story.flags.remove(f);
                }
                (Some("open"), _) => {
                    let Some(p) = self.placed.iter().find(|p| p.id == s.id).cloned() else {
                        continue;
                    };
                    let links = &self.site.world.structures[p.structure].interior.links;
                    for (li, l) in links.iter().enumerate() {
                        if l.state == PassageState::Blocked {
                            self.state.sim.opened.insert((p.structure, li));
                        }
                        self.state.sim.unbarred.insert((p.structure, li));
                    }
                }
                _ => {}
            }
        }
    }

    /// Debug: where each storylet was placed, and what has happened.
    pub fn storylets_truth(&self) -> serde_json::Value {
        serde_json::json!({
            "placed": self.placed,
            "happened": self.state.story.happened,
            "flags": self.state.story.flags,
            "hooks": self.hooks_seen,
        })
    }
}

impl Game {
    /// For the authoring tool: where a storylet lands in this world, what
    /// writing it brought, and its text as it would read there.
    pub fn preview_storylet(&mut self, id: &str) -> serde_json::Value {
        let Some(s) = self.pack.storylets().find(|s| s.id == id).cloned() else {
            return serde_json::json!({ "error": format!("no storylet '{id}'") });
            // DEBUG-TEXT
        };
        let start = self.site.start();
        let placed = self.placed.iter().find(|p| p.id == id).cloned();
        if let Some(p) = &placed {
            self.state.pos = p.pos;
            self.state.place = if s.at == "structure" {
                Place::Room {
                    structure: p.structure,
                    room: 0,
                }
            } else {
                Place::Outside
            };
        }
        let c = self.story_ctx("", "", &s);
        let text = self.say(&format!("story.{id}"), c.clone());
        let inscription = placed.as_ref().and_then(|p| p.text).map(|t| {
            let t = self.text(t).clone();
            let r = self.site.world.renderer(t.era);
            serde_json::json!({
                "era": t.era,
                "romanised": r.surface(&r.render(&t.meaning)),
                "gloss": scraped_lang::english::translate(&t.meaning, &|i| format!("#{i}")), // DEBUG-TEXT: spoiler gloss
                "meaning": t.meaning,
            })
        });
        let building = placed.as_ref().map(|p| {
            let st = self.site.structure(p.structure);
            serde_json::json!({
                "kind": label(&st.kind),
                "condition": label(&st.condition),
                "era": era_band(&self.site.world, st.era),
                "settlement": st.settlement,
                "metres_from_start": p.pos.dist(start).round() as i64,
                "bearing": scraped_sim::outdoors::bearing(start, p.pos).map(|b| scraped_sim::outdoors::BEARINGS[b]),
            })
        });
        serde_json::json!({
            "id": id,
            "at": s.at,
            "placed": placed.is_some(),
            "building": building,
            "inscription": inscription,
            "text": text,
            "context": c,
        })
    }
}

/// The choices the storylet editor offers.
pub fn schema() -> serde_json::Value {
    serde_json::json!({
        "at": AT,
        "hooks": HOOKS,
        "structures": STRUCTURES,
        "scenes": scene_kinds(),
        "biomes": BIOMES,
        "eras": ERAS,
        "effects": EFFECTS,
        "registers": REGISTERS,
        "items": scraped_sim::items::ids(),
        "variables": variables().iter().map(|(n, _, d)| serde_json::json!({ "name": n, "description": d })).collect::<Vec<_>>(),
    })
}

/// Problems with the pack's storylets, in Jb's words.
pub fn lint(pack: &Pack) -> Vec<Issue> {
    let mut out = Vec::new();
    let names: Vec<String> = variables().iter().map(|v| v.0.to_string()).collect();
    let mut ids: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &pack.files {
        for s in &f.storylets {
            *ids.entry(s.id.as_str()).or_default() += 1;
        }
    }
    for f in &pack.files {
        for s in &f.storylets {
            let mut issue = |severity: Severity, kind: &'static str, message: String| {
                out.push(Issue {
                    severity,
                    slot: format!("story.{}", s.id),
                    file: Some(f.path.clone()),
                    variant: None,
                    kind,
                    message,
                })
            };
            use Severity::{Error, Warning};
            if ids[s.id.as_str()] > 1 {
                issue(
                    Error,
                    "storylet-duplicate",
                    format!("two storylets are called '{}'; give each its own id", s.id),
                );
            }
            if s.id.is_empty() || !s.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                issue(
                    Error,
                    "storylet-id",
                    "ids use letters, digits and _ only".into(),
                );
            }
            if !AT.contains(&s.at.as_str()) {
                issue(
                    Error,
                    "storylet-at",
                    format!("'at' must be one of {}", AT.join(", ")),
                );
            }
            match (s.at.as_str(), s.hook.as_deref()) {
                ("hook", None) => issue(
                    Error,
                    "storylet-hook",
                    "this storylet waits on a beat of the spine, but none is named in 'hook'"
                        .into(),
                ),
                ("hook", Some(h)) if !HOOKS.contains(&h) => issue(
                    Error,
                    "storylet-hook",
                    format!(
                        "there is no beat called '{h}'; the beats are {}",
                        HOOKS.join(", ")
                    ),
                ),
                (at, Some(_)) if at != "hook" => issue(
                    Warning,
                    "storylet-hook",
                    "'hook' is only used when 'at' is \"hook\"".into(),
                ),
                _ => {}
            }
            for k in &s.place.structure {
                if !STRUCTURES.contains(&k.as_str()) {
                    issue(
                        Error,
                        "storylet-place",
                        format!(
                            "there are no buildings of kind '{k}'; the kinds are {}",
                            STRUCTURES.join(", ")
                        ),
                    );
                }
            }
            for k in &s.place.scene {
                if !scene_kinds().contains(&k.as_str()) {
                    issue(
                        Error,
                        "storylet-place",
                        format!(
                            "there are no scenes of kind '{k}'; the kinds are {}",
                            scene_kinds().join(", ")
                        ),
                    );
                }
            }
            for b in &s.place.biome {
                if !BIOMES.contains(&b.as_str()) {
                    issue(
                        Error,
                        "storylet-place",
                        format!("there is no land called '{b}'"),
                    );
                }
            }
            if let Some(e) = &s.place.era {
                if !ERAS.contains(&e.as_str()) {
                    issue(
                        Error,
                        "storylet-place",
                        format!("era must be old, middle or new, not '{e}'"),
                    );
                }
            }
            if !s.place.is_any() && s.at != "structure" && s.at != "outdoors" {
                issue(
                    Warning,
                    "storylet-place",
                    "placement rules only matter when 'at' is structure or outdoors".into(),
                );
            }
            if let Some(w) = &s.when {
                match parse_expr(w) {
                    Err(e) => issue(
                        Error,
                        "storylet-condition",
                        format!(
                            "the condition can't be read at character {}: {}",
                            e.at, e.message
                        ),
                    ),
                    Ok(e) => {
                        for v in expr_variables(&e) {
                            if !names.contains(&v) {
                                issue(
                                    Error,
                                    "storylet-condition",
                                    format!("the condition uses '{v}', which storylets don't know"),
                                );
                            }
                        }
                    }
                }
            }
            if let Some(a) = &s.after {
                if !ids.contains_key(a.as_str()) {
                    issue(
                        Error,
                        "storylet-after",
                        format!("'after' names '{a}', but there is no such storylet"),
                    );
                } else if a == &s.id {
                    issue(
                        Error,
                        "storylet-after",
                        "a storylet can't come after itself, so it would never happen".into(),
                    );
                }
            }
            let mut flags: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
            for e in &s.effects {
                let words: Vec<&str> = e.split_whitespace().collect();
                match words.as_slice() {
                    ["give", kind] if scraped_sim::items::kind(kind).is_none() => issue(Error, "storylet-effect", format!("there is no item '{kind}' to give")),
                    ["give", _] => {}
                    ["flag", f] => flags.entry(f).or_default().0 = true,
                    ["unflag", f] => flags.entry(f).or_default().1 = true,
                    ["open"] if s.at != "structure" => issue(Warning, "storylet-effect", "'open' opens the storylet's building, but it has none".into()),
                    ["open"] => {}
                    _ => issue(Error, "storylet-effect", format!("'{e}' is not an effect; use give <item>, flag <name>, unflag <name> or open")),
                }
            }
            for (f, (set, unset)) in flags {
                if set && unset {
                    issue(
                        Error,
                        "storylet-conflict",
                        format!("this storylet both sets and clears the flag '{f}'"),
                    );
                }
            }
            if let Some(req) = &s.inscription {
                if !REGISTERS.contains(&req.register.as_str()) {
                    issue(
                        Error,
                        "storylet-inscription",
                        "the register must be potent or everyday".into(),
                    );
                }
                let known = concepts::with_pos(PartOfSpeech::Noun).any(|c| {
                    !c.has_tag("departure") && (c.id == req.about || c.has_tag(&req.about))
                });
                if !known {
                    issue(
                        Error,
                        "storylet-inscription",
                        format!("the language has no word or kind of thing '{}'", req.about),
                    );
                }
                if let Some(e) = &req.era {
                    if !ERAS.contains(&e.as_str()) {
                        issue(
                            Error,
                            "storylet-inscription",
                            format!("era must be old, middle or new, not '{e}'"),
                        );
                    }
                }
                if s.at != "structure" && s.at != "outdoors" {
                    issue(
                        Error,
                        "storylet-inscription",
                        "writing needs somewhere to be: set 'at' to structure or outdoors".into(),
                    );
                }
            }
            if pack.variants(&format!("story.{}", s.id)).is_empty() {
                issue(
                    Error,
                    "storylet-text",
                    "this storylet has no text yet".into(),
                );
            }
        }
    }
    for h in HOOKS {
        if !pack
            .storylets()
            .any(|s| s.at == "hook" && s.hook.as_deref() == Some(h))
        {
            out.push(Issue {
                severity: Severity::Warning,
                slot: format!("spine.{h}"),
                file: None,
                variant: None,
                kind: "spine-unwritten",
                message: format!("no storylet is written for the '{h}' beat yet"),
            });
        }
    }
    out
}

/// Storylets that can't be placed in any of the given worlds: their rules
/// match no building. Slow (generates worlds); for the full lint.
pub fn unplaceable(pack: &Pack, seeds: &[u64]) -> Vec<Issue> {
    let mut out = Vec::new();
    let worlds: Vec<(World, usize)> = seeds
        .iter()
        .map(|&s| {
            let site = Site::new(s);
            let start = site.settlement;
            (site.world, start)
        })
        .collect();
    for f in &pack.files {
        for s in &f.storylets {
            if s.at != "structure" && s.at != "outdoors" {
                continue;
            }
            if worlds
                .iter()
                .all(|(w, start)| candidates(w, *start, s).is_empty())
            {
                out.push(Issue {
                    severity: Severity::Warning,
                    slot: format!("story.{}", s.id),
                    file: Some(f.path.clone()),
                    variant: None,
                    kind: "storylet-never",
                    message: format!(
                        "no building in {} sample worlds fits these placement rules, so it may never appear",
                        seeds.len()
                    ),
                });
            }
        }
    }
    out
}
