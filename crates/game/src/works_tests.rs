//! D05 works and calendar doors: solvable by trying and watching.

use crate::composing_tests::pack;
use crate::site::Place;
use crate::Game;

#[test]
fn works_are_solved_by_trying_parts_and_watching() {
    let mut solved = 0;
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.forced_light = true;
        g.start();
        for w in 0..g.site.fixtures.works.len() {
            let steps = g.site.fixtures.works[w].steps.clone();
            let (st, seal) = (
                g.site.fixtures.works[w].structure,
                g.site.fixtures.works[w].seal,
            );
            assert!(steps.len() >= 4);
            assert!(g.sealed(st, seal).is_some(), "sealed at first");
            // A bot with no plan: tries the parts back to front, again and
            // again, reading only whether each one moved.
            for _ in 0..steps.len() {
                for &p in steps.iter().rev() {
                    let at = g.site.fixtures.mechanisms[p].at;
                    if let scraped_sim::fixtures::Spot::Room { structure, room } = at {
                        g.state.place = Place::Room { structure, room };
                    }
                    g.operate(p, "operate");
                }
            }
            assert!(g.works_done(w));
            assert!(g.sealed(st, seal).is_none(), "open at the end");
            solved += 1;
        }
    }
    assert!(solved >= 2, "too few works: {solved}");
}

#[test]
fn a_part_out_of_order_says_what_it_waits_on() {
    let mut g = Game::new(42, pack());
    g.trace = true;
    g.forced_light = true;
    g.start();
    let Some(w) = g.site.fixtures.works.first().cloned() else {
        return;
    };
    let last = *w.steps.last().unwrap();
    let o = g.operate(last, "operate");
    assert!(
        o.renders.iter().any(|r| r.trace.slot == "mech.idle"),
        "{}",
        o.text
    );
}

#[test]
fn calendar_doors_open_on_their_day() {
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.start();
        let Some(c) = g.site.fixtures.calendar.first().cloned() else {
            continue;
        };
        let year = 4 * scraped_sim::region::SEASON_DAYS;
        g.state.minutes = ((c.day + year / 2) % year) * 1440 + 600;
        assert_eq!(g.sealed(c.structure, c.link), Some("calendar"));
        g.state.minutes = c.day * 1440 + 600;
        assert_eq!(g.sealed(c.structure, c.link), None);
        return;
    }
    panic!("no calendar door in any seed");
}

#[test]
fn a_rope_climbs_back_up_a_drop() {
    for seed in [1u64, 42, 9001] {
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.forced_light = true;
        g.start();
        // A hole down somewhere: stand below it.
        let found = g.site.world.greats.iter().find_map(|&(i, _)| {
            let int = &g.site.world.structures[i].interior;
            int.links
                .iter()
                .position(|l| {
                    l.one_way && l.passage == scraped_world::structures::Passage::Hole && !l.hidden
                })
                .map(|l| (i, int.links[l].b, l))
        });
        let Some((i, below, link)) = found else {
            continue;
        };
        g.state.place = Place::Room {
            structure: i,
            room: below,
        };
        let before = g.state.place;
        g.go_way(link);
        assert_eq!(g.state.place, before, "no climbing without a rope");
        let rope = (0..g.site.things.len())
            .find(|&t| g.site.things[t].kind == "rope")
            .expect("a rope somewhere");
        g.state.carried.push(rope);
        let o = g.go_way(link);
        assert_ne!(g.state.place, before, "{}", o.text);
        return;
    }
}
