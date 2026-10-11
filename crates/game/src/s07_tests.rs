//! S07: looking as a progression, say it once, take stock, recognition.

use crate::composing_tests::pack;
use crate::Game;

/// Looks never repeat a fact within a visit, every loose thing in view is
/// told by the second look, and when nothing is left the look says so.
#[test]
fn looking_is_a_progression() {
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.start();
        g.step("none");
        let into = g.step("go in").text;
        if !matches!(g.state.place, crate::site::Place::Room { .. }) {
            continue;
        }
        let mut said: Vec<String> = Vec::new();
        let mut done = false;
        for i in 0..10 {
            let t = g.step("look").text;
            let body = t.split("\n\n").next().unwrap_or("").to_string();
            if body.starts_with("Nothing more") {
                done = true;
                break;
            }
            for s in body.split(". ").filter(|s| s.len() > 3) {
                assert!(
                    !said.contains(&s.to_string()),
                    "seed {seed} look {i}: {s:?} again (entered: {into})"
                );
                said.push(s.to_string());
            }
        }
        assert!(done, "seed {seed}: looking never ran out: {said:?}");
    }
}

/// `take stock` lists what is here every time it is asked.
#[test]
fn take_stock_repeats_on_request() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.step("go worn temple");
    let a = g.step("take stock").text;
    let b = g.step("what's here").text;
    assert_eq!(a, b);
    assert!(a.contains("stele"), "{a}");
}
