//! How a run ends, and what it meant: end conditions beyond death, the run
//! record, the zoomed-out summary, the chronicle in the language, legacy
//! and the notebook.
//!
//! The summary separates what the player did from what the world would
//! have done anyway by running the regions a second time, from the same
//! start, with only history's great inscriptions pushing them.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use scraped_content::{Context, Value};
use scraped_lang::meaning::{
    Argument, Clause, Mood, NounPhrase, Number, Polarity, Role, Sentence, Tense,
};
use scraped_sim::outdoors::{bearing, BEARINGS};
use scraped_sim::region::{band, CLIMATE, LIFE, STABILITY, VARIABLES, WATER};
use scraped_sim::writing::claim_parts;

use crate::site::{ctx, Place};
use crate::{Death, Game, Output};

/// Ways a run ends. "death" covers every cause in `body::DEATHS`.
pub const ENDINGS: &[&str] = &["death", "left", "written_in", "old_age", "overtaken"];

/// The age at which a life ends of itself, in years.
// DESIGN-Q: eighty.
pub const OLD_AGE: u32 = 80;

/// Below this (thousandths), a region's ground and life have given way: a
/// player standing in it is overtaken by collapse.
// DESIGN-Q: stability and life both under 0.1.
pub const COLLAPSE: i32 = 100;

/// Who did something the record lists.
pub const WHO: &[&str] = &["you", "history"];

/// Why a region's aspect is where it is at the end, by comparing the run
/// with the same world left alone: "world" (it would have gone so anyway),
/// "you" (your doing alone), "both".
pub const CAUSES: &[&str] = &["world", "you", "both"];

/// A claim released during the run, or before it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    pub text: usize,
    /// Who scraped it: "you", or "history" (cast before play began).
    pub scraped_by: String,
    /// Who wrote it: "you", or "history".
    pub written_by: String,
    /// The scraper's power (0 for history's casts).
    pub power: u8,
    /// The claim, as a debug id ("not open door"); spoiler.
    pub claim: Option<String>,
}

/// Something the player did that changed what pushes the regions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    pub day: u32,
    pub text: usize,
    /// "released" (a push begins) or "silenced" (a push ends).
    pub act: String,
    /// "region" or "great".
    pub scale: String,
    pub region: usize,
    pub variable: usize,
    /// Which way the push leans: toward more (true) or less.
    pub rising: bool,
}

/// One aspect of one region at the end of the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub aspect: String,
    pub before: String,
    pub after: String,
    /// The band it would be in had the player done nothing.
    pub without: String,
    pub cause: String,
}

/// A region's start and end, and the same world left alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionOutcome {
    pub region: usize,
    pub biome: String,
    /// Which way it lies from where play began ("here" if there).
    pub bearing: String,
    pub start: [i32; 4],
    pub end: [i32; 4],
    pub without: [i32; 4],
    /// Aspects whose band moved, or would have moved without the player.
    pub changes: Vec<Change>,
}

/// A complete account of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub seed: u64,
    /// One of `ENDINGS`; empty while the run goes on.
    pub ending: String,
    /// The cause of death, or the ending again.
    pub cause: String,
    pub days: u32,
    pub age: u32,
    /// Buildings entered.
    pub places: Vec<usize>,
    /// Places the player named.
    pub named: Vec<String>,
    pub texts_read: Vec<usize>,
    pub written: usize,
    pub releases: Vec<Release>,
    pub acts: Vec<Act>,
    pub regions: Vec<RegionOutcome>,
    /// The player's last parsed text, if any: what legacy carries.
    pub final_inscription: Option<usize>,
}

/// What a run leaves to the next world: its final inscription's meaning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Legacy {
    pub from_seed: u64,
    pub ending: String,
    pub meaning: Sentence,
}

/// The player's records at the end: transcript, named places and the run
/// record, for export as files.
#[derive(Debug, Clone, Serialize)]
pub struct Notebook {
    /// Commands and what the game said, in order.
    pub transcript: Vec<(String, String)>,
    pub named: Vec<String>,
    pub record: Record,
}

impl Game {
    /// The way the run ended, if it has.
    pub fn ending(&self) -> Option<&str> {
        let d = self.state.dead.as_ref()?;
        Some(if ENDINGS.contains(&d.cause.as_str()) {
            d.cause.as_str()
        } else {
            "death"
        })
    }

    /// Ends the run some way other than death.
    pub(crate) fn end_run(&mut self, ending: &str) {
        if self.state.dead.is_some() {
            return;
        }
        let doing = self.log.last().cloned().unwrap_or_default();
        self.state.dead = Some(Death {
            cause: ending.to_string(),
            doing,
            minutes: self.state.minutes,
            place: match self.state.place {
                Place::Outside => format!("outside {} {}", self.state.pos.x, self.state.pos.y), // DEBUG-TEXT: machine-readable
                Place::Room { structure, room } => format!("structure {structure} room {room}"), // DEBUG-TEXT: machine-readable
            },
        });
        let c = ctx(&[
            ("day", Value::Number(i64::from(self.day()) + 1)),
            ("indoors", Value::Bool(self.state.place != Place::Outside)),
            ("years", Value::Number(i64::from(self.age()))),
        ]);
        let t = self.say(&format!("end.{ending}"), c);
        self.notes.push(t);
        self.interrupted = true;
    }

    /// Ends that time brings: old age, and the land giving way.
    pub(crate) fn check_time_endings(&mut self) {
        if self.state.dead.is_some() {
            return;
        }
        if self.age() >= OLD_AGE {
            self.end_run("old_age");
            return;
        }
        if let Some(r) = self.site.regions.at(self.state.pos) {
            let v = self.state.regions.vars[r];
            if v[STABILITY] < COLLAPSE && v[LIFE] < COLLAPSE {
                self.end_run("overtaken");
            }
        }
    }

    /// After a player's own text is scraped: a claim naming the self ends
    /// the run. "Let the self depart" is leaving; any other claim on the
    /// self writes the player into the world.
    // DESIGN-Q: writing yourself in is any potent claim whose subject is
    // the self, other than the departure ("let the self not depart" is the
    // plainest).
    pub(crate) fn check_self_claim(&mut self, text: usize) {
        let Some((verb, subject, negative)) = claim_parts(self.text(text)) else {
            return;
        };
        if subject != "self" {
            return;
        }
        if verb == "depart" && !negative {
            self.end_run("left");
        } else {
            self.end_run("written_in");
        }
    }

    /// Notes which regional pushes a scrape began or ended.
    pub(crate) fn record_acts(&mut self, text: usize, before: &[scraped_sim::region::Driver]) {
        let day = self.day();
        let key = |d: &scraped_sim::region::Driver| (d.region, d.variable, d.target, d.reach);
        let was: BTreeSet<_> = before.iter().map(key).collect();
        let now: BTreeSet<_> = self.drivers.iter().map(key).collect();
        for (act, set, other) in [("released", &now, &was), ("silenced", &was, &now)] {
            for &(region, variable, target, reach) in set.difference(other) {
                self.state.acts.push(Act {
                    day,
                    text,
                    act: act.to_string(),
                    scale: if reach > 0 { "great" } else { "region" }.to_string(),
                    region,
                    variable,
                    rising: target > 0,
                });
            }
        }
    }

    /// The regions as they would be today had the player done nothing.
    fn without_player(&self) -> scraped_sim::region::RegionState {
        let mut st = self.site.regions.initial.clone();
        self.site
            .regions
            .advance(&mut st, self.state.regions.day, &self.initial_drivers);
        st
    }

    /// The run so far, as a structured account.
    pub fn record(&self) -> Record {
        let base = self.site.writing.count(&self.site.world);
        let mut releases: Vec<Release> = Vec::new();
        for &t in &self.state.scraped {
            let history = self.site.writing.scraped.contains(&t);
            let claim = claim_parts(self.text(t)).map(|(v, s, n)| {
                format!("{}{v} {s}", if n { "not " } else { "" }) // DEBUG-TEXT: spoiler
            });
            if history && claim.is_none() {
                continue;
            }
            releases.push(Release {
                text: t,
                scraped_by: if history { "history" } else { "you" }.to_string(),
                written_by: if t >= base { "you" } else { "history" }.to_string(),
                power: self.state.released.get(&t).copied().unwrap_or(0),
                claim,
            });
        }
        let without = self.without_player();
        let start_pos = self.site.start();
        let mut regions = Vec::new();
        for r in &self.site.regions.regions {
            let s = self.site.regions.initial.vars[r.id];
            let e = self.state.regions.vars[r.id];
            let w = without.vars[r.id];
            let mut changes = Vec::new();
            for v in 0..4 {
                let (b0, b1, bw) = (band(v, s[v]), band(v, e[v]), band(v, w[v]));
                if b0 == b1 && bw == b1 {
                    continue;
                }
                let cause = if bw == b1 {
                    "world"
                } else if bw == b0 {
                    "you"
                } else {
                    "both"
                };
                changes.push(Change {
                    aspect: VARIABLES[v].to_string(),
                    before: b0.to_string(),
                    after: b1.to_string(),
                    without: bw.to_string(),
                    cause: cause.to_string(),
                });
            }
            regions.push(RegionOutcome {
                region: r.id,
                biome: r.biome.clone(),
                bearing: bearing(start_pos, r.centre)
                    .filter(|_| r.centre.dist(start_pos) > 1500.0)
                    .map_or("here", |b| BEARINGS[b])
                    .to_string(),
                start: s,
                end: e,
                without: w,
                changes,
            });
        }
        let final_inscription = (0..self.state.written.len())
            .rev()
            .find(|&i| {
                !matches!(&self.player_texts.get(i).map(|t| &t.meaning), Some(Sentence::List(l)) if l.is_empty())
            })
            .filter(|&i| i < self.player_texts.len())
            .map(|i| base + i);
        Record {
            seed: self.seed(),
            ending: self.ending().unwrap_or("").to_string(),
            cause: self
                .state
                .dead
                .as_ref()
                .map(|d| d.cause.clone())
                .unwrap_or_default(),
            days: self.day(),
            age: self.age(),
            places: self.state.visited.iter().copied().collect(),
            named: self.state.names.iter().map(|n| n.0.clone()).collect(),
            texts_read: self.state.read.iter().copied().collect(),
            written: self.state.written.len(),
            releases,
            acts: self.state.acts.clone(),
            regions,
            final_inscription,
        }
    }

    /// The run inspector (spoilers): understanding by root, what has been
    /// read and written, the record so far, storylets and live claims here.
    pub fn inspect(&self) -> serde_json::Value {
        let met: Vec<&String> = self.state.encountered.keys().collect();
        let unmet: Vec<&str> = scraped_lang::concepts::all()
            .iter()
            .filter(|c| c.pos != scraped_lang::concepts::Pos::Particle)
            .map(|c| c.id.as_str())
            .filter(|c| !self.state.encountered.contains_key(*c))
            .collect();
        serde_json::json!({
            "understanding": self.understanding(),
            "threshold": self.threshold,
            "met": met,
            "unmet": unmet,
            "record": self.record(),
            "storylets": self.storylets_truth(),
            "claims_here": self.claims_here(),
            "regions": self.regions_truth(),
            "plan": self.plan_here(),
        })
    }

    /// The floor plan of this level of the building underfoot, with the
    /// spaces the player has stood in (D04; spoilers).
    pub fn plan_here(&self) -> Option<String> {
        let crate::site::Place::Room { structure, room } = self.state.place else {
            return None;
        };
        let st = self.site.structure(structure);
        let seen: Vec<usize> = self
            .state
            .rooms_seen
            .iter()
            .filter(|(s, _)| *s == structure)
            .map(|&(_, r)| r)
            .collect();
        Some(scraped_world::debug::floor_plan(
            &st.interior,
            st.interior.rooms[room].level,
            &seen,
            Some(room),
        ))
    }

    /// What this run leaves to the next world, once it has ended.
    pub fn legacy(&self) -> Option<Legacy> {
        self.state.dead.as_ref()?;
        let rec = self.record();
        let t = rec.final_inscription?;
        Some(Legacy {
            from_seed: self.seed(),
            ending: rec.ending,
            meaning: self.text(t).meaning.clone(),
        })
    }

    /// The player's records, for export.
    pub fn notebook(&self) -> Notebook {
        Notebook {
            transcript: self.transcript.clone(),
            named: self.state.names.iter().map(|n| n.0.clone()).collect(),
            record: self.record(),
        }
    }

    /// The notebook as plain text files: (transcript, named places).
    pub fn notebook_files(&mut self) -> (String, String) {
        let mut transcript = String::new();
        transcript.push_str(&self.notebook_heading("transcript"));
        transcript.push_str("\n\n");
        for (cmd, text) in &self.transcript {
            transcript.push_str(&format!("> {cmd}\n{text}\n\n")); // DEBUG-TEXT: transcript layout
        }
        let mut places = self.notebook_heading("names");
        places.push('\n');
        for (name, _) in &self.state.names {
            places.push_str(&format!("\n{name}")); // DEBUG-TEXT: list layout
        }
        places.push('\n');
        (transcript, places)
    }

    fn notebook_heading(&mut self, section: &str) -> String {
        self.say(
            "notebook.heading",
            ctx(&[("section", Value::from(section))]),
        )
    }

    /// The chronicle's meaning, from the record: written as by those who
    /// came after, in the newest era's language.
    pub fn chronicle(&self) -> Sentence {
        chronicle_of(&self.record(), |t| claim_parts(self.text(t)))
    }

    /// The chronicle as the player sees it: glyph numbers (or the player's
    /// own labels), "/" between words, as `write` takes them.
    fn chronicle_glyphs(&self) -> (String, usize) {
        let era = self.writing_era();
        let r = self.site.world.renderer(era as u32);
        let script = &self.site.world.languages[era].script;
        let mut words: Vec<Vec<String>> = vec![Vec::new()];
        for k in r.glyphs(&r.render(&self.chronicle())) {
            match k {
                Some(k) => {
                    let i = script.index(&k);
                    let mark = self
                        .state
                        .labels
                        .get(&format!("{era}:{i}"))
                        .cloned()
                        .unwrap_or_else(|| i.to_string());
                    words.last_mut().expect("a word").push(mark);
                }
                None => words.push(Vec::new()),
            }
        }
        words.retain(|w| !w.is_empty());
        let n = words.len();
        let text = words
            .iter()
            .map(|w| w.join(" "))
            .collect::<Vec<_>>()
            .join(" / ");
        (text, n)
    }

    /// The end of the run: what it came to, region by region and act by
    /// act, then the chronicle.
    pub(crate) fn summary_parts(&mut self) -> Vec<String> {
        let rec = self.record();
        let mut out = Vec::new();
        let released = rec
            .releases
            .iter()
            .filter(|r| r.scraped_by == "you")
            .count();
        let c = ctx(&[
            ("ending", Value::from(rec.ending.as_str())),
            ("cause", Value::from(rec.cause.as_str())),
            ("days", Value::Number(i64::from(rec.days) + 1)),
            ("years", Value::Number(i64::from(rec.age))),
            ("places", Value::Number(rec.places.len() as i64)),
            ("named", Value::Number(rec.named.len() as i64)),
            ("read", Value::Number(rec.texts_read.len() as i64)),
            ("wrote", Value::Number(rec.written as i64)),
            ("released", Value::Number(released as i64)),
        ]);
        out.push(self.say("end.summary", c));
        // Regions that changed alike are told together; the player's doing
        // first.
        let mut groups: Vec<(&Change, Vec<&RegionOutcome>)> = Vec::new();
        for r in &rec.regions {
            for ch in &r.changes {
                let same = |c: &Change| {
                    (&c.aspect, &c.before, &c.after, &c.without, &c.cause)
                        == (&ch.aspect, &ch.before, &ch.after, &ch.without, &ch.cause)
                };
                match groups.iter_mut().find(|(c, _)| same(c)) {
                    Some((_, rs)) => rs.push(r),
                    None => groups.push((ch, vec![r])),
                }
            }
        }
        groups.sort_by_key(|(c, rs)| (c.cause == "world", std::cmp::Reverse(rs.len())));
        let any = !groups.is_empty();
        let lines: Vec<Context> = groups
            .iter()
            .map(|(ch, rs)| {
                let mut bearings: Vec<&str> = rs.iter().map(|r| r.bearing.as_str()).collect();
                bearings.sort_by_key(|b| BEARINGS.iter().position(|x| x == b).unwrap_or(8));
                bearings.dedup();
                let mut biomes: Vec<&str> = rs.iter().map(|r| r.biome.as_str()).collect();
                biomes.sort_unstable();
                biomes.dedup();
                ctx(&[
                    ("count", Value::Number(rs.len() as i64)),
                    (
                        "bearings",
                        Value::List(bearings.into_iter().map(Value::from).collect()),
                    ),
                    (
                        "biomes",
                        Value::List(biomes.into_iter().map(Value::from).collect()),
                    ),
                    ("aspect", Value::from(ch.aspect.as_str())),
                    ("before", Value::from(ch.before.as_str())),
                    ("after", Value::from(ch.after.as_str())),
                    ("without", Value::from(ch.without.as_str())),
                    ("cause", Value::from(ch.cause.as_str())),
                ])
            })
            .collect();
        for c in lines {
            out.push(self.say("end.region", c));
        }
        if !any {
            out.push(self.say("end.calm", Context::new()));
        }
        let start = self.site.start();
        for a in &rec.acts {
            let centre = self.site.regions.regions[a.region].centre;
            let c = ctx(&[
                ("day", Value::Number(i64::from(a.day) + 1)),
                ("act", Value::from(a.act.as_str())),
                ("scale", Value::from(a.scale.as_str())),
                ("aspect", Value::from(VARIABLES[a.variable])),
                ("rising", Value::Bool(a.rising)),
                (
                    "bearing",
                    Value::from(
                        bearing(start, centre)
                            .filter(|_| centre.dist(start) > 1500.0)
                            .map_or("here", |b| BEARINGS[b]),
                    ),
                ),
            ]);
            out.push(self.say("end.act", c));
        }
        let (glyphs, words) = self.chronicle_glyphs();
        out.push(self.say(
            "end.chronicle",
            ctx(&[("words", Value::Number(words as i64))]),
        ));
        out.push(glyphs);
        out
    }

    /// What the end of a run says to any further command.
    pub(crate) fn ended(&mut self) -> Output {
        let parts = self.summary_parts();
        let truth = self.end_truth();
        self.output(parts, Some(truth))
    }

    /// Spoiler: the record, and the chronicle's meaning and gloss.
    pub(crate) fn end_truth(&self) -> serde_json::Value {
        let era = self.writing_era();
        let r = self.site.world.renderer(era as u32);
        let meaning = self.chronicle();
        let rendered = r.render(&meaning);
        serde_json::json!({
            "record": self.record(),
            "chronicle": {
                "meaning": meaning,
                "romanised": r.surface(&rendered),
                "gloss": scraped_lang::english::translate(&meaning, &|i| format!("#{i}")), // DEBUG-TEXT: spoiler gloss
            },
        })
    }
}

fn np(concept: &str) -> NounPhrase {
    NounPhrase::concept(concept)
}

fn clause(verb: &str, tense: Tense, negative: bool, args: Vec<(Role, NounPhrase)>) -> Sentence {
    Sentence::Clause(Clause {
        predicate: verb.to_string(),
        mood: Mood::Declarative,
        tense,
        polarity: if negative {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args: args
            .into_iter()
            .map(|(role, np)| Argument { role, np })
            .collect(),
        adverbs: Vec::new(),
    })
}

/// What a regional change looks like in the chronicle.
// DESIGN-Q: life falling is "the fields burned"; water rising is "the
// people drank the water"; ground failing is "the walls broke"; warmth is
// "the sun burned" (each denied for the reverse).
fn outcome(variable: usize, rising: bool) -> Sentence {
    let people = || np("person").plural();
    match variable {
        LIFE => clause(
            "burn",
            Tense::Past,
            rising,
            vec![(Role::Subject, np("field").plural())],
        ),
        WATER => clause(
            "drink",
            Tense::Past,
            !rising,
            vec![(Role::Subject, people()), (Role::Object, np("water"))],
        ),
        STABILITY => clause(
            "break",
            Tense::Past,
            rising,
            vec![(Role::Subject, np("wall").plural())],
        ),
        _ => clause(
            "burn",
            Tense::Past,
            !rising,
            vec![(Role::Subject, np("sun"))],
        ),
    }
}

/// The chronicle of a run: the self came; what changed by their hand; how
/// the people remember them; how they ended.
// DESIGN-Q: at most six clauses: arrival, up to two outcomes (or one
// released claim restated), the people's verdict, the ending.
pub fn chronicle_of(
    rec: &Record,
    claim: impl Fn(usize) -> Option<(String, String, bool)>,
) -> Sentence {
    let mut parts = vec![clause(
        "enter",
        Tense::Past,
        false,
        vec![(Role::Subject, np("self")), (Role::Object, np("city"))],
    )];
    // The player's largest regional effects: end against the world left
    // alone.
    let mut effects: Vec<(i32, usize, bool)> = Vec::new();
    for r in &rec.regions {
        for v in [LIFE, WATER, STABILITY, CLIMATE] {
            let d = r.end[v] - r.without[v];
            let scale = if v == CLIMATE { d / 10 } else { d };
            if scale.abs() >= 50 {
                effects.push((-scale.abs(), v, d > 0));
            }
        }
    }
    effects.sort();
    let mut seen = BTreeSet::new();
    let mut verdict = 0;
    for &(size, v, rising) in &effects {
        if v != CLIMATE {
            verdict += if rising { -size } else { size };
        }
        if seen.len() < 2 && seen.insert(v) {
            parts.push(outcome(v, rising));
        }
    }
    if seen.is_empty() {
        if let Some((verb, subject, negative)) = rec
            .releases
            .iter()
            .filter(|r| r.scraped_by == "you")
            .find_map(|r| claim(r.text))
            .filter(|(_, s, _)| s != "self")
        {
            parts.push(clause(
                &verb,
                Tense::Past,
                negative,
                vec![(Role::Subject, np(&subject))],
            ));
        }
    }
    if verdict != 0 {
        let verb = if verdict > 0 { "honour" } else { "fear" };
        parts.push(clause(
            verb,
            Tense::Past,
            false,
            vec![
                (Role::Subject, np("person").plural()),
                (Role::Object, np("self")),
            ],
        ));
    }
    parts.push(match rec.ending.as_str() {
        "left" => clause(
            "depart",
            Tense::Past,
            false,
            vec![(Role::Subject, np("self"))],
        ),
        "written_in" => clause(
            "depart",
            Tense::Past,
            true,
            vec![(Role::Subject, np("self"))],
        ),
        _ => {
            let Sentence::Clause(mut c) = clause(
                "lie",
                Tense::NonPast,
                false,
                vec![(Role::Subject, np("self"))],
            ) else {
                unreachable!()
            };
            c.adverbs.push("here".to_string());
            Sentence::Clause(c)
        }
    });
    Sentence::Text(parts)
}

trait Plural {
    fn plural(self) -> Self;
}

impl Plural for NounPhrase {
    fn plural(mut self) -> Self {
        self.number = Number::Plural;
        self
    }
}
