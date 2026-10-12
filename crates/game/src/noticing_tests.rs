//! D13: keenness, habit and styles.

use crate::composing_tests::pack;
use crate::noticing::Noticing;
use crate::Game;

const DAY: u32 = 1440;

#[test]
fn habit_sharpens_and_neglect_fades() {
    let mut n = Noticing::default();
    assert_eq!(n.keenness("sounds", 0), 1.0);
    for i in 0..12 {
        n.attend("sounds", i * 300);
    }
    let now = 12 * 300;
    assert!(
        n.keenness("sounds", now) > 1.7,
        "{}",
        n.keenness("sounds", now)
    );
    // A week of neglect: halfway to the floor.
    let later = n.keenness("sounds", now + 8 * DAY);
    assert!(later < 1.3 && later > 0.7, "{later}");
    // Untouched categories fade below neutral after a week, never below the floor.
    assert!(n.keenness("smells", 30 * DAY) >= 0.7);
}

#[test]
fn a_style_starts_keen_and_keeps_an_edge() {
    let n = Noticing {
        style: Some("forager".into()),
        ..Default::default()
    };
    assert!(n.keenness("life", 0) >= 1.5);
    assert!(n.keenness("life", 60 * DAY) >= 1.15);
    assert!(n.keenness("marks", 0) == 1.0);
}

/// Choosing a style at the start lists its verbs in `help`; anything else
/// starts play with no style.
#[test]
fn the_first_command_may_choose_a_style() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("forager");
    let help = g.step("help").text;
    assert!(help.contains("forage") && help.contains("snare"), "{help}");
    assert!(
        !help.contains("scrape") && !help.contains("write"),
        "{help}"
    );
    let mut g = Game::new(42, pack());
    g.start();
    let o = g.step("look").text;
    assert!(!o.is_empty());
    assert_eq!(g.noticing().style.as_deref(), Some(""));
    assert!(!g.step("help").text.contains("forage"));
}

/// Keenness is deterministic and survives save and load.
#[test]
fn noticing_survives_save_and_load() {
    let mut g = Game::new(42, pack());
    g.start();
    for c in ["scribe", "listen", "listen", "listen", "look"] {
        g.step(c);
    }
    let before = g.noticing();
    let (again, _) = Game::load(&g.save(), pack());
    assert_eq!(again.noticing(), before);
    assert!(before.attended["sounds"].len() == 3);
}

/// Two players in one world keep their own noticing.
#[test]
fn two_players_keep_their_own() {
    let mut g = Game::new(42, pack());
    g.start();
    g.who = "jb".into();
    g.step("wayfarer");
    g.step("listen");
    g.who = "partner".into();
    g.step("mason");
    let partner = g.noticing();
    g.who = "jb".into();
    let jb = g.noticing();
    assert_eq!(jb.style.as_deref(), Some("wayfarer"));
    assert_eq!(partner.style.as_deref(), Some("mason"));
    assert!(jb.attended.contains_key("sounds") && !partner.attended.contains_key("sounds"));
}

/// Every verb the parser knows has a situation that suggests it.
#[test]
fn every_verb_has_a_situation() {
    let table = include_str!("../data/verbs.toml");
    for line in table.lines() {
        if let Some(id) = line.strip_prefix("id = \"") {
            let id = id.trim_end_matches('"');
            assert!(
                crate::suggest::TABLE
                    .iter()
                    .any(|s| s.verb == id && !s.after.is_empty()),
                "no situation suggests {id}"
            );
        }
    }
}

/// Suggestions: at most twice for a situation, never after use, never
/// scraping or writing first, never reading before marks are known.
#[test]
fn suggestions_keep_their_rules() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    let said = |v: &[&str]| {
        v.iter()
            .map(|s| (s.to_string(), scraped_content::Context::new()))
            .collect::<Vec<_>>()
    };
    let mut shown = 0;
    for _ in 0..10 {
        if g.suggest(&said(&["shout.done", "room.whole"])).is_some() {
            shown += 1;
        }
    }
    // listen and shout fit room.whole: each at most twice here.
    assert!(shown <= 4, "{shown}");
    assert!(g.state.suggested.values().all(|s| s.shown <= 2));
    // Used: never again.
    g.state.used.insert("listen".into());
    g.state.suggested.clear();
    for _ in 0..5 {
        assert!(g
            .suggest(&said(&["room.whole"]))
            .is_none_or(|t| !t.contains("listen")));
    }
    // Scraping, writing and reading are not suggested first.
    g.state.suggested.clear();
    for _ in 0..5 {
        let t = g.suggest(&said(&[
            "scrape.done",
            "write.done",
            "room.mark",
            "read.more",
        ]));
        if let Some(t) = t {
            for w in ["scrape", "write", "read"] {
                assert!(!t.contains(w), "{t}");
            }
        }
    }
}

/// The world's own measures (D13): a sundial carried into the sun and a
/// water clock tell the world's parts of the day; milestones count roads.
#[test]
fn the_world_keeps_its_own_measures() {
    let mut clocks = 0;
    let mut miles = 0;
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.start();
        g.forced = Some(("clear", "daylight"));
        let parts = scraped_sim::writing::Measures::of(&g.site.world).day_parts;
        // Noon.
        g.state.minutes = g.state.minutes / 1440 * 1440 + 720;
        for kind in ["sundial", "water clock"] {
            let Some(t) = g.site.things.iter().position(|th| th.kind == kind) else {
                continue;
            };
            clocks += 1;
            g.state.carried.push(t);
            g.state.place = crate::site::Place::Outside;
            let a = g.step(&format!("examine {kind}")).text;
            let n = if kind == "sundial" { parts } else { parts * 2 };
            assert!(a.contains(&n.to_string()), "seed {seed}: {a}");
            if kind == "sundial" {
                // The middle of the day: the middle line.
                assert!(a.contains(&(parts / 2 + 1).to_string()), "seed {seed}: {a}");
            }
            for h in ["hour", "o'clock"] {
                assert!(!a.contains(h), "{a}");
            }
        }
        let ids: Vec<usize> = g
            .site
            .writing
            .surfaces
            .iter()
            .flat_map(|s| s.layers.iter().copied())
            .collect();
        miles += ids
            .into_iter()
            .filter(|&id| {
                g.site
                    .writing
                    .text(&g.site.world, id)
                    .meaning
                    .noun_phrases()
                    .iter()
                    .any(|n| {
                        matches!(&n.head, scraped_lang::meaning::Head::Concept(c) if c == "road")
                            && n.quantity.is_some()
                    })
            })
            .count();
    }
    assert!(clocks >= 6, "{clocks} clocks on three seeds");
    assert!(miles >= 3, "{miles} milestones on three seeds");
}

/// A suggestion follows from something told (S07): no fishing without
/// water mentioned.
#[test]
fn suggestions_follow_from_what_was_told() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    let none = |v: &[&str]| {
        v.iter()
            .map(|s| (s.to_string(), scraped_content::Context::new()))
            .collect::<Vec<_>>()
    };
    for _ in 0..4 {
        if let Some(t) = g.suggest(&none(&["life.seen", "land.edge"])) {
            assert!(!t.contains("fish"), "{t}");
        }
    }
    let river = crate::site::ctx(&[("kind", scraped_content::Value::from("river"))]);
    g.state.suggested.clear();
    let mut fished = false;
    for _ in 0..10 {
        if let Some(t) = g.suggest(&[("land.edge".to_string(), river.clone())]) {
            fished |= t.contains("fish");
        }
    }
    assert!(fished);
}
