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
