//! M09 properties: the understanding gate, agreement, misfires, and an
//! end-to-end decipherer that composes and casts a claim from the grammar.

use scraped_content::Pack;
use scraped_lang::meaning::{Argument, Clause, Mood, NounPhrase, Polarity, Role, Sentence, Tense};
use scraped_lang::morphology::Number;
use scraped_sim::body::Activity;

use crate::composing::agrees;
use crate::site::Place;
use crate::Game;

pub(crate) fn pack() -> Pack {
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

pub(crate) fn claim(verb: &str, subject: &str, negative: bool) -> Sentence {
    Sentence::Clause(Clause {
        predicate: verb.into(),
        mood: Mood::Potent,
        tense: Tense::NonPast,
        polarity: if negative {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args: vec![Argument {
            role: Role::Subject,
            np: NounPhrase::concept(subject),
        }],
        adverbs: Vec::new(),
    })
}

/// A blank writable surface in a reachable room, with the player beside it
/// holding a stylus and a scraper, in daylight.
pub(crate) fn at_blank_surface(seed: u64) -> Option<(Game, usize)> {
    let mut g = Game::new(seed, pack());
    g.forced = Some(("clear", "daylight"));
    g.start();
    let thing = (0..g.site.things.len()).find(|&t| {
        let th = &g.site.things[t];
        let Place::Room { structure, room } = th.home else {
            return false;
        };
        th.texts.is_empty()
            && !th.portable
            && matches!(th.kind, "wall" | "stele" | "altar" | "niche")
            && g.site.structure(structure).interior.rooms[room].level >= 0
            && g.site
                .fixtures
                .reachable_rooms(&g.site.world, structure)
                .contains(&room)
    })?;
    g.state.place = g.site.things[thing].home;
    g.state.pos = g.site.things[thing].pos;
    g.make_item("stylus", true);
    g.make_item("scraper", true);
    Some((g, thing))
}

/// The glyph numbers a meaning is written with, in the player's era.
pub(crate) fn glyphs_for(g: &Game, m: &Sentence) -> String {
    let era = g.writing_era();
    let r = g.site.world.renderer(era as u32);
    let script = &g.site.world.languages[era].script;
    r.glyphs(&r.render(m))
        .into_iter()
        .map(|k| match k {
            Some(k) => script.index(&k).to_string(),
            None => "/".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn agreement_fixtures() {
    let ghost = claim("burn", "field", false);
    // Same register, same slots, the same kind of noun: a completion.
    assert!(agrees(&ghost, true, Some(&claim("burn", "house", true))));
    assert!(agrees(&ghost, true, Some(&claim("open", "gate", false))));
    // Another kind of noun in the subject slot.
    assert!(!agrees(&ghost, true, Some(&claim("burn", "tree", false))));
    // Wrong register over a potent trace, and the reverse.
    let mut plain = claim("burn", "tree", false);
    if let Sentence::Clause(c) = &mut plain {
        c.mood = Mood::Declarative;
    }
    assert!(!agrees(&ghost, true, Some(&plain)));
    assert!(!agrees(&plain, false, Some(&ghost)));
    // Number must match.
    let mut many = claim("burn", "tree", false);
    if let Sentence::Clause(c) = &mut many {
        c.args[0].np.number = Number::Plural;
    }
    assert!(!agrees(&ghost, true, Some(&many)));
    // Gibberish never fits a trace.
    assert!(!agrees(&ghost, true, None));
}

#[test]
fn the_gate_refuses_unfamiliar_words_and_is_configurable() {
    let (mut g, thing) = at_blank_surface(42).expect("a blank wall");
    let glyphs = glyphs_for(&g, &claim("burn", "house", false));
    let name = g.site.things[thing].kind;
    let out = g.step(&format!("write {glyphs} on {name}"));
    assert_eq!(
        out.state.wrote.as_ref().map(|w| w.accepted),
        Some(false),
        "{}",
        out.text
    );
    assert_eq!(
        out.state.wrote.unwrap().refused.as_deref(),
        Some("hesitate")
    );
    g.threshold = 0;
    let out = g.step(&format!("write {glyphs} on {name}"));
    assert_eq!(
        out.state.wrote.as_ref().map(|w| w.accepted),
        Some(true),
        "{}",
        out.text
    );
}

#[test]
fn a_decipherer_composes_and_casts_a_claim_on_most_seeds() {
    let seeds = [1u64, 3, 7, 42, 99];
    let mut cast = 0;
    for seed in seeds {
        let Some((mut g, thing)) = at_blank_surface(seed) else {
            continue;
        };
        g.threshold = 0;
        let Place::Room { structure, .. } = g.state.place else {
            unreachable!()
        };
        let before = g.env().room_heat(structure);
        let glyphs = glyphs_for(&g, &claim("burn", "house", false));
        let name = g.site.things[thing].kind;
        let wrote = g.step(&format!("write {glyphs} on {name}"));
        if wrote.state.wrote.as_ref().is_none_or(|w| !w.accepted) {
            continue;
        }
        g.advance(40, Activity::Resting);
        g.scrape(thing);
        if g.env().room_heat(structure) == before + 12 || g.env().room_heat(structure) > before {
            cast += 1;
        }
    }
    assert!(cast >= 4, "only {cast} of {} seeds", seeds.len());
}

#[test]
fn misfires_are_deterministic_and_negation_bites() {
    let run = |text: &str| {
        let (mut g, thing) = at_blank_surface(7).unwrap();
        g.threshold = 0;
        let glyphs = if text == "garbled" {
            // The potent opening formula, then nonsense.
            let era = g.writing_era();
            let r = g.site.world.renderer(era as u32);
            let script = &g.site.world.languages[era].script;
            let lang = &g.site.world.languages[era];
            let ipa = lang.phonology.to_ipa(&r.plain_word("pot.open").phonemes());
            let mut v: Vec<String> = script
                .spell(&ipa)
                .iter()
                .map(|k| script.index(k).to_string())
                .collect();
            v.extend(["/".into(), "1".into(), "2".into(), "1".into()]);
            v.join(" ")
        } else {
            glyphs_for(&g, &claim("burn", "house", true))
        };
        let name = g.site.things[thing].kind;
        let w = g.step(&format!("write {glyphs} on {name}")).text;
        g.advance(40, Activity::Resting);
        let s = g.scrape(thing).text;
        (w, s, g.state.body.clone(), g.claims.len())
    };
    // The same wrong text, the same result.
    assert_eq!(run("garbled"), run("garbled"));
    let (_, _, body, _) = run("garbled");
    assert_eq!(body.injury, 1, "a garbled potent text turns on its writer");
    // An unintended negation chills the room instead of warming it.
    let (_, _, _, claims) = run("negated");
    let (mut g, thing) = at_blank_surface(7).unwrap();
    let Place::Room { structure, .. } = g.site.things[thing].home else {
        unreachable!()
    };
    let base = g.env().room_heat(structure);
    g.threshold = 0;
    let glyphs = glyphs_for(&g, &claim("burn", "house", true));
    let name = g.site.things[thing].kind;
    g.step(&format!("write {glyphs} on {name}"));
    g.advance(40, Activity::Resting);
    g.scrape(thing);
    assert!(
        g.env().room_heat(structure) < base,
        "negated warmth should chill"
    );
    assert!(claims > 0);
}

#[test]
fn reading_records_roots_and_the_lens_reads_beneath() {
    let mut g = Game::new(42, pack());
    g.start();
    // Find a surface with a scraped layer above an older one.
    let thing = (0..g.site.things.len()).find(|&t| {
        let layers = g.layers(t);
        scraped_sim::writing::beneath_of(&layers, &g.state.scraped)
            .is_some_and(|b| !g.site.writing.deep.contains(&b))
    });
    let Some(thing) = thing else { return };
    g.state.place = g.site.things[thing].home;
    g.state.pos = g.site.things[thing].pos;
    g.forced = Some(("clear", "daylight"));
    g.make_item("torch", true);
    g.make_item("firesteel", true);
    let torch = g.state.carried[0];
    g.light_item(torch);
    let plain = g.act("read", crate::Target::Thing(thing)).text;
    assert!(!g.state.encountered.is_empty(), "reading should note roots");
    g.make_item("lens", true);
    let deep = g.act("read", crate::Target::Thing(thing)).text;
    assert_ne!(plain, deep, "the lens shows more");
}

/// D03: every room purpose and feature in the world's kinds table is a
/// value the slots know.
#[test]
fn slot_lists_cover_every_kind_of_room_and_thing() {
    let mut missing = Vec::new();
    for k in scraped_world::structures::StructureKind::ALL {
        for r in k.info().rooms {
            if !crate::slots::PURPOSES.contains(&r.purpose) {
                missing.push(r.purpose);
            }
            for (f, _) in r.features {
                if !crate::slots::KINDS.contains(f) {
                    missing.push(f);
                }
            }
        }
    }
    missing.dedup();
    assert!(missing.is_empty(), "{missing:?}");
}
