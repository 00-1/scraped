//! M08 properties: the three laws, the pivot, historic claims, and that no
//! description gives away what writing says.

use scraped_content::Pack;
use scraped_lang::english::translate;
use scraped_sim::fixtures::Spot;
use scraped_sim::writing::{claim_parts, Class, Property};
use scraped_world::history::EventKind;
use scraped_world::structures::{Passage, PassageState};

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

const SEEDS: [u64; 6] = [1, 3, 7, 42, 99, 2024];

/// Puts the player beside the pivot inscription with the scraper in hand.
fn at_pivot(seed: u64) -> (Game, usize) {
    let mut g = Game::new(seed, pack());
    g.forced = Some(("clear", "daylight"));
    g.start();
    let scraper = g
        .site
        .things
        .iter()
        .position(|t| t.kind == "scraper")
        .expect("a scraper lies somewhere");
    let pivot = g.site.writing.pivot.expect("a pivot inscription");
    let surface = g.site.writing.surface_of(pivot).unwrap();
    let thing = g
        .site
        .things
        .iter()
        .position(|t| t.surface == Some(surface))
        .expect("the pivot is on a thing");
    g.state.carried.push(scraper);
    g.state.place = g.site.things[thing].home;
    g.state.pos = g.site.things[thing].pos;
    (g, thing)
}

#[test]
fn the_scraper_and_a_safe_first_inscription_are_reachable() {
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        let tool = g
            .site
            .fixtures
            .items
            .iter()
            .find(|p| p.kind == "scraper")
            .unwrap_or_else(|| panic!("seed {seed}: no scraper"));
        let Spot::Room { structure, room } = tool.at else {
            panic!("scraper outdoors")
        };
        // On foot from the start, and inside by ordinary means.
        assert!(
            g.site
                .land
                .route(
                    &g.site.world,
                    g.site.start(),
                    g.site.land.structure_pos[structure]
                )
                .is_some(),
            "seed {seed}: scraper unreachable on foot"
        );
        let rooms = g.site.fixtures.reachable_rooms(&g.site.world, structure);
        assert!(
            rooms.contains(&room),
            "seed {seed}: scraper behind rubble or water"
        );
        // The pivot is beside it, latent, and does something safe and plain.
        let pivot = g
            .site
            .writing
            .pivot
            .unwrap_or_else(|| panic!("seed {seed}: no pivot"));
        let t = g.text(pivot);
        assert_eq!(
            t.structure, structure,
            "seed {seed}: pivot away from the tool"
        );
        assert!(rooms.contains(&t.room.unwrap()));
        assert!(!g.state.scraped.contains(&pivot));
        let claim = g
            .site
            .writing
            .claim(&g.site.world, &g.site.land, pivot)
            .unwrap();
        assert!(
            claim.amount > 0,
            "seed {seed}: the pivot should open or warm, not seal or chill"
        );
        assert!(matches!(
            claim.property,
            Property::Openness | Property::Heat
        ));
    }
}

#[test]
fn scraping_the_pivot_changes_the_world_and_loses_nothing() {
    for seed in SEEDS {
        let (mut g, thing) = at_pivot(seed);
        let layers_before: Vec<Vec<usize>> = g
            .site
            .writing
            .surfaces
            .iter()
            .map(|s| s.layers.clone())
            .collect();
        let scraped_before = g.state.scraped.clone();
        let Place::Room { structure, .. } = g.state.place else {
            panic!()
        };
        let heat_before = g.env().room_heat(structure);
        let held_before = g.env().held(structure);
        let out = g.scrape(thing);
        assert!(!out.text.is_empty());
        // Nothing lost: every layer still there, and the scraped set only grew.
        let layers_after: Vec<Vec<usize>> = g
            .site
            .writing
            .surfaces
            .iter()
            .map(|s| s.layers.clone())
            .collect();
        assert_eq!(layers_before, layers_after);
        assert!(g.state.scraped.is_superset(&scraped_before));
        assert_eq!(g.state.scraped.len(), scraped_before.len() + 1);
        // The world changed, as the claim said.
        let pivot = g.site.writing.pivot.unwrap();
        let (verb, _, _) = claim_parts(g.text(pivot)).unwrap();
        match verb.as_str() {
            "open" => {
                assert_eq!(held_before, None);
                assert_eq!(g.env().held(structure), Some(1), "seed {seed}");
                // Every unbarred door in the building now stands open.
                let open = g
                    .site
                    .structure(structure)
                    .interior
                    .links
                    .iter()
                    .filter(|l| l.passage == Passage::Door)
                    .count();
                assert!(open > 0);
            }
            _ => assert_eq!(
                g.env().room_heat(structure),
                heat_before + 12,
                "seed {seed}"
            ),
        }
        // Scraping again finds nothing fresh; the stack is unchanged.
        g.scrape(thing);
        assert_eq!(g.state.scraped.len(), scraped_before.len() + 1);
    }
}

#[test]
fn every_historic_writing_event_leaves_a_trace_and_an_effect() {
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        let w = &g.site.world;
        for e in &w.history.events {
            let EventKind::Writing { .. } = e.kind else {
                continue;
            };
            let text = w
                .texts
                .iter()
                .find(|t| t.event == Some(e.id))
                .unwrap_or_else(|| panic!("seed {seed}: writing event {} left no text", e.id));
            assert!(
                g.state.scraped.contains(&text.id),
                "seed {seed}: history's cast is not scraped"
            );
            let claim = g
                .site
                .writing
                .claim(w, &g.site.land, text.id)
                .unwrap_or_else(|| panic!("seed {seed}: event {} makes a vague claim", e.id));
            // Something perceptible lies within its reach.
            let touched = match claim.class {
                Class::Land => true,
                _ => w
                    .structures
                    .iter()
                    .any(|st| g.site.land.structure_pos[st.id].dist(claim.pos) <= claim.range),
            };
            assert!(touched, "seed {seed}: event {} acts on nothing", e.id);
        }
    }
}

#[test]
fn live_claims_are_the_top_scraped_layers_only() {
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        let live: Vec<usize> = g.claims.iter().map(|c| c.text).collect();
        for (i, s) in g.site.writing.surfaces.iter().enumerate() {
            let top = s.layers.iter().rev().find(|t| g.state.scraped.contains(t));
            for t in &s.layers {
                if live.contains(t) {
                    assert_eq!(Some(t), top, "seed {seed}: surface {i} has a ghost acting");
                }
            }
        }
    }
}

#[test]
fn reading_a_scraped_layer_shows_only_part_and_never_the_meaning() {
    for seed in SEEDS {
        let (mut g, thing) = at_pivot(seed);
        g.spoil = false;
        let pivot = g.site.writing.pivot.unwrap();
        let r = g.site.world.renderer(g.text(pivot).era);
        let gloss = translate(&g.text(pivot).meaning, &|p| r.name(p));
        let mut texts = Vec::new();
        texts.push(g.step("look").text);
        let full = g.act("read", Target::Thing(thing)).text;
        texts.push(g.scrape(thing).text);
        let partial = g.act("read", Target::Thing(thing)).text;
        texts.push(g.step("look").text);
        texts.push(full.clone());
        texts.push(partial.clone());
        assert_ne!(
            full, partial,
            "seed {seed}: scraping changed nothing in the reading"
        );
        for t in &texts {
            assert!(!t.contains(&gloss), "seed {seed}: the meaning shows: {t}");
            let (verb, subject, _) = claim_parts(g.text(pivot)).unwrap();
            for line in t.lines() {
                assert!(
                    !(line.contains(&format!(" {verb} ")) && line.contains(&format!(" {subject}"))),
                    "seed {seed}: a line states the claim: {line}"
                );
            }
        }
    }
}

#[test]
fn held_doors_will_not_move_by_hand() {
    for seed in SEEDS {
        let (mut g, thing) = at_pivot(seed);
        let pivot = g.site.writing.pivot.unwrap();
        if claim_parts(g.text(pivot)).unwrap().0 != "open" {
            continue;
        }
        g.scrape(thing);
        let Place::Room { structure, room } = g.state.place else {
            panic!()
        };
        for w in g.ways() {
            if w.passage == Some(Passage::Door) && !w.collapsed && w.state != PassageState::Blocked
            {
                assert_eq!(
                    w.state,
                    PassageState::Open,
                    "seed {seed}: room {room} of {structure}"
                );
                let before = g.ways();
                g.door(false, w.exit);
                assert_eq!(before, g.ways(), "seed {seed}: a held door was closed");
            }
        }
    }
}

#[test]
#[ignore]
fn show_the_pivot() {
    let (mut g, thing) = at_pivot(42);
    println!("{}", g.step("look").text);
    println!("--- read\n{}", g.act("read", Target::Thing(thing)).text);
    println!("--- scrape\n{}", g.scrape(thing).text);
    println!("--- look\n{}", g.step("look").text);
    println!("--- read\n{}", g.act("read", Target::Thing(thing)).text);
}
