//! Suggested verbs (D13): situations suggest what a person might try. A
//! verb is suggested after a response that says something it fits (by the
//! slots rendered), at most one a response, a couple of times for one
//! situation, never once the player has used it. Keenness brings a
//! category's suggestions sooner. Scraping and writing are suggested only
//! once the player has done them.

use serde::{Deserialize, Serialize};

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::site::{ctx, Place};
use crate::Game;

/// One verb's suggestion: the words to type, its category, and the slots
/// whose saying suggests it.
pub struct Suggestion {
    pub verb: &'static str,
    pub words: &'static str,
    pub category: &'static str,
    pub after: &'static [&'static str],
}

/// Every verb in the parser, with what suggests it.
// DESIGN-Q: the situations that suggest each verb.
pub const TABLE: &[Suggestion] = &[
    s(
        "take_stock",
        "take stock",
        "things",
        &["look.done", "room.items", "place.group"],
    ),
    s(
        "look",
        "look",
        "buildings",
        &["travel.arrive", "move.report", "say.intro"],
    ),
    s(
        "look_around",
        "look around",
        "land",
        &["travel.arrive", "land.landmark"],
    ),
    s(
        "look_closer",
        "look closer",
        "buildings",
        &["room.thing", "place.thing", "thing.examine"],
    ),
    s(
        "look_up",
        "look up",
        "land",
        &["sky.", "sense.sky", "weather.", "place.dark"],
    ),
    s(
        "look_down",
        "look down",
        "land",
        &["room.ways", "hazard.fall", "land.ground"],
    ),
    s(
        "listen",
        "listen",
        "sounds",
        &[
            "room.whole",
            "place.whole",
            "creature.",
            "life.sign",
            "spell.cue",
        ],
    ),
    s(
        "smell",
        "smell",
        "smells",
        &["fire.near", "air.", "life.plant", "spell.cue"],
    ),
    s(
        "touch",
        "touch",
        "buildings",
        &["room.mark", "spell.cue", "air.uncanny", "way.examine"],
    ),
    s(
        "taste",
        "taste",
        "smells",
        &["forage.found", "fill.done", "drink.none"],
    ),
    s(
        "count",
        "count",
        "things",
        &["place.group", "room.group", "place.count"],
    ),
    s(
        "examine",
        "examine",
        "things",
        &["room.thing", "room.items", "place.thing", "place.standout"],
    ),
    s(
        "go",
        "go",
        "buildings",
        &["place.building", "place.standout", "land.landmark"],
    ),
    s("out", "out", "buildings", &["room.ways"]),
    s(
        "take",
        "take",
        "things",
        &["room.items", "cache.found", "room.found"],
    ),
    s(
        "drop",
        "drop",
        "things",
        &["say.inventory", "item.too_heavy"],
    ),
    s(
        "read",
        "read",
        "marks",
        &["room.mark", "thing.examine", "read.glyph"],
    ),
    s("more", "read on", "marks", &["read.more"]),
    s("trace", "trace", "marks", &["read.glyph", "read.frame"]),
    s(
        "inventory",
        "inventory",
        "things",
        &["say.take", "gather.got", "forage.found"],
    ),
    s(
        "wait",
        "wait",
        "land",
        &["weather.coming", "sky.weather", "place.dark"],
    ),
    s("help", "help", "things", &["say.unknown_verb"]),
    s(
        "head",
        "head",
        "land",
        &["land.landmark", "land.region", "map.read"],
    ),
    s("follow", "follow", "land", &["land.edge", "life.sign"]),
    s(
        "back",
        "go back",
        "land",
        &["travel.arrive", "travel.interrupt"],
    ),
    s(
        "name",
        "name this place",
        "land",
        &["land.feature", "land.landmark", "travel.arrive"],
    ),
    s("north", "north", "buildings", &["room.ways"]),
    s("south", "south", "buildings", &["room.ways"]),
    s("east", "east", "buildings", &["room.ways"]),
    s("west", "west", "buildings", &["room.ways"]),
    s("northeast", "northeast", "land", &["travel.blocked"]),
    s("northwest", "northwest", "land", &["travel.blocked"]),
    s("southeast", "southeast", "land", &["travel.blocked"]),
    s("southwest", "southwest", "land", &["travel.blocked"]),
    s("up", "up", "buildings", &["room.ways"]),
    s("down", "down", "buildings", &["room.ways"]),
    s(
        "open",
        "open",
        "buildings",
        &["room.ways", "object.locked", "way.examine"],
    ),
    s("close", "close", "buildings", &["say.door_open"]),
    s(
        "light",
        "light",
        "things",
        &["place.dark", "say.too_dark", "read.dark", "trace.dark"],
    ),
    s("extinguish", "put out", "things", &["item.lit", "fire.lit"]),
    s(
        "drink",
        "drink",
        "life",
        &["land.edge", "fill.done", "body.felt"],
    ),
    s("fill", "fill", "things", &["land.edge", "drink.done"]),
    s(
        "eat",
        "eat",
        "life",
        &["forage.found", "fish.caught", "snare.caught", "body.felt"],
    ),
    s("sleep", "sleep", "life", &["body.felt", "place.dark"]),
    s(
        "forage",
        "forage",
        "life",
        &["life.plant", "land.ground", "body.felt"],
    ),
    s("fish", "fish", "life", &["land.edge", "life.seen"]),
    s(
        "snare",
        "set a snare",
        "life",
        &["life.sign", "life.home", "creature.seen"],
    ),
    s(
        "gather",
        "gather wood",
        "life",
        &["life.plant", "fire.out", "trace.no_tool"],
    ),
    s(
        "make",
        "make fire",
        "things",
        &["air.felt", "fire.out", "gather.found"],
    ),
    s(
        "feed",
        "feed the fire",
        "things",
        &["fire.near", "fire.lit"],
    ),
    s("operate", "work", "things", &["mech.examine", "mech.idle"]),
    s("wear", "wear", "things", &["say.take", "air.felt"]),
    s("remove", "take off", "things", &["item.wear"]),
    s(
        "use",
        "use",
        "things",
        &["object.locked", "item.cannot_use"],
    ),
    s(
        "pry",
        "pry",
        "buildings",
        &["hazard.barred", "say.door_stuck"],
    ),
    s("shout", "shout", "sounds", &["room.whole", "place.whole"]),
    s("cross", "cross", "land", &["travel.blocked", "land.edge"]),
    s("status", "how am I", "life", &["body.felt", "body.change"]),
    s("scrape", "scrape", "marks", &["scrape.done", "write.done"]),
    s("clean", "clean", "marks", &["read.grime", "thing.examine"]),
    s("beyond", "go beyond", "land", &["land.rim"]),
    s("write", "write", "marks", &["write.done", "scrape.done"]),
    s("manual", "manual", "things", &["say.help"]),
    s("mark", "mark", "marks", &["travel.lost", "travel.arrive"]),
    s(
        "dig",
        "dig",
        "buildings",
        &["say.blocked", "say.buried", "hazard.collapse"],
    ),
];

const fn s(
    verb: &'static str,
    words: &'static str,
    category: &'static str,
    after: &'static [&'static str],
) -> Suggestion {
    Suggestion {
        verb,
        words,
        category,
        after,
    }
}

/// Times a verb is suggested for one situation before it is dropped.
// DESIGN-Q: twice for a situation, then dropped until it changes.
const TIMES: u8 = 2;

/// Responses a situation must hold before a verb is suggested, for a
/// category at plain keenness; a keen one is suggested at once.
// DESIGN-Q: three responses at plain keenness, one when keen (1.4+).
const PATIENCE: u8 = 3;

/// What has been suggested, by verb.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suggested {
    /// The situation it was last suggested for (the place), how often
    /// for that, and how many responses it has held.
    pub situation: String,
    pub shown: u8,
    pub held: u8,
}

impl Game {
    /// Whether a verb could be suggested at all now: never one already
    /// used; scraping and writing only once done; reading only once marks
    /// are known for what they are; nothing a body can't do then.
    fn may_suggest(&self, v: &Suggestion) -> bool {
        if self.state.used.contains(v.verb) {
            return false;
        }
        match v.verb {
            "scrape" | "write" => self.state.used.contains(v.verb),
            "read" | "more" => !self.state.heard.is_empty(),
            "drink" => self.state.body.thirst > 240,
            "eat" => self.state.body.hunger > 360,
            "sleep" => self.state.body.fatigue > 960,
            "out" => !matches!(self.state.place, Place::Outside),
            "trace" => self.tracing_tool().is_some(),
            _ => true,
        }
    }

    fn needs_object(&self, verb: &str) -> bool {
        crate::parser::verbs()
            .iter()
            .any(|v| v.id == verb && v.object == crate::parser::ObjectRule::Required)
    }

    /// What a suggestion for a verb that needs a thing would name (S07):
    /// a marked thing for the marks verbs, something portable to take,
    /// else the first thing here.
    fn suggest_object(&self, verb: &str) -> Option<String> {
        let here = self.here();
        let pick = match verb {
            "read" | "trace" | "clean" | "more" => {
                here.iter().find(|&&t| !self.thing(t).texts.is_empty())
            }
            "take" => here.iter().find(|&&t| self.thing(t).portable),
            _ => here.first(),
        };
        pick.map(|&t| self.thing(t).kind.to_string())
    }

    /// The suggestion, if any, for a response that rendered these slots.
    pub(crate) fn suggest(&mut self, said: &[String]) -> Option<String> {
        let place = format!("{:?}", self.state.place); // DEBUG-TEXT: a key, never shown
        let mut best: Option<(f64, &Suggestion)> = None;
        for v in TABLE {
            let fits = said.iter().any(|s| {
                v.after
                    .iter()
                    .any(|a| s == a || (a.ends_with('.') && s.starts_with(a)))
            });
            if !fits || !self.may_suggest(v) {
                continue;
            }
            // A verb that needs a thing names one here, or isn't offered.
            if self.needs_object(v.verb) && self.suggest_object(v.verb).is_none() {
                continue;
            }
            let keen = self.keenness(v.category);
            let rec = self.state.suggested.entry(v.verb.to_string()).or_default();
            if rec.situation != place {
                rec.situation = place.clone();
                rec.shown = 0;
                rec.held = 0;
            }
            rec.held = rec.held.saturating_add(1);
            let wait = if keen >= 1.4 { 1 } else { PATIENCE };
            if rec.shown >= TIMES || rec.held < wait {
                continue;
            }
            if best.is_none_or(|(k, _)| keen > k) {
                best = Some((keen, v));
            }
        }
        let (_, v) = best?;
        let thing = if self.needs_object(v.verb) {
            self.suggest_object(v.verb).unwrap_or_default()
        } else {
            String::new()
        };
        let t = self.say(
            "suggest.verb",
            ctx(&[
                ("verb", Value::from(v.verb)),
                ("words", Value::from(v.words)),
                ("thing", Value::from(thing)),
            ]),
        );
        if t.is_empty() {
            return None;
        }
        if let Some(rec) = self.state.suggested.get_mut(v.verb) {
            rec.shown += 1;
        }
        Some(t)
    }
}

fn s_suggest(_: u64) -> Vec<Context> {
    ["listen", "look_up", "snare", "trace"]
        .iter()
        .map(|v| {
            let w = TABLE.iter().find(|s| s.verb == *v).map_or(*v, |s| s.words);
            ctx(&[
                ("verb", Value::from(*v)),
                ("words", Value::from(w)),
                (
                    "thing",
                    Value::from(if *v == "trace" { "stele" } else { "" }),
                ),
            ])
        })
        .collect()
}

/// D13's suggestion slot.
pub fn slots() -> Vec<SlotDef> {
    vec![SlotDef::new("suggest.verb", "A thing the player might try here, suggested by what was just said (D13): the character's own passing thought, never an instruction or a hint about what matters. At most one a response, shown a couple of times and then dropped. Never mention writing, reading, signs or glyphs unless the verb is one of those and the player already does it.")
        .var("verb", VarType::Enum { values: TABLE.iter().map(|s| s.verb.to_string()).collect() }, "The verb, as the parser knows it (look_up, snare, trace…), for variants per verb.")
        .var("words", VarType::Text, "What the player would type, in plain words ('look up', 'set a snare').")
        .var("thing", VarType::Text, "For a verb that needs one, the thing here it would be done to ('stele'); empty otherwise.")
        .max_len(160)
        .sampler(s_suggest)]
}
