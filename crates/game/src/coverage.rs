//! Reachability coverage (M13): bots play many seeds and every slot render
//! is recorded, so the authoring tool can rank the text players will
//! actually meet, the conditions they hit, the variable combinations that
//! fall back to generic text, the slots nobody reaches, and the variants a
//! player would see over and over.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use scraped_content::{Pack, Value};
use scraped_sim::outdoors::hash;

use crate::{Game, Output, Rendered};

/// Bots that play for coverage: the depth bots (D01). The old wanderer
/// below stays as the scripted player behind the determinism fixtures.
pub const BOTS: &[&str] = crate::bots::DEPTH_BOTS;

/// A deterministic player: looks, reads, takes, walks in and out of
/// buildings and across the land, and looks after its body. The scholar
/// starts with the writing tools and also scrapes and writes.
pub struct Bot {
    pub kind: &'static str,
    seed: u64,
    step: u64,
}

impl Bot {
    pub fn new(kind: &'static str, seed: u64) -> Self {
        Bot {
            kind,
            seed,
            step: 0,
        }
    }

    /// Gets a game ready for this bot.
    pub fn prepare(&self, g: &mut Game) {
        if self.kind == "scholar" {
            for k in [
                "scraper",
                "stylus",
                "lens",
                "torch",
                "firesteel",
                "waterskin",
            ] {
                g.make_item(k, true);
            }
        }
    }

    /// The next command, from what the last output showed.
    pub fn next(&mut self, g: &Game, last: &Output) -> String {
        self.step += 1;
        let s = &last.state;
        let body = |need: &str| s.body.get(need).map(String::as_str).unwrap_or("");
        let h = hash(&[self.seed, 0xb07, self.step]);
        // The body first, half the time (the rest is spent looking for
        // what it needs).
        let tend = h.is_multiple_of(2);
        if tend && matches!(body("thirst"), "thirsty" | "parched" | "dehydrated") {
            return if h.is_multiple_of(3) {
                "drink from waterskin"
            } else {
                "drink"
            }
            .to_string();
        }
        if tend && matches!(body("hunger"), "hungry" | "starving") {
            return if h.is_multiple_of(2) { "eat" } else { "forage" }.to_string();
        }
        if tend && matches!(body("rest"), "tired" | "exhausted") {
            return "sleep".to_string();
        }
        if tend && matches!(body("warmth"), "shivering" | "hypothermic") {
            return "make fire".to_string();
        }
        let last_word = |name: &str| name.split_whitespace().last().unwrap_or("").to_string();
        let mut options: Vec<String> = vec!["look".into(), "status".into(), "inventory".into()];
        for t in &s.things {
            let w = last_word(t);
            options.push(format!("read {w}"));
            options.push(format!("examine {w}"));
            options.push(format!("take {w}"));
            if self.kind == "scholar" {
                options.push(format!("read {w}"));
                options.push(format!("scrape {w}"));
                let marks: Vec<String> = (0..3)
                    .map(|k| format!("#{}", (h >> (8 * k)) % 20))
                    .collect();
                options.push(format!("write {} on {w}", marks.join(" ")));
            }
        }
        if s.place == "outside" {
            for e in &s.exits {
                options.push(format!("go {}", last_word(e)));
                options.push(format!("go {}", last_word(e)));
            }
            for d in ["north", "south", "east", "west"] {
                options.push(format!("head {d}"));
            }
            for l in s.landmarks.iter().take(2) {
                options.push(format!("go {}", last_word(&l.name)));
            }
            options.push("forage".into());
        } else {
            for e in &s.exits {
                options.push(e.clone());
            }
            options.push("out".into());
            if g.state.carried.is_empty() {
                options.push("out".into());
            }
        }
        if s.light == "dark" {
            options.push("light torch".into());
        }
        options[(h % options.len() as u64) as usize].clone()
    }
}

/// How one slot fared across the runs.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SlotCoverage {
    pub slot: String,
    /// Renders across all runs.
    pub hits: usize,
    /// Renders per player-hour.
    pub per_hour: f64,
    /// Renders that showed a placeholder or an agent's example.
    pub gaps: usize,
    /// Gap renders per player-hour: the ranking.
    pub gap_hours: f64,
    /// Hits per variant, keyed "file#index".
    pub variants: BTreeMap<String, usize>,
    /// For each variant, its `[if]` branches in order: (taken, not taken).
    pub branches: BTreeMap<String, Vec<(usize, usize)>>,
    /// Variable combinations (enum and yes/no variables) that were common
    /// but got a generic variant although the slot has conditional ones.
    pub fallbacks: Vec<(String, usize)>,
    /// Repetition: in a typical run, how often the most-seen variant was
    /// seen, and how many renders the slot had.
    pub same_per_run: f64,
    pub renders_per_run: f64,
    /// Whether a player would see the same variant often enough to notice.
    pub needs_more: bool,
}

/// The whole coverage report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub seeds: Vec<u64>,
    pub bots: Vec<String>,
    pub steps: usize,
    pub runs: usize,
    pub hours: f64,
    /// Every slot reached, most valuable gap first.
    pub slots: Vec<SlotCoverage>,
    /// Slots no run ever reached.
    pub never: Vec<String>,
}

/// A variant seen this often in one run, with fewer variants than that to
/// choose from, wants more.
// DESIGN-Q: four times a run.
pub const REPEAT: f64 = 4.0;

/// One bot's run on one seed: the renders it caused and the hours played.
pub fn play(pack: &Pack, seed: u64, kind: &'static str, steps: usize) -> (Vec<Rendered>, f64) {
    let run = crate::bots::play(pack, seed, kind, 24.0 * 365.0, steps);
    (run.renders, run.hours.max(0.1))
}

/// A fingerprint of a whole scripted run: every command, everything the
/// player was told, and the final state. The same on every platform, or
/// determinism is broken.
pub fn transcript_hash(pack: &Pack, seed: u64, preset: &str, steps: usize) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |s: &str| {
        for b in s.bytes().chain([0xff]) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    let mut g = Game::create(seed, pack.clone(), preset, None);
    let mut out = g.start();
    feed(&out.text);
    let mut bot = Bot::new("wanderer", seed);
    for _ in 0..steps {
        if g.state.dead.is_some() {
            break;
        }
        let cmd = bot.next(&g, &out);
        out = g.step(&cmd);
        feed(&cmd);
        feed(&out.text);
    }
    feed(&serde_json::to_string(&g.state).expect("state serialises"));
    format!("{h:016x}")
}

pub(crate) fn combo(vars: &scraped_content::Context) -> String {
    vars.iter()
        .filter(|(_, v)| {
            matches!(v, Value::Bool(_))
                || matches!(v, Value::Text(t) if t.len() <= 16 && !t.contains(' '))
        })
        .map(|(k, v)| format!("{k}={}", v.text()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs every bot on every seed and measures coverage.
pub fn run(pack: &Pack, seeds: &[u64], steps: usize) -> Report {
    let registry = crate::slots::registry_for(pack);
    let conditional: BTreeSet<&str> = pack
        .files
        .iter()
        .flat_map(|f| f.variants.iter())
        .filter(|v| v.when.is_some())
        .map(|v| v.slot.as_str())
        .collect();
    let mut slots: BTreeMap<String, SlotCoverage> = BTreeMap::new();
    let mut combos: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut per_run_max: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut per_run_hits: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut hours = 0.0;
    let mut runs = 0;
    for &seed in seeds {
        for &bot in BOTS {
            let (renders, h) = play(pack, seed, bot, steps);
            hours += h;
            runs += 1;
            let mut seen: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
            for r in &renders {
                let t = &r.trace;
                let c = slots.entry(t.slot.clone()).or_insert_with(|| SlotCoverage {
                    slot: t.slot.clone(),
                    ..Default::default()
                });
                c.hits += 1;
                let key = match (&t.file, t.variant) {
                    (Some(f), Some(i)) => format!("{f}#{i}"),
                    _ => "placeholder".to_string(),
                };
                if t.variant.is_none() || t.example {
                    c.gaps += 1;
                }
                *c.variants.entry(key.clone()).or_default() += 1;
                let b = c.branches.entry(key.clone()).or_default();
                for (i, &taken) in t.branches.iter().enumerate() {
                    if b.len() <= i {
                        b.push((0, 0));
                    }
                    if taken {
                        b[i].0 += 1;
                    } else {
                        b[i].1 += 1;
                    }
                }
                if t.variant.is_some() && !t.conditional && conditional.contains(t.slot.as_str()) {
                    *combos
                        .entry(t.slot.clone())
                        .or_default()
                        .entry(combo(&r.vars))
                        .or_default() += 1;
                }
                *seen
                    .entry(t.slot.clone())
                    .or_default()
                    .entry(key)
                    .or_default() += 1;
            }
            for (slot, vs) in seen {
                per_run_max
                    .entry(slot.clone())
                    .or_default()
                    .push(vs.values().copied().max().unwrap_or(0));
                per_run_hits
                    .entry(slot)
                    .or_default()
                    .push(vs.values().sum());
            }
        }
    }
    let runs_f = runs.max(1) as f64;
    for (slot, c) in slots.iter_mut() {
        c.per_hour = c.hits as f64 / hours;
        c.gap_hours = c.gaps as f64 / hours;
        c.same_per_run = per_run_max
            .get(slot)
            .map_or(0.0, |v| v.iter().sum::<usize>() as f64)
            / runs_f;
        c.renders_per_run = per_run_hits
            .get(slot)
            .map_or(0.0, |v| v.iter().sum::<usize>() as f64)
            / runs_f;
        let distinct = c.variants.keys().filter(|k| *k != "placeholder").count();
        c.needs_more = c.same_per_run >= REPEAT && (distinct as f64) < c.renders_per_run / REPEAT;
        if let Some(cs) = combos.get(slot) {
            let mut v: Vec<(String, usize)> = cs.iter().map(|(k, n)| (k.clone(), *n)).collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            v.truncate(5);
            c.fallbacks = v;
        }
    }
    let never: Vec<String> = registry
        .slots
        .iter()
        .map(|s| s.id.clone())
        .filter(|id| !slots.contains_key(id))
        .collect();
    let mut list: Vec<SlotCoverage> = slots.into_values().collect();
    list.sort_by(|a, b| {
        b.gap_hours
            .partial_cmp(&a.gap_hours)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.per_hour
                    .partial_cmp(&a.per_hour)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.slot.cmp(&b.slot))
    });
    Report {
        seeds: seeds.to_vec(),
        bots: BOTS.iter().map(|b| b.to_string()).collect(),
        steps,
        runs,
        hours,
        slots: list,
        never,
    }
}
