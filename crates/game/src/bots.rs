//! The depth bots (D01): players that go deep into a world, so the depth
//! metrics and sample transcripts measure what a real player would meet.
//!
//! - The **curious explorer** plays like a newcomer who finds the world
//!   interesting: it walks to landmarks it hasn't been to, enters every
//!   building, looks at, reads and handles what it finds, and keeps itself
//!   alive. It never scrapes or writes on purpose.
//! - The **scholar** explores the same way, but knows the grammar (a
//!   spoiler) and none of the map: it carries every tool it finds, scrapes
//!   what it reads, and comes back to rooms it had to leave (too dark, or
//!   writing too faint for its lens) once it can do better, until it has
//!   read the deepest text. Its body is kept well: it measures how far the
//!   late game can be reached, and the explorer measures survival.
//!
//! Both work only from what the player is shown (the output's summary and
//! text, and the variables of what was rendered) plus their own memory of
//! where they've been. Neither looks at the map.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use scraped_content::{Pack, Value};
use scraped_sim::outdoors::{hash, BEARINGS};

use crate::{Game, Output, Place, Rendered, Summary};

/// The depth bots.
pub const DEPTH_BOTS: &[&str] = &["explorer", "scholar"];

/// Buildings the explorer goes into at one spot before moving on (S01:
/// time split between towns, the land between and interiors).
const TOWN_BUILDINGS: usize = 4;

/// Minutes the explorer spends in one building before making its way out.
const BUILDING_MINUTES: u32 = 45;

/// Minutes in a great interior (more than eight spaces seen) before
/// making its way out.
const GREAT_MINUTES: u32 = 180;

/// What to type to go through a way: a direction as it is, a way given
/// by name ("the western door north") with `go`.
fn way_command(exit: &str) -> String {
    if opposite(exit).is_some() || matches!(exit, "out") {
        exit.to_string()
    } else {
        format!(
            "go {}",
            exit.trim_start_matches("the ").trim_start_matches("a ")
        )
    }
}

/// The direction back through an exit.
fn opposite(d: &str) -> Option<&'static str> {
    Some(match d {
        "north" => "south",
        "south" => "north",
        "east" => "west",
        "west" => "east",
        "up" => "down",
        "down" => "up",
        _ => return None,
    })
}

/// The words a player types for a thing: its name without the article,
/// with a number in front when several things share the name.
fn noun(names: &[String], i: usize) -> String {
    let strip = |n: &str| {
        let n = n.trim();
        for a in ["a ", "an ", "the ", "some "] {
            if let Some(r) = n.strip_prefix(a) {
                return r.to_string();
            }
        }
        n.to_string()
    };
    let me = strip(&names[i]);
    let same: Vec<usize> = (0..names.len())
        .filter(|&j| strip(&names[j]) == me)
        .collect();
    if same.len() > 1 {
        let k = same.iter().position(|&j| j == i).unwrap_or(0) + 1;
        format!("{k} {me}")
    } else {
        me
    }
}

/// The building a place is in, from "structure N room M".
fn building_of(place: &str) -> Option<usize> {
    place
        .strip_prefix("structure ")?
        .split(' ')
        .next()?
        .parse()
        .ok()
}

/// The kilometre square a position is in.
fn square_of(g: &Game) -> (i32, i32) {
    let p = g.state.pos;
    (p.x.div_euclid(1000), p.y.div_euclid(1000))
}

/// What a bot has learnt about a room.
#[derive(Debug, Clone, Default)]
struct Room {
    /// Things handled, by name and what was done.
    done: BTreeSet<(String, &'static str)>,
    /// Things close looking showed to bear writing, or to be carryable.
    written: BTreeSet<String>,
    items: BTreeSet<String>,
    /// Ways out, as last seen.
    exits: Vec<String>,
    /// Ways tried, and where they led.
    leads: BTreeMap<String, String>,
    /// Ways that wouldn't open, and whether a pry bar was tried on them.
    barred: BTreeSet<String>,
    pried: bool,
    /// Left unseen for want of light.
    dark: bool,
    /// Writing here showed fainter layers to this lens.
    ghosts: Option<u8>,
    /// The most layers seen beneath one text here.
    layers: u32,
    /// Old writing holds a way shut here: the way, and claims cast.
    held: Option<(String, u8)>,
    /// A hearth to make a fire in.
    hearth: bool,
    /// Ways that are holes or shafts down: a drop there may not climb back.
    drops: BTreeSet<String>,
    /// Ways tried that left it where it was.
    failed: BTreeSet<String>,
}

impl Room {
    fn open_exits(&self) -> impl Iterator<Item = &String> {
        self.exits
            .iter()
            .filter(|e| !self.leads.contains_key(*e) && !self.barred.contains(*e))
    }
}

/// A building whose way in wouldn't give (D11: a sealed place), and how
/// far the scholar has got with the ward on the stone at its door.
#[derive(Debug, Clone)]
struct SealedSite {
    square: (i32, i32),
    /// Its name for the spot, to come back by.
    spot: Option<String>,
    /// What it typed to go in.
    noun: String,
    /// What the response said would not move ("gate").
    ward: String,
    /// Tries so far, and the step reached: 0 to write, 1 written, 2 dried,
    /// 3 scraped.
    tries: u8,
    phase: u8,
    open: bool,
}

/// How to find a building again.
#[derive(Debug, Clone)]
struct Building {
    /// The scholar's name for the spot it went in from.
    spot: Option<String>,
    square: (i32, i32),
    /// What it typed to go in.
    noun: String,
}

/// Something the bot has set out to do over several commands.
#[derive(Debug, Clone, PartialEq)]
enum Goal {
    /// Back to a room, for a better look.
    Room { place: String, steps: u32 },
}

/// A feature noticed: its kind, bearing and distance by eye.
type Lead = (String, String, String);

/// A depth bot.
pub struct DepthBot {
    pub kind: &'static str,
    seed: u64,
    step: u64,
    rooms: BTreeMap<String, Room>,
    buildings: BTreeMap<usize, Building>,
    /// Building nouns tried at each outdoor square, and those that
    /// wouldn't let it in.
    tried_here: BTreeMap<(i32, i32), BTreeSet<String>>,
    /// Kinds of building the explorer has gone into, and how often.
    entered_kinds: BTreeMap<&'static str, usize>,
    /// Gone indoors for the night: stays in till morning.
    sheltering: bool,
    /// Turns taken wandering when no known way leads out (D04: after a
    /// drop that can't be climbed back).
    wander: std::cell::RefCell<BTreeMap<String, usize>>,
    shut: BTreeSet<((i32, i32), String)>,
    /// Sealed places met (D11).
    sealed: Vec<SealedSite>,
    /// Landmarks reached, or given up on.
    visited: BTreeSet<String>,
    /// Times each landmark was set out for.
    attempts: BTreeMap<String, u32>,
    /// Landmarks seen but not yet visited, and roughly where (its own
    /// reckoning from bearing and distance by eye).
    sighted: BTreeMap<String, (i32, i32)>,
    /// Kilometre squares stood on, and how often.
    walked: BTreeMap<(i32, i32), u32>,
    /// Headings that went nowhere, by square.
    blocked: BTreeSet<((i32, i32), usize)>,
    /// When each body command was last tried where.
    tries: BTreeMap<String, u32>,
    /// The last command, where it was given, and from what position.
    last: Option<(String, String, (i32, i32))>,
    /// The scholar's names for spots, by square.
    named: BTreeMap<(i32, i32), String>,
    goal: Option<Goal>,
    /// Rooms given up on.
    abandoned: BTreeSet<String>,
    lens: u8,
    lamp_empty: bool,
    /// Whether the waterskin has been drunk from since it was last filled.
    skin_used: bool,
    /// Outdoor squares looked around.
    looked: BTreeSet<(i32, i32)>,
    /// The last place outdoors with buildings to shelter in.
    shelter: Option<(i32, i32)>,
    /// Recent places, to notice going round in circles.
    recent: VecDeque<(String, (i32, i32))>,
    /// Renders already looked at, and where the last response's began.
    renders: usize,
    fresh: usize,
    /// Digging verbs tried, by place.
    dug: BTreeSet<(String, &'static str)>,
    /// Curiosities followed: (place, what).
    followed: BTreeSet<(String, String)>,
    /// A building it has given up on and is making its way out of.
    leaving: Option<String>,
    /// What each building's entrance was for ("gate-court"), to walk back
    /// to it by name; and the great buildings whose entrance it marked.
    entrances: BTreeMap<String, String>,
    marked: BTreeSet<String>,
    /// Whether it has put its cloak on.
    cloak_on: bool,
    /// Pages of the current text read on from the first (D08: long texts
    /// are skimmed past the third page).
    pages: u32,
    /// The building it is in, and when it went in.
    inside_since: Option<(String, u32)>,
    /// Features noticed from each square: kind, bearing, distance.
    feature_leads: BTreeMap<(i32, i32), Vec<Lead>>,
    /// Things of each kind examined that turned up nothing new, in a row:
    /// once a kind stops giving anything, the explorer stops examining it.
    dull: BTreeMap<String, u32>,
    /// Commands given since anything new was perceived.
    pub idle: u32,
    /// A grimy surface just read, to clean so it reads better (D10: the
    /// ordinary reason a naive player scrapes).
    clean_next: Option<String>,
    /// What the bot set out to do and why, for reading its runs.
    pub notes: Vec<String>,
}

impl DepthBot {
    pub fn new(kind: &'static str, seed: u64) -> Self {
        DepthBot {
            kind,
            seed,
            step: 0,
            rooms: BTreeMap::new(),
            buildings: BTreeMap::new(),
            tried_here: BTreeMap::new(),
            entered_kinds: BTreeMap::new(),
            sheltering: false,
            wander: std::cell::RefCell::new(BTreeMap::new()),
            shut: BTreeSet::new(),
            sealed: Vec::new(),
            visited: BTreeSet::new(),
            attempts: BTreeMap::new(),
            sighted: BTreeMap::new(),
            walked: BTreeMap::new(),
            blocked: BTreeSet::new(),
            tries: BTreeMap::new(),
            last: None,
            named: BTreeMap::new(),
            goal: None,
            abandoned: BTreeSet::new(),
            lens: 0,
            lamp_empty: false,
            skin_used: true,
            looked: BTreeSet::new(),
            shelter: None,
            recent: VecDeque::new(),
            renders: 0,
            fresh: 0,
            dug: BTreeSet::new(),
            followed: BTreeSet::new(),
            dull: BTreeMap::new(),
            feature_leads: BTreeMap::new(),
            leaving: None,
            inside_since: None,
            cloak_on: false,
            pages: 0,
            entrances: BTreeMap::new(),
            marked: BTreeSet::new(),
            idle: 0,
            clean_next: None,
            notes: Vec::new(),
        }
    }

    fn scholar(&self) -> bool {
        self.kind == "scholar"
    }

    fn roll(&self, salt: u64) -> u64 {
        hash(&[self.seed, 0xde97, self.step, salt])
    }

    /// The next command, from what the last output showed.
    pub fn next(&mut self, g: &Game, last: &Output) -> String {
        self.step += 1;
        self.learn(g, last);
        if let Some(n) = self.clean_next.take() {
            if last.state.dead.is_none() {
                return format!("clean {n}");
            }
        }
        let mut cmd = self.choose(g, last);
        // Going round in circles: drop the goal, or strike out somewhere.
        self.recent
            .push_back((last.state.place.clone(), square_of(g)));
        if self.recent.len() > 60 {
            self.recent.pop_front();
        }
        let places: BTreeSet<&(String, (i32, i32))> = self.recent.iter().collect();
        let stuck = self.recent.len() == 60 && places.len() <= 3 && self.idle > 40;
        if stuck || (self.idle > 150 && last.state.place == "outside") {
            if let Some(Goal::Room { place, .. }) = self.goal.take() {
                self.notes.push(format!(
                    "step {}: going round in circles; gave up on {place}",
                    self.step
                ));
                self.abandoned.insert(place);
            }
            self.recent.clear();
            self.idle = 0;
            if last.state.place == "outside" {
                cmd = format!("head {}", BEARINGS[(self.roll(9) % 8) as usize]);
            } else if let Some((b, _)) = last.state.place.rsplit_once(" room ") {
                // Going round in circles inside: done with this building.
                self.notes.push(format!(
                    "step {}: going round in circles in {b}; leaving",
                    self.step
                ));
                self.leaving = Some(b.to_string());
                cmd = self.towards_entrance(&last.state.place);
            }
        }
        self.last = Some((
            cmd.clone(),
            last.state.place.clone(),
            (g.state.pos.x, g.state.pos.y),
        ));
        // A way given by name is gone through by name.
        if last.state.place != "outside" && last.state.exits.contains(&cmd) {
            return way_command(&cmd);
        }
        cmd
    }

    /// Takes in what the last command did.
    fn learn(&mut self, g: &Game, last: &Output) {
        self.fresh = self.renders.min(g.renders.len());
        let s = &last.state;
        let here = s.place.clone();
        if let Some(b) = here.strip_suffix(" room 0") {
            for x in &g.renders[self.renders.min(g.renders.len())..] {
                if x.trace.slot == "room.whole" {
                    if let Some(Value::Text(p)) = x.vars.get("purpose") {
                        self.entrances.insert(b.to_string(), p.replace('_', " "));
                    }
                }
            }
        }
        if here != "outside" {
            let r = self.rooms.entry(here.clone()).or_default();
            r.exits = s.exits.clone();
            r.hearth |= s.things.iter().any(|t| t.contains("hearth"));
            for x in &g.renders[self.renders.min(g.renders.len())..] {
                let text = |k: &str| match x.vars.get(k) {
                    Some(Value::Text(t)) => t.clone(),
                    _ => String::new(),
                };
                if x.trace.slot == "place.exit"
                    && text("direction") == "down"
                    && matches!(text("passage").as_str(), "hole" | "shaft")
                {
                    r.drops.insert("down".to_string());
                }
            }
        }
        if let Some((cmd, from, pos)) = self.last.clone() {
            // Inside, through a way: note where it led, both ways.
            // (Only by a way's own word: an answer to "which?" or a walk
            // back to a named room isn't a way.)
            let is_way = self
                .rooms
                .get(&from)
                .is_some_and(|r| r.exits.contains(&cmd));
            if from.starts_with("structure")
                && here.starts_with("structure")
                && from != here
                && is_way
            {
                self.rooms
                    .entry(from.clone())
                    .or_default()
                    .leads
                    .insert(cmd.clone(), here.clone());
                if let Some(back) = opposite(&cmd) {
                    let r = self.rooms.entry(here.clone()).or_default();
                    if r.exits.iter().any(|e| e == back) {
                        r.leads.insert(back.to_string(), from.clone());
                    }
                }
            }
            // A way tried that didn't take it anywhere.
            if from == here && from.starts_with("structure") {
                let r = self.rooms.entry(from.clone()).or_default();
                if r.exits.contains(&cmd) {
                    r.failed.insert(cmd.clone());
                }
            }
            // Into a building from outside, or a heading that went nowhere.
            if from == "outside" {
                let sq = square_of(g);
                if let Some(n) = cmd.strip_prefix("go ") {
                    let held = g.renders[self.renders.min(g.renders.len())..]
                        .iter()
                        .find(|r| r.trace.slot == "effect.held")
                        .and_then(|r| match r.vars.get("thing") {
                            Some(Value::Text(t)) => t.split_whitespace().last().map(str::to_string),
                            _ => None,
                        });
                    if here == "outside" {
                        self.shut.insert((sq, n.to_string()));
                        if let Some(ward) = held {
                            if !self.sealed.iter().any(|x| x.square == sq && x.noun == n) {
                                self.sealed.push(SealedSite {
                                    square: sq,
                                    spot: self.named.get(&sq).cloned(),
                                    noun: n.to_string(),
                                    ward,
                                    tries: 0,
                                    phase: 0,
                                    open: false,
                                });
                            }
                        }
                    } else if let Some(x) = self
                        .sealed
                        .iter_mut()
                        .find(|x| x.square == sq && x.noun == n)
                    {
                        x.open = true;
                    }
                    if let (false, Some(b)) = (here == "outside", building_of(&here)) {
                        self.buildings.entry(b).or_insert(Building {
                            spot: self.named.get(&sq).cloned(),
                            square: sq,
                            noun: n.to_string(),
                        });
                    }
                }
                // Set out for a landmark: reached when the journey says so;
                // given up on when there's no way there, or after a few
                // tries.
                if let Some(n) = cmd.strip_prefix("go ") {
                    let name = format!("the {n}");
                    if let Some(k) = self.attempts.get(&name).copied() {
                        let said = |slot: &str| {
                            g.renders[self.renders.min(g.renders.len())..]
                                .iter()
                                .any(|r| r.trace.slot == slot)
                        };
                        let arrived = said("travel.arrive") || said("travel.already");
                        if arrived || said("travel.no_route") || k >= 4 {
                            self.visited.insert(name.clone());
                            self.sighted.remove(&name);
                        }
                    }
                }
                if let Some(b) = cmd.strip_prefix("head ") {
                    if (g.state.pos.x, g.state.pos.y) == pos {
                        if let Some(i) = BEARINGS.iter().position(|x| *x == b) {
                            self.blocked.insert((sq, i));
                        }
                    }
                }
            }
            if cmd == "light lamp" && g.state.lit.is_empty() {
                self.lamp_empty = true;
            }
            if cmd == "use oil" {
                self.lamp_empty = false;
            }
            if cmd == "drink from waterskin" {
                self.skin_used = true;
            }
            if cmd == "fill waterskin" && !s.carried.is_empty() {
                // Filled if it said so; otherwise there was no water here.
                let filled = g.renders[self.renders.min(g.renders.len())..]
                    .iter()
                    .any(|r| r.trace.slot == "fill.done");
                if filled {
                    self.skin_used = false;
                }
            }
            // What close looking showed.
            if let Some(rest) = cmd.strip_prefix("examine ") {
                let name = (0..s.things.len())
                    .find(|&i| noun(&s.things, i) == rest)
                    .map(|i| s.things[i].clone());
                if let Some(n) = name {
                    let mut told = false;
                    for r in &g.renders[self.renders.min(g.renders.len())..] {
                        let slot = r.trace.slot.as_str();
                        if slot != "thing.examine" && slot != "item.examine" {
                            continue;
                        }
                        let room = self.rooms.entry(here.clone()).or_default();
                        if matches!(r.vars.get("written"), Some(Value::Bool(true))) {
                            room.written.insert(n.clone());
                        }
                        if slot == "item.examine"
                            || matches!(r.vars.get("item"), Some(Value::Bool(true)))
                        {
                            room.items.insert(n.clone());
                        }
                        told |= room.written.contains(&n) || room.items.contains(&n);
                    }
                    // A kind that turns up nothing, time after time, stops
                    // being worth a look.
                    let kind = n.split_whitespace().last().unwrap_or("").to_string();
                    if told {
                        self.dull.remove(&kind);
                    } else {
                        *self.dull.entry(kind).or_default() += 1;
                    }
                }
            }
            // A way held shut by old writing.
            if let Some(way) = cmd.strip_prefix("open ") {
                let held = g.renders[self.renders.min(g.renders.len())..]
                    .iter()
                    .any(|r| r.trace.slot == "effect.held");
                if held && here != "outside" {
                    let r = self.rooms.entry(here.clone()).or_default();
                    if r.held.is_none() {
                        r.held = Some((way.to_string(), 0));
                        self.notes
                            .push(format!("step {}: a way held shut at {here}", self.step));
                    }
                } else if let Some(r) = self.rooms.get_mut(&here) {
                    // It gave: no more writing needed here.
                    if r.held.as_ref().is_some_and(|(w, _)| w == way) {
                        r.held = None;
                        r.barred.remove(way);
                        self.notes
                            .push(format!("step {}: the way {way} gave at {here}", self.step));
                    }
                }
            }
            // Moss or soot on what was read: the explorer cleans it once.
            if self.kind == "explorer" && cmd.starts_with("read ") {
                let grimy = g.renders[self.renders.min(g.renders.len())..]
                    .iter()
                    .any(|r| r.trace.slot == "read.grime");
                if grimy {
                    self.clean_next = Some(cmd.trim_start_matches("read ").to_string());
                }
            }
            // Fainter layers beneath what was read.
            if cmd.starts_with("read ") || cmd == "more" {
                let count = g.renders[self.renders.min(g.renders.len())..]
                    .iter()
                    .filter(|r| r.trace.slot == "read.ghosts")
                    .filter_map(|r| match r.vars.get("count") {
                        Some(scraped_content::Value::Number(n)) => Some(*n),
                        _ => None,
                    })
                    .max();
                if let (Some(count), true) = (count, here != "outside") {
                    let lens = self.lens;
                    let r = self.rooms.entry(here.clone()).or_default();
                    r.layers = r.layers.max(count as u32);
                    if r.ghosts.is_none() {
                        self.notes.push(format!(
                            "step {}: fainter layers at {here}, lens {lens}",
                            self.step
                        ));
                    }
                    r.ghosts = Some(lens);
                }
            }
        }
        self.renders = g.renders.len();
        self.cloak_on &= s.carried.iter().any(|c| c.contains("cloak"));
        self.lens = self.lens.max(carried_lens(s));
        if let Some((b, _)) = s.place.rsplit_once(" room ") {
            if self.inside_since.as_ref().is_none_or(|(x, _)| x != b) {
                self.inside_since = Some((b.to_string(), s.minutes));
            }
        }
        if s.place == "outside" {
            self.leaving = None;
            self.inside_since = None;
            *self.walked.entry(square_of(g)).or_default() += 1;
            if !s.exits.is_empty() {
                self.shelter = Some((g.state.pos.x, g.state.pos.y));
            }
        }
    }

    fn choose(&mut self, g: &Game, last: &Output) -> String {
        let s = &last.state;
        let here = s.place.clone();
        if g.state.pending.is_some() {
            return "first".into();
        }
        // A way that didn't let us through is noted before anything else,
        // or whatever leads back by it (thirst, the dark, leaving) would
        // try it for ever.
        if here != "outside" {
            if let Some(c) = self.blocked_way(s) {
                return c;
            }
        }
        if let Some(c) = self.body(g, last) {
            return c;
        }
        if let Some(c) = self.reading(g) {
            return c;
        }
        if here != "outside" {
            if let Some(c) = self.lighting(g, last) {
                return c;
            }
        } else if !g.state.lit.is_empty() {
            // Light is scarce: put it out under the sky.
            if let Some(c) = self.douse(g, s) {
                return c;
            }
        }
        if here != "outside" && s.light == "dark" {
            // Nothing to see by: note it, and go back the way we came.
            let r = self.rooms.entry(here.clone()).or_default();
            if !r.dark {
                r.dark = true;
                self.notes
                    .push(format!("step {}: too dark at {here}", self.step));
            }
            // The way that led into this darkness isn't worth taking again
            // without a light.
            if let Some((cmd, from, _)) = self.last.clone() {
                if from != here && from != "outside" && !self.scholar() {
                    self.rooms.entry(from).or_default().barred.insert(cmd);
                }
            }
            if matches!(&self.goal, Some(Goal::Room { place, .. }) if *place == here) {
                self.goal = None;
                self.abandoned.insert(here.clone());
            }
            return self.towards_entrance(&here);
        }
        if let Some(r) = self.rooms.get_mut(&here) {
            r.dark = false;
        }
        // Arrived where an errand led: look again with fresh eyes.
        if matches!(&self.goal, Some(Goal::Room { place, .. }) if *place == here) {
            self.notes
                .push(format!("step {}: back at {here}", self.step));
            self.goal = None;
            let lens = self.lens;
            if let Some(r) = self.rooms.get_mut(&here) {
                r.done.retain(|(_, v)| *v != "read");
                if r.ghosts.is_some() {
                    r.ghosts = Some(lens);
                }
                if !r.pried {
                    for w in std::mem::take(&mut r.barred) {
                        r.done.remove(&(format!("way {w}"), "open"));
                        r.done.remove(&(format!("way {w}"), "pry"));
                    }
                }
            }
        }
        // A curious newcomer looks around each new spot outdoors.
        if here == "outside" && !self.scholar() && self.looked.insert(square_of(g)) {
            return "look".into();
        }
        // Given up on, or seen for long enough (the explorer has the land
        // to see too): out.
        // (The scholar answers held ways with writing first: `unhold`.)
        if here != "outside" && !self.scholar() {
            if let Some(c) = self.blocked_way(s) {
                return c;
            }
        }
        if let Some((b, _)) = here.rsplit_once(" room ") {
            // A great interior is worth hours.
            let prefix = format!("{b} room ");
            let great = self.rooms.keys().filter(|p| p.starts_with(&prefix)).count() > 8;
            let stay = if great {
                GREAT_MINUTES
            } else {
                BUILDING_MINUTES
            };
            let long = !self.scholar()
                && self
                    .inside_since
                    .as_ref()
                    .is_some_and(|(_, t)| s.minutes > t + stay);
            if self.leaving.as_deref() == Some(b) || long {
                // Far in: back to the entrance by name, by the ways known.
                let last = self
                    .last
                    .as_ref()
                    .map(|(c, _, _)| c.clone())
                    .unwrap_or_default();
                if great && !self.scholar() && !here.ends_with(" room 0") {
                    if let Some(p) = self.entrances.get(b) {
                        let c = format!("go to the {p}");
                        if last != c {
                            return c;
                        }
                    }
                }
                return self.towards_entrance(&here);
            }
            // A great interior: mark its entrance to know it again, and
            // follow its long passages in one go now and then.
            if great && !self.scholar() {
                if here.ends_with(" room 0") && self.marked.insert(b.to_string()) {
                    return "mark".into();
                }
                let corridor = g.renders[self.fresh.min(g.renders.len())..].iter().any(|x| {
                    x.trace.slot == "room.whole"
                        && matches!(x.vars.get("space"), Some(Value::Text(t)) if t == "corridor")
                });
                if corridor && self.roll(11) % 100 < 30 {
                    return "follow the passage".into();
                }
            }
        }
        // A curious player digs in where it is, and follows what catches
        // its attention.
        if !self.scholar() {
            if let Some(c) = self.curious(g, last) {
                return c;
            }
        }
        // What's here first, then errands elsewhere.
        if let Some(c) = self.handle_things(last) {
            return c;
        }
        if self.scholar() {
            if let Some(c) = self.unhold(g, last) {
                return c;
            }
            if let Some(c) = self.pursue(g, last) {
                return c;
            }
        }
        if here == "outside" {
            self.outside(g, last)
        } else {
            self.inside(last)
        }
    }

    /// Reading: the scholar reads every page closely; the explorer reads
    /// closely now and then, looks harder at a sign or traces one, and
    /// otherwise takes a text in at a glance and moves on (S01).
    fn reading(&mut self, g: &Game) -> Option<String> {
        let last = self.last.as_ref().map(|(c, _, _)| c.as_str()).unwrap_or("");
        let after_glance = last.starts_with("read ");
        let after_close = matches!(last, "read closely" | "more");
        if self.scholar() {
            self.pages = if last == "more" { self.pages + 1 } else { 0 };
            // DESIGN-Q (bot): three pages of a text, then on to the next.
            return (g.state.reading.is_some() && self.pages < 3).then(|| "more".into());
        }
        if after_glance && g.state.reading.is_some() && self.roll(1) % 100 < 45 {
            return Some("read closely".into());
        }
        if after_close {
            let r = self.roll(2) % 100;
            if r < 12 {
                return Some(format!("examine sign {}", 1 + self.roll(3) % 6));
            }
            if r < 16 {
                return Some(format!("trace sign {}", 1 + self.roll(3) % 6));
            }
            if g.state.reading.is_some() && r < 50 {
                return Some("more".into());
            }
        }
        None
    }

    /// What a curious player does on reaching somewhere: digs in with the
    /// senses (not every one, not everywhere), then goes after whatever the
    /// last response made interesting: a feature close by or in view, a
    /// sound or a smell from somewhere, a group of alike buildings.
    fn curious(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let outside = s.place == "outside";
        let key = Self::spot_key(g, s);
        let key_hash = hash(&[
            self.seed,
            key.len() as u64,
            u64::from(
                key.bytes()
                    .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(u32::from(b))),
            ),
        ]);
        // Out under the night sky, a look up once a night (D06).
        let hour = (s.minutes / 60) % 24;
        if outside
            && !(5..21).contains(&hour)
            && self
                .dug
                .insert((format!("night {}", (s.minutes + 600) / 1440), "look up"))
        {
            return Some("look up".into());
        }
        // Follow what the last response turned up, once the buildings here
        // have had their turn.
        let fresh: Vec<&Rendered> = g.renders[self.fresh.min(g.renders.len())..]
            .iter()
            .collect();
        let tried = self.tried_here.get(&square_of(g)).map_or(0, BTreeSet::len);
        let town_done = (0..s.exits.len()).all(|i| {
            self.tried_here
                .get(&square_of(g))
                .is_some_and(|t| t.contains(&noun(&s.exits, i)))
        }) || (tried >= TOWN_BUILDINGS && equipped(s));
        // Features noticed here: kept until the buildings have had their
        // turn, then gone after, nearest first.
        let sq = square_of(g);
        if outside {
            for r in &fresh {
                let text = |k: &str| match r.vars.get(k) {
                    Some(Value::Text(t)) => t.replace('_', " "),
                    _ => String::new(),
                };
                if r.trace.slot == "land.feature" {
                    let lead = (text("kind"), text("bearing"), text("distance"));
                    let leads = self.feature_leads.entry(sq).or_default();
                    if !self
                        .followed
                        .contains(&(format!("feature {}", lead.0), "went".into()))
                        && !leads.contains(&lead)
                    {
                        leads.push(lead);
                    }
                }
            }
        }
        if outside && town_done {
            let order = ["here", "near", "short", "middle", "far", "horizon"];
            let mut leads = self.feature_leads.remove(&sq).unwrap_or_default();
            leads.sort_by_key(|(_, b, d)| {
                let d = if b == "here" { "here" } else { d.as_str() };
                order.iter().position(|o| *o == d).unwrap_or(9)
            });
            while let Some((kind, bearing, distance)) = (!leads.is_empty()).then(|| leads.remove(0))
            {
                if self
                    .followed
                    .contains(&(format!("feature {kind}"), "went".into()))
                {
                    continue;
                }
                if !leads.is_empty() {
                    self.feature_leads.insert(sq, leads);
                }
                if bearing == "here" || matches!(distance.as_str(), "near" | "short") {
                    self.followed
                        .insert((format!("feature {kind}"), "went".into()));
                    return Some(format!("go to the {kind}"));
                }
                if BEARINGS.contains(&bearing.as_str()) {
                    return Some(format!("head {bearing}"));
                }
                break;
            }
            for r in &fresh {
                let text = |k: &str| match r.vars.get(k) {
                    Some(Value::Text(t)) => t.replace('_', " "),
                    _ => String::new(),
                };
                if matches!(r.trace.slot.as_str(), "sense.sound" | "sense.smell") {
                    let bearing = text("bearing");
                    if BEARINGS.contains(&bearing.as_str())
                        && hash(&[key_hash, 0x50d]) % 100 < 35
                        && self
                            .followed
                            .insert((format!("sense {bearing}"), key.clone()))
                    {
                        return Some(format!("head {bearing}"));
                    }
                }
            }
        }
        // Dig in, once per place, a few senses each time.
        const OUT: &[(&str, u64)] = &[
            ("look around", 80),
            ("listen", 60),
            ("smell", 30),
            ("look down", 40),
            ("look up", 30),
            ("look closer", 45),
            ("wait", 20),
        ];
        const IN: &[(&str, u64)] = &[
            ("look closer", 60),
            ("listen", 35),
            ("smell", 30),
            ("look up", 20),
            ("look down", 20),
            ("touch", 15),
        ];
        let digs = if outside { OUT } else { IN };
        for (k, &(verb, pct)) in digs.iter().enumerate() {
            if hash(&[key_hash, k as u64, 0xd16]) % 100 < pct
                && self.dug.insert((key.clone(), verb))
            {
                return Some(verb.to_string());
            }
        }
        // Alike buildings together: a look at the group.
        if outside {
            let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            for e in &s.exits {
                *counts.entry(kind_named(e)).or_default() += 1;
            }
            for (kind, n) in counts {
                if n >= 2
                    && kind != "house"
                    && hash(&[key_hash, 0x6a0]) % 100 < 40
                    && self.followed.insert((format!("group {kind}"), key.clone()))
                {
                    return Some(format!("look at the {}", crate::senses::plural(kind)));
                }
            }
        }
        None
    }

    // ---------- the body ----------

    fn spot_key(g: &Game, s: &Summary) -> String {
        if s.place == "outside" {
            let sq = square_of(g);
            format!("outside {} {}", sq.0, sq.1)
        } else {
            s.place.clone()
        }
    }

    /// Whether a command is worth trying here: not tried here in the last
    /// two hours.
    fn worth(&mut self, g: &Game, s: &Summary, cmd: &str) -> bool {
        let key = format!("{} | {cmd}", Self::spot_key(g, s));
        let now = s.minutes;
        match self.tries.get(&key) {
            Some(&t) if now < t + 120 => false,
            _ => {
                self.tries.insert(key, now);
                true
            }
        }
    }

    /// Looks after the body when it asks, and keeps out of the night.
    fn body(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let outside = s.place == "outside";
        let has = |w: &str| s.carried.iter().any(|c| c.contains(w));
        let lights = has("torch") || has("lamp");
        if g.sustain {
            // Kept well: only the dark outdoors stops it.
            if outside && s.light == "dark" {
                return Some("wait 1 hour".into());
            }
            return None;
        }
        let need = |n: &str| s.body.get(n).map(String::as_str).unwrap_or("").to_string();
        if matches!(
            need("thirst").as_str(),
            "thirsty" | "parched" | "dehydrated"
        ) {
            if has("waterskin") && self.worth(g, s, "drink from waterskin") {
                return Some("drink from waterskin".into());
            }
            if self.worth(g, s, "drink") {
                return Some("drink".into());
            }
            if outside {
                // Water lies along rivers and shores.
                for e in &s.edges {
                    if ["river", "stream", "lake", "shore", "spring", "coast"]
                        .iter()
                        .any(|w| e.contains(w))
                    {
                        let w = e.split_whitespace().last().unwrap_or(e).to_string();
                        let c = format!("follow {w}");
                        if self.worth(g, s, &c) {
                            return Some(c);
                        }
                    }
                }
            } else {
                return Some(self.towards_entrance(&s.place));
            }
        }
        if matches!(need("hunger").as_str(), "hungry" | "starving") {
            if self.worth(g, s, "eat") {
                return Some("eat".into());
            }
            // Fishing means sitting still for an hour and a half: only
            // when warm, and in daylight.
            let warm = need("warmth") == "warm";
            let day = (7..18).contains(&((s.minutes / 60) % 24));
            let water =
                outside && warm && day && g.water_near(1).iter().any(|(_, d, _)| *d <= 300.0);
            if water && self.worth(g, s, "fish") {
                return Some("fish".into());
            }
            let shivering = matches!(need("warmth").as_str(), "shivering" | "hypothermic");
            if outside && !shivering && self.worth(g, s, "forage") {
                return Some("forage".into());
            }
        }
        let cold = matches!(
            need("warmth").as_str(),
            "cold" | "chilled" | "shivering" | "hypothermic"
        );
        // Shivering is a warning: get under a roof, whatever the hour.
        let shivering = matches!(need("warmth").as_str(), "shivering" | "hypothermic");
        let hour = (s.minutes / 60) % 24;
        let night = !(6..20).contains(&hour);
        // Evening: find shelter before dark rather than roam.
        let evening = !(6..17).contains(&hour);
        if cold && has("cloak") && !self.cloak_on {
            self.cloak_on = true;
            return Some("wear cloak".into());
        }
        // Once a night, while warm, a look at the sky (D06): stepping out
        // of a shelter for it if need be.
        let dark = !(5..21).contains(&hour);
        let tonight = (format!("night {}", (s.minutes + 600) / 1440), "look up");
        if dark && !cold && self.kind == "explorer" {
            if outside && self.dug.insert(tonight.clone()) {
                return Some("look up".into());
            }
            if !outside && !self.dug.contains(&tonight) && self.worth(g, s, "out") {
                return Some("out".into());
            }
        }
        if outside && (night || cold) {
            // A fire, if there's wood to burn and the means to light it.
            if !has("wood") && self.worth(g, s, "gather") {
                return Some("gather".into());
            }
            if cold && has("wood") && has("firesteel") && self.worth(g, s, "make fire") {
                return Some("make fire".into());
            }
        }
        if outside && (night || evening) && g.fire_here_pub() {
            // Camped by a fire: stay by it till morning, and keep it fed.
            if has("wood") && self.worth(g, s, "feed") {
                return Some("feed".into());
            }
            if !has("wood") && self.worth(g, s, "gather") {
                return Some("gather".into());
            }
            // By the fire after dark, a look at the sky before sleeping (D06).
            if !(5..21).contains(&hour)
                && self
                    .dug
                    .insert((format!("night {}", (s.minutes + 600) / 1440), "look up"))
            {
                return Some("look up".into());
            }
            if self.worth(g, s, "sleep") {
                return Some("sleep".into());
            }
            return Some("wait 1 hour".into());
        }
        // Cold in the open: back to shelter, as in the evening (D03's
        // curious explorer leaves towns sooner).
        if outside && (night || evening || shivering || cold) && s.exits.is_empty() {
            // The nearest town or building in view, while it's still light.
            let order = ["near", "short", "middle", "far", "horizon"];
            let shelter = s
                .landmarks
                .iter()
                .filter(|l| !["hill", "mountain"].iter().any(|k| l.name.ends_with(k)))
                // A cairn or a waterfall is no shelter (D03).
                .filter(|l| {
                    !scraped_world::features::KINDS
                        .iter()
                        .any(|k| l.name.ends_with(k.id))
                })
                .min_by_key(|l| order.iter().position(|o| *o == l.distance).unwrap_or(9));
            if let Some(l) = shelter {
                let c = format!("go {}", l.name.trim_start_matches("the "));
                if self.worth(g, s, &c) {
                    return Some(c);
                }
            }
            // Or back towards the last buildings seen (walking keeps the
            // cold off).
            if let Some((x, y)) = self.shelter {
                let p = g.state.pos;
                if (x - p.x).abs() + (y - p.y).abs() > 400 {
                    let b = bearing_to(x - p.x, y - p.y);
                    if !self.blocked.contains(&(square_of(g), b)) {
                        return Some(format!("head {}", BEARINGS[b]));
                    }
                }
            }
        }
        if outside && (night || evening || shivering) {
            // Shelter: a house, where there's a hearth, if there is one.
            let sq = square_of(g);
            let open: Vec<usize> = (0..s.exits.len())
                .filter(|&i| !self.shut.contains(&(sq, noun(&s.exits, i))))
                .collect();
            let pick = ["house", "waystation", "storehouse", "temple"]
                .iter()
                .find_map(|k| open.iter().copied().find(|&i| s.exits[i].ends_with(k)))
                .or(open.first().copied());
            if let Some(i) = pick {
                self.sheltering = true;
                return Some(format!("go {}", noun(&s.exits, i)));
            }
        }
        if cold && !outside && has("wood") && has("firesteel") {
            // A fire needs a hearth: go to one in this building if known.
            let here_hearth = self.rooms.get(&s.place).is_some_and(|r| r.hearth);
            if here_hearth {
                if self.worth(g, s, "make fire") {
                    return Some("make fire".into());
                }
            } else if let Some((b, _)) = s.place.rsplit_once(" room ") {
                let prefix = format!("{b} room ");
                let hearth = self
                    .rooms
                    .iter()
                    .find(|(p, r)| p.starts_with(&prefix) && r.hearth)
                    .map(|(p, _)| p.clone());
                if let Some(step) = hearth.and_then(|h| self.route(&s.place, &h)) {
                    return Some(step);
                }
            }
        }
        if matches!(need("rest").as_str(), "tired" | "exhausted") && self.worth(g, s, "sleep") {
            return Some("sleep".into());
        }
        let blind = s.light == "dark" && !lights;
        // Out again in the morning, once warm.
        if !evening && !shivering {
            self.sheltering = false;
        }
        // A roof without a fire doesn't warm: by day, cold and with no
        // wood, out to gather some rather than wait the cold out.
        if !outside && self.sheltering && !evening && cold && !has("wood") && has("firesteel") {
            self.sheltering = false;
        }
        if !outside && self.sheltering && !evening {
            // In from the cold by day: warm up indoors first.
            return Some("wait 1 hour".into());
        }
        let settled = !outside && (self.sheltering || self.next_way(s).is_none());
        if (blind && outside) || (evening && settled) {
            // Nothing to see by, or nowhere to go before morning: rest
            // (from the evening, once under a roof).
            if self.worth(g, s, "sleep") {
                return Some("sleep".into());
            }
            return Some("wait 1 hour".into());
        }
        // Refill the waterskin after drinking from it, when water's to hand.
        if has("waterskin") && self.skin_used && self.worth(g, s, "fill waterskin") {
            return Some("fill waterskin".into());
        }
        None
    }

    // ---------- light ----------

    /// Lights up in a dark room, if it can; now and then puts the light
    /// out where the room needs none.
    fn lighting(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let has = |w: &str| s.carried.iter().any(|t| t.contains(w));
        if !g.state.lit.is_empty() {
            return None;
        }
        if s.light != "dark" {
            return None;
        }
        if !has("firesteel") {
            return None;
        }
        if has("lamp") && !self.lamp_empty && self.worth(g, s, "light lamp") {
            return Some("light lamp".into());
        }
        if has("lamp") && has("oil") && self.lamp_empty {
            return Some("use oil".into());
        }
        if has("torch") && has("firesteel") && self.worth(g, s, "light torch") {
            return Some("light torch".into());
        }
        None
    }

    fn douse(&mut self, g: &Game, s: &Summary) -> Option<String> {
        for w in ["lamp", "torch"] {
            let c = format!("extinguish {w}");
            if s.carried.iter().any(|t| t.contains(w)) && self.worth(g, s, &c) {
                return Some(c);
            }
        }
        None
    }

    // ---------- things ----------

    /// Looks at, reads and handles what's here, once each.
    fn handle_things(&mut self, last: &Output) -> Option<String> {
        let s = &last.state;
        let kind = self.kind;
        let scholar = self.scholar();
        let can_scrape = s.carried.iter().any(|c| scrapes(c));
        let load_room = s.load.0 + 3 <= s.load.1;
        let room = self.rooms.entry(s.place.clone()).or_default();
        for (i, name) in s.things.iter().enumerate() {
            let n = noun(&s.things, i);
            let thing_kind = name.split_whitespace().last().unwrap_or("");
            // Nor what it already carries one of.
            let known = s.carried.iter().any(|c| c == name);
            // The scholar looks for writing, which objects never bear.
            let object = scraped_world::objects::KINDS
                .iter()
                .any(|k| name.ends_with(k.id));
            let dull = (scholar && object)
                || (!scholar && (known || self.dull.get(thing_kind).copied().unwrap_or(0) >= 3));
            if !dull && room.done.insert((name.clone(), "examine")) {
                return Some(format!("examine {n}"));
            }
            // D05: boxes and jars are opened, old maps read, for their
            // own sake.
            if !scholar
                && [
                    "casket", "coffer", "basket", "pouch", "urn", "amphora", "jug", "pitcher",
                ]
                .iter()
                .any(|k| name.ends_with(k))
                && room.done.insert((name.clone(), "open"))
            {
                return Some(format!("open {n}"));
            }
            if !scholar && name.ends_with("map") && room.done.insert((name.clone(), "read")) {
                return Some(format!("read {n}"));
            }
            if room.written.contains(name) {
                if room.done.insert((name.clone(), "read")) {
                    return Some(format!("read {n}"));
                }
                if scholar && can_scrape && !room.done.contains(&(name.clone(), "scrape")) {
                    // Listening as the strokes come away, for their sounds.
                    if room.done.insert((name.clone(), "listen")) {
                        return Some("listen".into());
                    }
                    room.done.insert((name.clone(), "scrape"));
                    return Some(format!("scrape {n}"));
                }
            }
            let w = want(kind, name);
            // One of each is enough.
            let bare = |n: &str| {
                let n = n.trim();
                ["a ", "an ", "the ", "some "]
                    .iter()
                    .find_map(|a| n.strip_prefix(a))
                    .unwrap_or(n)
                    .to_string()
            };
            // Food, fuel and oil are worth more of; tools one of each.
            // (The explorer only, and two at most: the scholar is kept fed.)
            let stock = !scholar
                && ["provisions", "berries", "wood", "oil"]
                    .iter()
                    .any(|k| name.contains(k));
            let held = s.carried.iter().filter(|c| bare(c) == bare(name)).count();
            let have = held >= if stock { 2 } else { 1 };
            if room.items.contains(name)
                && w > 0
                && !have
                && !room.done.contains(&(name.clone(), "take"))
            {
                if load_room {
                    room.done.insert((name.clone(), "take"));
                    return Some(format!("take {n}"));
                }
                // Make room by leaving something less wanted.
                if let Some((k, _)) = s
                    .carried
                    .iter()
                    .enumerate()
                    .map(|(k, c)| (k, want(kind, c)))
                    .filter(|&(_, cw)| cw < w)
                    // Never the light, away from the sky.
                    .filter(|&(k, _)| {
                        s.place == "outside"
                            || !["lamp", "torch", "candle"]
                                .iter()
                                .any(|l| s.carried[k].contains(l))
                    })
                    .min_by_key(|&(_, cw)| cw)
                {
                    return Some(format!("drop {}", noun(&s.carried, k)));
                }
                room.done.insert((name.clone(), "take"));
            }
        }
        for i in 0..s.mechanisms.len() {
            let n = noun(&s.mechanisms, i);
            if room.done.insert((format!("mechanism {n}"), "operate")) {
                return Some(format!("operate {n}"));
            }
        }
        None
    }

    // ---------- inside ----------

    /// The first step of the shortest known way between two rooms of a
    /// building, by the ways already walked.
    fn route(&self, from: &str, to: &str) -> Option<String> {
        if from == to {
            return None;
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut queue: VecDeque<(&str, Option<&str>)> = VecDeque::new();
        queue.push_back((from, None));
        seen.insert(from);
        while let Some((at, first)) = queue.pop_front() {
            let Some(r) = self.rooms.get(at) else {
                continue;
            };
            for (exit, next) in &r.leads {
                if r.barred.contains(exit) || !seen.insert(next.as_str()) {
                    continue;
                }
                let first = first.or(Some(exit.as_str()));
                if next == to {
                    return first.map(str::to_string);
                }
                queue.push_back((next.as_str(), first));
            }
        }
        None
    }

    /// The way towards the entrance, and out.
    fn towards_entrance(&self, here: &str) -> String {
        if here.ends_with(" room 0") {
            return "out".into();
        }
        let entrance = here
            .rsplit_once(" room ")
            .map(|(b, _)| format!("{b} room 0"))
            .unwrap_or_default();
        if let Some(step) = self.route(here, &entrance) {
            return step;
        }
        // No known way back (a drop, a one-way door behind): look for one
        // as one explores, by a way not yet tried here, else the nearest
        // known room with one.
        if let Some(e) = self
            .rooms
            .get(here)
            .and_then(|r| r.open_exits().find(|e| !r.failed.contains(*e)))
        {
            return e.clone();
        }
        if let Some((b, _)) = here.rsplit_once(" room ") {
            let prefix = format!("{b} room ");
            for (place, r) in &self.rooms {
                if place.starts_with(&prefix)
                    && place != here
                    && r.open_exits().any(|e| !r.failed.contains(e))
                {
                    if let Some(step) = self.route(here, place) {
                        return step;
                    }
                }
            }
        }
        // Else wander on by the ways here, a different one each time,
        // until the way out is found.
        let mut ways: Vec<&String> = self
            .rooms
            .get(here)
            .map(|r| {
                r.exits
                    .iter()
                    .filter(|e| !r.barred.contains(*e) && !r.failed.contains(*e))
                    .collect()
            })
            .unwrap_or_default();
        let all_barred = ways.is_empty();
        if all_barred {
            // Every way here barred: try them again (a held door may give).
            ways = self
                .rooms
                .get(here)
                .map(|r| r.exits.iter().collect())
                .unwrap_or_default();
        }
        if ways.is_empty() {
            return "look closer".into();
        }
        // Each room's ways in turn, so it never just paces between two.
        let mut turns = self.wander.borrow_mut();
        let n = turns.entry(here.to_string()).or_default();
        *n += 1;
        // Shut in: let time pass between tries, since what holds a door
        // may hold only by night, by day or in the rain.
        if all_barred && (*n).is_multiple_of(2) {
            return "wait 1 hour".into();
        }
        ways[*n % ways.len()].clone()
    }

    /// A way not yet taken from this room; stairs only with a light to
    /// see the way back by.
    fn next_way(&self, s: &Summary) -> Option<String> {
        let stairs = can_light(s, self.lamp_empty);
        self.rooms.get(&s.place).and_then(|r| {
            // The explorer doesn't drop down holes it may not climb back.
            let open: Vec<&String> = r
                .open_exits()
                .filter(|e| stairs || !matches!(e.as_str(), "up" | "down"))
                .filter(|e| self.scholar() || !r.drops.contains(*e))
                .collect();
            open.first().map(|e| (*e).clone())
        })
    }

    /// The scholar answers writing that holds a way shut with writing of
    /// its own: "the door opens", written on a blank surface here, left to
    /// dry and scraped, until the way gives.
    fn unhold(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let here = s.place.clone();
        let (way, cast) = self.rooms.get(&here)?.held.clone()?;
        let tools =
            s.carried.iter().any(|c| c.contains("stylus")) && s.carried.iter().any(|c| scrapes(c));
        if !self.scholar() || !tools || cast >= 4 {
            return None;
        }
        // Something blank to write on: looked at, no writing, not an item.
        let room = self.rooms.get(&here)?;
        let blank = (0..s.things.len()).find(|&i| {
            let n = &s.things[i];
            room.done.contains(&(n.clone(), "examine"))
                && !room.written.contains(n)
                && !room.items.contains(n)
        })?;
        let surface = noun(&s.things, blank);
        let last_cmd = self
            .last
            .as_ref()
            .map(|(c, _, _)| c.clone())
            .unwrap_or_default();
        let r = self.rooms.get_mut(&here)?;
        if last_cmd.starts_with("write ") {
            return Some("wait 1 hour".into());
        }
        if last_cmd == "wait 1 hour" {
            r.held = Some((way, cast + 1));
            return Some(format!("scrape {surface}"));
        }
        if last_cmd.starts_with("scrape ") {
            // Try the way again.
            r.done.remove(&(format!("way {way}"), "open"));
            r.barred.remove(&way);
            r.failed.remove(&way);
            r.leads.remove(&way);
            return Some(format!("open {way}"));
        }
        if (last_cmd.starts_with("open ") || cast == 0) && g.state.pending.is_none() {
            // Only words it has met often enough to write.
            let known = |w: &str| g.state.encountered.get(w).map_or(0, |t| t.len()) >= g.threshold;
            let subject = ["door", "gate", "tomb", "box"]
                .into_iter()
                .find(|w| known(w));
            if let (true, Some(subject)) = (known("open"), subject) {
                let glyphs = claim_glyphs(g, "open", subject);
                return Some(format!("write {glyphs} on {surface}"));
            }
        }
        None
    }

    /// A sealed place (D11), once the scholar can write the words: back
    /// to its door, an opening spell written over the ward on a stone
    /// there, left to dry, scraped, and in. Each stone in turn; four tries.
    fn unseal(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let tools =
            s.carried.iter().any(|c| c.contains("stylus")) && s.carried.iter().any(|c| scrapes(c));
        if !self.scholar() || !tools || g.state.pending.is_some() {
            return None;
        }
        let known = |w: &str| g.state.encountered.get(w).map_or(0, |t| t.len()) >= g.threshold;
        let i = self
            .sealed
            .iter()
            .position(|x| !x.open && x.tries < 4 && known("open") && known(&x.ward))?;
        let square = square_of(g);
        let last_cmd = self
            .last
            .as_ref()
            .map(|(c, _, _)| c.clone())
            .unwrap_or_default();
        let x = self.sealed[i].clone();
        if square != x.square {
            // Back by the name it gave the spot; once, then it's lost.
            let spot = x.spot.clone()?;
            let c = format!("go {spot}");
            if last_cmd == c {
                self.sealed[i].tries = 4;
                return None;
            }
            return Some(c);
        }
        let stones: Vec<usize> = (0..s.things.len())
            .filter(|&k| s.things[k].contains("inscription"))
            .collect();
        if stones.is_empty() {
            self.sealed[i].tries = 4;
            return None;
        }
        let stone = noun(&s.things, stones[x.tries as usize % stones.len()]);
        let x = &mut self.sealed[i];
        match x.phase {
            1 if last_cmd.starts_with("write ") => {
                // Left to dry (a stone that took nothing costs a try).
                x.phase = 2;
                Some("wait 1 hour".into())
            }
            2 => {
                x.phase = 3;
                Some(format!("scrape {stone}"))
            }
            3 => {
                x.phase = 0;
                x.tries += 1;
                self.shut.remove(&(x.square, x.noun.clone()));
                Some(format!("go {}", x.noun))
            }
            _ => {
                x.phase = 1;
                // As strongly as it can say: a ward may ask for "greatly".
                let mut glyphs = claim_glyphs(g, "open", &x.ward);
                if known("greatly") {
                    glyphs = claim_glyphs_greatly(g, "open", &x.ward);
                }
                Some(format!("write {glyphs} on {stone}"))
            }
        }
    }

    /// A way that didn't let us through: open it, or prise it open, or
    /// note it barred; once opened, go through.
    fn blocked_way(&mut self, s: &Summary) -> Option<String> {
        let here = s.place.clone();

        // A way that didn't let us through: open it, or prise it open.
        if let Some((cmd, from, _)) = self.last.clone() {
            // A way known from before counts too: writing can hide a door
            // from the room's list while the route still runs through it.
            let known = self
                .rooms
                .get(&here)
                .is_some_and(|r| r.exits.contains(&cmd));
            if from == here && (s.exits.contains(&cmd) || known) {
                let can_pry = s.carried.iter().any(|c| c.contains("pry"));
                let r = self.rooms.entry(here.clone()).or_default();
                if r.done.insert((format!("way {cmd}"), "open")) {
                    return Some(format!("open {cmd}"));
                }
                if can_pry && r.done.insert((format!("way {cmd}"), "pry")) {
                    return Some(format!("pry {cmd}"));
                }
                r.barred.insert(cmd.clone());
                r.pried |= can_pry;
            }
            if from == here && (cmd.starts_with("open ") || cmd.starts_with("pry ")) {
                // Opened (or tried to): go through.
                let way = cmd
                    .split_once(' ')
                    .map_or(String::new(), |(_, w)| w.to_string());
                if s.exits.contains(&way)
                    && !self
                        .rooms
                        .get(&here)
                        .is_some_and(|r| r.barred.contains(&way))
                {
                    return Some(way);
                }
            }
        }
        None
    }

    fn inside(&mut self, last: &Output) -> String {
        let s = &last.state;
        let here = s.place.clone();
        if let Some(c) = self.blocked_way(s) {
            return c;
        }
        // A great interior (D04) could take days: after forty of its
        // rooms, the explorer makes its way out and on (the scholar keeps
        // looking for writing).
        if let Some((b, _)) = here.rsplit_once(" room ").filter(|_| !self.scholar()) {
            let prefix = format!("{b} room ");
            let seen = self.rooms.keys().filter(|p| p.starts_with(&prefix)).count();
            if seen >= 40 {
                return self.towards_entrance(&here);
            }
        }
        // A way not yet taken from here.
        if let Some(e) = self.next_way(s) {
            return e;
        }
        // The nearest room of this building with ways not yet taken.
        if let Some((b, _)) = here.rsplit_once(" room ") {
            let prefix = format!("{b} room ");
            let mut best: Option<String> = None;
            for (place, r) in &self.rooms {
                if !place.starts_with(&prefix) || *place == here || r.dark {
                    continue;
                }
                if !self.unfinished_room(place, s) {
                    continue;
                }
                if let Some(step) = self.route(&here, place) {
                    best = Some(step);
                    break;
                }
            }
            if let Some(step) = best {
                return step;
            }
        }
        self.towards_entrance(&here)
    }

    // ---------- the scholar's errands ----------

    /// A room worth coming back to: too dark before and now it can see,
    /// faint writing and now a better lens, or a door that wouldn't open
    /// and now a pry bar.
    fn errand(&self, g: &Game, s: &Summary) -> Option<String> {
        let known = |w: &str| g.state.encountered.get(w).map_or(0, |t| t.len()) >= g.threshold;
        let words = known("open") && ["door", "gate", "tomb", "box"].iter().any(|w| known(w));
        let lit = can_light(s, self.lamp_empty);
        let pry = s.carried.iter().any(|c| c.contains("pry"));
        let write =
            s.carried.iter().any(|c| c.contains("stylus")) && s.carried.iter().any(|c| scrapes(c));
        // With a stronger lens, the deepest stack seen first (D09: shallow
        // stacks of everyday spells are everywhere).
        let deepest = self
            .rooms
            .iter()
            .filter(|(p, _)| !self.abandoned.contains(*p))
            .filter(|(_, r)| r.ghosts.is_some_and(|l| l < self.lens))
            .max_by_key(|(_, r)| r.layers)
            .filter(|(_, r)| r.layers >= 3)
            .map(|(p, _)| p.clone());
        if deepest.is_some() {
            return deepest;
        }
        self.rooms
            .iter()
            .filter(|(p, _)| !self.abandoned.contains(*p))
            .filter(|(p, _)| building_of(p).is_some_and(|b| self.buildings.contains_key(&b)))
            .find(|(p, r)| {
                (r.dark && lit)
                    || (!r.dark && self.unfinished_room(p, s))
                    || r.ghosts.is_some_and(|l| l < self.lens)
                    || (!r.barred.is_empty() && !r.pried && pry)
                    || (write && words && r.held.as_ref().is_some_and(|(_, cast)| *cast == 0))
            })
            .map(|(p, _)| p.clone())
    }

    /// Whether a room has ways not yet taken that the bot could take now.
    fn unfinished_room(&self, place: &str, s: &Summary) -> bool {
        let stairs = can_light(s, self.lamp_empty);
        self.rooms.get(place).is_some_and(|r| {
            r.open_exits().any(|e| {
                (stairs || !matches!(e.as_str(), "up" | "down"))
                    && (self.scholar() || !r.drops.contains(e))
            })
        })
    }

    /// Whether any room of this one's building is unfinished.
    fn unfinished(&self, place: &str) -> bool {
        let Some((b, _)) = place.rsplit_once(" room ") else {
            return false;
        };
        let prefix = format!("{b} room ");
        self.rooms
            .iter()
            .any(|(p, r)| p.starts_with(&prefix) && !r.dark && r.open_exits().next().is_some())
    }

    /// The scholar goes back to rooms it can now do better in.
    fn pursue(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        if self.goal.is_none() {
            // Finish the building it's in first.
            if s.place != "outside" && self.unfinished(&s.place) {
                return None;
            }
            let place = self.errand(g, s)?;
            self.notes
                .push(format!("step {}: going back to {place}", self.step));
            self.goal = Some(Goal::Room { place, steps: 0 });
        }
        let Some(Goal::Room { place, steps }) = self.goal.clone() else {
            return None;
        };
        if steps > 120 {
            self.notes
                .push(format!("step {}: gave up on {place}", self.step));
            self.abandoned.insert(place);
            self.goal = None;
            return None;
        }
        self.goal = Some(Goal::Room {
            place: place.clone(),
            steps: steps + 1,
        });
        if s.place == place {
            self.notes
                .push(format!("step {}: back at {place}", self.step));
            self.goal = None;
            let lens = self.lens;
            if let Some(r) = self.rooms.get_mut(&place) {
                r.done.retain(|(_, v)| *v != "read");
                if r.ghosts.is_some() {
                    r.ghosts = Some(lens);
                }
                // Try the barred ways again, with the pry bar.
                if !r.pried {
                    for w in std::mem::take(&mut r.barred) {
                        r.done.remove(&(format!("way {w}"), "open"));
                        r.done.remove(&(format!("way {w}"), "pry"));
                    }
                }
            }
            return None;
        }
        let b = building_of(&place)?;
        let target = self.buildings.get(&b)?.clone();
        if s.place == "outside" {
            if square_of(g) == target.square
                && (0..s.exits.len()).any(|i| noun(&s.exits, i) == target.noun)
            {
                return Some(format!("go {}", target.noun));
            }
            return match &target.spot {
                Some(spot) => Some(format!("go {spot}")),
                None => {
                    self.abandoned.insert(place);
                    self.goal = None;
                    None
                }
            };
        }
        if building_of(&s.place) == Some(b) {
            // By the ways known; otherwise explore on in here.
            return self.route(&s.place, &place);
        }
        Some(self.towards_entrance(&s.place))
    }

    // ---------- outside ----------

    fn outside(&mut self, g: &Game, last: &Output) -> String {
        let s = &last.state;
        let p = g.state.pos;
        let square = square_of(g);
        if let Some(c) = self.unseal(g, last) {
            return c;
        }

        // The scholar names the spot before going in, to find it again.
        if self.scholar() && !s.exits.is_empty() && !self.named.contains_key(&square) {
            // Letters, not numbers: a number reads as "the third".
            let n = self.named.len();
            let letter = |k: usize| char::from(b'a' + (k % 26) as u8);
            let name = format!("cairn {}{}", letter(n / 26), letter(n));
            self.named.insert(square, name.clone());
            return format!("name here as {name}");
        }
        // Buildings here not yet entered. The scholar goes where writing
        // is kept (temples, archives, tombs, houses) and passes by mills
        // and smithies, as a reader would.
        // The explorer is curious: kinds it has been into least first
        // (D03's towns have many).
        let scholar = self.scholar();
        let equipped = equipped(s);
        let entered = &self.entered_kinds;
        let tried = self.tried_here.entry(square).or_default();
        let mut order: Vec<(usize, usize)> = (0..s.exits.len())
            .map(|i| {
                let rank = if scholar {
                    writing_rank(&s.exits[i])
                } else {
                    entered
                        .get(kind_named(&s.exits[i]))
                        .copied()
                        .unwrap_or(0)
                        .min(5)
                        * 2
                };
                (rank, i)
            })
            .filter(|(r, _)| *r < 50)
            .collect();
        order.sort();
        for (_, i) in order {
            let n = noun(&s.exits, i);
            // The explorer sees a few of a town's buildings and moves on,
            // once it has the means to keep warm.
            if !scholar && tried.len() >= TOWN_BUILDINGS && equipped {
                break;
            }
            if tried.insert(n.clone()) {
                *self
                    .entered_kinds
                    .entry(kind_named(&s.exits[i]))
                    .or_default() += 1;
                return format!("go {n}");
            }
        }
        // A careful walker doesn't set out from a town late in the day:
        // with every building here seen, it waits for evening and the
        // shelter of a house (D03's larger towns take longer to see).
        let hour = (s.minutes / 60) % 24;
        if !scholar && !s.exits.is_empty() && (14..17).contains(&hour) {
            return "wait 1 hour".into();
        }
        // Note where unvisited landmarks lie, by eye.
        for l in &s.landmarks {
            if self.visited.contains(&l.name) {
                continue;
            }
            let m = match l.distance.as_str() {
                "near" => 300,
                "short" => 1000,
                "middle" => 3000,
                "far" => 7000,
                _ => 14000,
            };
            let (dx, dy) = bearing_step(&l.bearing);
            let k = if dx != 0 && dy != 0 { 707 } else { 1000 };
            self.sighted.insert(
                l.name.clone(),
                (p.x + dx * m * k / 1000, p.y + dy * m * k / 1000),
            );
        }
        // Landmarks in view not yet visited, nearest first.
        let order = ["near", "short", "middle", "far", "horizon"];
        let mut fresh: Vec<&crate::Sighting> = s
            .landmarks
            .iter()
            .filter(|l| !self.visited.contains(&l.name))
            .collect();
        // From noon the explorer makes for towns, where there is shelter,
        // before the cairns and shrines it would otherwise wander to.
        let afternoon = hour >= 12 && !scholar;
        let town = |l: &&crate::Sighting| l.name.ends_with("town") || l.name.ends_with("ruins");
        // In the morning, the odd things first (a pillar, standing stones,
        // a fall), then heights, then towns.
        let plain = |l: &&crate::Sighting| {
            if town(l) {
                2
            } else if ["mountain", "hill", "hills", "ridge", "peak"]
                .iter()
                .any(|w| l.name.ends_with(w))
            {
                1
            } else {
                0
            }
        };
        fresh.sort_by_key(|l| {
            (
                afternoon && !town(l),
                if afternoon || scholar { 0 } else { plain(l) },
                order.iter().position(|o| *o == l.distance).unwrap_or(9),
            )
        });
        if let Some(l) = fresh.first() {
            *self.attempts.entry(l.name.clone()).or_default() += 1;
            return format!("go {}", l.name.trim_start_matches("the "));
        }
        // Somewhere seen before and not yet visited: head that way.
        let target = self
            .sighted
            .iter()
            .map(|(n, &(x, y))| {
                let d = (i64::from(x - p.x)).pow(2) + (i64::from(y - p.y)).pow(2);
                (d, n.clone(), (x, y))
            })
            .min();
        if let Some((d, name, (x, y))) = target {
            let b = bearing_to(x - p.x, y - p.y);
            if d < 1500 * 1500 || self.blocked.contains(&(square, b)) {
                // It isn't where it seemed, or the way is barred.
                self.sighted.remove(&name);
            } else {
                return format!("head {}", BEARINGS[b]);
            }
        }
        // A road leads to towns.
        if let Some(e) = s.edges.iter().find(|e| e.contains("road")) {
            let c = format!("follow {}", e.split_whitespace().last().unwrap_or(e));
            if self.roll(4).is_multiple_of(3) && self.last.as_ref().is_none_or(|(l, _, _)| *l != c)
            {
                return c;
            }
        }
        // Barred by water: the scholar (kept well) wades or swims across;
        // the explorer follows the bank, as a careful walker would.
        if let Some((cmd, _, pos)) = &self.last {
            if cmd.starts_with("head ") && (p.x, p.y) == *pos {
                let water = s.edges.iter().find(|e| {
                    ["river", "stream", "lake", "shore"]
                        .iter()
                        .any(|w| e.contains(w))
                });
                if let Some(e) = water {
                    let w = e.split_whitespace().last().unwrap_or(e);
                    let c = if self.scholar() && !e.contains("lake") && !e.contains("shore") {
                        format!("cross {w}")
                    } else {
                        format!("follow {w}")
                    };
                    return c;
                }
            }
        }
        // Hemmed in: let an hour pass (snow, flood and dark lift in time),
        // then forget what blocked us here and try again.
        if (0..BEARINGS.len()).all(|b| self.blocked.contains(&(square, b))) {
            self.blocked.retain(|(sq, _)| *sq != square);
            return "wait 1 hour".into();
        }
        // Otherwise head for the least-walked direction.
        let mut best = (u32::MAX, 0usize);
        for (b, name) in BEARINGS.iter().enumerate() {
            if self.blocked.contains(&(square, b)) {
                continue;
            }
            let (dx, dy) = bearing_step(name);
            let mut n = 0;
            for k in 1..=4 {
                n += self
                    .walked
                    .get(&(square.0 + dx * k, square.1 + dy * k))
                    .copied()
                    .unwrap_or(0);
            }
            let jitter = (self.roll(b as u64) % 3) as u32;
            if n * 4 + jitter < best.0 {
                best = (n * 4 + jitter, b);
            }
        }
        format!("head {}", BEARINGS[best.1])
    }
}

/// The glyph numbers for a potent claim ("the door opens"), in the
/// writing of the player's era: the scholar's spoiler knowledge of the
/// grammar.
fn claim_glyphs(g: &Game, verb: &str, subject: &str) -> String {
    use scraped_lang::meaning::{
        Argument, Clause, Mood, NounPhrase, Polarity, Role, Sentence, Tense,
    };
    let m = Sentence::Clause(Clause {
        predicate: verb.into(),
        mood: Mood::Potent,
        tense: Tense::NonPast,
        polarity: Polarity::Positive,
        args: vec![Argument {
            role: Role::Subject,
            np: NounPhrase::concept(subject),
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    });
    g.sound_words(&m)
}

/// `claim_glyphs`, said "greatly".
fn claim_glyphs_greatly(g: &Game, verb: &str, subject: &str) -> String {
    use scraped_lang::meaning::{Clause, NounPhrase, Sentence};
    let m =
        Sentence::Clause(Clause::potent(verb, NounPhrase::concept(subject)).with_adverb("greatly"));
    g.sound_words(&m)
}

/// The kind of building a name ends with ("the worn market hall": "market
/// hall"), as a player reads it.
fn kind_named(name: &str) -> &'static str {
    crate::slots::STRUCTURES
        .iter()
        .filter(|k| name.ends_with(*k))
        .max_by_key(|k| k.len())
        .copied()
        .unwrap_or("")
}

/// How likely a building, by its name, is to hold writing, as a reader
/// would guess: lower first, 50 and over not worth the scholar's time.
fn writing_rank(name: &str) -> usize {
    const KEPT: [&str; 10] = [
        "archive",
        "temple",
        "tomb",
        "storehouse",
        "cemetery",
        "house",
        "wall",
        "waystation",
        "bridge",
        "tower",
    ];
    KEPT.iter()
        .position(|k| name.split_whitespace().any(|w| w == *k))
        .map_or(50, |i| i * 2)
}

/// How much a bot wants to carry something, by its name: 0 not at all.
fn want(kind: &str, name: &str) -> u8 {
    let has = |w: &str| name.contains(w);
    let tool = scrapes(name) || has("lens") || has("loupe") || has("stylus");
    let light = has("lamp") || has("torch") || has("oil") || has("firesteel");
    if kind == "scholar" {
        if (has("graver") || has("loupe") || has("mason") || has("penknife")) && tool {
            9
        } else if tool {
            8
        } else if has("oil") || has("torch") {
            7
        } else if light || has("pry") {
            6
        } else {
            0
        }
    } else if has("waterskin")
        || has("provisions")
        || has("food")
        || has("cloak")
        || has("firesteel")
    {
        // Water, food, warmth and the means of fire first: cold kills
        // more explorers than the dark.
        7
    } else if has("wood") {
        6
    } else if has("torch") || has("lamp") {
        5
    } else if light {
        4
    } else if has("key") || has("rope") || has("map") {
        // D05: a key opens something somewhere, a rope climbs back up.
        3
    } else if tool {
        2
    } else {
        0
    }
}

/// Whether the bot carries the means of light: a lamp it hasn't found
/// empty, a lamp and oil to fill it, or a torch and a firesteel.
/// Whether the explorer has what it needs to leave a town for the land:
/// the means to make a fire, and a cloak.
fn equipped(s: &Summary) -> bool {
    let has = |w: &str| s.carried.iter().any(|t| t.contains(w));
    has("firesteel") && has("cloak")
}

fn can_light(s: &Summary, lamp_empty: bool) -> bool {
    let has = |w: &str| s.carried.iter().any(|t| t.contains(w));
    // Any light needs a flame to start it.
    has("firesteel") && ((has("lamp") && (!lamp_empty || has("oil"))) || has("torch"))
}

/// Whether a carried thing, by its name, can scrape a surface (D10:
/// ordinary edged and abrasive tools).
fn scrapes(name: &str) -> bool {
    ["knife", "chisel", "graver", "pumice"]
        .iter()
        .any(|w| name.contains(w))
}

/// The best lens carried: 0 none, 1 a lens, 2 a loupe.
fn carried_lens(s: &Summary) -> u8 {
    if s.carried.iter().any(|c| c.contains("loupe")) {
        2
    } else if s.carried.iter().any(|c| c.contains("lens")) {
        1
    } else {
        0
    }
}

/// The bearing nearest a direction.
fn bearing_to(dx: i32, dy: i32) -> usize {
    let mut best = (f64::MIN, 0);
    for (i, b) in BEARINGS.iter().enumerate() {
        let (bx, by) = bearing_step(b);
        let n = f64::from(bx * bx + by * by).sqrt();
        let dot = (f64::from(bx) * f64::from(dx) + f64::from(by) * f64::from(dy)) / n;
        if dot > best.0 {
            best = (dot, i);
        }
    }
    best.1
}

/// One kilometre square further in a bearing.
fn bearing_step(b: &str) -> (i32, i32) {
    let dx = if b.contains("east") {
        1
    } else if b.contains("west") {
        -1
    } else {
        0
    };
    let dy = if b.contains("north") {
        1
    } else if b.contains("south") {
        -1
    } else {
        0
    };
    (dx, dy)
}

/// One step of the novelty curve: something perceived for the first time.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Novelty {
    pub minutes: u32,
    /// "slot kind", e.g. "thing.name stele".
    pub what: String,
    /// Whether it was a new kind of thing, not just a new variety.
    pub new_kind: bool,
}

/// A bot's whole run on one seed.
#[derive(Debug, Clone, Serialize)]
pub struct BotRun {
    pub bot: String,
    pub seed: u64,
    pub commands: Vec<String>,
    /// Everything the player was told, command by command.
    pub texts: Vec<String>,
    /// The kind of response, for brevity measures: "look", "arrival",
    /// "travel" or "other".
    pub kinds: Vec<&'static str>,
    pub hours: f64,
    pub died: Option<String>,
    pub novelty: Vec<Novelty>,
    pub buildings: usize,
    pub texts_read: usize,
    pub read_deepest: bool,
    pub read_great: bool,
    /// Sealed places (D11) found shut, and those gone into.
    pub sealed_met: usize,
    pub sealed_opened: usize,
    /// What it carried at the end.
    pub carried: Vec<String>,
    /// The bot's own notes on what it set out to do.
    pub notes: Vec<String>,
    /// Buildings entered, by id, and rooms stood in (for diagnosing runs).
    pub entered: Vec<usize>,
    pub rooms: BTreeSet<String>,
    /// Kilometre squares walked.
    pub walked: Vec<(i32, i32)>,
    /// How many texts the words of an opening claim were met in.
    pub words: Vec<(String, usize)>,
    /// For each response, how many facts it mentioned: renders of slots
    /// that say one thing (no lists), outside the parser's replies.
    pub facts: Vec<u32>,
    /// For each arrival somewhere, the facts shown and how many more
    /// digging there could turn up.
    pub on_demand: Vec<(u32, u32)>,
    /// Natural features and old marks stood by (close by: within 300 m),
    /// by id.
    pub features: BTreeSet<usize>,
    /// Towns whose buildings it went into.
    pub towns: usize,
    /// Signs whose sound it heard (S01).
    pub heard: usize,
    /// Per species met: the minute its first sign and the minute it was
    /// first seen were put before the player (D06).
    pub life: BTreeMap<usize, [Option<u32>; 2]>,
    #[serde(skip)]
    pub renders: Vec<Rendered>,
    /// D10: when writing was first set loose by the player's hand (hours
    /// in), and the command that did it.
    pub first_release: Option<(f64, String)>,
    /// D10: responses in the first hour that bring writing forward
    /// (scraping, writing, a release, a great inscription).
    pub pointers_first_hour: u32,
}

/// The family a command's verb belongs to, for the verb mix (S01).
pub fn verb_family(cmd: &str) -> &'static str {
    let c = cmd.trim();
    let first = c.split_whitespace().next().unwrap_or("");
    if c == "look" || c == "l" {
        return "look";
    }
    if c.starts_with("look closer")
        || c.starts_with("look around")
        || c.starts_with("look up")
        || c.starts_with("look down")
        || c.starts_with("look at the ")
        || matches!(first, "listen" | "smell" | "touch" | "taste")
    {
        return "dig";
    }
    if c.starts_with("examine sign") || c.starts_with("trace") {
        return "read";
    }
    match first {
        "examine" | "x" | "inspect" => "examine",
        "read" | "more" | "study" => "read",
        "go" | "head" | "follow" | "out" | "back" | "north" | "south" | "east" | "west" | "up"
        | "down" | "northeast" | "northwest" | "southeast" | "southwest" | "cross" | "first" => {
            "move"
        }
        "take" | "drop" | "open" | "close" | "pry" | "operate" | "use" | "pull" | "push" => {
            "handle"
        }
        "scrape" | "write" => "writing",
        "eat" | "drink" | "sleep" | "forage" | "fish" | "snare" | "gather" | "make" | "fill"
        | "light" | "wear" | "extinguish" | "status" | "wait" | "feed" => "body",
        _ => "other",
    }
}

/// The verb families, in the order they are reported.
pub const VERB_FAMILIES: &[&str] = &[
    "look", "dig", "examine", "read", "move", "handle", "writing", "body", "other",
];

/// The variables that say what something is, for novelty.
const KIND_VARS: &[&str] = &["kind", "purpose", "feature", "mark", "species", "material"];

fn kind_of(r: &Rendered) -> Option<String> {
    let mut parts = Vec::new();
    for k in KIND_VARS {
        if let Some(Value::Text(t)) = r.vars.get(*k) {
            if !t.is_empty() && t.len() <= 24 {
                parts.push(t.clone());
            }
        }
    }
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// What kind of response this was, from the command.
fn response_kind(cmd: &str, before: &Place, after: &Place, moved: bool) -> &'static str {
    let verb = cmd.split_whitespace().next().unwrap_or("");
    if verb == "look" {
        "look"
    } else if moved && *after == Place::Outside && matches!(verb, "go" | "head" | "follow") {
        "travel"
    } else if before != after {
        "arrival"
    } else {
        "other"
    }
}

/// Plays one bot on one seed until `hours` of game time pass, it dies, or
/// `max_steps` commands.
pub fn play(pack: &Pack, seed: u64, kind: &'static str, hours: f64, max_steps: usize) -> BotRun {
    play_with(pack, seed, kind, hours, max_steps, |_| {})
}

/// Like `play`, with the game set up first (a tool in hand, say).
pub fn play_with(
    pack: &Pack,
    seed: u64,
    kind: &'static str,
    hours: f64,
    max_steps: usize,
    setup: impl FnOnce(&mut Game),
) -> BotRun {
    let mut g = Game::new(seed, pack.clone());
    g.keep_renders = true;
    // The scholar measures how far the late game can be reached, not
    // survival (that's the explorer's measure), so its body is kept well.
    // DESIGN-Q: the scholar plays sustained.
    g.sustain = kind == "scholar";
    let mut out = g.start();
    let before = g.state.place;
    setup(&mut g);
    if g.state.place != before {
        out = g.step("look");
    }
    let mut bot = DepthBot::new(kind, seed);
    let start = g.state.minutes;
    let mut run = BotRun {
        bot: kind.to_string(),
        seed,
        commands: Vec::new(),
        texts: vec![out.text.clone()],
        kinds: vec!["arrival"],
        hours: 0.0,
        died: None,
        novelty: Vec::new(),
        buildings: 0,
        texts_read: 0,
        read_deepest: false,
        sealed_met: 0,
        sealed_opened: 0,
        read_great: false,
        carried: Vec::new(),
        notes: Vec::new(),
        entered: Vec::new(),
        rooms: BTreeSet::new(),
        walked: Vec::new(),
        words: Vec::new(),
        facts: Vec::new(),
        on_demand: Vec::new(),
        features: BTreeSet::new(),
        towns: 0,
        heard: 0,
        life: BTreeMap::new(),
        renders: Vec::new(),
        first_release: None,
        pointers_first_hour: 0,
    };
    let scraped_at_start = g.state.scraped.clone();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut kinds_seen: BTreeSet<String> = BTreeSet::new();
    let mut taken = 0;
    let mut note = |g: &Game, run: &mut BotRun, taken: &mut usize| {
        for r in &g.renders[*taken..] {
            if let Some(k) = kind_of(r) {
                let family = r.trace.slot.split('.').next().unwrap_or("").to_string();
                let what = format!("{} {k}", r.trace.slot);
                if seen.insert(what.clone()) {
                    let first_word = k.split_whitespace().next().unwrap_or("").to_string();
                    let new_kind = kinds_seen.insert(format!("{family} {first_word}"));
                    run.novelty.push(Novelty {
                        minutes: r.minutes - start,
                        what,
                        new_kind,
                    });
                }
            }
        }
        *taken = g.renders.len();
    };
    // A fact: a render of a slot, outside the parser's replies, whose text
    // the player was actually shown.
    // Facts are said as sentences, so compare without case.
    // A name said inside a sentence is part of that fact, not another.
    // A list ("Ways out: …") is one fact, as the attention model weighs it;
    // the names within it are part of it.
    let count_facts = |rs: &[Rendered], text: &str| {
        let text = text.to_lowercase();
        let shown: Vec<String> = rs
            .iter()
            .filter(|r| !r.trace.slot.starts_with("say."))
            .map(|r| r.trace.text.trim().to_lowercase())
            .filter(|t| !t.is_empty() && text.contains(t.as_str()))
            .collect();
        shown
            .iter()
            .filter(|t| {
                !shown
                    .iter()
                    .any(|o| o.len() > t.len() && o.contains(t.as_str()))
            })
            .count() as u32
    };
    run.facts.push(count_facts(&g.renders, &out.text));
    note(&g, &mut run, &mut taken);
    for _ in 0..max_steps {
        if g.state.dead.is_some() || f64::from(g.state.minutes - start) >= hours * 60.0 {
            break;
        }
        let cmd = bot.next(&g, &out);
        let before = g.state.place;
        let pos = g.state.pos;
        out = g.step(&cmd);
        let moved = g.state.pos.dist(pos) > 100.0;
        run.kinds
            .push(response_kind(&cmd, &before, &g.state.place, moved));
        run.commands.push(cmd);
        if out.state.place != "outside" {
            let lit = if out.state.light == "dark" {
                " (dark)"
            } else {
                ""
            };
            run.rooms.insert(format!("{}{lit}", out.state.place));
        }
        run.texts.push(out.text.clone());
        let before_novel = run.novelty.len();
        run.facts.push(count_facts(&g.renders[taken..], &out.text));
        if matches!(run.kinds.last(), Some(&"arrival" | &"travel")) && g.state.dead.is_none() {
            let shown = run.facts.last().copied().unwrap_or(0);
            run.on_demand.push((shown, g.on_demand() as u32));
        }
        let hour = f64::from(g.state.minutes - start) / 60.0;
        if hour <= 1.0
            && g.renders[taken.min(g.renders.len())..].iter().any(|r| {
                let sl = r.trace.slot.as_str();
                // A great site's air is felt, not named as writing (D10).
                sl.starts_with("scrape.")
                    || sl.starts_with("write.")
                    || (sl.starts_with("great.") && sl != "great.site")
                    || matches!(sl, "story.release" | "story.first_write")
            })
        {
            run.pointers_first_hour += 1;
        }
        if run.first_release.is_none()
            && g.state.scraped.iter().any(|t| {
                !scraped_at_start.contains(t)
                    && g.text(*t).kind == scraped_lang::corpus::Kind::Potent
            })
        {
            run.first_release = Some((hour, run.commands.last().cloned().unwrap_or_default()));
        }
        note(&g, &mut run, &mut taken);
        if g.state.place == Place::Outside {
            for f in &g.site.world.features {
                let fp = scraped_sim::outdoors::Pos::of_cell(f.cell.ux(), f.cell.uy());
                if fp.dist(g.state.pos) <= f64::from(scraped_sim::outdoors::LOCAL) {
                    run.features.insert(f.id);
                }
            }
        }
        if run.novelty.len() == before_novel {
            bot.idle += 1;
        } else {
            bot.idle = 0;
        }
    }
    run.hours = f64::from(g.state.minutes - start) / 60.0;
    run.died = g.state.dead.as_ref().map(|d| d.cause.clone());
    run.buildings = g.state.visited.len();
    run.texts_read = g.state.read.len();
    let deep = &g.site.writing.deep;
    run.read_deepest = deep.first().is_some_and(|d| g.state.read.contains(d));
    run.sealed_met = bot
        .sealed
        .iter()
        .map(|x| x.noun.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    run.sealed_opened = g
        .site
        .writing
        .sealed
        .iter()
        .filter(|x| g.state.visited.contains(&x.structure))
        .count();
    run.read_great = g
        .site
        .greats
        .iter()
        .any(|gr| g.state.read.contains(&gr.text));
    run.carried = out.state.carried.clone();
    run.notes = std::mem::take(&mut bot.notes);
    run.walked = bot.walked.keys().copied().collect();
    run.words = ["open", "door", "gate", "tomb", "box"]
        .iter()
        .map(|w| {
            (
                w.to_string(),
                g.state.encountered.get(*w).map_or(0, |t| t.len()),
            )
        })
        .collect();
    run.entered = g.state.visited.iter().copied().collect();
    run.heard = g.state.heard.len();
    run.life = g.life_log.clone();
    run.towns = g
        .state
        .visited
        .iter()
        .filter_map(|&b| g.site.world.structures[b].settlement)
        .collect::<BTreeSet<usize>>()
        .len();
    run.renders = std::mem::take(&mut g.renders);
    run
}

/// One response as the player would see it, with the slots behind it.
#[derive(Debug, Clone, Serialize)]
pub struct Assembled {
    pub command: String,
    pub text: String,
    pub slots: Vec<String>,
}

/// The digging verbs tried first, so a writer sees every kind of response.
const DIGGING: &[&str] = &[
    "look",
    "look closer",
    "look around",
    "listen",
    "smell",
    "look up",
    "look down",
    "touch ground",
    "check myself",
];

/// Responses as the attention model assembles them, for the authoring
/// tool: the digging verbs, then an explorer's first steps, keeping those
/// in which `slot` was said (all of them when `slot` is empty), up to
/// `max`. Jb writes for the budget by seeing a piece among its
/// neighbours.
pub fn in_context(pack: &Pack, seed: u64, slot: &str, max: usize) -> Vec<Assembled> {
    let mut g = Game::new(seed, pack.clone());
    g.trace = true;
    let mut out = g.start();
    let mut bot = DepthBot::new("explorer", seed);
    let mut found = Vec::new();
    let keep = |command: &str, out: &Output, found: &mut Vec<Assembled>| {
        let slots: Vec<String> = out.renders.iter().map(|r| r.trace.slot.clone()).collect();
        if slot.is_empty() || slots.iter().any(|s| s == slot) {
            found.push(Assembled {
                command: command.to_string(),
                text: out.text.clone(),
                slots,
            });
        }
    };
    keep("", &out, &mut found);
    for c in DIGGING {
        out = g.step(c);
        keep(c, &out, &mut found);
    }
    for _ in 0..120 {
        if found.len() >= max || g.state.dead.is_some() {
            break;
        }
        let c = bot.next(&g, &out);
        out = g.step(&c);
        keep(&c, &out, &mut found);
    }
    found.truncate(max);
    found
}
