//! M14 fairness: the checker catches crafted unsolvable worlds, every
//! preset's seeds pass (after rejection), and the metrics bands hold.

use crate::fairness::{check, fair_seed, within, Metrics, GOALS};
use crate::site::Site;

fn failing(site: &Site) -> Vec<String> {
    check(site, "standard")
        .goals
        .into_iter()
        .filter(|g| !g.ok)
        .map(|g| g.goal)
        .collect()
}

#[test]
fn crafted_worlds_fail_where_they_should() {
    let base = Site::create(1, "standard", None);
    let r = check(&base, "standard");
    assert!(r.ok, "{:?}", r.goals);
    assert_eq!(r.goals.len(), GOALS.len());
    // Remove each tool in turn.
    for (tool, goal) in [
        ("scraper", "scraper"),
        ("stylus", "first_write"),
        ("first_scraper", "great"),
        ("first_lens", "deepest"),
    ] {
        let mut s = Site::create(1, "standard", None);
        s.fixtures.items.retain(|p| p.kind != tool);
        assert_eq!(failing(&s), [goal], "without the {tool}");
    }
    // No pivot inscription.
    let mut s = Site::create(1, "standard", None);
    s.writing.pivot = None;
    assert_eq!(failing(&s), ["first_release"]);
    // The deepest text missing: neither it nor leaving can be done.
    let mut s = Site::create(1, "standard", None);
    s.writing.deep.truncate(1);
    assert_eq!(failing(&s), ["deepest", "leaving"]);
    // No great inscription.
    let mut s = Site::create(1, "standard", None);
    s.greats.clear();
    assert_eq!(failing(&s), ["great"]);
}

#[test]
fn metric_bands() {
    let m = |density, ambiguity, anchors| Metrics {
        texts: 100,
        concepts: 50,
        density,
        ambiguity,
        anchors,
        bridge: 30,
    };
    assert!(within("standard", &m(2.5, 0.1, 20)));
    assert!(!within("standard", &m(1.5, 0.1, 20)));
    assert!(!within("standard", &m(2.5, 0.1, 3)));
    assert!(!within("gentle", &m(2.5, 0.0, 20)));
    assert!(!within("gentle", &m(4.0, 0.1, 20)));
    assert!(within("gentle", &m(4.0, 0.0, 20)));
}

#[test]
fn every_preset_gives_fair_worlds() {
    for preset in ["gentle", "standard", "archaeologist"] {
        let mut raw = 0;
        let all = [1u64, 2, 3, 5, 8, 13];
        // Debug builds (CI's `cargo test`) check fewer.
        let seeds = if cfg!(debug_assertions) {
            &all[..3]
        } else {
            &all[..]
        };
        for &seed in seeds {
            if check(&Site::create(seed, preset, None), preset).ok {
                raw += 1;
            }
            let fair = fair_seed(seed, preset);
            let r = check(&Site::create(fair, preset, None), preset);
            assert!(r.ok, "{preset} {seed} → {fair}: {:?}", r.goals);
            assert_eq!(fair_seed(seed, preset), fair, "deterministic");
        }
        assert!(
            raw * 3 >= seeds.len() * 2,
            "{preset}: only {raw} seeds fair before repair"
        );
        // And the game plays at this preset.
        let mut g = crate::Game::create(seeds[0], crate::composing_tests::pack(), preset, None);
        let mut out = g.start();
        // The wanderer starts with nothing, so its save replays exactly.
        let mut bot = crate::coverage::Bot::new("wanderer", seeds[0]);
        for _ in 0..40 {
            let c = bot.next(&g, &out);
            out = g.step(&c);
        }
        assert_eq!(g.save().difficulty, preset);
        let (again, _) = crate::Game::load(&g.save(), crate::composing_tests::pack());
        assert_eq!(
            serde_json::to_string(&again.state).unwrap(),
            serde_json::to_string(&g.state).unwrap()
        );
    }
}

/// Balance numbers for the log: how long the wanderer lasts, what ends it,
/// and how much of the land changes on its own in a month.
#[test]
#[ignore]
fn balance_probe() {
    for preset in ["gentle", "standard", "archaeologist"] {
        let mut hours = Vec::new();
        let mut causes = std::collections::BTreeMap::new();
        for seed in 1..=8u64 {
            let mut g = crate::Game::create(seed, crate::composing_tests::pack(), preset, None);
            let mut out = g.start();
            let mut bot = crate::coverage::Bot::new("wanderer", seed);
            for _ in 0..400 {
                if g.state.dead.is_some() {
                    break;
                }
                let c = bot.next(&g, &out);
                out = g.step(&c);
            }
            hours.push((g.state.minutes - 8 * 60) / 60);
            *causes
                .entry(
                    g.state
                        .dead
                        .as_ref()
                        .map_or("alive".to_string(), |d| d.cause.clone()),
                )
                .or_insert(0) += 1;
        }
        let mut month = 0;
        let mut aspects = std::collections::BTreeMap::new();
        for seed in 1..=8u64 {
            let mut g = crate::Game::create(seed, crate::composing_tests::pack(), preset, None);
            g.start();
            g.state.minutes += 30 * 1440;
            g.step_regions();
            month += g
                .record()
                .regions
                .iter()
                .filter(|r| !r.changes.is_empty())
                .count();
            for r in g.record().regions {
                for c in r.changes {
                    *aspects
                        .entry(format!("{}:{}>{}", c.aspect, c.before, c.after))
                        .or_insert(0) += 1;
                }
            }
        }
        eprintln!("BALANCE {preset}: hours {hours:?} ends {causes:?}; regions changed alone in a month (8 worlds): {month} {aspects:?}");
    }
}
