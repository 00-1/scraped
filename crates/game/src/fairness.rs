//! Fairness (M14): every world must be solvable with a notebook. For each
//! key goal the checker traces what a player needs (things they must reach,
//! words they must have met, constructions they must have seen) and checks
//! the world holds enough evidence. Seeds that fail are passed over at
//! generation time in favour of a derived seed that passes.
//!
//! It also measures how hard a world's language is to decipher: evidence
//! density, ambiguity and anchors.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use scraped_lang::corpus::Kind;
use scraped_lang::meaning::{Head, Sentence};
use scraped_sim::fixtures::Spot;
use scraped_sim::outdoors::{hash, Pos};

use crate::composing::{concepts_of, THRESHOLD};
use crate::site::Site;

pub use scraped_lang::difficulty::PRESETS;

/// The goals a world must make reachable, in the order a player meets
/// them.
pub const GOALS: &[&str] = &[
    "constructions",
    "tool",
    "latent",
    "first_write",
    "powers",
    "great",
    "deepest",
    "leaving",
    "edge",
    "sealed",
];

/// Concepts that must be attested in both the oldest and the newest era's
/// readable writing, so the sound changes between them can be worked out
/// (the departure is read in the oldest language and written in the
/// newest).
// DESIGN-Q: ten shared concepts.
pub const BRIDGE: usize = 10;

/// One goal's verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Goal {
    pub goal: String,
    pub ok: bool,
    /// What is missing, as debug ids.
    pub missing: Vec<String>,
}

/// Difficulty measurements of a world's writing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    /// Readable texts.
    pub texts: usize,
    /// Distinct concepts attested in readable writing.
    pub concepts: usize,
    /// Mean readable texts per attested concept.
    pub density: f64,
    /// Share of the newest era's roots whose written form another root
    /// shares.
    pub ambiguity: f64,
    /// Footholds: numerals, potent formulae and names met more than once.
    pub anchors: usize,
    /// Concepts attested in both the oldest and newest era.
    pub bridge: usize,
}

/// A world's fairness report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub seed: u64,
    pub preset: String,
    pub ok: bool,
    pub goals: Vec<Goal>,
    pub metrics: Metrics,
}

/// Where a player can stand to read or take things.
struct Reach<'a> {
    site: &'a Site,
    start: Pos,
    routes: BTreeMap<usize, bool>,
    /// Sealed places (D11) taken as opened: the rest stay shut.
    opened: BTreeSet<usize>,
}

impl Reach<'_> {
    fn structure(&mut self, s: usize) -> bool {
        if let Some(&r) = self.routes.get(&s) {
            return r;
        }
        let w = &self.site.world;
        let sealed = self.site.writing.sealed.iter().any(|x| x.structure == s);
        let r = self.site.enterable(s)
            && (!sealed || self.opened.contains(&s))
            && self
                .site
                .land
                .route(w, self.start, self.site.land.structure_pos[s])
                .is_some();
        self.routes.insert(s, r);
        r
    }

    fn room(&mut self, s: usize, room: Option<usize>) -> bool {
        match room {
            None => self
                .site
                .land
                .route(
                    &self.site.world,
                    self.start,
                    self.site.land.structure_pos[s],
                )
                .is_some(),
            Some(r) => {
                self.site
                    .fixtures
                    .reachable_rooms(&self.site.world, s)
                    .contains(&r)
                    && self.structure(s)
            }
        }
    }

    fn spot(&mut self, at: Spot, pos: Pos) -> bool {
        match at {
            Spot::Room { structure, room } => self.room(structure, Some(room)),
            Spot::Out { .. } => self
                .site
                .land
                .route(&self.site.world, self.start, pos)
                .is_some(),
        }
    }

    fn item(&mut self, kind: &str) -> bool {
        let found: Vec<(Spot, Pos)> = self
            .site
            .fixtures
            .items
            .iter()
            .filter(|p| p.kind == kind)
            .map(|p| (p.at, p.pos))
            .collect();
        found.into_iter().any(|(at, pos)| self.spot(at, pos))
    }

    fn text(&mut self, id: usize) -> bool {
        let t = self.site.writing.text(&self.site.world, id);
        let (s, r) = (t.structure, t.room);
        self.room(s, r)
    }
}

pub(crate) fn names(s: &Sentence, out: &mut Vec<usize>) {
    for n in s.noun_phrases() {
        if let Head::Name(i) = n.head {
            out.push(i);
        }
    }
}

fn potent(s: &Sentence) -> bool {
    s.is_potent()
}

/// Checks a site's world for fairness.
pub fn check(site: &Site, preset: &str) -> Report {
    let w = &site.world;
    let newest = (w.languages.len() as u32).saturating_sub(1);
    let mut reach = Reach {
        site,
        start: site.start(),
        routes: BTreeMap::new(),
        opened: BTreeSet::new(),
    };
    let writing = &site.writing;
    // Readable writing: every text on a surface a player can stand at,
    // except the deepest accounts (they need the first lens).
    let mut contexts: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut by_era: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut texts = 0;
    let mut anchors = 0;
    let mut name_uses: BTreeMap<usize, usize> = BTreeMap::new();
    let mut newest_potent = false;
    let mut seen_built: BTreeMap<&'static str, BTreeSet<usize>> = BTreeMap::new();
    for id in 0..writing.count(w) {
        if writing.deep.contains(&id) || Some(id) == writing.legacy {
            continue;
        }
        if !reach.text(id) {
            continue;
        }
        let t = writing.text(w, id);
        texts += 1;
        let mut roots = BTreeSet::new();
        concepts_of(
            &t.meaning,
            &mut roots,
            &w.languages[t.era as usize].numerals,
        );
        for r in &roots {
            contexts.entry(r.clone()).or_default().insert(id);
            by_era.entry(t.era).or_default().insert(r.clone());
        }
        if potent(&t.meaning) {
            anchors += 1;
            newest_potent |= t.era == newest;
        }
        if t.kind == Kind::Ledger {
            anchors += 1;
        }
        let mut n = Vec::new();
        names(&t.meaning, &mut n);
        for i in n {
            *name_uses.entry(i).or_default() += 1;
        }
        for f in t.meaning.function_words() {
            seen_built.entry(f).or_default().insert(id);
        }
    }
    // D08: every construction the stories use is met in enough readable
    // texts to be learnt.
    let mut story_words: BTreeSet<&'static str> = BTreeSet::new();
    for id in 0..writing.count(w) {
        let t = writing.text(w, id);
        if t.arc.is_some() {
            story_words.extend(t.meaning.function_words());
        }
    }
    let unlearnt: Vec<String> = story_words
        .iter()
        .filter(|f| seen_built.get(*f).map_or(0, |s| s.len()) < THRESHOLD)
        .map(|f| format!("construction {f} met too seldom"))
        .collect();
    anchors += name_uses.values().filter(|&&n| n > 1).count();
    let known = |c: &str| contexts.get(c).map_or(0, |s| s.len()) >= THRESHOLD;
    let mut goals = Vec::new();
    let mut goal = |name: &str, missing: Vec<String>| {
        goals.push(Goal {
            goal: name.to_string(),
            ok: missing.is_empty(),
            missing,
        })
    };
    let need = |ok: bool, what: &str| (!ok).then(|| what.to_string());
    goal("constructions", unlearnt);
    // D10: no path, only reachability. Some tool that scrapes (a knife,
    // pumice, a chisel), and more than one spell lying in wait, so a first
    // release can come in more than one place and order.
    let tool = ["knife", "pumice", "penknife", "mason_chisel", "graver"]
        .iter()
        .any(|k| reach.item(k));
    goal(
        "tool",
        need(tool, "no scraping tool reachable")
            .into_iter()
            .collect(),
    );
    let latent: Vec<usize> = (0..writing.count(w))
        .filter(|&t| {
            writing.text(w, t).kind == scraped_lang::corpus::Kind::Potent
                && !writing.scraped.contains(&t)
                && writing
                    .surface_of(t)
                    .and_then(|s| writing.top_unscraped(s, &writing.scraped))
                    == Some(t)
        })
        .collect();
    let reachable_latent = latent.iter().filter(|&&t| reach.text(t)).take(2).count();
    goal(
        "latent",
        need(
            reachable_latent >= 2,
            "fewer than two latent spells reachable",
        )
        .into_iter()
        .collect(),
    );
    // Writing a claim: the stylus, and some potent verb and subject the
    // player can have met often enough, in a construction they have seen.
    let verbs = ["open", "burn", "break"];
    let table = scraped_sim::writing::Table::get();
    let claimable = verbs.iter().any(|v| {
        known(v)
            && contexts
                .keys()
                .any(|s| known(s) && table.effect(v, s, false).is_some())
    });
    goal(
        "first_write",
        [
            need(reach.item("stylus"), "stylus unreachable"),
            need(claimable, "no potent verb and subject met in enough texts"),
            need(
                newest_potent,
                "no potent writing in the newest era to copy the form from",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    // Powers (D09): every quality a great inscription pushes is also seen
    // pushed somewhere a player can reach, by another live spell, so its
    // effect can be met and its words learnt before the great one matters.
    let live = writing.live_claims(w, &site.land, &writing.scraped);
    let unattested: Vec<String> = site
        .greats
        .iter()
        .filter_map(|g| live.iter().find(|c| c.text == g.text))
        .filter(|g| {
            !live.iter().any(|c| {
                c.text != g.text && c.property == g.property && reach.structure(c.structure)
            })
        })
        .map(|g| format!("{} pushed only by a great inscription", g.property.name()))
        .collect();
    goal("powers", unattested);
    // A great inscription, and the first scraper.
    let great = site.greats.iter().any(|g| reach.text(g.text));
    goal(
        "great",
        [
            need(reach.item("graver"), "first scraper unreachable"),
            need(great, "no great inscription reachable"),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    // The deepest text, and the first lens.
    let deepest = writing.deep.len() >= 2 && reach.text(writing.deep[0]);
    goal(
        "deepest",
        [
            need(reach.item("loupe"), "first lens unreachable"),
            need(deepest, "deepest text unreachable or missing"),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    // Leaving: the departure words met in the deepest text, and enough
    // words shared between the oldest and newest languages to carry them
    // across.
    let mut deep_contexts: BTreeMap<String, usize> = BTreeMap::new();
    for &d in &writing.deep {
        let t = writing.text(w, d);
        let mut roots = BTreeSet::new();
        concepts_of(&t.meaning, &mut roots, &w.languages[0].numerals);
        for r in roots {
            *deep_contexts.entry(r).or_default() += 1;
        }
    }
    let bridge = match (by_era.get(&0), by_era.get(&newest)) {
        (Some(a), Some(b)) if newest > 0 => a.intersection(b).count(),
        (Some(a), _) if newest == 0 => a.len(),
        _ => 0,
    };
    goal(
        "leaving",
        [
            need(
                ["self", "depart"]
                    .iter()
                    .all(|c| deep_contexts.get(*c).copied().unwrap_or(0) >= THRESHOLD),
                "departure words not met in enough deep texts",
            ),
            need(
                bridge >= BRIDGE,
                "too few words shared between oldest and newest writing",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    );
    // A way out without writing (D10): the world's rim, walked to.
    goal(
        "edge",
        need(site.edge.is_some(), "no rim a walker can reach")
            .into_iter()
            .collect(),
    );
    // Sealed places (D11): each ward stone stands where a walker can reach
    // it, and the words to open it are met in enough readable texts, given
    // the lenses and the places before it opened. The chain is walked in
    // order, so it has no loop.
    let roots_of = |id: usize| {
        let t = writing.text(w, id);
        let mut roots = BTreeSet::new();
        concepts_of(
            &t.meaning,
            &mut roots,
            &w.languages[t.era as usize].numerals,
        );
        roots
    };
    let lens = reach.item("lens") || reach.item("loupe");
    let loupe = reach.item("loupe");
    let mut known = contexts.clone();
    let learn = |known: &mut BTreeMap<String, BTreeSet<usize>>, id: usize| {
        for r in roots_of(id) {
            known.entry(r).or_default().insert(id);
        }
    };
    let ward_layers: BTreeSet<usize> = writing
        .sealed
        .iter()
        .flat_map(|x| [Some(x.ward), x.clue, x.inside])
        .flatten()
        .collect();
    // What lies open from the start: accounts in ordinary buildings, and
    // the oldest under the first ward (the first lens reads it).
    let free: Vec<usize> = writing
        .sealed_texts
        .iter()
        .map(|t| t.id)
        .filter(|id| !ward_layers.contains(id))
        .collect();
    for id in free {
        let t = writing.text(w, id);
        let ok = if t.room.is_none() {
            loupe && reach.room(t.structure, None)
        } else {
            reach.text(id)
        };
        if ok {
            learn(&mut known, id);
        }
    }
    let mut missing: Vec<String> = Vec::new();
    if writing.sealed.is_empty() {
        missing.push("no sealed place".into());
    }
    for (k, x) in writing.sealed.iter().enumerate() {
        if !reach.room(x.structure, None) {
            missing.push(format!("ward {k} unreachable"));
            break;
        }
        let strong = (x.degree >= 3).then_some("greatly");
        for root in ["open", x.noun].into_iter().chain(strong) {
            if known.get(root).map_or(0, BTreeSet::len) < THRESHOLD {
                missing.push(format!("ward {k}: {root} too rarely met"));
            }
        }
        if !missing.is_empty() {
            break;
        }
        reach.opened.insert(x.structure);
        reach.routes.remove(&x.structure);
        if let Some(c) = x.clue.filter(|_| lens) {
            learn(&mut known, c);
        }
        if let Some(i) = x.inside.filter(|&i| reach.text(i)) {
            learn(&mut known, i);
        }
    }
    goal("sealed", missing);
    // Ambiguity: roots of the newest era that write the same as another.
    let lang = &w.languages[newest as usize];
    let mut forms: BTreeMap<String, usize> = BTreeMap::new();
    let roots: Vec<String> = scraped_lang::concepts::all()
        .iter()
        .filter(|c| lang.lexicon.has(&c.id))
        .map(|c| {
            let ipa = lang.phonology.to_ipa(&lang.lexicon.root(&c.id));
            lang.script
                .spell(&ipa)
                .iter()
                .map(scraped_lang::parse::glyph_sym)
                .collect::<Vec<_>>()
                .join(".")
        })
        .collect();
    for f in &roots {
        *forms.entry(f.clone()).or_default() += 1;
    }
    let shared = roots.iter().filter(|f| forms[*f] > 1).count();
    let concepts = contexts.len();
    let metrics = Metrics {
        texts,
        concepts,
        density: contexts.values().map(|s| s.len()).sum::<usize>() as f64 / concepts.max(1) as f64,
        ambiguity: shared as f64 / roots.len().max(1) as f64,
        anchors,
        bridge,
    };
    let ok = goals.iter().all(|g| g.ok) && within(preset, &metrics);
    Report {
        seed: w.seed,
        preset: preset.to_string(),
        ok,
        goals,
        metrics,
    }
}

/// Whether a world's metrics sit in its preset's band.
// DESIGN-Q: every preset needs some footholds (8 anchors) and evidence
// (density of 2); gentle worlds need more (3) and little ambiguity (under
// 5%).
pub fn within(preset: &str, m: &Metrics) -> bool {
    let (density, ambiguity) = match preset {
        "gentle" => (3.0, 0.05),
        _ => (2.0, 0.25),
    };
    m.anchors >= 8 && m.density >= density && m.ambiguity <= ambiguity
}

/// How many derived seeds to try before giving up.
pub const TRIES: u64 = 24;

/// The seed to use for a requested one: itself if its world is fair, else
/// the first fair seed derived from it. Deterministic.
pub fn fair_seed(seed: u64, preset: &str) -> u64 {
    for k in 0..TRIES {
        let s = if k == 0 {
            seed
        } else {
            hash(&[seed, 0xfa1e, k]) % 1_000_000_000
        };
        if check(&Site::create(s, preset, None), preset).ok {
            return s;
        }
    }
    seed
}
