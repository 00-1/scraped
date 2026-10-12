//! D15: ways of playing. Each play style has an arc of its own, with
//! small payoffs, growing mastery and large payoffs, and a way for a run
//! to end well that needs no writing. Steps are worked out from the state
//! after every command and said once, in Jb's words (`arc.step`).

use std::collections::BTreeSet;

use scraped_content::{Context, SlotDef, Value, VarType};
use serde::{Deserialize, Serialize};

use crate::site::{ctx, Place};
use crate::{Game, Output};

/// The arcs, one per style (D13), by the style's id.
pub const ARCS: &[&str] = &["forager", "mason", "wayfarer", "scribe"];

/// Every step of every arc: (arc, step, large). Large steps change what
/// the player can do or are the way to an ending.
// DESIGN-Q: the steps and which are large are proposed defaults (D15).
pub const STEPS: &[(&str, &str, bool)] = &[
    ("forager", "first_food", false),
    ("forager", "fruit", false),
    ("forager", "practised", false),
    ("forager", "camp", true),
    ("forager", "store", true),
    ("forager", "companion", true),
    ("mason", "hidden", false),
    ("mason", "first_works", false),
    ("mason", "works_known", false),
    ("mason", "hidden_way", true),
    ("mason", "works_run", true),
    ("mason", "opened_by_hand", true),
    ("mason", "whole_interior", true),
    ("wayfarer", "landmark", false),
    ("wayfarer", "recognised", false),
    ("wayfarer", "far_walked", false),
    ("wayfarer", "summit", true),
    ("wayfarer", "map_walked", true),
    ("wayfarer", "rim_seen", true),
    ("scribe", "first_sound", false),
    ("scribe", "first_read", false),
    ("scribe", "first_written", true),
    ("scribe", "released", true),
];

/// Food the land gives or a house holds, for a store and for feeding.
pub const FOODS: &[&str] = &[
    "provisions",
    "fish",
    "game",
    "berries",
    "nuts",
    "fungi",
    "greens",
    "fruit",
];

/// Food finds that make a forager practised.
// DESIGN-Q: ten finds; practice adds up to 20 points to foraging and half
// again to fishing.
pub const PRACTISED: u32 = 10;

/// Food lying at a camp, or carried there, that makes a store.
// DESIGN-Q: six.
pub const STORE: usize = 6;

/// Times an animal must be fed before it follows.
// DESIGN-Q: three feedings of a scavenger.
pub const TAMED: u8 = 3;

/// Rooms a building must have for seeing all of it to count.
// DESIGN-Q: ten.
pub const GREAT_ROOMS: usize = 10;

/// Blocks of ground walked (about 3 km across each) that make a far
/// walker.
// DESIGN-Q: twenty-five.
pub const FAR: usize = 25;

/// What the arcs remember that the rest of the state doesn't.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArcState {
    /// Steps reached, as "arc:step".
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub reached: BTreeSet<String>,
    /// Things that happened that the state keeps no other trace of.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub flags: BTreeSet<String>,
    /// Food found on the land: foraged, fished, snared.
    #[serde(default)]
    pub gathered: u32,
    /// Where the camp is, once made: a shelter with a fire by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camp: Option<scraped_sim::outdoors::Pos>,
    /// Feedings of an animal so far, and its kind once it follows.
    #[serde(default)]
    pub fed: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion: Option<String>,
    /// Towns shown on old maps the player has read, by town.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub map_towns: BTreeSet<usize>,
}

impl Game {
    fn is_food(&self, t: usize) -> bool {
        FOODS.contains(&self.thing(t).kind)
    }

    /// Food at the camp: carried, or lying within 100 m of it.
    pub(crate) fn camp_store(&self) -> usize {
        let Some(c) = self.state.arcs.camp else {
            return 0;
        };
        let carried = self
            .state
            .carried
            .iter()
            .filter(|&&t| self.is_food(t))
            .count();
        let here = self.state.place == Place::Outside && self.state.pos.dist(c) <= 100.0;
        let lying = self
            .state
            .dropped
            .iter()
            .filter(|(&t, p)| {
                p.dist(c) <= 100.0 && self.is_food(t) && !self.state.gone.contains(&t)
            })
            .count();
        lying + if here { carried } else { 0 }
    }

    /// A building whose works all run, or whose rooms (ten or more) have
    /// all been seen.
    pub(crate) fn restored(&self, s: usize) -> bool {
        let works = self
            .site
            .fixtures
            .works
            .iter()
            .enumerate()
            .any(|(w, x)| x.structure == s && self.works_done(w));
        let n = self.site.world.structures[s].interior.rooms.len();
        let seen = self
            .state
            .rooms_seen
            .iter()
            .filter(|(st, _)| *st == s)
            .count();
        // DESIGN-Q: or a hidden way found in it, and all of it seen.
        let hidden = self.state.revealed.iter().any(|(st, _)| *st == s);
        works || (seen >= n && (n >= GREAT_ROOMS || hidden))
    }

    /// Whether a step has been reached, from the state.
    fn step_reached(&self, arc: &str, step: &str) -> bool {
        let a = &self.state.arcs;
        let flag = |f: &str| a.flags.contains(f);
        let payoff = |kinds: &[&str]| {
            self.state.payoffs_done.iter().any(|&t| {
                self.payoffs
                    .iter()
                    .any(|p| p.thing == t && kinds.contains(&p.kind))
            })
        };
        let works = &self.site.fixtures.works;
        let parts_run = works
            .iter()
            .flat_map(|w| w.steps.iter())
            .filter(|s| self.state.sim.moved.contains(s))
            .count();
        match (arc, step) {
            ("forager", "first_food") => a.gathered >= 1,
            ("forager", "fruit") => payoff(&["fruit_tree"]),
            ("forager", "practised") => a.gathered >= PRACTISED,
            ("forager", "camp") => a.camp.is_some(),
            ("forager", "store") => self.camp_store() >= STORE,
            ("forager", "companion") => a.companion.is_some(),
            ("mason", "hidden") => payoff(&["hollow_wall", "loose_stone", "jammed_lid"]),
            ("mason", "first_works") => parts_run >= 1,
            ("mason", "works_known") => parts_run >= 3,
            ("mason", "hidden_way") => !self.state.revealed.is_empty(),
            ("mason", "works_run") => (0..works.len()).any(|w| self.works_done(w)),
            // Through the way the works open, into the room beyond.
            ("mason", "opened_by_hand") => (0..works.len()).any(|w| {
                let st = &self.site.world.structures[works[w].structure];
                self.works_done(w)
                    && st.interior.links.get(works[w].seal).is_some_and(|l| {
                        self.state.rooms_seen.contains(&(works[w].structure, l.a))
                            && self.state.rooms_seen.contains(&(works[w].structure, l.b))
                    })
            }),
            ("mason", "whole_interior") => (0..self.site.world.structures.len()).any(|s| {
                let n = self.site.world.structures[s].interior.rooms.len();
                n >= GREAT_ROOMS
                    && self
                        .state
                        .rooms_seen
                        .iter()
                        .filter(|(st, _)| *st == s)
                        .count()
                        >= n
            }),
            ("wayfarer", "landmark") => flag("landmark"),
            ("wayfarer", "recognised") => flag("recognised"),
            ("wayfarer", "far_walked") => self.state.walked.len() >= FAR,
            ("wayfarer", "summit") => flag("summit"),
            ("wayfarer", "map_walked") => {
                !a.map_towns.is_empty() && a.map_towns.is_subset(&self.state.towns_met)
            }
            ("wayfarer", "rim_seen") => {
                self.site.edge.is_some_and(|e| {
                    self.state.place == Place::Outside && self.state.pos.dist(e) <= 3000.0
                }) || flag("rim_seen")
            }
            ("scribe", "first_sound") => !self.state.heard.is_empty(),
            ("scribe", "first_read") => !self.state.read.is_empty(),
            ("scribe", "first_written") => !self.state.written.is_empty(),
            ("scribe", "released") => flag("released"),
            _ => false,
        }
    }

    /// Notes a new step reached since the last command, said once; one
    /// at a time, so two reached together are said on two replies.
    pub(crate) fn arc_news(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        if self.state.dead.is_some() {
            return out;
        }
        for &(arc, step, large) in STEPS {
            let key = format!("{arc}:{step}"); // DEBUG-TEXT: a key
            if self.state.arcs.reached.contains(&key) || !self.step_reached(arc, step) {
                continue;
            }
            self.state.arcs.reached.insert(key);
            if step == "rim_seen" {
                self.state.arcs.flags.insert("rim_seen".into());
            }
            let t = self.say(
                "arc.step",
                ctx(&[
                    ("arc", Value::from(arc)),
                    ("step", Value::from(step)),
                    ("large", Value::Bool(large)),
                ]),
            );
            if !t.trim().is_empty() {
                out.push(t);
            }
            // One step a reply; the next is said with the next reply.
            break;
        }
        out
    }

    /// A camp, once made: outdoors, a shelter within 100 m and a fire
    /// burning here.
    pub(crate) fn find_camp(&mut self) {
        if self.state.arcs.camp.is_some() || self.state.place != Place::Outside {
            return;
        }
        let pos = self.state.pos;
        let shelter = self.state.sim.shelters.iter().any(|s| s.dist(pos) <= 100.0);
        if shelter && self.fire_here_pub() {
            self.state.arcs.camp = Some(pos);
        }
    }

    /// Marks something that happened, for the arcs.
    pub(crate) fn arc_flag(&mut self, f: &str) {
        self.state.arcs.flags.insert(f.to_string());
    }

    /// `settle`: ends the run well, by staying (D15). At a camp with a
    /// store: settled. In a building made sound or seen whole: restored.
    // DESIGN-Q: one verb for both endings, chosen by where it is said.
    pub(crate) fn settle(&mut self) -> Output {
        let reason = match self.state.place {
            Place::Outside => match self.state.arcs.camp {
                Some(c) if self.state.pos.dist(c) <= 100.0 => {
                    if self.camp_store() >= STORE {
                        self.end_run("settled");
                        return self.ended();
                    }
                    "no_store"
                }
                _ => "no_camp",
            },
            Place::Room { structure, .. } => {
                if self.restored(structure) {
                    self.end_run("restored");
                    return self.ended();
                }
                "not_restored"
            }
        };
        let t = self.say("settle.not_yet", ctx(&[("reason", Value::from(reason))]));
        self.output(vec![t], None)
    }

    /// `feed <animal>`: food given to a scavenger close by; after a few
    /// times it follows (D15).
    pub(crate) fn feed_creature(&mut self, words: &[String]) -> Output {
        let near = self.state.creatures.iter().enumerate().find(|(i, c)| {
            let spawn = &self.site.fixtures.creatures[*i];
            let form = spawn
                .species
                .map_or("", |sp| self.site.world.life.species[sp].form);
            c.seen
                && c.pos.dist(self.state.pos) <= 300.0
                && words
                    .iter()
                    .any(|w| spawn.archetype.contains(w.as_str()) || form == w || w == "animal")
        });
        let Some((i, _)) = near else {
            let t = self.say("feed.no_animal", Context::new());
            return self.output(vec![t], None);
        };
        let kind = self.site.fixtures.creatures[i].archetype.to_string();
        let food = self
            .state
            .carried
            .iter()
            .copied()
            .find(|&t| self.is_food(t));
        let Some(f) = food else {
            let t = self.say("feed.no_food", ctx(&[("kind", Value::from(kind.as_str()))]));
            return self.output(vec![t], None);
        };
        self.state.carried.retain(|&t| t != f);
        self.state.gone.insert(f);
        let tame = scraped_sim::creatures::archetype(&kind).thief;
        if tame {
            self.state.arcs.fed = self.state.arcs.fed.saturating_add(1);
            if self.state.arcs.fed >= TAMED && self.state.arcs.companion.is_none() {
                self.state.arcs.companion = Some(kind.clone());
            }
        }
        self.advance(10, scraped_sim::body::Activity::Resting);
        let t = self.say(
            "feed.animal",
            ctx(&[
                ("kind", Value::from(kind.as_str())),
                ("times", Value::Number(i64::from(self.state.arcs.fed))),
                ("follows", Value::Bool(self.state.arcs.companion.is_some())),
                ("takes", Value::Bool(tame)),
            ]),
        );
        self.output(vec![t], None)
    }

    /// How many arcs the player went some way along (one summary line
    /// each).
    #[cfg(test)]
    pub(crate) fn arcs_moved(&self) -> usize {
        ARCS.iter()
            .filter(|&&arc| {
                STEPS.iter().any(|(a, s, _)| {
                    *a == arc
                        && (self.state.arcs.reached.contains(&format!("{a}:{s}"))
                            || self.step_reached(a, s))
                })
            })
            .count()
    }

    /// The end summary's line per arc that moved (D15).
    pub(crate) fn arc_summary(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        for &arc in ARCS {
            let steps: Vec<(&str, bool)> = STEPS
                .iter()
                .filter(|(a, s, _)| {
                    *a == arc
                        && (self.state.arcs.reached.contains(&format!("{a}:{s}"))
                            || self.step_reached(a, s))
                })
                .map(|&(_, s, l)| (s, l))
                .collect();
            if steps.is_empty() {
                continue;
            }
            let large = steps.iter().filter(|(_, l)| *l).count();
            let c = ctx(&[
                ("arc", Value::from(arc)),
                (
                    "steps",
                    Value::List(steps.iter().map(|(s, _)| Value::from(*s)).collect()),
                ),
                ("large", Value::Number(large as i64)),
            ]);
            out.push(self.say("end.arc", c));
        }
        out
    }
}

fn e(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|v| v.to_string()).collect(),
    }
}

fn step_ids() -> Vec<&'static str> {
    STEPS.iter().map(|s| s.1).collect()
}

fn s_step(_: u64) -> Vec<Context> {
    STEPS
        .iter()
        .map(|&(a, s, l)| {
            ctx(&[
                ("arc", Value::from(a)),
                ("step", Value::from(s)),
                ("large", Value::Bool(l)),
            ])
        })
        .collect()
}

fn s_ending(_: u64) -> Vec<Context> {
    vec![ctx(&[
        ("day", Value::Number(40)),
        ("indoors", Value::Bool(false)),
        ("years", Value::Number(26)),
    ])]
}

/// D15's slots: steps along each way of playing, staying, feeding an
/// animal, and the endings and summary lines they add.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("arc.step", "A step along one way of playing (D15), said once, after whatever the command said: the first food found on the land, a camp made, a building's works all running, a summit reached, the first sound heard. A quiet sense of having got somewhere, never a score or a badge; a large step changes what the player can do (a camp to come back to, a way opened by hand) and may be warmer. Never name the arc or the style.")
            .var("arc", e(ARCS), "Whose way it is: forager, mason, wayfarer or scribe.")
            .var("step", e(&step_ids()), "Which step: forager first_food, fruit, practised (ten finds), camp (a shelter with a fire by it), store (food laid up at the camp), companion (an animal that follows); mason hidden (a hidden thing found by touch or knock), first_works (a part of some works set moving), works_known (three parts), hidden_way (a hidden way found by looking closely), works_run (a building's works all running), opened_by_hand (through the way the works opened), whole_interior (every room of a great building seen); wayfarer landmark (a landmark reached), recognised (a place known again), far_walked (much ground walked), summit (a hill or mountain top reached), map_walked (every town an old map shows, visited), rim_seen (the world's rim in sight); scribe first_sound, first_read, first_written, released (something scraped).")
            .var("large", VarType::Bool, "A large step: it changes what the player can do, or opens a way to end the run well.")
            .max_len(220)
            .sampler(s_step),
        SlotDef::new("settle.not_yet", "The player tries to settle (stay for good) but it isn't the place for it yet (D15). Say what is missing in the world's terms, never as a rule: no camp here (a shelter with a fire by it), not enough food laid up at the camp, or this building isn't sound or known enough to stay in.")
            .var("reason", e(&["no_camp", "no_store", "not_restored"]), "What is missing.")
            .max_len(200)
            .sampler(|_| vec![ctx(&[("reason", Value::from("no_camp"))]), ctx(&[("reason", Value::from("no_store"))]), ctx(&[("reason", Value::from("not_restored"))])]),
        SlotDef::new("feed.animal", "The player holds out food to an animal close by (D15). A scavenger takes it, warier at first; after a few times it follows. Other kinds take no food from a hand.")
            .var("kind", e(&["scavenger", "grazer", "predator", "deep"]), "What kind of animal it is.")
            .var("times", VarType::Number, "How many times the player has fed a scavenger so far.")
            .var("follows", VarType::Bool, "It follows the player from now on.")
            .var("takes", VarType::Bool, "Whether it took the food.")
            .max_len(200)
            .sampler(|_| vec![
                ctx(&[("kind", Value::from("scavenger")), ("times", Value::Number(1)), ("follows", Value::Bool(false)), ("takes", Value::Bool(true))]),
                ctx(&[("kind", Value::from("scavenger")), ("times", Value::Number(3)), ("follows", Value::Bool(true)), ("takes", Value::Bool(true))]),
                ctx(&[("kind", Value::from("grazer")), ("times", Value::Number(0)), ("follows", Value::Bool(false)), ("takes", Value::Bool(false))]),
            ]),
        SlotDef::new("companion.warns", "The animal that follows the player (D15) warns of a hunter coming into view, a moment before the player sees it: hackles, a growl, it presses close.")
            .var("kind", e(&["scavenger", "grazer", "predator", "deep"]), "What kind of animal follows.")
            .var("threat", e(&["scavenger", "grazer", "predator", "deep"]), "What is coming.")
            .max_len(160)
            .sampler(|_| vec![ctx(&[("kind", Value::from("scavenger")), ("threat", Value::from("predator"))])]),
        SlotDef::new("feed.no_animal", "The player tries to feed an animal, but none is close enough (D15).")
            .sampler(|_| vec![Context::new()]),
        SlotDef::new("feed.no_food", "The player would feed an animal but carries no food (D15).")
            .var("kind", e(&["scavenger", "grazer", "predator", "deep"]), "What kind of animal it is.")
            .sampler(|_| vec![ctx(&[("kind", Value::from("scavenger"))])]),
        SlotDef::new("end.settled", "D15: the player stays at the camp they made, with food laid up: the run ends by choice, living off the land. The world stays as it is now.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.restored", "D15: the player stays in a building they made sound (its works running) or came to know whole (every room, and its hidden way, or every room of a great one): the run ends by choice. The world stays as it is now.")
            .var("day", VarType::Number, "Day of the run.")
            .var("indoors", VarType::Bool, "Whether they were indoors.")
            .var("years", VarType::Number, "The player's age.")
            .max_len(600)
            .sampler(s_ending),
        SlotDef::new("end.arc", "The end summary (D15): one line for each way of playing the player went some way along, after the opening counts: what they mastered and achieved there, from the steps reached. Never name the style.")
            .var("arc", e(ARCS), "Which way: forager, mason, wayfarer or scribe.")
            .var("steps", VarType::List, "The steps reached, in order (ids as in arc.step).")
            .var("large", VarType::Number, "How many of them were large.")
            .max_len(400)
            .sampler(|_| vec![
                ctx(&[("arc", Value::from("forager")), ("steps", Value::List(vec![Value::from("first_food"), Value::from("camp")])), ("large", Value::Number(1))]),
                ctx(&[("arc", Value::from("wayfarer")), ("steps", Value::List(vec![Value::from("landmark")])), ("large", Value::Number(0))]),
            ]),
    ]
}
