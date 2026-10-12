//! Ways of noticing (D13): what the character notices unasked depends on
//! a play style chosen at the start and on habit. Each category of fact
//! has a keenness, never shown as a number, that weighs what arrives
//! unasked; deliberate attention raises it and neglect lets it fade.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use scraped_content::{Context, SlotDef, Value, VarType};

use crate::site::ctx;
use crate::Game;

/// The categories a character can be keen on.
pub const CATEGORIES: [&str; 7] = [
    "marks",
    "sounds",
    "smells",
    "life",
    "buildings",
    "land",
    "things",
];

/// The play styles, each keen on one category from the start.
pub const STYLES: [(&str, &str); 4] = [
    ("scribe", "marks"),
    ("forager", "life"),
    ("mason", "buildings"),
    ("wayfarer", "land"),
];

/// The verbs each style knows from the start, listed in `help` beside the
/// basics. Scraping and writing are in no set.
// DESIGN-Q: starting verbs per style, from the D13 proposal.
pub const STYLE_VERBS: [(&str, &[&str]); 4] = [
    ("scribe", &["read", "trace", "look_closer", "clean"]),
    ("forager", &["forage", "fish", "snare", "gather", "follow"]),
    ("mason", &["touch", "look_closer", "operate", "dig"]),
    ("wayfarer", &["look_up", "head", "follow", "name"]),
];

// DESIGN-Q: keenness. Everyone starts at 1; a style's own category at
// 1.5, and it never falls below 1.15 (a small edge kept). A dozen
// deliberate attentions in about three days bring a category to 1.8; a
// week of neglect fades it halfway to its floor (0.7), two weeks three
// quarters.
const NEUTRAL: f64 = 1.0;
const STYLE_START: f64 = 1.5;
const STYLE_EDGE: f64 = 1.15;
const FLOOR: f64 = 0.7;
const CAP: f64 = 1.8;
const DOZEN: f64 = 12.0;
const DAY: u32 = 1440;
/// Attentions remembered per category.
const KEPT: usize = 24;

/// A player's half of noticing: their style, and when they last gave each
/// category deliberate attention (game minutes). Kept with the player, so
/// two players in one world keep their own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Noticing {
    /// The style chosen at the start ("" for none; `None` before the
    /// choice is made).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attended: BTreeMap<String, Vec<u32>>,
    /// Categories the player has been told have sharpened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sharpened: Vec<String>,
}

impl Noticing {
    fn own(&self) -> Option<&'static str> {
        let s = self.style.as_deref()?;
        STYLES.iter().find(|(n, _)| *n == s).map(|(_, c)| *c)
    }

    /// How keen the character is on a category now.
    pub fn keenness(&self, category: &str, now: u32) -> f64 {
        let own = self.own() == Some(category);
        let (base, floor) = if own {
            (STYLE_START, STYLE_EDGE)
        } else {
            (NEUTRAL, FLOOR)
        };
        let times = self.attended.get(category).map_or(&[][..], |v| &v[..]);
        // Recent attention counts in full, older less, old not at all.
        let habit: f64 = times
            .iter()
            .map(|&t| match now.saturating_sub(t) / DAY {
                0..=3 => 1.0,
                4..=10 => 0.5,
                11..=17 => 0.25,
                _ => 0.0,
            })
            .sum();
        let keen = base + (CAP - base).max(0.0) * habit.min(DOZEN) / DOZEN;
        let last = times.last().copied().unwrap_or(0);
        let idle = now.saturating_sub(last) / DAY;
        let faded = match idle {
            0..=6 => 1.0,
            7..=13 => 0.5,
            _ => 0.25,
        };
        floor + (keen - floor) * faded
    }

    /// Deliberate attention to a category. True the first time it brings
    /// the category to full keenness.
    pub fn attend(&mut self, category: &str, now: u32) -> bool {
        let v = self.attended.entry(category.to_string()).or_default();
        v.push(now);
        if v.len() > KEPT {
            v.remove(0);
        }
        let sharp = self.keenness(category, now) >= CAP - 0.05;
        if sharp && !self.sharpened.iter().any(|c| c == category) {
            self.sharpened.push(category.to_string());
            return true;
        }
        false
    }
}

/// The category a fact belongs to, by its slot; `None` for what anyone
/// notices (danger, the air, a cue).
pub fn category_of_slot(slot: &str) -> Option<&'static str> {
    Some(match slot {
        "room.mark" | "cache.sign" | "great.site" => "marks",
        "sense.sound" => "sounds",
        "sense.smell" => "smells",
        s if s.starts_with("life.") || s == "creature.seen" => "life",
        s if s.starts_with("place.") && s != "place.thing" => "buildings",
        "room.whole" | "room.ways" | "room.window" | "room.group" | "danger.unstable" => {
            "buildings"
        }
        s if s.starts_with("land.") || s.starts_with("sky.") || s.starts_with("weather.") => "land",
        "sense.sky" | "sense.ground" | "ground.wet" | "evidence.region" | "evidence.season" => {
            "land"
        }
        "room.items" | "room.thing" | "place.thing" | "cache.found" | "room.found" => "things",
        _ => return None,
    })
}

/// The category of a fact, by its slot and what it says: a thing bearing
/// marks is noticed by an eye for marks.
pub fn category_of(slot: &str, vars: &Context) -> Option<&'static str> {
    if matches!(slot, "room.thing" | "place.thing")
        && matches!(vars.get("marked"), Some(Value::Bool(true)))
    {
        return Some("marks");
    }
    // A small payoff's cue (S07): in its own category.
    if slot == "payoff.cue" {
        let c = vars.get("category").map(Value::text).unwrap_or_default();
        return CATEGORIES.iter().find(|x| **x == c).copied();
    }
    category_of_slot(slot)
}

/// The category a command gives deliberate attention to, if any.
// DESIGN-Q: which verbs count for which category.
pub fn category_of_verb(verb: &str) -> Option<&'static str> {
    Some(match verb {
        "read" | "more" | "trace" | "clean" | "mark" | "count" => "marks",
        "listen" | "shout" => "sounds",
        "smell" | "taste" => "smells",
        "forage" | "fish" | "snare" | "feed" | "gather" => "life",
        "touch" | "look_closer" | "dig" | "pry" => "buildings",
        "look_up" | "look_around" | "head" | "follow" | "name" | "cross" | "beyond" => "land",
        "examine" | "take" | "open" | "close" | "operate" | "use" => "things",
        _ => return None,
    })
}

/// The two categories each kind of old spell shows itself in (D13), so
/// every style has its own way to the slow realisation.
// DESIGN-Q: which categories each quality is noticed in.
pub fn evidence_of(quality: &str) -> [&'static str; 2] {
    match quality {
        "heat" => ["buildings", "land"],
        "light" => ["land", "things"],
        "wetness" => ["land", "smells"],
        "flow" => ["land", "sounds"],
        "sound" => ["sounds", "life"],
        "growth" => ["life", "smells"],
        "lure" | "calm" => ["life", "sounds"],
        "weight" | "binding" => ["things", "buildings"],
        "visibility" | "rising" => ["things", "land"],
        "keeping" => ["things", "smells"],
        _ => ["buildings", "sounds"],
    }
}

impl Game {
    /// Quiet signs of the spells acting here (D13), one per category each
    /// kind of spell shows in; a keen player notices them first.
    pub(crate) fn spell_signs(&self, out: &mut Vec<crate::attention::Fact>) {
        use scraped_sim::writing::Class;
        let env = self.env();
        let (at, inside, place_key) = match self.state.place {
            crate::site::Place::Room { structure, .. } => (
                self.site.land.structure_pos[structure],
                true,
                format!("s{structure}"), // DEBUG-TEXT: a key
            ),
            crate::site::Place::Outside => {
                let (x, y) = self.state.pos.cell();
                (self.state.pos, false, format!("c{x}:{y}")) // DEBUG-TEXT: a key
            }
        };
        let mut seen: Vec<(Class, &str)> = Vec::new();
        // What can be perceived here (S08: "the statue sounds wrong" in a
        // room with no statue).
        let kinds: Vec<&str> = self
            .here()
            .into_iter()
            .map(|t| self.thing(t).kind)
            .collect();
        let ways = inside && !self.ways().is_empty();
        for c in &self.claims {
            if c.class == Class::Person
                || c.class.indoors() != inside
                || c.pos.dist(at) > c.range
                || !c.acts(&env.when)
            {
                continue;
            }
            // A kind of thing (a statue, a jar) is named only if one is here.
            let thing_kind = crate::slots::kinds().contains(&c.subject.as_str());
            let present = match c.class {
                Class::Passage => ways,
                _ if thing_kind || c.class == Class::Thing => kinds.contains(&c.subject.as_str()),
                _ => true,
            };
            if !present {
                continue;
            }
            let q = c.property.name();
            if seen.contains(&(c.class, q)) {
                continue;
            }
            seen.push((c.class, q));
            let strength = c.amount.abs().min(3);
            for cat in evidence_of(q) {
                out.push(crate::attention::Fact::new(
                    "spell.sign",
                    format!("sign:{place_key}:{cat}:{}:{q}", c.class.name()), // DEBUG-TEXT: a key
                    14.0 + 4.0 * f64::from(strength),
                    ctx(&[
                        ("category", Value::from(cat)),
                        ("quality", Value::from(q)),
                        ("rising", Value::Bool(c.amount > 0)),
                        ("thing", Value::from(c.subject.replace('_', " "))),
                        ("indoors", Value::Bool(inside)),
                    ]),
                ));
            }
        }
    }

    /// How keen the player is on a category now.
    pub(crate) fn keenness(&self, category: &str) -> f64 {
        self.noticing().keenness(category, self.state.minutes)
    }

    /// The noticing of whoever is playing now.
    pub fn noticing(&self) -> Noticing {
        self.state
            .noticing
            .get(&self.who)
            .cloned()
            .unwrap_or_default()
    }

    fn noticing_mut(&mut self) -> &mut Noticing {
        self.state.noticing.entry(self.who.clone()).or_default()
    }

    /// A fact's weight for this player: its category's keenness. A sign
    /// of a spell is noticed only by someone keen on its category.
    pub(crate) fn keen_weight(&self, f: &crate::attention::Fact) -> f64 {
        if f.slot == "spell.sign" {
            let cat = f.vars.get("category").map(Value::text).unwrap_or_default();
            let k = self.keenness(&cat);
            return if k > 1.05 { k } else { 0.0 };
        }
        category_of(f.slot, &f.vars).map_or(1.0, |c| self.keenness(c))
    }

    /// Records a followed command as attention; says so, once, when a
    /// category first sharpens (if Jb has written it).
    pub(crate) fn habit(&mut self, verb: &str) -> Option<String> {
        let cat = category_of_verb(verb)?;
        let now = self.state.minutes;
        if self.noticing_mut().attend(cat, now) {
            let t = self.say("notice.sharpened", ctx(&[("category", Value::from(cat))]));
            return (!t.is_empty()).then_some(t);
        }
        None
    }

    /// The style offered at the start, and the first command's answer:
    /// one of the styles, `none`, or anything else (no style, and the
    /// command is played as usual).
    pub(crate) fn choose_style(&mut self, input: &str) -> Option<crate::Output> {
        if self.noticing().style.is_some() {
            return None;
        }
        let word = input.trim().to_lowercase();
        let chosen = STYLES
            .iter()
            .map(|(s, _)| *s)
            .chain(["none"])
            .find(|s| *s == word);
        let Some(style) = chosen else {
            self.noticing_mut().style = Some(String::new());
            return None;
        };
        let style = if style == "none" { "" } else { style };
        self.noticing_mut().style = Some(style.to_string());
        if let Some((_, verbs)) = STYLE_VERBS.iter().find(|(s, _)| *s == style) {
            self.state.used.extend(verbs.iter().map(|v| v.to_string()));
        }
        let t = self.say("style.chosen", ctx(&[("style", Value::from(style))]));
        let look = self.look();
        Some(self.output(vec![t, look], None))
    }
}

fn styles_enum() -> VarType {
    VarType::Enum {
        values: STYLES
            .iter()
            .map(|(s, _)| s.to_string())
            .chain([String::new()])
            .collect(),
    }
}

fn s_style(_: u64) -> Vec<Context> {
    STYLES
        .iter()
        .map(|(s, _)| *s)
        .chain([""])
        .map(|s| ctx(&[("style", Value::from(s))]))
        .collect()
}

fn s_sharpened(_: u64) -> Vec<Context> {
    CATEGORIES
        .iter()
        .map(|c| ctx(&[("category", Value::from(*c))]))
        .collect()
}

fn s_sign(_: u64) -> Vec<Context> {
    [
        ("sounds", "flow", true, "water"),
        ("life", "lure", false, "deer"),
        ("buildings", "heat", false, "house"),
        ("smells", "growth", true, "barley"),
    ]
    .iter()
    .map(|(c, q, r, t)| {
        ctx(&[
            ("category", Value::from(*c)),
            ("quality", Value::from(*q)),
            ("rising", Value::Bool(*r)),
            ("thing", Value::from(*t)),
            ("indoors", Value::Bool(false)),
        ])
    })
    .collect()
}

fn s_clock(_: u64) -> Vec<Context> {
    [(3, 10, true), (12, 12, true), (1, 8, false)]
        .iter()
        .map(|(p, o, s)| {
            ctx(&[
                ("part", Value::Number(*p)),
                ("of", Value::Number(*o)),
                ("shown", Value::Bool(*s)),
            ])
        })
        .collect()
}

fn s_offer(_: u64) -> Vec<Context> {
    vec![Context::new()]
}

/// D13's slots.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new("style.offer", "Said once, after the opening, before the first command (D13): the player may say what kind of person they were, which shapes what they notice. The four answers are typed as the words scribe, forager, mason and wayfarer (or none); anything else just starts playing. Never mention writing, reading, signs or glyphs, nor what the game is about. May say nothing.")
            .min_variants(0)
            .max_len(260)
            .sampler(s_offer),
        SlotDef::new("style.chosen", "The player has said what kind of person they were (D13). A line in the character's voice about what such a person tends to notice; '' when they chose none. Never mention writing, reading, signs or glyphs.")
            .var("style", styles_enum(), "scribe (marks and their shapes), forager (living things), mason (buildings, materials, wear), wayfarer (land, routes, sky), or '' for none.")
            .max_len(200)
            .sampler(s_style),
        SlotDef::new("notice.sharpened", "The first time a kind of noticing sharpens through habit (D13): the character realises they have begun to notice more of it. Optional; may stay empty. Never a number.")
            .var("category", VarType::Enum { values: CATEGORIES.iter().map(|c| c.to_string()).collect() }, "marks, sounds, smells, life (living things), buildings (materials and wear), land (routes and sky) or things (objects and mechanisms).")
            .min_variants(0)
            .max_len(160)
            .sampler(s_sharpened),
        SlotDef::new("clock.sundial", "A sundial looked at (D13): a stone face marked off in the world's own parts of the daylight, and where the shadow falls among them. Say which line it falls on, counted from the morning side, never an hour. With no sun (night, cloud, fog), there is no shadow.")
            .var("part", VarType::Number, "The line the shadow falls on, from 1 (morning side).")
            .var("of", VarType::Number, "How many lines the face has.")
            .var("shown", VarType::Bool, "Whether there is a shadow at all.")
            .max_len(200)
            .sampler(s_clock),
        SlotDef::new("clock.water", "A water clock looked at (D13): a vessel that drips through the day and night, its inside marked off in the world's own parts of the day, and where the water stands among them. Say which mark, never an hour.")
            .var("part", VarType::Number, "The mark the water stands at, from 1.")
            .var("of", VarType::Number, "How many marks there are.")
            .var("shown", VarType::Bool, "Always true.")
            .max_len(200)
            .sampler(s_clock),
        SlotDef::new("spell.sign", "A quiet sign, in one kind of noticing, that something here is not as it should be (D13): an old spell pushing one quality of a kind of thing, noticed only by someone keen on that kind of thing. In that category's terms: a sound (sounds), a smell (smells), how plants or beasts behave (life), the stone, joints and wear (buildings), the ground, water or sky (land), an object (things). The strangeness, never its cause: never mention writing, spells, words or magic, and never say what was asked for.")
            .var("category", VarType::Enum { values: CATEGORIES.iter().map(|c| c.to_string()).collect() }, "The kind of noticing it shows in.")
            .var("quality", VarType::Text, "What the spell pushes: heat, light, wetness, flow, sound, growth, lure, calm, weight, visibility, binding, keeping, rising, openness, stability.")
            .var("rising", VarType::Bool, "More of it (true) or less.")
            .var("thing", VarType::Text, "The kind of thing it acts on, in plain English ('door', 'deer', 'water').")
            .var("indoors", VarType::Bool, "Indoors or out.")
            .max_len(200)
            .sampler(s_sign),
    ]
}
