//! D11: sealed places. Each is visible and reachable to its door without
//! writing; its way in won't give while its ward holds; an opening spell
//! written over the ward and scraped lets the reader in; and nothing
//! written anywhere else does.

use scraped_lang::meaning::{NounPhrase, Sentence};
use scraped_sim::body::Activity;

use crate::composing_tests::{claim, glyphs_for, pack};
use crate::site::Place;
use crate::{Game, Target};

/// "let this <noun> open": the plainest counter to a ward.
fn open_this(noun: &str) -> Sentence {
    let mut s = claim("open", noun, false);
    if let Sentence::Clause(c) = &mut s {
        c.args[0].np = NounPhrase::concept(noun).det("this");
    }
    s
}

/// A game standing at a sealed place's ward stone, by day, with a stylus
/// and a strong scraper, and every word allowed.
fn at_ward(seed: u64, k: usize) -> Option<(Game, usize, usize)> {
    let mut g = Game::new(seed, pack());
    g.trace = true;
    g.forced = Some(("clear", "daylight"));
    g.start();
    let s = g.site.writing.sealed.get(k)?.structure;
    let stone = g.ward_stone(s)?;
    g.state.place = Place::Outside;
    g.state.pos = g.site.things[stone].pos;
    g.make_item("stylus", true);
    g.make_item("graver", true);
    g.threshold = 0;
    Some((g, s, stone))
}

#[test]
fn every_world_has_sealed_places_with_a_stone_out_of_doors() {
    for seed in [1, 42, 9001] {
        let g = Game::new(seed, pack());
        assert!(
            !g.site.writing.sealed.is_empty(),
            "seed {seed}: no sealed place"
        );
        for x in &g.site.writing.sealed {
            let stone = g.ward_stone(x.structure).expect("a ward stone");
            assert_eq!(g.site.things[stone].home, Place::Outside);
            // The ward is the stone's live layer when play begins.
            let layers = g.layers(stone);
            assert_eq!(
                scraped_sim::writing::live_of(&layers, &g.state.scraped),
                Some(x.ward)
            );
            assert!(g.sealed_shut(x.structure), "seed {seed}: open at start");
        }
    }
}

#[test]
fn a_sealed_place_opens_only_to_writing_on_its_ward() {
    for seed in [1, 42] {
        let Some((mut g, s, stone)) = at_ward(seed, 0) else {
            panic!("seed {seed}: no sealed place");
        };
        let o = g.act("go", Target::Structure(s));
        assert_eq!(g.state.place, Place::Outside, "{}", o.text);
        assert!(o.renders.iter().any(|r| r.trace.slot == "effect.held"));
        // An opening spell written over the ward and scraped: in.
        let noun = g.site.writing.sealed[0].noun;
        let glyphs = glyphs_for(&mut g, &open_this(noun));
        g.state.it = Some(Target::Thing(stone));
        let w = g.step(&format!("write {glyphs} on it"));
        assert!(
            g.state.written.iter().any(|x| x.thing == stone),
            "seed {seed}: not written: {}",
            w.text
        );
        g.advance(40, Activity::Resting);
        g.scrape(stone);
        assert!(!g.sealed_shut(s), "seed {seed}: still shut");
        g.act("go", Target::Structure(s));
        assert!(matches!(g.state.place, Place::Room { structure, .. } if structure == s));
    }
}

#[test]
fn writing_elsewhere_does_not_open_a_sealed_place() {
    let Some((g, s, _)) = at_ward(42, 0) else {
        panic!("no sealed place");
    };
    // Every ward holds whatever the rest of the world says.
    let _ = g;
    let mut g = Game::new(42, pack());
    g.start();
    let all: Vec<usize> = (0..g.site.things.len()).collect();
    for t in all {
        if g.thing(t).texts.is_empty() {
            continue;
        }
        if let Some(top) = scraped_sim::writing::top_unscraped_of(&g.layers(t), &g.state.scraped) {
            g.state.scraped.insert(top);
        }
    }
    g.recompute_claims();
    assert!(
        g.sealed_shut(s),
        "scraping the world's latent writing opened it"
    );
}

/// The chain is walked in order and each step teaches the next: beneath
/// each ward and inside each place lies the next place's word, and the
/// fairness check finds every ward's words attested before it is needed.
#[test]
fn sealed_places_form_an_attested_chain() {
    for seed in [1, 42, 9001] {
        let site = crate::site::Site::create(seed, "standard", None);
        let report = crate::fairness::check(&site, "standard");
        let sealed = report.goals.iter().find(|g| g.goal == "sealed").unwrap();
        assert!(sealed.ok, "seed {seed}: {:?}", sealed.missing);
        let w = &site.writing;
        for (k, x) in w.sealed.iter().enumerate() {
            let Some(next) = w.sealed.get(k + 1) else {
                break;
            };
            for id in [x.clue, x.inside].into_iter().flatten() {
                let mut roots = std::collections::BTreeSet::new();
                let t = w.text(&site.world, id);
                crate::composing::concepts_of(
                    &t.meaning,
                    &mut roots,
                    &site.world.languages[t.era as usize].numerals,
                );
                assert!(
                    roots.contains(next.noun),
                    "seed {seed}: place {k} doesn't teach the next"
                );
            }
        }
    }
}
