//! M13 properties: coverage is deterministic, every render links back to
//! the variant that made it, and hot reload changes text only.

use scraped_content::{Pack, Renderer};
use scraped_lang::slots::LangHooks;

use crate::composing_tests::pack;
use crate::coverage::{play, run, Bot};
use crate::Game;

#[test]
fn coverage_is_deterministic_and_ranked() {
    let a = run(&pack(), &[1, 42], 80);
    let b = run(&pack(), &[1, 42], 80);
    assert_eq!(a, b);
    assert!(a.hours > 1.0);
    assert!(a.slots.len() > 20);
    assert!(a.slots.windows(2).all(|w| w[0].gap_hours >= w[1].gap_hours));
    // Every registered slot is either reached or listed as never reached.
    let reg = crate::slots::registry_for(&pack());
    for s in &reg.slots {
        assert!(
            a.never.contains(&s.id) || a.slots.iter().any(|c| c.slot == s.id),
            "{}",
            s.id
        );
    }
}

/// Renders a slot again with only the variant the trace names.
fn again(p: &Pack, g: &Game, r: &crate::Rendered) -> String {
    let (file, index) = (r.trace.file.clone().unwrap(), r.trace.variant.unwrap());
    let mut only = p.clone();
    for f in &mut only.files {
        let keep: Vec<_> = f
            .variants
            .iter()
            .enumerate()
            .filter(|(i, v)| v.slot != r.trace.slot || (f.path == file && *i == index))
            .map(|(_, v)| v.clone())
            .collect();
        f.variants = keep;
    }
    let reg = crate::slots::registry_for(&only);
    let hooks = LangHooks {
        lang: &g.site.world.languages[0],
    };
    let mut rr = Renderer::new(&reg, &only, r.seed, &hooks);
    rr.render(&r.trace.slot, &r.vars)
}

#[test]
fn every_line_links_to_the_variant_that_made_it() {
    let p = pack();
    let mut slots = std::collections::BTreeSet::new();
    for (seed, bot) in [(1u64, "wanderer"), (42, "scholar"), (7, "scholar")] {
        let mut g = Game::new(seed, p.clone());
        g.trace = true;
        let mut out = g.start();
        let mut b = Bot::new(bot, seed);
        b.prepare(&mut g);
        for _ in 0..60 {
            if g.state.dead.is_some() {
                break;
            }
            // Every line comes from some render (glyph numbers and the
            // rule between pieces of writing aside).
            for line in out.text.lines().filter(|l| !l.trim().is_empty()) {
                let made = out
                    .renders
                    .iter()
                    .any(|r| r.trace.text.contains(line.trim()))
                    || line
                        .chars()
                        .all(|c| c.is_ascii_digit() || " /—".contains(c));
                assert!(made, "seed {seed}: no render made {line:?}");
            }
            for r in &out.renders {
                slots.insert(r.trace.slot.clone());
                if r.trace.variant.is_none() || r.trace.depth > 0 || r.vars.is_empty() {
                    continue;
                }
                assert_eq!(again(&p, &g, r), r.trace.text, "{}", r.trace.slot);
            }
            let cmd = b.next(&g, &out);
            out = g.step(&cmd);
        }
    }
    assert!(slots.len() > 25, "only {} slots exercised", slots.len());
}

#[test]
fn hot_reload_changes_text_never_state() {
    let p = pack();
    let mut changed = p.clone();
    for f in &mut changed.files {
        for v in &mut f.variants {
            v.text = format!("[changed] {}", v.text);
        }
        for s in &mut f.storylets {
            // Rules changed after the world was made must not apply.
            s.at = "anywhere".to_string();
            s.effects = vec!["give torch".to_string()];
        }
    }
    let script: Vec<String> = {
        let mut g = Game::new(3, p.clone());
        let mut out = g.start();
        let mut b = Bot::new("scholar", 3);
        b.prepare(&mut g);
        (0..60)
            .map(|_| {
                let c = b.next(&g, &out);
                out = g.step(&c);
                c
            })
            .collect()
    };
    let replay = |swap: bool| {
        let mut g = Game::new(3, p.clone());
        g.start();
        Bot::new("scholar", 3).prepare(&mut g);
        let mut last = String::new();
        for (i, c) in script.iter().enumerate() {
            if swap && i == 20 {
                g.set_pack(changed.clone());
            }
            last = g.step(c).text;
        }
        (serde_json::to_string(&g.state).unwrap(), last)
    };
    let (a, text_a) = replay(false);
    let (b, text_b) = replay(true);
    assert_eq!(a, b, "a hot reload changed the game's state");
    assert_ne!(text_a, text_b, "and the text did change");
    assert!(text_b.contains("[changed]"));
    let _ = play;
}
