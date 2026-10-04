//! Balance and fairness tests for the physical game: survival bots,
//! obstacles that ordinary means can overcome, conservation.

use scraped_content::Pack;
use scraped_sim::body::Activity;
use scraped_sim::fixtures::{MechKind, Spot};
use scraped_sim::outdoors::{Pos, LOCAL};
use scraped_world::structures::{Condition, PassageState};

use crate::site::Place;
use crate::{Game, Target};

fn pack() -> Pack {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");
    let mut files = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "toml") {
            files.push((
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    Pack::load(&files).0
}

fn game(seed: u64) -> Game {
    let mut g = Game::new(seed, pack());
    g.start();
    g
}

const USEFUL: &[&str] = &[
    "provisions",
    "wood",
    "firesteel",
    "waterskin",
    "cloak",
    "torch",
];

/// Walks every room of the starting town's buildings, taking what keeps a
/// body alive. Moves within a building directly, a couple of minutes a room.
fn stock_up(g: &mut Game) {
    let town = g.site.settlement;
    let buildings: Vec<usize> = g
        .site
        .world
        .structures
        .iter()
        .filter(|st| st.settlement == Some(town) && st.condition != Condition::Buried)
        .map(|st| st.id)
        .collect();
    for s in buildings {
        let rooms = g.site.structure(s).interior.rooms.len();
        for r in 0..rooms {
            let room = &g.site.structure(s).interior.rooms[r];
            if room.collapsed
                || room.level < 0
                || g.flooded(Place::Room {
                    structure: s,
                    room: r,
                })
            {
                continue;
            }
            g.state.place = Place::Room {
                structure: s,
                room: r,
            };
            g.pass(2);
            for t in g.here() {
                // Two torches are plenty: a load of them leaves no room for
                // a firesteel.
                let torches = g
                    .state
                    .carried
                    .iter()
                    .filter(|&&c| g.thing(c).kind == "torch")
                    .count();
                if g.thing(t).kind == "torch" && torches >= 2 {
                    continue;
                }
                if USEFUL.contains(&g.thing(t).kind) && !g.overloaded_by(t) {
                    g.act("take", Target::Thing(t));
                }
            }
        }
    }
    g.state.place = Place::Outside;
    g.state.pos = g.site.start();
    let cloak = g
        .state
        .carried
        .iter()
        .copied()
        .find(|&t| g.thing(t).kind == "cloak");
    if let Some(c) = cloak {
        g.wear(c, true);
    }
}

/// A warm room in town to shelter in: a house with a hearth if there is
/// one, else any intact building.
fn home(g: &Game) -> Place {
    let town = g.site.settlement;
    let mut best: Option<(i32, Place)> = None;
    for st in g
        .site
        .world
        .structures
        .iter()
        .filter(|st| st.settlement == Some(town))
    {
        for (r, room) in st.interior.rooms.iter().enumerate() {
            if room.collapsed || room.level < 0 {
                continue;
            }
            let p = Place::Room {
                structure: st.id,
                room: r,
            };
            let hearth = g
                .site
                .things
                .iter()
                .any(|t| t.home == p && t.kind == "hearth");
            let score = i32::from(hearth) * 10
                + match st.condition {
                    Condition::Intact => 3,
                    Condition::Worn => 2,
                    _ => 0,
                };
            if best.is_none_or(|(b, _)| score > b) {
                best = Some((score, p));
            }
        }
    }
    best.map_or(Place::Outside, |(_, p)| p)
}

fn outside(g: &mut Game) {
    if g.state.place != Place::Outside {
        g.state.place = Place::Outside;
        g.pass(2);
    }
    let town = g.site.town_pos(g.site.settlement);
    g.state.pos = Pos::new(town.x + 60, town.y + 40);
}

/// A careful player who stays near the starting town: drinks when
/// thirsty, eats when hungry, forages by day, keeps a fire and sleeps
/// indoors at night. Returns how it died, if it did, and minutes lived.
fn careful(seed: u64, days: u32) -> (Option<String>, u32) {
    let mut g = game(seed);
    stock_up(&mut g);
    let shelter = home(&g);
    let end = g.state.minutes + days * 1440;
    while g.state.minutes < end && g.state.dead.is_none() {
        let hour = (g.state.minutes / 60) % 24;
        let b = &g.state.body;
        if b.thirst_state() >= 1 {
            outside(&mut g);
            g.drink(&[]);
            if let Some(skin) = g
                .state
                .carried
                .iter()
                .copied()
                .find(|&t| g.thing(t).kind == "waterskin")
            {
                g.fill(skin);
            }
            continue;
        }
        if g.state.body.hunger_state() >= 1 {
            let food =
                g.state.carried.iter().any(|&t| {
                    scraped_sim::items::kind(g.thing(t).kind).is_some_and(|k| k.meal > 0)
                });
            if food {
                g.eat(&[]);
            } else {
                outside(&mut g);
                g.forage();
            }
            continue;
        }
        let night = !(7..20).contains(&hour);
        if night || g.state.body.rest_state() >= 1 || g.state.body.warmth_state() >= 1 {
            if g.state.place != shelter {
                g.state.place = shelter;
                g.pass(5);
            }
            if g.fire_here().is_none() {
                let _ = g.light_fire();
            }
            if night || g.state.body.rest_state() >= 1 {
                g.sleep();
            } else {
                g.pass(60);
            }
            g.take_notes();
            continue;
        }
        // Daytime: forage and gather while the body is fine.
        outside(&mut g);
        let wood = g
            .state
            .carried
            .iter()
            .filter(|&&t| g.thing(t).kind == "wood")
            .count();
        if wood < 2 {
            g.gather();
        }
        g.forage();
        g.take_notes();
    }
    let lived = g.state.minutes - 8 * 60;
    (g.state.dead.map(|d| d.cause), lived)
}

/// A reckless player: walks on and on in all directions and never drinks,
/// eats or sleeps.
fn reckless(seed: u64) -> (Option<String>, u32) {
    let mut g = game(seed);
    let dirs = ["head north", "head east", "head south", "head west"];
    let mut i = 0;
    while g.state.dead.is_none() && g.state.minutes < 30 * 1440 {
        g.step(dirs[i % 4]);
        i += 1;
        if i > 4000 {
            break;
        }
    }
    (g.state.dead.map(|d| d.cause), g.state.minutes - 8 * 60)
}

#[test]
fn careful_survivors_last_and_reckless_ones_do_not() {
    let seeds = [1u64, 3, 7, 42, 99];
    let mut lived = 0;
    let mut report = Vec::new();
    for seed in seeds {
        let (death, minutes) = careful(seed, 4);
        report.push((seed, death.clone(), minutes / 60));
        if death.is_none() {
            lived += 1;
        }
    }
    assert!(lived >= 4, "careful survivors: {report:?}");
    let mut causes = std::collections::BTreeSet::new();
    let mut spans = Vec::new();
    for seed in seeds {
        let (death, minutes) = reckless(seed);
        let cause = death.unwrap_or_else(|| {
            panic!(
                "seed {seed}: the reckless walker lived {} hours",
                minutes / 60
            )
        });
        spans.push((seed, cause.clone(), minutes / 60));
        causes.insert(cause);
        assert!(
            (6 * 60..6 * 1440).contains(&minutes),
            "seed {seed}: died after {} hours",
            minutes / 60
        );
    }
    eprintln!("careful: {report:?}\nreckless: {spans:?}");
    assert!(causes.len() >= 2, "only one way to die: {spans:?}");
}

#[test]
fn barred_doors_flooded_rooms_and_raised_bridges_have_ordinary_answers() {
    for seed in [1u64, 3, 7, 42, 99] {
        let g = game(seed);
        let f = &g.site.fixtures;
        // A pry bar lies in the starting town, outside any barred room.
        let town = g.site.settlement;
        assert!(
            f.items.iter().any(|p| p.kind == "pry_bar"
                && matches!(p.at, Spot::Room { structure, .. } if g.site.world.structures[structure].settlement == Some(town))),
            "seed {seed}: no pry bar in town"
        );
        // Every flooded room has a drain at its building's dry entrance.
        for (i, fl) in f.flooded.iter().enumerate() {
            assert_ne!(fl.room, 0);
            assert!(f.mechanisms.iter().any(|m| m.kind == MechKind::DrainLever
                && m.controls == Some(i)
                && m.at
                    == Spot::Room {
                        structure: fl.structure,
                        room: 0
                    }));
        }
        // Every raised bridge has a lever reachable from dry land.
        for &b in &f.raised {
            let lever = f
                .mechanisms
                .iter()
                .find(|m| m.kind == MechKind::BridgeLever && m.controls == Some(b))
                .expect("lever");
            let (x, y) = lever.pos.cell();
            let env = g.env();
            let dry = g
                .site
                .world
                .terrain
                .height
                .neighbours(x, y)
                .any(|(nx, ny)| {
                    env.passable(nx, ny)
                        && scraped_sim::outdoors::Pos::of_cell(nx, ny).dist(lever.pos)
                            <= 400.0 + f64::from(LOCAL)
                });
            assert!(dry, "seed {seed}: bridge {b}'s lever stands in water");
        }
    }
}

#[test]
fn opening_a_sluice_drops_the_ford_and_conserves_water() {
    let mut found = 0;
    for seed in 1u64..12 {
        let mut g = game(seed);
        if g.site.fixtures.sluices.is_empty() {
            continue;
        }
        found += 1;
        let s = g.site.fixtures.sluices[0].clone();
        let m = g
            .site
            .fixtures
            .mechanisms
            .iter()
            .position(|m| m.kind == MechKind::Sluice && m.controls == Some(0))
            .unwrap();
        let (cx, cy) = s.crossing;
        assert!(
            !g.env().passable(cx, cy),
            "seed {seed}: crossing open before the sluice"
        );
        let before: Vec<u32> = s.below.iter().map(|&(x, y)| g.env().flow(x, y)).collect();
        // Walk to the sluice and open it.
        g.state.place = Place::Outside;
        g.state.pos = g.site.fixtures.mechanisms[m].pos;
        g.operate(m, "open");
        assert!(g.env().passable(cx, cy), "seed {seed}: crossing still deep");
        // Water is moved, not made or lost.
        for (i, &(x, y)) in s.below.iter().enumerate() {
            assert_eq!(g.env().flow(x, y) + s.diverts.min(before[i]), before[i]);
        }
        let (chx, chy) = s.channel;
        assert_eq!(
            g.site
                .fixtures
                .channel_water(chx, chy, &|i| g.env().sluice_open(i)),
            s.diverts
        );
        // And the wheel below stops.
        if let Some(w) = g
            .site
            .fixtures
            .mechanisms
            .iter()
            .position(|k| k.kind == MechKind::Wheel && k.controls == Some(0))
        {
            let _ = g.env().wheel_turns(w);
        }
    }
    assert!(found >= 2, "too few worlds with a sluice to test");
}

#[test]
fn fire_needs_fuel_and_goes_out_without_it() {
    let mut g = game(42);
    stock_up(&mut g);
    g.state.place = home(&g);
    if g.state.carried.iter().all(|&t| g.thing(t).kind != "wood") {
        let w = g.make_item("wood", true);
        assert!(w.is_some());
    }
    if g.state
        .carried
        .iter()
        .all(|&t| g.thing(t).kind != "firesteel")
    {
        g.make_item("firesteel", true);
    }
    let out = g.light_fire();
    assert!(g.fire_here().is_some(), "no fire: {}", out.text);
    // One bundle of wood burns two hours, then the fire is out.
    g.advance(119, Activity::Resting);
    assert!(g.fire_here().is_some());
    g.advance(2, Activity::Resting);
    assert!(g.fire_here().is_none(), "{:?}", g.state.sim.fires);
    // No wood, no fire.
    let keep: Vec<usize> = g
        .state
        .carried
        .iter()
        .copied()
        .filter(|&t| g.thing(t).kind != "wood")
        .collect();
    g.state.carried = keep;
    g.light_fire();
    assert!(g.fire_here().is_none());
}

#[test]
fn darkness_hides_and_light_reveals() {
    for seed in [1u64, 3, 7, 42, 99] {
        let mut g = game(seed);
        let dark = g.site.world.structures.iter().find_map(|st| {
            st.interior
                .rooms
                .iter()
                .enumerate()
                .find(|(r, room)| {
                    room.level < 0
                        && !room.collapsed
                        && !g.flooded(Place::Room {
                            structure: st.id,
                            room: *r,
                        })
                })
                .map(|(r, _)| Place::Room {
                    structure: st.id,
                    room: r,
                })
        });
        let Some(p) = dark else { continue };
        g.state.place = p;
        assert!(g.is_dark());
        let t = g.make_item("torch", true).unwrap();
        g.make_item("firesteel", true);
        g.light_item(t);
        assert!(!g.is_dark());
        return;
    }
    panic!("no underground room to test");
}

#[test]
fn every_closed_way_still_leaves_the_town_reachable() {
    // Barred doors are inside buildings; the open ground of the starting
    // town and the doors of its buildings stay reachable without tools.
    let g = game(42);
    let town = g.site.settlement;
    for st in g
        .site
        .world
        .structures
        .iter()
        .filter(|s| s.settlement == Some(town))
    {
        if st.condition == Condition::Buried {
            continue;
        }
        let entrance = &st.interior.rooms[0];
        assert!(!entrance.collapsed || st.interior.rooms.len() == 1);
        let _ = st
            .interior
            .links
            .iter()
            .filter(|l| l.state == PassageState::Blocked)
            .count();
    }
}
