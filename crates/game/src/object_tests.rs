//! D05 objects in play: each traces to an owner, examining twice looks
//! closer, and the same owner's emblem always reads the same.

use scraped_world::objects::Owner;

use crate::composing_tests::pack;
use crate::{Game, Target};

#[test]
fn every_object_traces_to_an_owner() {
    let g = Game::new(42, pack());
    let h = &g.site.world.history;
    assert!(g.site.world.objects.len() > 200);
    for o in &g.site.world.objects {
        match o.owner {
            Owner::Faction(f) => assert!(f < h.factions.len()),
            Owner::Family(p) => assert!(p < h.people.len()),
            Owner::Temple(s) => assert!(s < g.site.world.structures.len()),
            Owner::Era(e) => assert!((e as usize) < h.eras.len()),
        }
    }
}

#[test]
fn examining_twice_looks_closer() {
    let mut g = Game::new(42, pack());
    g.trace = true;
    g.forced_light = true;
    g.start();
    let t = (0..g.site.things.len())
        .find(|&t| g.site.things[t].object.is_some())
        .unwrap();
    g.state.place = g.site.things[t].home;
    g.state.pos = g.site.things[t].pos;
    let o = g.act("examine", Target::Thing(t));
    assert!(
        o.renders.iter().any(|r| r.trace.slot == "object.examine"),
        "{}",
        o.text
    );
    let o = g.act("examine", Target::Thing(t));
    assert!(
        o.renders.iter().any(|r| r.trace.slot == "object.closer"),
        "{}",
        o.text
    );
}

#[test]
fn one_owner_one_emblem_in_words() {
    let mut g = Game::new(42, pack());
    let seed = g.site.world.seed;
    let e = scraped_world::objects::emblem(seed, Owner::Faction(0));
    let a = g.emblem_words(e);
    let b = g.emblem_words(scraped_world::objects::emblem(seed, Owner::Faction(0)));
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn every_key_has_its_lock_and_opens_it() {
    let mut g = Game::new(42, pack());
    g.trace = true;
    g.forced_light = true;
    g.start();
    let objs = g.site.world.objects.clone();
    let mut boxes = 0;
    for k in objs.iter().filter(|o| o.kind == "key") {
        match k.opens.expect("a key opens something") {
            scraped_world::objects::Opens::Object { id } => {
                assert_eq!(objs[id].key, Some(k.id));
                if boxes < 3 {
                    boxes += 1;
                    let b = g.object_thing(id).unwrap();
                    g.state.place = g.site.things[b].home;
                    let o = g.act("open", Target::Thing(b));
                    assert!(
                        o.renders.iter().any(|r| r.trace.slot == "object.locked"),
                        "{}",
                        o.text
                    );
                    let kt = g.object_thing(k.id).unwrap();
                    g.state.carried.push(kt);
                    let o = g.act("open", Target::Thing(b));
                    assert!(
                        o.renders.iter().any(|r| r.trace.slot == "object.opened"),
                        "{}",
                        o.text
                    );
                }
            }
            scraped_world::objects::Opens::Door { structure, link } => {
                assert!(g
                    .site
                    .world
                    .locks
                    .iter()
                    .any(|l| l.structure == structure && l.link == link && l.key == k.id));
            }
        }
    }
    assert!(boxes > 0);
}

#[test]
fn caches_are_found_and_maps_show_their_era() {
    for seed in [1u64, 42, 9001] {
        let g = Game::new(seed, pack());
        let w = &g.site.world;
        let maps: Vec<_> = w.objects.iter().filter(|o| o.map.is_some()).collect();
        assert!(maps.len() >= 2, "seed {seed}: {} maps", maps.len());
        for m in maps {
            let cross = m.map.unwrap().cross.expect("a cross");
            assert_eq!(w.objects[cross].cache, "buried");
            assert!(w.objects[cross].feature.is_some());
        }
        // A cache in a room is found by looking closer.
        let c = w
            .objects
            .iter()
            .find(|o| o.cache == "floor" || o.cache == "wall")
            .unwrap();
        let mut g = Game::new(seed, pack());
        g.trace = true;
        g.forced_light = true;
        g.start();
        g.state.place = crate::site::Place::Room {
            structure: c.structure,
            room: c.room,
        };
        let o = g.step("look closer");
        assert!(
            o.renders.iter().any(|r| r.trace.slot == "cache.found"),
            "{}",
            o.text
        );
        assert!(g.state.uncovered.contains(&c.id));
    }
}

#[test]
fn buried_things_are_dug_up_where_the_map_marks() {
    let mut g = Game::new(42, pack());
    g.trace = true;
    g.forced = Some(("clear", "day"));
    g.start();
    let m = g
        .site
        .world
        .objects
        .iter()
        .find(|o| o.map.is_some())
        .unwrap()
        .clone();
    let cross = m.map.unwrap().cross.unwrap();
    let f = g.site.world.objects[cross].feature.unwrap();
    let c = g.site.world.features[f].cell;
    g.state.place = crate::site::Place::Outside;
    g.state.pos = scraped_sim::outdoors::Pos::of_cell(c.ux(), c.uy());
    let o = g.step("dig");
    assert!(
        o.renders.iter().any(|r| r.trace.slot == "dig.found"),
        "{}",
        o.text
    );
}
