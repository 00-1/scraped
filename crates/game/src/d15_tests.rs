//! D15: ways of playing.

use crate::composing_tests::pack;
use crate::Game;

fn outside(seed: u64) -> Game {
    let mut g = Game::new(seed, pack());
    g.keep_renders = true;
    g.start();
    g.step("forager");
    g.forced = Some(("clear", "daylight"));
    g
}

fn said(g: &Game, slot: &str) -> usize {
    g.renders.iter().filter(|r| r.trace.slot == slot).count()
}

/// A step is said once, when first reached, and kept.
#[test]
fn steps_are_said_once() {
    let mut g = outside(42);
    g.make_item("berries", true);
    g.state.arcs.gathered = 1;
    g.step("look");
    assert!(g.state.arcs.reached.contains("forager:first_food"));
    let n = said(&g, "arc.step");
    assert!(n >= 1);
    g.step("look");
    assert_eq!(said(&g, "arc.step"), n, "said again");
}

/// Settling ends the run at a camp with food laid up, and only there.
#[test]
fn settling_needs_a_camp_and_a_store() {
    let mut g = outside(42);
    if g.state.place != crate::site::Place::Outside {
        g.step("out");
    }
    g.step("settle");
    assert!(g.state.dead.is_none());
    assert!(said(&g, "settle.not_yet") >= 1);
    g.state.arcs.camp = Some(g.state.pos);
    for _ in 0..crate::arcs::STORE {
        g.make_item("fish", true);
    }
    let out = g.step("settle");
    assert_eq!(g.ending(), Some("settled"), "{}", out.text);
    // The end summary tells the arc's story.
    assert!(said(&g, "end.arc") >= 1);
}

/// A scavenger fed three times follows.
#[test]
fn a_fed_scavenger_follows() {
    for seed in [1u64, 42, 7, 9001, 3] {
        let mut g = outside(seed);
        let Some(i) = g
            .site
            .fixtures
            .creatures
            .iter()
            .position(|c| c.archetype == "scavenger")
        else {
            continue;
        };
        for _ in 0..crate::arcs::TAMED {
            g.state.creatures[i].seen = true;
            g.state.creatures[i].pos = g.state.pos;
            g.make_item("fish", true);
            g.step("feed the scavenger");
        }
        assert_eq!(g.state.arcs.companion.as_deref(), Some("scavenger"));
        return;
    }
    panic!("no scavenger in any world tried");
}

/// Every command a style bot gives gets an answer (D15 found a heading at
/// the land's end and a second look down answered with nothing).
#[test]
fn no_command_goes_unanswered() {
    for (style, seed) in [
        ("forager", 5u64),
        ("forager", 3),
        ("mason", 1),
        ("wayfarer", 2),
    ] {
        let r = crate::bots::play(&pack(), seed, style, 30.0, 300);
        for (i, t) in r.texts.iter().enumerate().skip(1) {
            if r.died.is_some() && i + 1 == r.texts.len() {
                continue;
            }
            assert!(
                !t.trim().is_empty(),
                "{style} {seed}: no answer to {:?}",
                r.commands.get(i - 1)
            );
        }
    }
}
