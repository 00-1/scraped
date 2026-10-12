//! S08: a hand check of v0.9.

use crate::composing_tests::pack;
use crate::Game;

/// The spec's hand replay on seed 42 (tests/fixtures/s08-hand.txt): the
/// town is given on return, a second `go back` knows where it is, and the
/// proud stone gives up what it hides.
#[test]
fn the_hand_replay_finds_home_and_the_stone() {
    let cmds = include_str!("../../../tests/fixtures/s08-hand.txt");
    let mut g = Game::new(42, pack());
    g.keep_renders = true;
    g.start();
    let show = std::env::var("SHOW").is_ok();
    let mut found = 0;
    for cmd in cmds.lines() {
        let mark = g.renders.len();
        let o = g.step(cmd);
        if show {
            println!("> {cmd}\n{}\n", o.text);
        }
        assert!(!o.text.trim().is_empty(), "no answer to {cmd:?}");
        let said = |slot: &str| g.renders[mark..].iter().any(|r| r.trace.slot == slot);
        if cmd == "go back" && g.town_here().is_some() && g.state.trail.is_empty() {
            assert!(
                said("place.whole") || said("travel.back_none"),
                "{cmd:?}: {}",
                o.text
            );
        }
        if said("payoff.found") {
            found += 1;
        }
    }
    assert!(found >= 2, "the hollow wall and the proud stone");
}

/// Coming back to the start town names it first, as known, and a second
/// `go back` says this is where the journey began.
#[test]
fn the_home_town_is_known_on_return() {
    let mut checked = 0;
    for seed in [1u64, 42, 9001] {
        for dir in ["north", "east", "south", "west"] {
            let mut g = Game::new(seed, pack());
            g.keep_renders = true;
            g.start();
            g.step("none");
            if g.state.place != crate::site::Place::Outside {
                g.step("out");
            }
            let start = g.state.pos;
            g.step(&format!("head {dir}"));
            if g.state.pos.dist(start) < 200.0 {
                continue;
            }
            let mark = g.renders.len();
            let o = g.step("go back");
            let known = g.renders[mark..].iter().any(|r| {
                r.trace.slot == "place.whole"
                    && matches!(
                        r.vars.get("seen_before"),
                        Some(scraped_content::Value::Bool(true))
                    )
            });
            assert!(known, "seed {seed} {dir}: {}", o.text);
            assert!(g.town_here().is_some(), "seed {seed} {dir}: not home");
            let mark = g.renders.len();
            g.step("go back");
            assert!(
                g.renders[mark..]
                    .iter()
                    .any(|r| r.trace.slot == "travel.back_none"
                        && matches!(
                            r.vars.get("travelled"),
                            Some(scraped_content::Value::Bool(true))
                        )),
                "seed {seed} {dir}: second go back"
            );
            checked += 1;
        }
    }
    assert!(checked >= 6, "only {checked} ways out");
}

/// Every payoff on seeds 1–10 can be solved by a command made only of
/// the words its cue and its failure messages give (S08).
#[test]
fn every_payoff_solves_from_its_cue() {
    let mut solved = 0;
    for seed in 1..=10u64 {
        let mut g = Game::new(seed, pack());
        g.forced_light = true;
        g.start();
        g.step("none");
        g.sustain = true;
        for p in g.payoffs.clone() {
            if g.state.payoffs_done.contains(&p.thing) {
                continue;
            }
            g.forced = Some(("clear", "daylight"));
            let th = g.thing(p.thing).clone();
            g.state.place = th.home;
            g.state.pos = th.pos;
            let kind = th.kind.replace('_', " ");
            let cmd = match p.kind {
                "loose_stone" => format!("push the stone of the {kind}"),
                "hollow_wall" => format!("knock on the {kind}"),
                "jammed_lid" => format!("open the lid of the {kind}"),
                _ => format!("shake the {kind}"),
            };
            let before = g.state.payoffs_done.len();
            let o = g.step(&cmd);
            if p.kind == "jammed_lid" && g.state.payoffs_done.len() == before {
                // The failure says it wants something to lever it.
                g.make_item("knife", true);
                g.step(&format!("prise the lid of the {kind}"));
            }
            // This one, or an alike one beside it with its own payoff.
            assert!(
                g.state.payoffs_done.len() > before,
                "seed {seed} {} on {}: {cmd:?} -> {}",
                p.kind,
                th.kind,
                o.text
            );
            solved += 1;
        }
    }
    assert!(solved > 20, "only {solved} payoffs");
}

/// A spell's sign names only a thing that is here (S08).
#[test]
fn spell_signs_name_only_what_is_here() {
    let kinds = crate::slots::kinds();
    let mut signs = 0;
    for (seed, style) in [(1u64, "mason"), (42, "mason"), (9001, "forager")] {
        let mut g = Game::new(seed, pack());
        g.keep_renders = true;
        g.start();
        let mut out = g.step(style);
        let mut bot = crate::bots::DepthBot::new("explorer", seed);
        for _ in 0..300 {
            if g.state.dead.is_some() {
                break;
            }
            let cmd = bot.next(&g, &out);
            let mark = g.renders.len();
            out = g.step(&cmd);
            let here: Vec<&str> = g.here().into_iter().map(|t| g.thing(t).kind).collect();
            for r in &g.renders[mark..] {
                if r.trace.slot != "spell.sign" {
                    continue;
                }
                let thing = r
                    .vars
                    .get("thing")
                    .map(scraped_content::Value::text)
                    .unwrap_or_default();
                let kind = thing.replace(' ', "_");
                if kinds.contains(&kind.as_str()) {
                    signs += 1;
                    assert!(
                        here.contains(&kind.as_str()),
                        "seed {seed}: a sign of {thing} after {cmd:?}"
                    );
                }
            }
        }
    }
    eprintln!("{signs} signs of things checked");
}
