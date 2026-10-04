//! M11 properties: every ending can be reached, the deepest text holds the
//! words for leaving, the summary agrees with the simulation, the
//! chronicle parses back, and legacy appears only when asked for.

use scraped_lang::meaning::{Argument, Clause, Mood, NounPhrase, Polarity, Role, Sentence, Tense};
use scraped_lang::parse::{Mode, Parser};
use scraped_sim::body::Activity;
use scraped_sim::region::{LIFE, STABILITY};
use scraped_sim::writing::{ghosts_of, visible_of};

use crate::composing_tests::{at_blank_surface, glyphs_for, pack};
use crate::ending::{chronicle_of, Legacy, OLD_AGE};
use crate::site::Place;
use crate::trajectory::START_AGE;
use crate::{Game, Target};

const SEEDS: [u64; 5] = [1, 3, 7, 42, 99];

fn self_claim(negative: bool) -> Sentence {
    Sentence::Clause(Clause {
        predicate: "depart".to_string(),
        mood: Mood::Potent,
        tense: Tense::NonPast,
        polarity: if negative {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args: vec![Argument {
            role: Role::Subject,
            np: NounPhrase::concept("self"),
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

/// The thing carrying the deepest text (beneath the root, or beneath its
/// recopy where the root can't be reached).
fn root_thing(g: &Game) -> usize {
    let root = g.site.writing.deep[0];
    (0..g.site.things.len())
        .find(|&t| g.site.things[t].texts.contains(&root))
        .expect("the root is on a thing")
}

/// Reads the deepest text through the first lens, where it lies.
fn read_the_deepest(g: &mut Game) {
    let t = root_thing(g);
    let back = (g.state.place, g.state.pos);
    g.state.place = g.site.things[t].home;
    g.state.pos = g.site.things[t].pos;
    g.make_item("first_lens", true);
    g.make_item("firesteel", true);
    let torch = g.make_item("torch", true).unwrap();
    g.light_item(torch);
    g.act("read", Target::Thing(t));
    (g.state.place, g.state.pos) = back;
}

/// Writes a claim on a blank surface and scrapes it once dry.
fn cast(g: &mut Game, thing: usize, m: &Sentence) -> bool {
    let name = g.site.things[thing].kind;
    let words = glyphs_for(g, m);
    let out = g.step(&format!("write {words} on {name}"));
    if out.state.wrote.as_ref().is_none_or(|w| !w.accepted) {
        return false;
    }
    g.advance(40, Activity::Resting);
    g.scrape(thing);
    true
}

#[test]
fn the_deepest_text_is_deepest_and_needs_the_first_lens() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.forced = Some(("clear", "daylight"));
        g.start();
        let w = &g.site.writing;
        assert!(w.deep.len() >= 2, "seed {seed}: no deep stack");
        let t = root_thing(&g);
        let own = g.layers(t).len();
        for (i, s) in w.surfaces.iter().enumerate() {
            if !s.layers.contains(&w.deep[0]) {
                assert!(s.layers.len() < own, "seed {seed}: surface {i} as deep");
            }
        }
        assert!(
            g.site.fixtures.items.iter().any(|p| p.kind == "first_lens"),
            "seed {seed}: no first lens"
        );
        // By eye and through the lens, no account shows.
        g.state.place = g.site.things[t].home;
        g.state.pos = g.site.things[t].pos;
        let deep = g.site.writing.deep.clone();
        g.make_item("lens", true);
        assert!(g.layers_seen(t).iter().all(|(x, _)| !deep.contains(x)));
        g.make_item("first_lens", true);
        let seen: Vec<usize> = g.layers_seen(t).iter().map(|x| x.0).collect();
        assert!(deep.iter().all(|d| seen.contains(d)), "seed {seed}");
    }
}

#[test]
fn reading_the_deepest_text_is_enough_to_write_the_departure() {
    for seed in SEEDS {
        let (mut g, thing) = at_blank_surface(seed).expect("a blank wall");
        read_the_deepest(&mut g);
        for c in ["self", "depart"] {
            assert!(
                g.state.encountered.get(c).map_or(0, |s| s.len()) >= 2,
                "seed {seed}: {c} met too rarely"
            );
        }
        assert!(cast(&mut g, thing, &self_claim(false)), "seed {seed}");
        assert_eq!(g.ending(), Some("left"), "seed {seed}");
        let out = g.step("look");
        assert!(out.state.dead.is_some());
    }
}

#[test]
fn the_departure_cannot_be_written_without_the_deepest_text() {
    let (mut g, thing) = at_blank_surface(42).expect("a blank wall");
    assert!(!cast(&mut g, thing, &self_claim(false)));
    assert_eq!(g.ending(), None);
}

#[test]
fn every_ending_is_reachable() {
    // Leaving and writing yourself in.
    let (mut g, thing) = at_blank_surface(7).expect("a blank wall");
    read_the_deepest(&mut g);
    assert!(cast(&mut g, thing, &self_claim(true)));
    assert_eq!(g.ending(), Some("written_in"));
    // Old age.
    let mut g = Game::new(42, pack());
    g.start();
    g.state.minutes = (OLD_AGE - START_AGE) * 360 * 1440 - 60;
    g.state.regions.day = g.day();
    g.step("wait 2 hours");
    assert_eq!(g.ending(), Some("old_age"));
    // Overtaken by collapse.
    let mut g = Game::new(42, pack());
    g.start();
    let r = g.site.regions.at(g.state.pos).expect("a region");
    g.state.regions.vars[r][STABILITY] = 20;
    g.state.regions.vars[r][LIFE] = 20;
    g.step("wait 1 hour");
    assert_eq!(g.ending(), Some("overtaken"));
    // Death.
    let mut g = Game::new(42, pack());
    g.start();
    for _ in 0..20 {
        g.step("wait 1 day");
    }
    assert_eq!(g.ending(), Some("death"));
    // No start region is already collapsing.
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        let r = g.site.regions.at(g.state.pos).unwrap();
        let v = g.state.regions.vars[r];
        assert!(v[STABILITY] >= 200 || v[LIFE] >= 200, "seed {seed}");
    }
}

#[test]
fn the_summary_agrees_with_the_simulation() {
    let mut checked = 0;
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        if g.site.greats.is_empty() {
            continue;
        }
        if !crate::trajectory_tests::counter(&mut g, 0) {
            continue;
        }
        g.state.minutes += 180 * 1440;
        g.step_regions();
        g.end_run("left");
        let rec = g.record();
        assert_eq!(rec.ending, "left");
        // (A great inscription already covered by fresh writing pushed
        // nothing to begin with.)
        if rec.acts.is_empty() {
            continue;
        }
        assert!(rec.releases.iter().any(|r| r.scraped_by == "you"));
        for r in &rec.regions {
            assert_eq!(r.end, g.state.regions.vars[r.region]);
            assert_eq!(r.start, g.site.regions.initial.vars[r.region]);
            for c in &r.changes {
                match c.cause.as_str() {
                    "world" => assert_eq!(c.after, c.without),
                    "you" => assert_eq!(c.before, c.without),
                    _ => assert!(c.after != c.without && c.before != c.without),
                }
            }
        }
        // Left alone, the same world gives no "you" at all.
        let mut alone = Game::new(seed, pack());
        alone.start();
        alone.state.minutes = g.state.minutes;
        alone.step_regions();
        alone.end_run("left");
        let rec = alone.record();
        assert!(rec.acts.is_empty());
        assert!(rec
            .regions
            .iter()
            .flat_map(|r| &r.changes)
            .all(|c| c.cause == "world"));
        // One summary line per kind of change, or the calm line; one per
        // act; the chronicle's frame and glyphs; the frame.
        let rec = g.record();
        let mut kinds: Vec<_> = rec
            .regions
            .iter()
            .flat_map(|r| &r.changes)
            .map(|c| (&c.aspect, &c.before, &c.after, &c.without, &c.cause))
            .collect();
        kinds.sort();
        kinds.dedup();
        let changes = kinds.len();
        let parts = g.summary_parts();
        assert_eq!(
            parts.len(),
            1 + changes.max(1) + g.state.acts.len() + 2,
            "seed {seed}"
        );
        checked += 1;
    }
    assert!(checked >= 3, "only {checked} seeds checked");
}

#[test]
fn the_chronicle_parses_back() {
    for seed in [1u64, 42, 9001] {
        for ending in ["death", "left", "written_in"] {
            let mut g = Game::new(seed, pack());
            g.start();
            g.step("wait 3 days");
            if g.ending().is_none() {
                g.end_run(ending);
            }
            let mut rec = g.record();
            // An eventful run too: something changed by the player's hand.
            for vary in [false, true] {
                if vary {
                    for r in rec.regions.iter_mut().take(1) {
                        r.end[LIFE] = r.without[LIFE] - 300;
                        r.end[STABILITY] = r.without[STABILITY] + 200;
                    }
                }
                let m = chronicle_of(&rec, |_| None);
                let era = g.writing_era();
                let r = g.site.world.renderer(era as u32);
                let p = Parser::without_names(&r, Mode::Phonemes);
                let words = r.render(&m).words;
                let parsed = p.sentence(&p.tokens(&words));
                assert_eq!(parsed.as_ref(), Some(&m), "seed {seed} {ending}");
            }
        }
    }
}

#[test]
fn legacy_appears_only_when_enabled() {
    let (mut g, thing) = at_blank_surface(3).expect("a blank wall");
    read_the_deepest(&mut g);
    assert!(cast(&mut g, thing, &self_claim(false)));
    let legacy = g.legacy().expect("a final inscription");
    assert_eq!(legacy.meaning, self_claim(false));
    let json = serde_json::to_string(&legacy).unwrap();
    let back: Legacy = serde_json::from_str(&json).unwrap();
    assert_eq!(back, legacy);
    for seed in SEEDS {
        let plain = Game::new(seed, pack());
        assert!(plain.site.writing.legacy.is_none());
        let n = plain.site.writing.count(&plain.site.world)
            - plain.placed.iter().filter(|p| p.text.is_some()).count();
        let with = Game::with_legacy(seed, pack(), Some(legacy.clone()));
        let id = with.site.writing.legacy.expect("placed");
        assert_eq!(with.text(id).meaning, legacy.meaning);
        assert_eq!(id, n, "seed {seed}: legacy comes after history's texts");
        // A ghost: older than everything on its surface, under a cast.
        let t = (0..with.site.things.len())
            .find(|&t| with.site.things[t].texts.contains(&id))
            .expect("on a thing");
        let layers = with.layers(t);
        assert_eq!(layers[0], id);
        assert!(ghosts_of(&layers, &with.state.scraped) > 0);
        assert!(visible_of(&layers, &with.state.scraped)
            .iter()
            .all(|x| x.0 != id));
        // The legacy world is the same world otherwise.
        assert_eq!(plain.site.things.len(), with.site.things.len());
        // Saves carry it.
        let mut g = Game::with_legacy(seed, pack(), Some(legacy.clone()));
        g.start();
        g.step("look");
        let (loaded, _) = Game::load(&g.save(), pack());
        assert_eq!(loaded.site.writing.legacy, Some(id));
    }
    // The ending has no inscription of its own when nothing was written.
    let mut g = Game::new(42, pack());
    g.start();
    g.end_run("old_age");
    assert!(g.legacy().is_none());
}

#[test]
fn notebook_holds_the_run() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("look");
    g.step("wait 1 hour");
    g.end_run("left");
    let n = g.notebook();
    assert_eq!(n.transcript.len(), 2);
    assert_eq!(n.transcript[0].0, "look");
    let (transcript, places) = g.notebook_files();
    assert!(transcript.contains("> wait 1 hour"));
    assert!(!places.is_empty());
    let _ = Place::Outside;
}
