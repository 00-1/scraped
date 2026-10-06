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
fn every_world_has_sealed_places_with_a_ward_at_the_way_in() {
    for seed in [1, 42, 9001] {
        let g = Game::new(seed, pack());
        assert!(
            !g.site.writing.sealed.is_empty(),
            "seed {seed}: no sealed place"
        );
        for x in &g.site.writing.sealed {
            let stone = g.ward_stone(x.structure).expect("a ward stone");
            let home = if x.inner {
                Place::Room {
                    structure: x.structure,
                    room: 0,
                }
            } else {
                Place::Outside
            };
            assert_eq!(g.site.things[stone].home, home);
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

/// After the first place, a ward gives only to a counter as strong as
/// itself: "open this door" leaves it shut, "open this door greatly" opens.
#[test]
fn a_later_ward_asks_for_a_strong_counter() {
    let mut tried = 0;
    for seed in [1, 42, 9001] {
        let Some((mut g, s, stone)) = at_ward(seed, 1) else {
            continue;
        };
        tried += 1;
        let noun = g.site.writing.sealed[1].noun;
        g.state.it = Some(Target::Thing(stone));
        let glyphs = glyphs_for(&mut g, &open_this(noun));
        g.step(&format!("write {glyphs} on it"));
        g.advance(40, Activity::Resting);
        g.scrape(stone);
        assert!(g.sealed_shut(s), "seed {seed}: a plain counter opened it");
        let mut strong = open_this(noun);
        if let Sentence::Clause(c) = &mut strong {
            c.adverbs.push("greatly".into());
        }
        let glyphs = glyphs_for(&mut g, &strong);
        g.state.it = Some(Target::Thing(stone));
        let w = g.step(&format!("write {glyphs} on it"));
        g.advance(40, Activity::Resting);
        g.scrape(stone);
        assert!(!g.sealed_shut(s), "seed {seed}: still shut: {}", w.text);
    }
    assert!(tried > 0, "no world with a second sealed place");
}

/// The chain often ends at a great inscription (a journey that needs
/// walking, surviving and writing), and its last place always points to
/// where the first great writing was done: a town named in its account.
#[test]
fn the_chain_ends_at_a_great_inscription_and_points_to_the_root() {
    let mut greats = 0;
    for seed in 1..=8u64 {
        let site = crate::site::Site::create(seed, "standard", None);
        let w = &site.writing;
        let Some(last) = w.sealed.last() else {
            continue;
        };
        if last.great {
            greats += 1;
            assert!(site
                .greats
                .iter()
                .any(|g| { site.world.texts[g.text].structure == last.structure }));
        }
        let Some(inside) = last.inside else { continue };
        let t = w.text(&site.world, inside);
        let named = t.meaning.noun_phrases().iter().any(|n| {
            matches!(n.head, scraped_lang::meaning::Head::Name(i) if i >= site.world.history.people.len())
        });
        assert!(named, "seed {seed}: the last place names no town");
    }
    assert!(
        greats >= 2,
        "only {greats} of 8 chains end at a great inscription"
    );
}

/// A counter can carry a condition (D09), and then it opens the way only
/// while the condition holds: "let this gate open when night comes".
#[test]
fn a_conditional_counter_opens_only_when_it_holds() {
    let Some((mut g, s, stone)) = at_ward(42, 0) else {
        panic!("no sealed place");
    };
    let noun = g.site.writing.sealed[0].noun;
    let mut by_night = open_this(noun);
    if let Sentence::Clause(c) = &mut by_night {
        c.subordinate.push(scraped_lang::meaning::Subordinate {
            link: scraped_lang::meaning::Link::When,
            clause: scraped_lang::meaning::Clause::plain("come", NounPhrase::concept("night")),
        });
    }
    let glyphs = glyphs_for(&mut g, &by_night);
    g.state.it = Some(Target::Thing(stone));
    let w = g.step(&format!("write {glyphs} on it"));
    g.advance(40, Activity::Resting);
    g.scrape(stone);
    assert!(
        g.state.written.iter().any(|x| x.thing == stone),
        "{}",
        w.text
    );
    g.forced = Some(("clear", "daylight"));
    assert!(g.sealed_shut(s), "open by day");
    g.forced = Some(("clear", "dark"));
    assert!(!g.sealed_shut(s), "shut by night");
}

/// Sealed from within: the entrance room is open, every way on from it
/// holds until a counter is written over the ward on its wall.
#[test]
fn a_great_place_sealed_from_within() {
    for seed in 1..=8u64 {
        let mut g = Game::new(seed, pack());
        let Some(k) = g.site.writing.sealed.iter().position(|x| x.inner) else {
            continue;
        };
        g.trace = true;
        g.forced = Some(("clear", "daylight"));
        g.start();
        let x = g.site.writing.sealed[k].clone();
        let stone = g.ward_stone(x.structure).unwrap();
        g.state.place = Place::Outside;
        g.state.pos = g.site.land.structure_pos[x.structure];
        g.act("go", Target::Structure(x.structure));
        assert_eq!(
            g.state.place,
            Place::Room {
                structure: x.structure,
                room: 0
            }
        );
        let Some(way) = g.ways().into_iter().find(|w| !w.collapsed) else {
            continue;
        };
        let o = if way.state == crate::PassageState::Closed {
            g.act("open", Target::Way(way.link))
        } else {
            g.go_way(way.link)
        };
        assert!(
            o.renders.iter().any(|r| r.trace.slot == "effect.held"),
            "{}",
            o.text
        );
        g.make_item("stylus", true);
        g.make_item("graver", true);
        g.forced_light = true;
        g.threshold = 0;
        let mut strong = open_this(x.noun);
        if let Sentence::Clause(c) = &mut strong {
            c.adverbs.push("greatly".into());
        }
        let glyphs = glyphs_for(&mut g, &strong);
        g.state.it = Some(Target::Thing(stone));
        let w = g.step(&format!("write {glyphs} on it"));
        g.advance(40, Activity::Resting);
        g.scrape(stone);
        assert!(
            !g.sealed_shut(x.structure),
            "seed {seed}: still held: {}",
            w.text
        );
        return;
    }
    panic!("no world in 1..=8 sealed from within");
}
