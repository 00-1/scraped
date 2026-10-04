//! M08 properties: the three laws, the pivot, historic claims, and that no
//! description gives away what writing says.

use scraped_content::Pack;
use scraped_lang::english::translate;
use scraped_sim::fixtures::Spot;
use scraped_sim::writing::{claim_parts, Class, Property};
use scraped_world::history::EventKind;

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

/// Puts the player beside a latent spell (D10: written by history and
/// never cast; nothing places one beside a tool) with a knife in hand.
fn at_latent(seed: u64) -> Option<(Game, usize, usize)> {
    let mut g = Game::new(seed, pack());
    g.forced = Some(("clear", "daylight"));
    g.start();
    let knife = g.site.things.iter().position(|t| t.kind == "knife")?;
    g.state.carried.push(knife);
    for t in 0..g.site.things.len() {
        if !matches!(g.site.things[t].home, Place::Room { .. }) || g.site.things[t].texts.is_empty()
        {
            continue;
        }
        let layers = g.layers(t);
        let Some(top) = scraped_sim::writing::top_unscraped_of(&layers, &g.state.scraped) else {
            continue;
        };
        let Some(c) = g.site.writing.claim(&g.site.world, &g.site.land, top) else {
            continue;
        };
        if c.condition.is_some() || c.class == Class::Person {
            continue;
        }
        g.state.place = g.site.things[t].home;
        g.state.pos = g.site.things[t].pos;
        // Light enough to work by.
        if !g.is_dark() {
            return Some((g, t, top));
        }
    }
    None
}

#[test]
fn scraping_a_latent_spell_sets_it_loose_and_loses_nothing() {
    let mut tried = 0;
    for seed in SEEDS {
        let Some((mut g, thing, text)) = at_latent(seed) else {
            continue;
        };
        tried += 1;
        let layers_before: Vec<Vec<usize>> = g
            .site
            .writing
            .surfaces
            .iter()
            .map(|s| s.layers.clone())
            .collect();
        let scraped_before = g.state.scraped.clone();
        assert!(!g.claims.iter().any(|c| c.text == text));
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
        assert_eq!(
            g.state.scraped.len(),
            scraped_before.len() + 1,
            "seed {seed}"
        );
        // It acts now.
        assert!(g.claims.iter().any(|c| c.text == text), "seed {seed}");
        // Scraping again finds nothing fresh; the stack is unchanged.
        g.scrape(thing);
        assert_eq!(g.state.scraped.len(), scraped_before.len() + 1);
    }
    assert!(tried >= 4, "latent spells within reach on {tried} seeds");
}

/// D10: cleaning with a tool in hand releases potent writing exactly as
/// scraping does; by hand it takes off only the grime.
#[test]
fn cleaning_with_a_tool_releases_as_scraping_does() {
    for seed in SEEDS {
        let Some((mut a, thing, _)) = at_latent(seed) else {
            continue;
        };
        let (mut b, _, _) = at_latent(seed).unwrap();
        a.scrape(thing);
        b.clean(thing);
        assert_eq!(a.state.scraped, b.state.scraped, "seed {seed}");
        assert_eq!(a.claims, b.claims, "seed {seed}");
        // By hand: nothing scraped.
        let (mut c, thing, _) = at_latent(seed).unwrap();
        c.state.carried.clear();
        let before = c.state.scraped.clone();
        c.clean(thing);
        assert_eq!(c.state.scraped, before);
    }
}

/// D10: nothing places an unscraped spell beside a tool: the tools lie
/// where people kept them, and latent spells where history left them.
#[test]
fn no_spell_waits_beside_every_tool() {
    let mut rooms = 0;
    let mut beside = 0;
    for seed in SEEDS {
        let g = Game::new(seed, pack());
        for p in g
            .site
            .fixtures
            .items
            .iter()
            .filter(|p| scraped_sim::items::scrape_power(p.kind) > 0)
        {
            let Spot::Room { structure, room } = p.at else {
                continue;
            };
            rooms += 1;
            let here = Place::Room { structure, room };
            if (0..g.site.things.len()).any(|t| {
                g.site.things[t].home == here
                    && !g.site.things[t].texts.is_empty()
                    && scraped_sim::writing::top_unscraped_of(&g.layers(t), &g.state.scraped)
                        .is_some_and(|x| g.text(x).kind == scraped_lang::corpus::Kind::Potent)
            }) {
                beside += 1;
            }
        }
    }
    assert!(
        rooms > 0 && beside * 4 < rooms,
        "{beside} of {rooms} tool rooms hold a latent spell"
    );
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
        let Some((mut g, thing, pivot)) = at_latent(seed) else {
            continue;
        };
        g.spoil = false;
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
                let l = line.to_lowercase();
                assert!(
                    !(l.contains(&format!("{verb} the {subject}"))
                        || l.contains(&format!("{subject} {verb}"))
                        || l.contains(&format!("{verb} {subject}"))),
                    "seed {seed}: a line states the claim: {line}"
                );
            }
        }
    }
}

/// D09: every live spell that isn't waiting on a condition can be
/// perceived where it acts, through some cue (and none names it: the
/// cue slots carry qualities and things, never words or meanings).
#[test]
fn every_live_spell_is_perceptible_where_it_acts() {
    for seed in [1u64, 42] {
        let mut g = Game::new(seed, pack());
        g.forced = Some(("clear", "daylight"));
        g.start();
        let claims: Vec<_> = g
            .claims
            .iter()
            .filter(|c| c.condition.is_none() && c.class != Class::Person)
            .cloned()
            .collect();
        assert!(claims.len() > 100, "seed {seed}: {}", claims.len());
        let mut unseen = Vec::new();
        for c in &claims {
            if c.class.indoors() {
                g.state.place = Place::Room {
                    structure: c.structure,
                    room: 0,
                };
            } else {
                g.state.place = Place::Outside;
                g.state.pos = c.pos;
            }
            let mut facts = Vec::new();
            g.felt_facts(&mut facts);
            let seen = facts
                .iter()
                .any(|f| matches!(f.slot, "spell.cue" | "air.uncanny" | "danger.unstable"))
                || (c.property == Property::Openness && g.env().held(c.structure).is_some())
                || (c.property == Property::Stability && g.env().stability(c.structure).is_some())
                || c.property == Property::Heat;
            if !seen {
                unseen.push(format!(
                    "{} {} {:?} {:?}",
                    c.verb, c.subject, c.class, c.property
                ));
            }
        }
        assert!(unseen.is_empty(), "seed {seed}: {unseen:?}");
    }
}
