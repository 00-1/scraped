//! Obvious next steps (S07 §5) and small, quick payoffs (S07 §7).
//!
//! Verbs a person standing there would try (put, offer, pray, play, unlock,
//! break, knock, shake, lift) are understood and answered in the world.
//! Small payoffs are scattered by the game layer over things already
//! there (a hollow wall, a loose stone, a jammed lid, a fruit tree), so
//! worlds generate as before; each has a cue in one category of noticing
//! (D13) and gives something.

use serde::{Deserialize, Serialize};

use scraped_content::{Context, SlotDef, Value, VarType};
use scraped_sim::outdoors::hash;

use crate::parser::{resolve, Candidate, Resolution};
use crate::site::{ctx, Place, Site};
use crate::{Game, Output, Target};

/// Instruments a player can play.
pub const INSTRUMENTS: &[&str] = &[
    "bell", "drum", "flute", "horn", "lyre", "rattle", "sistrum", "whistle",
];

/// A small payoff on a thing (S07 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payoff {
    pub thing: usize,
    pub kind: &'static str,
    /// The item it gives.
    pub gives: &'static str,
}

/// The kinds of payoff, the category their cue is noticed in, and the
/// verbs that solve them.
// DESIGN-Q: four kinds to start (a hollow wall, a loose stone, a jammed
// lid, a fruit tree); the rest of S07's list waits for Jb.
pub const KINDS: &[(&str, &str, &[&str])] = &[
    ("hollow_wall", "sounds", &["knock", "touch", "listen"]),
    ("loose_stone", "buildings", &["lift", "operate", "pry"]),
    ("jammed_lid", "things", &["pry", "open", "operate"]),
    ("fruit_tree", "life", &["shake", "operate"]),
];

/// The payoffs of a world, from its things: about one in two buildings
/// has one, some standing stones in the open too, and the start town's
/// nearest building always has one in its first room.
// DESIGN-Q: one in two buildings; one in three stones outdoors; what
// each gives.
pub fn payoffs(site: &Site) -> Vec<Payoff> {
    let seed = site.world.seed;
    let start = site.start();
    // Each thing that could carry one: (thing, kind, structure, room).
    let mut cands: Vec<(usize, &'static str, Option<usize>, usize)> = Vec::new();
    for (i, th) in site.things.iter().enumerate() {
        if site.dormant.contains(&i) {
            continue;
        }
        let (structure, room) = match th.home {
            Place::Room { structure, room } => (Some(structure), room),
            Place::Outside => (None, 0),
        };
        let kind = match (th.kind, structure) {
            ("wall", Some(_)) => "hollow_wall",
            ("hearth" | "altar" | "niche" | "bench", _) => "loose_stone",
            ("stele" | "cairn" | "statue" | "pillar" | "block" | "carving" | "well", None) => {
                "loose_stone"
            }
            ("tree", _) => "fruit_tree",
            _ => match th.object {
                Some(o) => {
                    let ob = &site.world.objects[o];
                    let holds = ob.family == scraped_world::objects::Family::Box
                        || matches!(ob.kind, "urn" | "jar" | "amphora");
                    if holds && ob.key.is_none() {
                        "jammed_lid"
                    } else {
                        continue;
                    }
                }
                None => continue,
            },
        };
        cands.push((i, kind, structure, room));
    }
    // Within a short walk of the start: a stone in the open there, else
    // the start town's nearest building, first room.
    // DESIGN-Q: within 300 m of the start counts as a short walk.
    let open_near = cands
        .iter()
        .filter(|c| c.2.is_none() && site.things[c.0].pos.dist(start) <= 300.0)
        .min_by(|a, b| {
            site.things[a.0]
                .pos
                .dist(start)
                .partial_cmp(&site.things[b.0].pos.dist(start))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        })
        .map(|c| c.0);
    let first = open_near.or(cands
        .iter()
        .filter(|c| c.2.is_some_and(|s| site.structures.contains(&s)))
        .min_by(|a, b| {
            let d = |c: &(usize, &str, Option<usize>, usize)| {
                site.land.structure_pos[c.2.unwrap_or(0)].dist(start)
            };
            d(a).partial_cmp(&d(b))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.3.cmp(&b.3))
                .then(a.0.cmp(&b.0))
        })
        .map(|c| c.0));
    let mut out: Vec<Payoff> = Vec::new();
    let mut by_structure: std::collections::BTreeSet<usize> = Default::default();
    for (i, kind, structure, _) in cands {
        if let Some(s) = structure {
            if by_structure.contains(&s) && Some(i) != first {
                continue;
            }
        }
        let h = hash(&[seed, 0x9a70, i as u64]);
        let odds = if structure.is_some() { 2 } else { 3 };
        if !h.is_multiple_of(odds) && Some(i) != first {
            continue;
        }
        let gives = match kind {
            "fruit_tree" => ["berries", "nuts"][(h / 4 % 2) as usize],
            "jammed_lid" => ["provisions", "oil", "nuts", "charcoal"][(h / 4 % 4) as usize],
            _ => [
                "provisions",
                "torch",
                "knife",
                "penknife",
                "oil",
                "firesteel",
                "provisions",
                "torch",
            ][(h / 4 % 8) as usize],
        };
        if let Some(s) = structure {
            by_structure.insert(s);
        }
        out.push(Payoff {
            thing: i,
            kind,
            gives,
        });
    }
    out
}

impl Game {
    /// The undone payoff on a thing, if any.
    fn payoff_on(&self, t: usize) -> Option<Payoff> {
        self.payoffs
            .iter()
            .find(|p| p.thing == t && !self.state.payoffs_done.contains(&t))
            .copied()
    }

    /// Cues of the payoffs here (S07 §7): each noticed in its category.
    pub(crate) fn payoff_facts(&self, out: &mut Vec<crate::attention::Fact>) {
        for t in self.here() {
            if let Some(p) = self.payoff_on(t) {
                let cat = KINDS
                    .iter()
                    .find(|k| k.0 == p.kind)
                    .map_or("things", |k| k.1);
                out.push(crate::attention::Fact::new(
                    "payoff.cue",
                    format!("payoff:{t}"), // DEBUG-TEXT: a key
                    // DESIGN-Q: as noticeable as a felt strangeness.
                    44.0,
                    ctx(&[
                        ("kind", Value::from(p.kind)),
                        ("thing", Value::from(self.thing(t).kind)),
                        ("category", Value::from(cat)),
                    ]),
                ));
            }
        }
    }

    /// A thing named, among what is here or carried.
    fn named(&mut self, words: &[String]) -> Option<usize> {
        let cands: Vec<Candidate<Target>> = self
            .visible_targets()
            .into_iter()
            .filter(|c| matches!(c.target, Target::Thing(_)))
            .collect();
        match resolve(words, &cands, self.state.it.as_ref()) {
            Resolution::One(Target::Thing(t)) => Some(t),
            Resolution::Many(ts) => ts.into_iter().find_map(|t| match t {
                Target::Thing(t) => Some(t),
                _ => None,
            }),
            _ => None,
        }
    }

    fn carrying_tool(&self, kinds: &[&str]) -> Option<&'static str> {
        self.state
            .carried
            .iter()
            .map(|&t| self.thing(t).kind)
            .find(|k| kinds.contains(k))
    }

    /// Solves a payoff if the verb fits it (S07 §7): one step, in one place,
    /// with an ordinary verb, giving something.
    pub(crate) fn try_payoff(&mut self, verb: &str, t: usize) -> Option<Output> {
        let p = self.payoff_on(t)?;
        let verbs = KINDS.iter().find(|k| k.0 == p.kind)?.2;
        if !verbs.contains(&verb) {
            return None;
        }
        let thing = self.thing(t).kind;
        // A jammed lid wants a blade or a bar under it.
        // DESIGN-Q: a knife, penknife, pry bar or chisel works it loose.
        let tool = if p.kind == "jammed_lid" {
            match self.carrying_tool(&["pry_bar", "knife", "penknife", "mason_chisel"]) {
                Some(k) => k,
                None => {
                    self.pass(2);
                    let tx = self.say(
                        "payoff.stuck",
                        ctx(&[("kind", Value::from(p.kind)), ("thing", Value::from(thing))]),
                    );
                    return Some(self.output(vec![tx], None));
                }
            }
        } else {
            ""
        };
        self.pass(5);
        self.state.payoffs_done.insert(t);
        let item = self.make_item(p.gives, false);
        let name = item.map(|i| self.thing_name(i)).unwrap_or_default();
        let tx = self.say(
            "payoff.found",
            ctx(&[
                ("kind", Value::from(p.kind)),
                ("thing", Value::from(thing)),
                ("item", Value::from(name)),
                ("tool", Value::from(tool)),
            ]),
        );
        Some(self.output(vec![tx], None))
    }

    /// The verbs of the obvious next steps (S07 §5).
    pub(crate) fn obvious(&mut self, verb: &str, words: &[String]) -> Output {
        // "put the token on the altar", "unlock the door with the key".
        let split = words.iter().position(|w| {
            matches!(
                w.as_str(),
                "on" | "onto" | "in" | "into" | "upon" | "with" | "at"
            )
        });
        let (first, second) = match split {
            Some(i) => (&words[..i], &words[i + 1..]),
            None => (words, &[][..]),
        };
        let what = if first.is_empty() {
            None
        } else {
            self.named(first)
        };
        let onto = if second.is_empty() {
            None
        } else {
            self.named(second)
        };
        if let Some(t) = what {
            if let Some(o) = self.try_payoff(verb, t) {
                return o;
            }
        }
        match verb {
            "put" => {
                let Some(t) = what.filter(|t| self.state.carried.contains(t)) else {
                    return self.not_here(first);
                };
                self.pass(1);
                // Set down where the player is: here, or on the thing named.
                self.state.carried.retain(|&c| c != t);
                if self.state.place == Place::Outside {
                    self.state.dropped.insert(t, self.state.pos);
                } else {
                    self.state.moved.insert(t, self.state.place);
                }
                let on = onto.map(|o| self.thing(o).kind).unwrap_or("");
                let name = self.thing_name(t);
                let tx = self.say(
                    "put.done",
                    ctx(&[
                        ("thing", Value::from(name)),
                        ("on", Value::from(on)),
                        ("altar", Value::Bool(matches!(on, "altar" | "shrine"))),
                    ]),
                );
                self.output(vec![tx], None)
            }
            "pray" => {
                self.pass(5);
                let altar = self
                    .here()
                    .into_iter()
                    .any(|t| matches!(self.thing(t).kind, "altar" | "shrine" | "statue"));
                let tx = self.say("pray.done", ctx(&[("altar", Value::Bool(altar))]));
                self.output(vec![tx], None)
            }
            "play" => {
                let Some(t) = what else {
                    return self.not_here(first);
                };
                let kind = self.kind_of(t);
                let instrument = INSTRUMENTS.contains(&kind);
                self.pass(if instrument { 5 } else { 1 });
                if instrument {
                    self.state.noise = self.state.noise.max(60);
                }
                let tx = self.say(
                    "play.done",
                    ctx(&[
                        ("thing", Value::from(kind)),
                        ("instrument", Value::Bool(instrument)),
                    ]),
                );
                self.output(vec![tx], None)
            }
            // A key or token tried in a lock is opening with it in hand.
            "unlock" => self.command(crate::parser::Command {
                verb: "open".into(),
                words: first.to_vec(),
            }),
            "break" => {
                let Some(t) = what else {
                    return self.not_here(first);
                };
                self.pass(2);
                let kind = self.thing(t).kind;
                let tx = self.say("break.no", ctx(&[("thing", Value::from(kind))]));
                self.output(vec![tx], None)
            }
            "knock" | "shake" | "lift" => {
                let Some(t) = what else {
                    return self.not_here(first);
                };
                self.pass(1);
                let kind = self.thing(t).kind;
                let tx = self.say(
                    "try.nothing",
                    ctx(&[("verb", Value::from(verb)), ("thing", Value::from(kind))]),
                );
                self.output(vec![tx], None)
            }
            _ => self.not_here(words),
        }
    }

    fn not_here(&mut self, words: &[String]) -> Output {
        let tx = self.say(
            "say.not_here",
            ctx(&[("words", Value::from(words.join(" ")))]),
        );
        self.output(vec![tx], None)
    }
}

fn s_cue(_: u64) -> Vec<Context> {
    KINDS
        .iter()
        .map(|(k, c, _)| {
            ctx(&[
                ("kind", Value::from(*k)),
                (
                    "thing",
                    Value::from(match *k {
                        "hollow_wall" => "wall",
                        "loose_stone" => "hearth",
                        "jammed_lid" => "chest",
                        _ => "tree",
                    }),
                ),
                ("category", Value::from(*c)),
            ])
        })
        .collect()
}

fn s_found(_: u64) -> Vec<Context> {
    s_cue(0)
        .into_iter()
        .map(|mut c| {
            c.insert("item".into(), Value::from("a torch"));
            c.insert("tool".into(), Value::from("knife"));
            c
        })
        .collect()
}

fn kinds_enum() -> VarType {
    VarType::Enum {
        values: KINDS.iter().map(|k| k.0.to_string()).collect(),
    }
}

/// S07's slots for obvious next steps and small payoffs.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("payoff.cue", "A small thing that catches the eye or ear (S07 §7), hinting at a quick find: a hollow note in a wall, a stone that sits proud, a lid askew, fruit hanging heavy. Noticed by keenness in its category. Never say what to do or what is there.")
            .var("kind", kinds_enum(), "hollow_wall, loose_stone, jammed_lid or fruit_tree.")
            .var("thing", VarType::Text, "The thing it is on.")
            .var("category", VarType::Text, "The kind of noticing it shows in (sounds, buildings, things, life).")
            .max_len(160)
            .sampler(s_cue),
        SlotDef::new("payoff.found", "A small payoff found (S07 §7): the wall's loose stone comes away, the stone lifts, the lid gives, the fruit falls; and what was there.")
            .var("kind", kinds_enum(), "Which kind.")
            .var("thing", VarType::Text, "The thing.")
            .var("item", VarType::Text, "What it gave, with its article.")
            .var("tool", VarType::Text, "For a lid: what worked it loose; else empty.")
            .max_len(220)
            .sampler(s_found),
        SlotDef::new("payoff.stuck", "A jammed lid that won't give by hand (S07 §7): it wants something thin and strong under it. Never name the tool outright.")
            .var("kind", kinds_enum(), "Which kind.")
            .var("thing", VarType::Text, "The thing.")
            .sampler(s_cue),
        SlotDef::new("put.done", "The player puts or offers something somewhere (S07 §5). Said in the world; if it is an altar, nothing happens (yet).")
            .var("thing", VarType::Text, "What was put, with its article.")
            .var("on", VarType::Text, "What it was put on or in, or empty for here.")
            .var("altar", VarType::Bool, "Set on an altar or shrine.")
            .sampler(|_| vec![ctx(&[("thing", Value::from("a token")), ("on", Value::from("altar")), ("altar", Value::Bool(true))])]),
        SlotDef::new("pray.done", "The player prays (S07 §5). Said in the world; nothing answers.")
            .var("altar", VarType::Bool, "Before an altar, shrine or statue.")
            .sampler(|_| vec![ctx(&[("altar", Value::Bool(true))]), ctx(&[("altar", Value::Bool(false))])]),
        SlotDef::new("play.done", "The player plays an instrument, or tries to play something that isn't one (S07 §5).")
            .var("thing", VarType::Text, "What they played.")
            .var("instrument", VarType::Bool, "Whether it is an instrument.")
            .sampler(|_| vec![ctx(&[("thing", Value::from("flute")), ("instrument", Value::Bool(true))]), ctx(&[("thing", Value::from("jar")), ("instrument", Value::Bool(false))])]),
        SlotDef::new("break.no", "The player tries to break something (S07 §5): it doesn't break, or they think better of it. In the world, never 'you can't'.")
            .var("thing", VarType::Text, "What they tried to break.")
            .sampler(|_| vec![ctx(&[("thing", Value::from("stair"))])]),
        SlotDef::new("try.nothing", "The player knocks on, shakes or lifts something with nothing to find (S07 §5).")
            .var("verb", VarType::Enum { values: vec!["knock".into(), "shake".into(), "lift".into()] }, "What they did.")
            .var("thing", VarType::Text, "To what.")
            .sampler(|_| vec![ctx(&[("verb", Value::from("knock")), ("thing", Value::from("wall"))])]),
    ]
}
