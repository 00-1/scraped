//! D04 in play: great interiors can be mapped on paper from what the game
//! says, hidden spaces show as gaps in a careful plan, and the ways of
//! finding one's way (marks, going back, following passages) work.

use scraped_world::structures::Passage;

use crate::composing_tests::pack;
use crate::mapper::map;
use crate::site::Place;
use crate::{Game, Target};

const SEEDS: [u64; 2] = [42, 9001];

#[test]
fn the_mapper_draws_a_true_plan() {
    let p = pack();
    for seed in SEEDS {
        let g = Game::new(seed, p.clone());
        let greats: Vec<usize> = g
            .site
            .world
            .greats
            .iter()
            .map(|&(i, _)| i)
            .take(3)
            .collect();
        let mut most = 0;
        for i in greats {
            let run = map(&p, seed, i);
            assert!(run.topology_exact, "seed {seed} structure {i}: {run:?}");
            assert!(run.error_max <= 0.10, "seed {seed} structure {i}: {run:?}");
            // Some are held shut by old writing or barred: most are open.
            if run.visited * 2 >= run.spaces {
                most += 1;
            }
        }
        assert!(most >= 2, "seed {seed}: the mapper got into too few");
    }
}

#[test]
fn the_largest_interior_takes_days_to_explore() {
    let p = pack();
    let g = Game::new(42, p.clone());
    let (i, _) = *g
        .site
        .world
        .greats
        .iter()
        .max_by_key(|(i, _)| g.site.world.structures[*i].interior.rooms.len())
        .unwrap();
    let run = map(&p, 42, i);
    assert!(run.hours >= 24.0, "{run:?}");
}

#[test]
fn hidden_spaces_are_gaps_in_the_plan() {
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        for &(i, _) in &g.site.world.greats {
            let int = &g.site.world.structures[i].interior;
            for (n, h) in int.rooms.iter().enumerate().filter(|(_, r)| r.hidden) {
                // In caves, hidden passages lie behind rubble off a chamber:
                // found by a draught, not by symmetry.
                if h.style == "natural" {
                    continue;
                }
                // In a building it touches a visible space (the one its way
                // opens from), and a twin of the same size stands across
                // that space: a careful plan shows the gap.
                let visible: Vec<_> = int
                    .rooms
                    .iter()
                    .filter(|r| !r.hidden && r.level == h.level)
                    .collect();
                assert!(
                    visible.iter().any(|r| r.touches(h).is_some()),
                    "seed {seed} structure {i}: room {n}"
                );
                let twin = visible.iter().any(|v| {
                    let side = v.touches(h);
                    side.is_some()
                        && visible
                            .iter()
                            .any(|o| scraped_world::interiors::mirrors(v, h, o, side))
                });
                let hidden_way_from_twin_wing =
                    int.links.iter().any(|l| l.hidden && (l.a == n || l.b == n));
                assert!(
                    twin && hidden_way_from_twin_wing,
                    "seed {seed} structure {i}: hidden room {n} has no visible twin"
                );
            }
        }
    }
}

/// A game standing in a great interior's entrance, in good light.
fn inside(seed: u64) -> (Game, usize) {
    let mut g = Game::new(seed, pack());
    g.trace = true;
    g.sustain = true;
    g.forced_light = true;
    g.start();
    let (i, _) = g.site.world.greats[0];
    g.state.place = Place::Room {
        structure: i,
        room: 0,
    };
    g.state.pos = g.site.land.structure_pos[i];
    (g, i)
}

#[test]
fn marks_stay_and_known_spaces_can_be_walked_back_to() {
    for seed in SEEDS {
        let (mut g, i) = inside(seed);
        g.step("mark");
        // Walk a few ways in.
        for _ in 0..4 {
            let w = g.ways().into_iter().find(|w| {
                w.passage != Some(Passage::Window)
                    && w.state != scraped_world::structures::PassageState::Blocked
                    && !w.collapsed
                    && !w.against
                    && !g.under_water(w.to)
            });
            if let Some(w) = w {
                g.step_quiet_door(w.link);
                g.go_way(w.link);
            }
        }
        if g.state.place
            == (Place::Room {
                structure: i,
                room: 0,
            })
        {
            continue; // held shut
        }
        let o = g.act("go", Target::Room(0));
        assert_eq!(
            g.state.place,
            Place::Room {
                structure: i,
                room: 0
            },
            "seed {seed}: {}",
            o.text
        );
        assert!(
            o.renders.iter().any(|r| r.trace.slot == "room.mark"),
            "seed {seed}: {}",
            o.text
        );
    }
}

#[test]
fn windows_are_seen_through_not_passed() {
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        let found = g.site.world.greats.iter().find_map(|&(i, _)| {
            let int = &g.site.world.structures[i].interior;
            int.links
                .iter()
                .position(|l| l.passage == Passage::Window)
                .map(|l| (i, int.links[l].a, l))
        });
        let Some((i, room, link)) = found else {
            continue;
        };
        let mut g = g;
        g.trace = true;
        g.forced_light = true;
        g.state.place = Place::Room { structure: i, room };
        let o = g.go_way(link);
        assert_eq!(g.state.place, Place::Room { structure: i, room });
        assert!(o.renders.iter().any(|r| r.trace.slot == "move.window"));
    }
}

#[test]
fn passages_are_followed_in_one_go() {
    for seed in SEEDS {
        let (mut g, i) = inside(seed);
        // Find a corridor and stand in it.
        let int = &g.site.world.structures[i].interior;
        let Some(c) = (1..int.rooms.len())
            .find(|&r| int.rooms[r].space == "corridor" && !int.rooms[r].hidden)
        else {
            continue;
        };
        g.state.place = Place::Room {
            structure: i,
            room: c,
        };
        let o = g.step("follow the passage");
        assert!(
            o.renders
                .iter()
                .any(|r| r.trace.slot == "move.run" || r.trace.slot == "say.no_exit"),
            "seed {seed}: {}",
            o.text
        );
    }
}
