//! D02 properties: budgets hold, selection is deterministic, repetition
//! fades, towns are never listed, every member of a group can be reached,
//! seasons are shown by evidence, and nothing states what should be shown.

use scraped_content::{Context, Value};
use scraped_world::terrain::Biome;

use crate::attention::{Fact, Response, SEASON_EVIDENCE};
use crate::composing_tests::pack;
use crate::site::{label, Place};
use crate::{Game, Target};

const SEEDS: [u64; 4] = [1, 42, 7, 9001];

fn many_facts() -> Vec<Fact> {
    let mut v: Vec<Fact> = (0..12)
        .map(|i| {
            let slot: &'static str =
                ["place.thing", "land.ground", "sky.weather", "air.felt"][i % 4];
            Fact::new(slot, format!("test:{i}"), 10.0 + i as f64, Context::new())
        })
        .collect();
    v.push(Fact::new("danger.unstable", "test:danger", 70.0, Context::new()).interrupting());
    v
}

#[test]
fn budget_is_never_exceeded_except_by_interruptions() {
    let mut g = Game::new(1, pack());
    g.start();
    for r in [
        Response::Arrival,
        Response::Travel,
        Response::Room,
        Response::Look,
        Response::Closer,
        Response::Around,
    ] {
        let said = g.attend(many_facts(), r);
        assert!(said.len() <= r.budget() + 1, "{r:?}: {said:?}");
        // The interruption always gets through.
        assert!(!said.is_empty());
    }
    // Even with nothing new, the interruption is said.
    let said = g.attend(many_facts(), Response::Arrival);
    assert!(said.len() <= 3);
    assert!(!said.is_empty());
}

#[test]
fn selection_is_deterministic() {
    for seed in SEEDS {
        let run = || {
            let mut g = Game::new(seed, pack());
            let mut out = vec![g.start().text];
            for c in [
                "look",
                "look closer",
                "look around",
                "listen",
                "smell",
                "look",
                "look up",
            ] {
                out.push(g.step(c).text);
            }
            (out, serde_json::to_string(&g.state.told).unwrap())
        };
        assert_eq!(run(), run(), "seed {seed}");
    }
}

#[test]
fn a_second_look_says_less() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        let first = g.step("look").text;
        let second = g.step("look").text;
        let third = g.step("look").text;
        let sentences = |t: &str| t.matches(['.', '!', '?']).count();
        assert!(
            sentences(&third) <= sentences(&first) && sentences(&third) <= 2,
            "seed {seed}: {first:?} then {second:?} then {third:?}"
        );
    }
}

#[test]
fn no_arrival_or_look_names_more_than_two_buildings() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        let mut outs = vec![g.start()];
        for c in ["look", "look", "out", "look"] {
            outs.push(g.step(c));
        }
        for o in outs {
            let named = o
                .renders
                .iter()
                .filter(|r| r.trace.slot == "place.standout")
                .count();
            assert!(named <= 2, "seed {seed}: {}", o.text);
            assert!(!o.renders.iter().any(|r| r.trace.slot == "place.member"));
        }
    }
}

#[test]
fn every_member_of_a_group_can_be_reached() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        let local = g.local_structures();
        let kinds: std::collections::BTreeSet<String> = local
            .iter()
            .map(|&s| label(&g.site.world.structures[s].kind))
            .collect();
        for kind in kinds {
            let k = crate::slots::STRUCTURES
                .iter()
                .position(|s| *s == kind)
                .expect("a known kind");
            let members = local
                .iter()
                .filter(|&&s| label(&g.site.world.structures[s].kind) == kind)
                .count();
            let mut reached = std::collections::BTreeSet::new();
            for _ in 0..members.div_ceil(3) {
                g.examine_group(Target::Buildings(k, None));
                reached.extend(
                    g.state
                        .told
                        .keys()
                        .filter(|key| key.starts_with("member:"))
                        .cloned(),
                );
            }
            let all = local
                .iter()
                .filter(|&&s| label(&g.site.world.structures[s].kind) == kind)
                .all(|s| reached.contains(&format!("member:{s}")));
            assert!(all, "seed {seed}: not every {kind} reached");
        }
    }
}

#[test]
fn each_season_shows_at_least_three_kinds_of_evidence_in_each_biome() {
    use Biome::*;
    for season in scraped_sim::region::SEASONS {
        for biome in [
            Shore, Marsh, Grassland, Scrub, Desert, Forest, Pine, Tundra, Rock, Snow,
        ] {
            let n = SEASON_EVIDENCE
                .iter()
                .filter(|(s, _, lands)| *s == season && lands.contains(&biome))
                .count();
            assert!(n >= 3, "{season} in {biome:?}: {n}");
        }
    }
}

/// Slots allowed to speak of need levels: `check myself` and the
/// sensations that cross a threshold.
fn may_state_needs(slot: &str) -> bool {
    slot.starts_with("body.")
}

#[test]
fn nothing_states_the_season_the_regions_state_or_needs() {
    let seasons = scraped_sim::region::SEASONS;
    let needs: Vec<&str> = crate::slots::need_states()
        .iter()
        .flat_map(|(_, states)| states.iter().copied())
        .collect();
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        let mut outs = vec![g.start()];
        for c in [
            "look",
            "look around",
            "look closer",
            "listen",
            "smell",
            "look up",
            "look down",
            "touch ground",
            "wait",
            "wait",
            "look",
            "out",
            "look",
        ] {
            outs.push(g.step(c));
        }
        for o in outs {
            // DESIGN-Q: storylets keep the season and the region's state
            // as variables, for Jb's conditions ("only in winter"); their
            // example text never says them, which the text check below
            // covers.
            for r in o
                .renders
                .iter()
                .filter(|r| !r.trace.slot.starts_with("story."))
            {
                for (k, v) in &r.vars {
                    let Value::Text(t) = v else { continue };
                    assert!(
                        !(k == "season" || seasons.contains(&t.as_str())),
                        "seed {seed}: {} has the season in {k}",
                        r.trace.slot
                    );
                    if scraped_sim::region::VARIABLES.contains(&k.as_str()) {
                        panic!("seed {seed}: {} states the region's {k}", r.trace.slot);
                    }
                    if !may_state_needs(&r.trace.slot)
                        && matches!(
                            k.as_str(),
                            "warmth" | "thirst" | "hunger" | "rest" | "injury"
                        )
                    {
                        assert!(
                            !needs.contains(&t.as_str()),
                            "seed {seed}: {} states a need",
                            r.trace.slot
                        );
                    }
                }
            }
            let lower = o.text.to_lowercase();
            for s in seasons {
                assert!(!lower.contains(s), "seed {seed}: {:?}", o.text);
            }
        }
    }
}

#[test]
fn digging_finds_much_more_than_arrival_shows() {
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        let shown = g.step("look").text.matches('.').count().max(1);
        let more = g.on_demand();
        assert!(more >= 3 * shown, "seed {seed}: {more} for {shown}");
        // And measuring leaves the game as it was.
        let before = serde_json::to_string(&g.state).unwrap();
        g.on_demand();
        assert_eq!(before, serde_json::to_string(&g.state).unwrap());
        assert_eq!(g.state.place, Place::Outside);
    }
}

#[test]
fn spoiled_truth_keeps_every_fact_weighed() {
    let mut g = Game::new(42, pack());
    g.spoil = true;
    g.start();
    let o = g.step("look");
    let weighed = o
        .truth
        .as_ref()
        .and_then(|t| t["attention"].as_array().cloned());
    let weighed = weighed.expect("the facts weighed");
    let said = o.text.matches('.').count();
    assert!(
        weighed.len() > said,
        "{} weighed, {said} said",
        weighed.len()
    );
    // Without spoilers, nothing is exposed.
    let mut g = Game::new(42, pack());
    g.start();
    assert!(g.step("look").truth.is_none());
}

#[test]
fn the_parser_understands_groups() {
    // A world whose starting town has several worn tombs.
    let seed = (1u64..60)
        .find(|&s| {
            let g = Game::new(s, pack());
            let town = g.site.settlement;
            g.site
                .world
                .structures
                .iter()
                .filter(|st| {
                    st.settlement == Some(town)
                        && st.kind == scraped_world::structures::StructureKind::Tomb
                        && st.condition == scraped_world::structures::Condition::Worn
                })
                .count()
                >= 2
        })
        .expect("a town with tombs");
    let mut g = Game::new(seed, pack());
    g.trace = true;
    g.start();
    let slots = |o: &crate::Output| -> Vec<String> {
        o.renders.iter().map(|r| r.trace.slot.clone()).collect()
    };
    for c in ["look at the tombs", "look at the worn tombs"] {
        let o = g.step(c);
        assert!(
            slots(&o).iter().any(|s| s == "place.member"),
            "{c}: {}",
            o.text
        );
    }
    let o = g.step("count the tombs");
    assert!(slots(&o).iter().any(|s| s == "place.count"), "{}", o.text);
    for c in [
        "go among the tombs",
        "go to another tomb",
        "go to the nearest tomb",
        "go to a tomb",
    ] {
        g.step("out");
        g.step(c);
        assert!(matches!(g.state.place, Place::Room { .. }), "{c}");
    }
}
