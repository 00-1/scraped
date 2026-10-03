//! M12 properties: conditions evaluate as written, placement is
//! deterministic and follows its rules, generated writing is real language,
//! every spine beat can be reached, and lint catches each problem.

use scraped_content::{Pack, PackFile, Storylet};
use scraped_lang::parse::{Mode, Parser};

use crate::composing_tests::{at_blank_surface, glyphs_for, pack};
use crate::site::Place;
use crate::storylets::{candidates, era_band, lint, HOOKS};
use crate::{Game, Target};

const SEEDS: [u64; 5] = [1, 3, 7, 42, 99];

/// The pack plus one extra file.
fn with(extra: &str) -> Pack {
    let mut p = pack();
    p.files
        .push(PackFile::parse("extra.toml", extra).expect("extra parses"));
    p
}

fn storylet(text: &str) -> Storylet {
    PackFile::parse("x.toml", text).unwrap().storylets.remove(0)
}

#[test]
fn conditions_evaluate_against_the_game() {
    let p = with(
        r#"
[[variant]]
slot = "story.torchlit"
text = "torchlit"

[[storylet]]
id = "torchlit"
about = "t"
when = "carrying has 'torch' and day >= 1 and not (flags has 'x')"
"#,
    );
    let mut g = Game::new(42, p);
    g.start();
    g.step("look");
    assert!(!g.state.story.happened.contains(&"torchlit".to_string()));
    g.make_item("torch", true);
    let out = g.step("look");
    assert!(g.state.story.happened.contains(&"torchlit".to_string()));
    assert!(out.text.contains("torchlit"));
    // Once only.
    let again = g.step("look");
    assert!(!again.text.contains("torchlit"));
}

#[test]
fn after_and_flags_chain_storylets() {
    let p = with(
        r#"
[[variant]]
slot = "story.a"
text = "first"

[[variant]]
slot = "story.b"
text = "second"

[[storylet]]
id = "a"
about = "a"
effects = ["flag seen_a", "give scraper"]

[[storylet]]
id = "b"
about = "b"
after = "a"
when = "flags has 'seen_a' and carrying has 'scraper'"
"#,
    );
    let mut g = Game::new(7, p);
    g.start();
    g.step("look");
    let happened: Vec<&String> = g
        .state
        .story
        .happened
        .iter()
        .filter(|h| *h != "opening")
        .collect();
    assert_eq!(happened, ["a", "b"]);
    assert!(g
        .state
        .carried
        .iter()
        .any(|&t| g.thing(t).kind == "scraper"));
}

#[test]
fn placement_is_deterministic_and_follows_its_rules() {
    const SRC: &str = r#"
[[storylet]]
id = "old_temple"
about = "x"
at = "structure"

[storylet.place]
structure = ["temple", "archive"]
era = "old"
away = true
"#;
    let s = storylet(SRC);
    let mut placed = 0;
    for seed in SEEDS {
        let g = Game::new(seed, with(SRC));
        let w = &g.site.world;
        let cands = candidates(w, g.site.settlement, &s);
        assert_eq!(cands, candidates(w, g.site.settlement, &s), "seed {seed}");
        for &c in &cands {
            let st = &w.structures[c];
            assert!(matches!(
                crate::site::label(&st.kind).as_str(),
                "temple" | "archive"
            ));
            assert_eq!(era_band(w, st.era), "old");
            assert_ne!(st.settlement, Some(g.site.settlement));
        }
        let again = Game::new(seed, g.pack.clone());
        assert_eq!(g.placed, again.placed, "seed {seed}");
        if let Some(p) = g.placed.iter().find(|p| p.id == "old_temple") {
            assert!(cands.contains(&p.structure));
            placed += 1;
        }
    }
    assert!(placed >= 3, "placed in only {placed} worlds");
}

#[test]
fn generated_writing_is_real_language_where_the_storylet_is() {
    let mut found = 0;
    for seed in SEEDS {
        let mut g = Game::new(seed, pack());
        g.start();
        let Some(p) = g.placed.iter().find(|p| p.id == "shrine").cloned() else {
            continue;
        };
        found += 1;
        let thing = p.thing.expect("the shrine has writing");
        let text = p.text.unwrap();
        assert_eq!(g.thing(thing).texts, [text]);
        let Place::Room { structure, .. } = g.thing(thing).home else {
            panic!("indoors");
        };
        assert_eq!(structure, p.structure);
        // It parses back to its meaning in its own era.
        let t = g.text(text).clone();
        let r = g.site.world.renderer(t.era);
        let parser = Parser::without_names(&r, Mode::Phonemes);
        let words = r.render(&t.meaning).words;
        assert_eq!(
            parser.sentence(&parser.tokens(&words)),
            Some(t.meaning.clone())
        );
        // Entering the building tells the storylet, once, and can be read.
        g.state.place = Place::Room { structure, room: 0 };
        g.state.pos = p.pos;
        let out = g.step("look");
        assert!(g.state.story.happened.contains(&"shrine".to_string()));
        assert!(g.state.story.flags.contains("shrine_seen"));
        assert!(!out.text.is_empty());
    }
    assert!(found >= 3, "shrine placed in only {found} worlds");
}

#[test]
fn a_spine_run_reaches_every_beat() {
    let (mut g, wall) = at_blank_surface(7).expect("a blank wall");
    // Opening.
    let mut g2 = Game::new(7, pack());
    g2.start();
    assert!(g2.hooks_seen.contains("opening"));
    assert!(g2.state.story.happened.contains(&"opening".to_string()));
    g.hooks_seen.insert("opening".to_string());
    // A tool picked up.
    let scraper = g.make_item("scraper", false).unwrap();
    g.act("take", Target::Thing(scraper));
    // The deepest text through the first lens (also scraped writing).
    let root = g.site.writing.deep[0];
    let t = (0..g.site.things.len())
        .find(|&t| g.site.things[t].texts.contains(&root))
        .unwrap();
    let back = (g.state.place, g.state.pos);
    g.state.place = g.site.things[t].home;
    g.state.pos = g.site.things[t].pos;
    g.make_item("first_lens", true);
    g.make_item("firesteel", true);
    let torch = g.make_item("torch", true).unwrap();
    g.light_item(torch);
    g.act("read", Target::Thing(t));
    (g.state.place, g.state.pos) = back;
    // A great inscription in a room.
    let great = g
        .site
        .greats
        .iter()
        .find_map(|gr| {
            (0..g.site.things.len()).find(|&t| {
                scraped_sim::writing::live_of(&g.layers(t), &g.state.scraped) == Some(gr.text)
                    && matches!(g.site.things[t].home, Place::Room { .. })
            })
        })
        .expect("a live great inscription indoors");
    g.state.place = g.site.things[great].home;
    g.state.pos = g.site.things[great].pos;
    g.step("look");
    (g.state.place, g.state.pos) = back;
    // Write and release a claim.
    let claim = crate::composing_tests::claim("burn", "house", false);
    g.threshold = 0;
    let out = g.step(&format!(
        "write {} on {}",
        glyphs_for(&g, &claim),
        g.site.things[wall].kind
    ));
    assert!(out.state.wrote.unwrap().accepted);
    g.advance(40, scraped_sim::body::Activity::Resting);
    g.scrape(wall);
    // The end.
    g.end_run("left");
    g.step("look");
    for h in HOOKS {
        assert!(g.hooks_seen.contains(*h), "beat {h} never reached");
    }
    for id in [
        "tool",
        "scraped_seen",
        "deepest",
        "great",
        "first_write",
        "release",
        "ending",
    ] {
        assert!(
            g.state.story.happened.contains(&id.to_string()),
            "{id} never happened"
        );
    }
}

#[test]
fn lint_catches_each_storylet_problem() {
    let kinds = |extra: &str| -> Vec<&'static str> {
        let p = with(extra);
        lint(&p)
            .into_iter()
            .filter(|i| i.slot.starts_with("story.x"))
            .map(|i| i.kind)
            .collect()
    };
    let base = "[[variant]]\nslot = \"story.x\"\ntext = \"x\"\n\n[[storylet]]\nid = \"x\"\nabout = \"x\"\n";
    assert!(kinds(base).is_empty(), "{:?}", kinds(base));
    let cases: &[(&str, &str)] = &[
        ("at = \"somewhere\"\n", "storylet-at"),
        ("at = \"hook\"\n", "storylet-hook"),
        ("at = \"hook\"\nhook = \"teatime\"\n", "storylet-hook"),
        ("when = \"day >\"\n", "storylet-condition"),
        ("when = \"mood == 'sad'\"\n", "storylet-condition"),
        ("after = \"nobody\"\n", "storylet-after"),
        ("after = \"x\"\n", "storylet-after"),
        ("effects = [\"give dragon\"]\n", "storylet-effect"),
        ("effects = [\"dance\"]\n", "storylet-effect"),
        ("effects = [\"flag a\", \"unflag a\"]\n", "storylet-conflict"),
        ("at = \"structure\"\n[storylet.place]\nstructure = [\"windmill\"]\n", "storylet-place"),
        ("at = \"structure\"\n[storylet.place]\nera = \"future\"\n", "storylet-place"),
        ("[storylet.inscription]\nregister = \"everyday\"\nabout = \"water\"\n", "storylet-inscription"),
        ("at = \"structure\"\n[storylet.inscription]\nregister = \"loud\"\nabout = \"water\"\n", "storylet-inscription"),
        ("at = \"structure\"\n[storylet.inscription]\nregister = \"potent\"\nabout = \"spaceship\"\n", "storylet-inscription"),
    ];
    for (extra, kind) in cases {
        let k = kinds(&format!("{base}{extra}"));
        assert!(k.contains(kind), "{extra:?} gave {k:?}, not {kind}");
    }
    // No text.
    let k = kinds("[[storylet]]\nid = \"x\"\nabout = \"x\"\n");
    assert!(k.contains(&"storylet-text"));
    // Duplicates.
    let k = kinds(&format!(
        "{base}\n[[storylet]]\nid = \"x\"\nabout = \"again\"\n"
    ));
    assert!(k.contains(&"storylet-duplicate"));
    // A beat with no storylet.
    let mut bare = pack();
    for f in &mut bare.files {
        f.storylets
            .retain(|s| s.hook.as_deref() != Some("deepest_found"));
    }
    assert!(lint(&bare)
        .iter()
        .any(|i| i.kind == "spine-unwritten" && i.slot == "spine.deepest_found"));
    // Never placed: no building can be both a mine and a bridge.
    let never = with(
        "[[variant]]\nslot = \"story.x\"\ntext = \"x\"\n\n[[storylet]]\nid = \"x\"\nabout = \"x\"\nat = \"structure\"\n[storylet.place]\nstructure = [\"bridge\"]\nbiome = [\"sea\"]\n",
    );
    let issues = crate::storylets::unplaceable(&never, &[1, 42]);
    assert!(issues.iter().any(|i| i.kind == "storylet-never"));
    // The real pack lints clean of storylet errors.
    assert!(lint(&pack())
        .iter()
        .all(|i| i.severity != scraped_content::Severity::Error));
}
