//! Depth metrics (D01): how much a world holds, and how much of it a
//! curious player meets and is told at once. Every later depth milestone
//! is judged by these numbers, so each is defined in `docs/DEPTH.md` and
//! stays deterministic.
//!
//! World measures read the generated world directly. Variety, brevity and
//! depth on demand come from the curious explorer's play, rendered with
//! the example variants, so they measure the engine and not the prose.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use scraped_content::Pack;
use scraped_lang::corpus::Kind;
use scraped_lang::meaning::Sentence;
use scraped_world::terrain::SIZE;

use crate::bots;
use crate::composing::concepts_of;
use crate::fairness::names as names_in;
use crate::Game;

/// Median and 95th percentile.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Spread {
    pub median: f64,
    pub p95: f64,
    pub n: usize,
}

fn spread(mut v: Vec<f64>) -> Spread {
    if v.is_empty() {
        return Spread::default();
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
    Spread {
        median: at(0.5),
        p95: at(0.95),
        n: v.len(),
    }
}

/// One world's measures.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WorldDepth {
    pub seed: u64,
    /// Every number below, flattened by name, for averaging and tables.
    pub metrics: BTreeMap<String, f64>,
    /// Texts by genre.
    pub genres: BTreeMap<String, usize>,
    /// History events by kind.
    pub events: BTreeMap<String, usize>,
    /// For each slot family, distinct variable combinations the explorer
    /// met.
    pub variety: BTreeMap<String, usize>,
    /// Words and facts per response, by kind of response.
    pub brevity: BTreeMap<String, (Spread, Spread)>,
}

/// The measures over several worlds, with their means.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DepthReport {
    pub seeds: Vec<u64>,
    pub hours: f64,
    pub worlds: Vec<WorldDepth>,
    pub mean: BTreeMap<String, f64>,
}

/// A meaning's shape: its structure with names and numbers abstracted.
fn shape(s: &Sentence) -> String {
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Number(_) => *v = serde_json::Value::from(0),
            serde_json::Value::Array(a) => a.iter_mut().for_each(walk),
            serde_json::Value::Object(o) => o.values_mut().for_each(walk),
            _ => {}
        }
    }
    let mut v = serde_json::to_value(s).unwrap_or_default();
    walk(&mut v);
    v.to_string()
}

fn genre(k: Kind) -> &'static str {
    match k {
        Kind::Tomb => "tomb",
        Kind::Ledger => "ledger",
        Kind::Warning => "warning",
        Kind::Dedication => "dedication",
        Kind::Label => "label",
        Kind::Letter => "letter",
        Kind::Potent => "potent",
        Kind::Account => "account",
    }
}

/// Measures one world: the world itself, and an explorer's `hours` in it.
pub fn measure(pack: &Pack, seed: u64, hours: f64) -> WorldDepth {
    let g = Game::new(seed, pack.clone());
    let site = &g.site;
    let w = &site.world;
    let mut m: BTreeMap<String, f64> = BTreeMap::new();
    let mut put = |k: &str, v: f64| {
        m.insert(k.to_string(), v);
    };

    // ---------- places ----------
    let kinds: BTreeSet<_> = w.structures.iter().map(|s| s.kind).collect();
    put("places.structure_kinds", kinds.len() as f64);
    let purposes: BTreeSet<&str> = w
        .structures
        .iter()
        .flat_map(|s| s.interior.rooms.iter().map(|r| r.purpose))
        .collect();
    put("places.room_purposes", purposes.len() as f64);
    let mut natural: BTreeSet<&str> = BTreeSet::new();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let e = *site.land.edges.get(x, y);
            for (i, k) in scraped_sim::outdoors::EDGES.iter().enumerate() {
                if e & (1 << i) != 0 && *k != "road" {
                    natural.insert(k);
                }
            }
        }
    }
    for l in &site.land.landmarks {
        if l.settlement.is_none() && l.structure.is_none() && l.feature.is_none() {
            natural.insert(l.kind);
        }
    }
    // Natural features (D03), not the old marks people left.
    for f in &w.features {
        if scraped_world::features::kind(f.kind).group != scraped_world::features::Group::Marks {
            natural.insert(f.kind);
        }
    }
    put("places.natural_kinds", natural.len() as f64);
    put("places.landmarks", site.land.landmarks.len() as f64);
    // Landmark names as the example text renders them.
    let mut probe = Game::new(seed, pack.clone());
    let names: Vec<String> = (0..site.land.landmarks.len())
        .map(|i| probe.landmark_name(i))
        .collect();
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for n in &names {
        *count.entry(n.as_str()).or_default() += 1;
    }
    let unique = names.iter().filter(|n| count[n.as_str()] == 1).count();
    put(
        "places.landmarks_unique_share",
        unique as f64 / names.len().max(1) as f64,
    );
    let land_cells = (0..SIZE)
        .flat_map(|y| (0..SIZE).map(move |x| (x, y)))
        .filter(|&(x, y)| w.terrain.is_land(x, y))
        .count();
    let km2 = land_cells as f64 * 0.09;
    let worth = w.structures.len()
        + site
            .land
            .landmarks
            .iter()
            .filter(|l| l.settlement.is_none() && l.structure.is_none() && l.feature.is_none())
            .count()
        + w.features.len();
    put("places.per_km2", worth as f64 / km2.max(1.0));

    // ---------- things ----------
    let thing_kinds: BTreeSet<&str> = site.things.iter().map(|t| t.kind).collect();
    put("things.kinds", thing_kinds.len() as f64);
    put(
        "things.per_structure",
        site.things.len() as f64 / w.structures.len().max(1) as f64,
    );
    let mechs = &site.fixtures.mechanisms;
    put("things.mechanisms", mechs.len() as f64);
    let chain = mechs
        .iter()
        .map(|mc| 1 + usize::from(mc.controls.is_some()))
        .max()
        .unwrap_or(0);
    put("things.longest_chain", chain as f64);

    // ---------- life ----------
    let species: BTreeSet<&str> = site
        .fixtures
        .creatures
        .iter()
        .map(|c| c.archetype)
        .collect();
    put("life.species", species.len() as f64);
    // No creature leaves signs (tracks, nests) yet.
    put("life.with_signs", 0.0);

    // ---------- history ----------
    let mut events: BTreeMap<String, usize> = BTreeMap::new();
    for e in &w.history.events {
        let v = serde_json::to_value(&e.kind).unwrap_or_default();
        let k = match v {
            serde_json::Value::Object(o) => o.keys().next().cloned().unwrap_or_default(),
            serde_json::Value::String(s) => s,
            _ => String::new(),
        };
        *events.entry(k.to_lowercase()).or_default() += 1;
    }
    put("history.event_kinds", events.len() as f64);
    put("history.events", w.history.events.len() as f64);

    // ---------- writing ----------
    let count_texts = site.writing.count(w);
    let mut genres: BTreeMap<String, usize> = BTreeMap::new();
    let mut shapes = BTreeSet::new();
    let mut concepts = BTreeSet::new();
    let mut words = 0usize;
    let mut traces: BTreeMap<usize, usize> = BTreeMap::new();
    for id in 0..count_texts {
        let t = site.writing.text(w, id);
        *genres.entry(genre(t.kind).to_string()).or_default() += 1;
        shapes.insert(shape(&t.meaning));
        concepts_of(
            &t.meaning,
            &mut concepts,
            &w.languages[t.era as usize].numerals,
        );
        words += w.renderer(t.era).render(&t.meaning).words.len();
        let mut n = Vec::new();
        names_in(&t.meaning, &mut n);
        for p in n.into_iter().collect::<BTreeSet<_>>() {
            *traces.entry(p).or_default() += 1;
        }
    }
    let largest = genres.values().copied().max().unwrap_or(0);
    put("writing.texts", count_texts as f64);
    put("writing.genres", genres.len() as f64);
    put(
        "writing.largest_genre_share",
        largest as f64 / count_texts.max(1) as f64,
    );
    put("writing.sentence_shapes", shapes.len() as f64);
    put(
        "writing.words_per_text",
        words as f64 / count_texts.max(1) as f64,
    );
    put("writing.concepts", concepts.len() as f64);
    put(
        "history.people_with_traces",
        traces.values().filter(|&&n| n > 1).count() as f64,
    );

    // ---------- magic ----------
    let claims = site
        .writing
        .live_claims(w, &site.land, &site.writing.scraped);
    put("magic.live_spells", claims.len() as f64);
    let types: BTreeSet<String> = claims
        .iter()
        .map(|c| format!("{:?} {:?} {}", c.class, c.property, c.amount.signum()))
        .collect();
    put("magic.claim_types", types.len() as f64);
    let strange = w
        .structures
        .iter()
        .filter(|st| {
            let p = site.land.structure_pos[st.id];
            claims.iter().any(|c| c.pos.dist(p) <= c.range)
        })
        .count();
    put("magic.places_with_writing_cause", strange as f64);
    // Every strangeness has a writing cause so far: there are no natural
    // oddities yet (D06).
    put("magic.strange_without_writing_share", 0.0);

    // ---------- play ----------
    let run = bots::play(pack, seed, "explorer", hours, 20_000);
    let mut variety: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &run.renders {
        let family = r.trace.slot.split('.').next().unwrap_or("").to_string();
        variety.entry(family).or_default().insert(format!(
            "{} {}",
            r.trace.slot,
            crate::coverage::combo(&r.vars)
        ));
    }
    let variety: BTreeMap<String, usize> = variety.into_iter().map(|(k, v)| (k, v.len())).collect();
    put(
        "variety.combinations",
        variety.values().sum::<usize>() as f64,
    );
    let mut brevity: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
    for (i, text) in run.texts.iter().enumerate() {
        let kind = run.kinds.get(i).copied().unwrap_or("other");
        let words = text.split_whitespace().count() as f64;
        let facts = f64::from(run.facts.get(i).copied().unwrap_or(0));
        let e = brevity.entry(kind.to_string()).or_default();
        e.0.push(words);
        e.1.push(facts);
    }
    let brevity: BTreeMap<String, (Spread, Spread)> = brevity
        .into_iter()
        .map(|(k, (w, f))| (k, (spread(w), spread(f))))
        .collect();
    for (k, (wd, f)) in &brevity {
        put(&format!("brevity.{k}.words_median"), wd.median);
        put(&format!("brevity.{k}.words_p95"), wd.p95);
        put(&format!("brevity.{k}.facts_median"), f.median);
        put(&format!("brevity.{k}.facts_p95"), f.p95);
    }
    // Depth on demand: at each arrival, how many facts digging could turn
    // up for each one shown.
    let ratios: Vec<f64> = run
        .on_demand
        .iter()
        .map(|&(shown, more)| f64::from(more) / f64::from(shown.max(1)))
        .collect();
    put("depth_on_demand.per_fact_shown", spread(ratios).median);
    let more: Vec<f64> = run.on_demand.iter().map(|&(_, m)| f64::from(m)).collect();
    put("depth_on_demand.per_place", spread(more).median);
    put("play.hours", run.hours);
    put(
        "play.novel_per_hour",
        run.novelty.len() as f64 / run.hours.max(0.1),
    );
    let kinds: Vec<u32> = run
        .novelty
        .iter()
        .filter(|n| n.new_kind)
        .map(|n| n.minutes)
        .collect();
    let gaps: Vec<f64> = kinds.windows(2).map(|w| f64::from(w[1] - w[0])).collect();
    put("play.minutes_between_new_kinds", spread(gaps).median);

    WorldDepth {
        seed,
        metrics: m,
        genres,
        events,
        variety,
        brevity,
    }
}

/// Measures several worlds and averages them.
pub fn report(pack: &Pack, seeds: &[u64], hours: f64) -> DepthReport {
    let worlds: Vec<WorldDepth> = seeds.iter().map(|&s| measure(pack, s, hours)).collect();
    let mut sum: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for w in &worlds {
        for (k, v) in &w.metrics {
            let e = sum.entry(k.clone()).or_default();
            e.0 += v;
            e.1 += 1;
        }
    }
    let mean = sum
        .into_iter()
        .map(|(k, (s, n))| (k, s / n.max(1) as f64))
        .collect();
    DepthReport {
        seeds: seeds.to_vec(),
        hours,
        worlds,
        mean,
    }
}

/// A plain table of the means, for the terminal.
pub fn table(r: &DepthReport) -> String {
    let mut out = format!(
        "depth over seeds {:?} (explorer {} h each)\n", // DEBUG-TEXT
        r.seeds, r.hours
    );
    for (k, v) in &r.mean {
        out.push_str(&format!("  {k:<40} {v:>10.2}\n")); // DEBUG-TEXT
    }
    out
}
