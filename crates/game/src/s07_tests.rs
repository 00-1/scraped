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

/// Chains: one reply per step, headed by its command; a failure stops it.
#[test]
fn chains_run_in_order_and_stop_when_something_happens() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.step("go worn temple");
    let o = g.step("examine the stele, the basin and the signet");
    assert_eq!(o.steps.len(), 3, "{}", o.text);
    assert_eq!(o.steps[1].command, "examine the basin");
    let o = g.step("examine the moon and look");
    assert_eq!(o.steps.len(), 1, "{}", o.text);
    assert!(o.text.contains("look"), "says what wasn't done: {}", o.text);
    // A part with no verb takes the one before; `it` is the last thing.
    assert_eq!(
        crate::chain_of("take the knife and the rope"),
        ["take the knife", "take the rope"]
    );
    assert_eq!(crate::chain_of("write ka and ru on wall").len(), 1);
}

/// The sweep goes over a place step by step and says when it's done.
#[test]
fn the_sweep_runs_out() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.step("go worn temple");
    let mut done = false;
    for _ in 0..20 {
        if g.step("search").text.contains("gone over") {
            done = true;
            break;
        }
    }
    assert!(done);
}

/// `read X fully` gives every page, one part each; `examine all` acts on
/// each thing here.
#[test]
fn studying_fully_and_all() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.step("go worn temple");
    let o = g.step("read stele fully");
    assert!(o.steps.len() >= 2, "{}", o.text);
    assert!(g.state.reading.is_none() || o.text.contains("all of it"));
    let all = g.step("examine all").text;
    assert!(all.contains("stele") && all.contains("basin"), "{all}");
}

/// Pieces of writing on one thing are studied one at a time, with no
/// separator; the first glance says how the others look.
#[test]
fn pieces_of_writing_are_studied_apart() {
    let mut found = false;
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.start();
        g.step("none");
        let Some(t) = (0..g.site.things.len()).find(|&t| g.layers_seen(t).len() >= 2) else {
            continue;
        };
        g.state.place = g.site.things[t].home;
        g.state.pos = g.site.things[t].pos;
        g.forced = Some(("clear", "daylight"));
        let kind = g.site.things[t].kind;
        let a = g.step(&format!("read {kind} closely")).text;
        assert!(!a.contains('—'), "{a}");
        let b = g.step("study the fainter marks").text;
        assert_ne!(a, b);
        assert_eq!(g.piece(t), 1);
        found = true;
        break;
    }
    assert!(found, "no thing with two pieces on seeds 1, 42, 9001");
}

/// Small payoffs (S07 §7): every world has one near the start, and the
/// verb that fits it gives something, once.
#[test]
fn small_payoffs_give_something() {
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.start();
        g.step("none");
        let p = g.payoffs.clone();
        assert!(!p.is_empty(), "seed {seed}: no payoffs");
        let first = p
            .iter()
            .min_by(|a, b| {
                let d =
                    |x: &crate::actions::Payoff| g.site.things[x.thing].pos.dist(g.site.start());
                d(a).partial_cmp(&d(b)).unwrap()
            })
            .copied()
            .unwrap();
        assert!(
            g.site.things[first.thing].pos.dist(g.site.start()) <= 2000.0,
            "seed {seed}: none near the start"
        );
        g.state.place = g.site.things[first.thing].home;
        g.state.pos = g.site.things[first.thing].pos;
        g.make_item("knife", true);
        let verb = crate::actions::KINDS
            .iter()
            .find(|k| k.0 == first.kind)
            .unwrap()
            .2[0];
        let before = g.state.made.len();
        let o = g.act(verb, crate::Target::Thing(first.thing));
        assert!(
            g.state.payoffs_done.contains(&first.thing),
            "seed {seed}: {}",
            o.text
        );
        assert!(g.state.made.len() > before);
        // Once only.
        let again = g.act(verb, crate::Target::Thing(first.thing)).text;
        assert_ne!(again, o.text);
    }
}

/// Obvious next steps (S07 §5) are understood and answered in the world.
#[test]
fn obvious_verbs_are_understood() {
    let mut g = Game::new(42, pack());
    g.start();
    g.step("none");
    g.step("go worn temple");
    g.step("take drum");
    for cmd in [
        "play drum",
        "pray",
        "put drum on stele",
        "break stele",
        "unlock door with drum",
    ] {
        let t = g.step(cmd).text;
        assert!(!t.contains("don't know how"), "{cmd}: {t}");
        assert!(!t.is_empty(), "{cmd}");
    }
}
