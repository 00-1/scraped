//! M10 properties: great inscriptions are within reach of the strongest
//! scraper, revisits notice real change only, and countering a great
//! inscription changes where its regions are heading.

use scraped_content::Pack;
use scraped_lang::meaning::{Mood, Polarity, Sentence};
use scraped_sim::body::Activity;
use scraped_sim::region::{CLIMATE, LIFE, STABILITY, WATER};

use crate::site::Place;
use crate::Game;

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

const SEEDS: [u64; 5] = [1, 3, 7, 42, 99];

/// The thing carrying a great inscription.
fn great_thing(g: &Game, text: usize) -> usize {
    (0..g.site.things.len())
        .find(|&t| g.site.things[t].texts.contains(&text))
        .expect("every great inscription is on a thing")
}

/// Writes the opposite claim over a great inscription with the strongest
/// scraper, and scrapes it.
pub(crate) fn counter(g: &mut Game, great: usize) -> bool {
    let text = g.site.greats[great].text;
    let thing = great_thing(g, text);
    g.state.place = g.site.things[thing].home;
    g.state.pos = g.site.things[thing].pos;
    g.forced = Some(("clear", "daylight"));
    g.threshold = 0;
    for k in ["first_scraper", "stylus", "torch", "firesteel"] {
        g.make_item(k, true);
    }
    let torch = *g
        .state
        .carried
        .iter()
        .find(|&&t| g.thing(t).kind == "torch")
        .unwrap();
    g.light_item(torch);
    // Fresh writing over it must come off first; that alone ends its power
    // (an inert layer becomes the live one).
    for _ in 0..12 {
        if !g
            .layers(thing)
            .last()
            .is_some_and(|t| !g.state.scraped.contains(t))
        {
            break;
        }
        g.scrape(thing);
    }
    if !g.claims.iter().any(|c| c.text == text) {
        return true;
    }
    let Sentence::Clause(mut c) = g.text(text).meaning.clone() else {
        return false;
    };
    assert_eq!(c.mood, Mood::Potent);
    c.polarity = match c.polarity {
        Polarity::Positive => Polarity::Negative,
        Polarity::Negative => Polarity::Positive,
    };
    let glyphs = crate::composing_tests::glyphs_for(g, &Sentence::Clause(c));
    let name = g.site.things[thing].kind;
    let out = g.step(&format!("write {glyphs} on {name}"));
    if out.state.wrote.as_ref().is_none_or(|w| !w.accepted) {
        return false;
    }
    g.advance(40, Activity::Resting);
    let torch = g.make_item("torch", true).unwrap();
    g.light_item(torch);
    g.scrape(thing);
    !g.claims.iter().any(|c| c.text == text)
}

#[test]
fn every_great_inscription_is_reachable_and_affectable_with_the_strongest_tool() {
    for seed in SEEDS {
        let base = Game::new(seed, pack());
        assert!(
            !base.site.greats.is_empty(),
            "seed {seed}: no great inscriptions"
        );
        for (i, gr) in base.site.greats.iter().enumerate() {
            let t = base.text(gr.text);
            let rooms = base
                .site
                .fixtures
                .reachable_rooms(&base.site.world, t.structure);
            assert!(
                t.room.is_none_or(|r| rooms.contains(&r)),
                "seed {seed}: great {i} lies behind rubble or water"
            );
            assert!(
                base.site
                    .land
                    .route(
                        &base.site.world,
                        base.site.start(),
                        base.site.land.structure_pos[t.structure]
                    )
                    .is_some(),
                "seed {seed}: great {i} unreachable on foot"
            );
            // With a weaker scraper it won't bite; with the first scraper it can
            // be overwritten and its power ends.
            let mut g = Game::new(seed, pack());
            g.start();
            let thing = great_thing(&g, gr.text);
            g.make_item("old_scraper", true);
            assert!(g.power() < g.needs_power(thing));
            assert!(
                counter(&mut g, i),
                "seed {seed}: great {i} could not be countered"
            );
        }
    }
}

#[test]
fn countering_a_great_inscription_changes_where_its_regions_head() {
    let mut changed = 0;
    for seed in SEEDS {
        let base = Game::new(seed, pack());
        for (i, gr) in base.site.greats.iter().enumerate() {
            let region = gr.region;
            let mut left = Game::new(seed, pack());
            left.start();
            let mut acted = Game::new(seed, pack());
            acted.start();
            if !counter(&mut acted, i) {
                continue;
            }
            for g in [&mut left, &mut acted] {
                let day = g.day() + 120;
                let drivers = g.drivers.clone();
                g.site.regions.advance(&mut g.state.regions, day, &drivers);
            }
            let a = acted.state.regions.vars[region];
            let b = left.state.regions.vars[region];
            if (0..4).any(|v| a[v] != b[v]) {
                changed += 1;
            }
            let _ = (LIFE, WATER, STABILITY, CLIMATE);
        }
    }
    assert!(changed >= 3, "only {changed} counters changed their region");
}

#[test]
fn revisits_notice_only_real_change() {
    use crate::attention::Response;
    let mut g = Game::new(42, pack());
    g.start();
    g.state.place = Place::Outside;
    // What the water of the region shows, if anything.
    let water = |g: &mut Game| -> Option<String> {
        g.outdoor_facts(Response::Look)
            .into_iter()
            .find(|f| f.key == "region:water")
            .map(|f| f.vars.get("evidence").map(|v| v.text()).unwrap_or_default())
    };
    let r = g.site.regions.at(g.state.pos).unwrap();
    // A small drift that crosses no band shows the same evidence.
    g.state.regions.vars[r][WATER] = 50;
    let low = water(&mut g);
    assert!(low.is_some(), "very low water shows");
    g.state.regions.vars[r][WATER] = 150;
    assert_eq!(water(&mut g), low, "a change within a band is not news");
    // A real change, across bands, shows something else.
    g.state.regions.vars[r][WATER] = 300;
    let after = water(&mut g);
    assert!(
        after.is_some() && after != low,
        "crossing a band is noticed"
    );
    // Middling water shows nothing at all.
    g.state.regions.vars[r][WATER] = 500;
    assert_eq!(water(&mut g), None);
}
