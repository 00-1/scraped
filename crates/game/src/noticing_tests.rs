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
