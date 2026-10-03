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
    /// Old writing holds a way shut here: the way, and claims cast.
    held: Option<(String, u8)>,
    /// A hearth to make a fire in.
    hearth: bool,
}

impl Room {
    fn open_exits(&self) -> impl Iterator<Item = &String> {
        self.exits
            .iter()
            .filter(|e| !self.leads.contains_key(*e) && !self.barred.contains(*e))
    }
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
    shut: BTreeSet<((i32, i32), String)>,
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
    /// Renders already looked at.
    renders: usize,
    /// Commands given since anything new was perceived.
    pub idle: u32,
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
            shut: BTreeSet::new(),
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
            idle: 0,
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
            }
        }
        self.last = Some((
            cmd.clone(),
            last.state.place.clone(),
            (g.state.pos.x, g.state.pos.y),
        ));
        cmd
    }

    /// Takes in what the last command did.
    fn learn(&mut self, g: &Game, last: &Output) {
        let s = &last.state;
        let here = s.place.clone();
        if here != "outside" {
            let r = self.rooms.entry(here.clone()).or_default();
            r.exits = s.exits.clone();
            r.hearth |= s.things.iter().any(|t| t.contains("hearth"));
        }
        if let Some((cmd, from, pos)) = self.last.clone() {
            // Inside, through a way: note where it led, both ways.
            if from.starts_with("structure") && here.starts_with("structure") && from != here {
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
            // Into a building from outside, or a heading that went nowhere.
            if from == "outside" {
                let sq = square_of(g);
                if let Some(n) = cmd.strip_prefix("go ") {
                    if here == "outside" {
                        self.shut.insert((sq, n.to_string()));
                    } else if let Some(b) = building_of(&here) {
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
            // Fainter layers beneath what was read.
            if cmd.starts_with("read ") || cmd == "more" {
                let ghosts = g.renders[self.renders.min(g.renders.len())..]
                    .iter()
                    .any(|r| r.trace.slot == "read.ghosts");
                if ghosts && here != "outside" {
                    let lens = self.lens;
                    let r = self.rooms.entry(here.clone()).or_default();
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
        self.lens = self.lens.max(carried_lens(s));
        if s.place == "outside" {
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
        if let Some(c) = self.body(g, last) {
            return c;
        }
        if g.state.reading.is_some() && (self.scholar() || !self.roll(1).is_multiple_of(3)) {
            return "more".into();
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
            if outside && self.worth(g, s, "forage") {
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
        if cold && has("cloak") && self.worth(g, s, "wear cloak") {
            return Some("wear cloak".into());
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
            if self.worth(g, s, "sleep") {
                return Some("sleep".into());
            }
            return Some("wait 1 hour".into());
        }
        if outside && (night || evening || shivering) && s.exits.is_empty() {
            // The nearest town or building in view, while it's still light.
            let order = ["near", "short", "middle", "far", "horizon"];
            let shelter = s
                .landmarks
                .iter()
                .filter(|l| !["hill", "mountain"].iter().any(|k| l.name.ends_with(k)))
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
        let settled = !outside && self.next_way(s).is_none();
        if (blind && outside) || (night && settled) {
            // Nothing to see by, or nowhere to go before morning: rest.
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
        let can_scrape = s.carried.iter().any(|c| c.contains("scraper"));
        let load_room = s.load.0 + 3 <= s.load.1;
        let room = self.rooms.entry(s.place.clone()).or_default();
        for (i, name) in s.things.iter().enumerate() {
            let n = noun(&s.things, i);
            if room.done.insert((name.clone(), "examine")) {
                return Some(format!("examine {n}"));
            }
            if room.written.contains(name) {
                if room.done.insert((name.clone(), "read")) {
                    return Some(format!("read {n}"));
                }
                if scholar && can_scrape && room.done.insert((name.clone(), "scrape")) {
                    return Some(format!("scrape {n}"));
                }
            }
            let w = want(kind, name);
            if room.items.contains(name) && w > 0 && !room.done.contains(&(name.clone(), "take")) {
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
        self.route(here, &entrance).unwrap_or_else(|| "out".into())
    }

    /// A way not yet taken from this room; stairs only with a light to
    /// see the way back by.
    fn next_way(&self, s: &Summary) -> Option<String> {
        let stairs = can_light(s, self.lamp_empty);
        self.rooms.get(&s.place).and_then(|r| {
            r.open_exits()
                .find(|e| stairs || !matches!(e.as_str(), "up" | "down"))
                .cloned()
        })
    }

    /// The scholar answers writing that holds a way shut with writing of
    /// its own: "the door opens", written on a blank surface here, left to
    /// dry and scraped, until the way gives.
    fn unhold(&mut self, g: &Game, last: &Output) -> Option<String> {
        let s = &last.state;
        let here = s.place.clone();
        let (way, cast) = self.rooms.get(&here)?.held.clone()?;
        let tools = ["stylus", "scraper"]
            .iter()
            .all(|t| s.carried.iter().any(|c| c.contains(t)));
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

    fn inside(&mut self, last: &Output) -> String {
        let s = &last.state;
        let here = s.place.clone();
        // A way that didn't let us through: open it, or prise it open.
        if let Some((cmd, from, _)) = self.last.clone() {
            if from == here && s.exits.contains(&cmd) {
                let can_pry = s.carried.iter().any(|c| c.contains("pry"));
                let r = self.rooms.entry(here.clone()).or_default();
                if r.done.insert((format!("way {cmd}"), "open")) {
                    return format!("open {cmd}");
                }
                if can_pry && r.done.insert((format!("way {cmd}"), "pry")) {
                    return format!("pry {cmd}");
                }
                r.barred.insert(cmd.clone());
                r.pried |= can_pry;
            }
            if from == here && (cmd.starts_with("open ") || cmd.starts_with("pry ")) {
                // Opened (or tried to): go through.
                let way = cmd.split_whitespace().last().unwrap_or("").to_string();
                if s.exits.contains(&way)
                    && !self
                        .rooms
                        .get(&here)
                        .is_some_and(|r| r.barred.contains(&way))
                {
                    return way;
                }
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
                if r.open_exits().next().is_none() {
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
        let write = ["stylus", "scraper"]
            .iter()
            .all(|t| s.carried.iter().any(|c| c.contains(t)));
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
            r.open_exits()
                .any(|e| stairs || !matches!(e.as_str(), "up" | "down"))
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

        // The scholar names the spot before going in, to find it again.
        if self.scholar() && !s.exits.is_empty() && !self.named.contains_key(&square) {
            // Letters, not numbers: a number reads as "the third".
            let n = self.named.len();
            let letter = |k: usize| char::from(b'a' + (k % 26) as u8);
            let name = format!("cairn {}{}", letter(n / 26), letter(n));
            self.named.insert(square, name.clone());
            return format!("name here as {name}");
        }
        // Buildings here not yet entered.
        let tried = self.tried_here.entry(square).or_default();
        for i in 0..s.exits.len() {
            let n = noun(&s.exits, i);
            if tried.insert(n.clone()) {
                return format!("go {n}");
            }
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
        fresh.sort_by_key(|l| order.iter().position(|o| *o == l.distance).unwrap_or(9));
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
        // Hemmed in: forget what blocked us here and try again.
        if (0..BEARINGS.len()).all(|b| self.blocked.contains(&(square, b))) {
            self.blocked.retain(|(sq, _)| *sq != square);
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
    });
    let era = g.writing_era();
    let r = g.site.world.renderer(era as u32);
    let script = &g.site.world.languages[era].script;
    r.glyphs(&r.render(&m))
        .into_iter()
        .map(|k| match k {
            Some(k) => script.index(&k).to_string(),
            None => "/".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// How much a bot wants to carry something, by its name: 0 not at all.
fn want(kind: &str, name: &str) -> u8 {
    let has = |w: &str| name.contains(w);
    let tool = has("scraper") || has("lens") || has("stylus");
    let light = has("lamp") || has("torch") || has("oil") || has("firesteel");
    if kind == "scholar" {
        if has("first") && tool {
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
    } else if has("waterskin") || has("provisions") || has("food") {
        7
    } else if light || has("cloak") {
        6
    } else if has("wood") {
        3
    } else if tool {
        2
    } else {
        0
    }
}

/// Whether the bot carries the means of light: a lamp it hasn't found
/// empty, a lamp and oil to fill it, or a torch and a firesteel.
fn can_light(s: &Summary, lamp_empty: bool) -> bool {
    let has = |w: &str| s.carried.iter().any(|t| t.contains(w));
    (has("lamp") && (!lamp_empty || has("oil"))) || (has("torch") && has("firesteel"))
}

/// The best lens carried: 0 none, 1 a lens, 2 the first lens.
fn carried_lens(s: &Summary) -> u8 {
    if s.carried
        .iter()
        .any(|c| c.contains("first") && c.contains("lens"))
    {
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
    #[serde(skip)]
    pub renders: Vec<Rendered>,
}

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
    let mut g = Game::new(seed, pack.clone());
    g.keep_renders = true;
    // The scholar measures how far the late game can be reached, not
    // survival (that's the explorer's measure), so its body is kept well.
    // DESIGN-Q: the scholar plays sustained.
    g.sustain = kind == "scholar";
    let mut out = g.start();
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
        read_great: false,
        carried: Vec::new(),
        notes: Vec::new(),
        entered: Vec::new(),
        rooms: BTreeSet::new(),
        walked: Vec::new(),
        words: Vec::new(),
        facts: Vec::new(),
        renders: Vec::new(),
    };
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
    // A fact: a render of a slot that says one thing (no lists), outside
    // the parser's replies, whose text the player was actually shown.
    let count_facts = |rs: &[Rendered], text: &str| {
        rs.iter()
            .filter(|r| !r.trace.slot.starts_with("say."))
            .filter(|r| !r.vars.values().any(|v| matches!(v, Value::List(_))))
            .filter(|r| !r.trace.text.trim().is_empty() && text.contains(r.trace.text.trim()))
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
        note(&g, &mut run, &mut taken);
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
    run.renders = std::mem::take(&mut g.renders);
    run
}
