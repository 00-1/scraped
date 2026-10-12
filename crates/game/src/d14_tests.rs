//! D14: finding your way.

use crate::composing_tests::pack;
use crate::Game;

/// The same command from the same place, in the same conditions, gives
/// the same result; a place's signature is the same every visit and
/// across a save.
#[test]
fn heading_is_reproducible_and_signatures_stable() {
    for seed in [1u64, 42, 9001] {
        let run = || {
            let mut g = Game::new(seed, pack());
            g.start();
            g.step("none");
            g.forced = Some(("clear", "daylight"));
            let a = g.step("head north").text;
            (a, g.state.pos, g.signature(g.state.pos))
        };
        assert_eq!(run(), run(), "seed {seed}");
        let mut g = Game::new(seed, pack());
        g.start();
        g.step("none");
        g.step("head south");
        let sig = g.signature(g.state.pos);
        let (again, _) = Game::load(&g.save(), pack());
        assert_eq!(again.signature(again.state.pos), sig);
    }
}

/// A barrier is seen coming before it bars the way.
#[test]
fn barriers_are_seen_before_they_block() {
    let mut checked = 0;
    for seed in [1u64, 42, 9001, 7, 3] {
        for dir in ["north", "east", "south", "west"] {
            let mut g = Game::new(seed, pack());
            g.start();
            g.step("none");
            g.forced = Some(("clear", "daylight"));
            g.keep_renders = true;
            let mut approached = false;
            for _ in 0..4 {
                let mark = g.renders.len();
                g.step(&format!("head {dir}"));
                let slots: Vec<String> = g.renders[mark..]
                    .iter()
                    .map(|r| r.trace.slot.clone())
                    .collect();
                if slots.iter().any(|s| s == "travel.approach") {
                    approached = true;
                }
                if slots.iter().any(|s| s == "travel.blocked") {
                    assert!(approached, "seed {seed} {dir}: blocked without warning");
                    checked += 1;
                    break;
                }
            }
        }
    }
    assert!(checked > 0, "no barrier met");
}

/// `go back` retraces a route of several legs to where it began.
#[test]
fn go_back_retraces_several_legs() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.forced = Some(("clear", "daylight"));
    let start = g.state.pos;
    for d in ["head north", "head west", "head north"] {
        g.step(d);
    }
    for _ in 0..4 {
        g.step("go back");
    }
    assert!(
        g.state.pos.dist(start) <= 600.0,
        "{} m from the start",
        g.state.pos.dist(start)
    );
}

/// `go back` always answers, even after blocked steps left the same point
/// twice in the trail.
#[test]
fn go_back_after_blocked_steps_answers() {
    for seed in [4u64, 6] {
        let r = crate::bots::out_and_back(&pack(), seed, Some(("fog", "daylight")));
        assert!(
            !r.transcript.contains("> go back\n\n\n"),
            "seed {seed}: an empty reply"
        );
    }
}

/// In the explorer's runs a return is never told as a first sight: once a
/// place's signature has been said, it is said as known ever after.
#[test]
fn returns_are_always_recognised() {
    let mut returns = 0;
    for seed in [1u64, 42, 7] {
        let mut g = Game::new(seed, pack());
        g.keep_renders = true;
        let mut bot = crate::bots::DepthBot::new("explorer", seed);
        let mut out = g.start();
        let mut told = std::collections::BTreeSet::new();
        for _ in 0..800 {
            if g.state.dead.is_some() {
                break;
            }
            let cmd = bot.next(&g, &out);
            let mark = g.renders.len();
            out = g.step(&cmd);
            let (key, _) = g.signature(g.state.pos);
            for r in &g.renders[mark..] {
                if r.trace.slot != "place.signature" {
                    continue;
                }
                let again = matches!(
                    r.vars.get("seen_before"),
                    Some(scraped_content::Value::Bool(true))
                );
                if told.contains(&key) {
                    returns += 1;
                    assert!(again, "seed {seed}: {key} told as new after {cmd:?}");
                }
                told.insert(key.clone());
            }
        }
    }
    assert!(returns > 0, "no returns to check");
}
