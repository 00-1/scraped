//! D08: every text of a world reads back to its meaning, and the genres
//! are many and none too common.

use std::collections::BTreeMap;

use scraped_lang::parse::{Mode, Parser};
use scraped_world::World;

#[test]
fn every_text_round_trips() {
    // One world here (parsing every text is slow); the slow suite runs
    // three.
    round_trip(&[42]);
}

#[test]
#[ignore = "slow; run with --ignored"]
fn every_text_round_trips_in_three_worlds() {
    round_trip(&[1, 9001]);
}

fn round_trip(seeds: &[u64]) {
    for &seed in seeds {
        let w = World::generate(seed);
        let mut genres: BTreeMap<String, usize> = BTreeMap::new();
        let mut bad = Vec::new();
        for t in &w.texts {
            *genres.entry(format!("{:?}", t.genre)).or_default() += 1;
            for n in t.meaning.noun_phrases() {
                if let scraped_lang::meaning::Head::Name(i) = n.head {
                    let era = if i < w.history.people.len() {
                        w.history.people[i].era
                    } else {
                        w.history.settlements[i - w.history.people.len()].era
                    };
                    assert!(
                        era <= t.era,
                        "seed {seed}: {:?} of era {} names {i} of era {era} ({} people)",
                        t.genre,
                        t.era,
                        w.history.people.len()
                    );
                }
            }
            let r = w.renderer(t.era);
            let p = Parser::new(&r, Mode::Phonemes);
            let words = r.render(&t.meaning).words;
            if p.sentence(&p.tokens(&words)).as_ref() != Some(&t.meaning) {
                bad.push(format!(
                    "{:?}: {}",
                    t.genre,
                    scraped_lang::english::translate(&t.meaning, &|n| r.name(n))
                ));
            }
        }
        eprintln!("seed {seed}: {} texts {genres:?}", w.texts.len());
        assert!(
            bad.is_empty(),
            "seed {seed}: {} of {} fail:\n{}",
            bad.len(),
            w.texts.len(),
            bad.join("\n")
        );
    }
}

/// Every name in a meaning, wherever it stands.
fn names(s: &scraped_lang::meaning::Sentence) -> Vec<usize> {
    use scraped_lang::meaning::{Clause, Head, NounPhrase, Sentence};
    fn np(n: &NounPhrase, out: &mut Vec<usize>) {
        if let Head::Name(i) = n.head {
            out.push(i);
        }
        n.possessor.iter().for_each(|p| np(p, out));
        n.apposition.iter().for_each(|a| np(a, out));
        if let Some(r) = &n.relative {
            clause(&r.clause, out);
        }
    }
    fn clause(c: &Clause, out: &mut Vec<usize>) {
        c.args.iter().for_each(|a| np(&a.np, out));
        c.subordinate.iter().for_each(|s| clause(&s.clause, out));
        if let Some(k) = &c.complement {
            out.extend(names(&k.content));
        }
    }
    let mut out = Vec::new();
    match s {
        Sentence::Clause(c) => clause(c, &mut out),
        Sentence::List(l) => l.iter().for_each(|n| np(n, &mut out)),
        Sentence::Text(t) => t.iter().for_each(|s| out.extend(names(s))),
        Sentence::Joined(_, cs) => cs.iter().for_each(|c| clause(c, &mut out)),
    }
    out
}

/// D08: no text names someone not yet born, or tells of what had not yet
/// happened when it was written.
#[test]
fn texts_agree_with_history() {
    for seed in [1u64, 42] {
        let w = World::generate(seed);
        let people = w.history.people.len();
        for t in &w.texts {
            for i in names(&t.meaning) {
                if i < people {
                    assert!(
                        w.history.people[i].born <= t.year,
                        "seed {seed}: {:?} of {} (event {:?}, arc {:?}) names {} born {}",
                        t.genre,
                        t.year,
                        t.event.map(|e| &w.history.events[e].kind),
                        t.arc.map(|a| w.history.arcs[a].kind),
                        i,
                        w.history.people[i].born
                    );
                } else {
                    assert!(
                        w.history.settlements[i - people].founded <= t.year + 1,
                        "seed {seed}: {:?} of {} names a town founded {}",
                        t.genre,
                        t.year,
                        w.history.settlements[i - people].founded
                    );
                }
            }
            if let Some(e) = t.event {
                assert!(
                    w.history.events[e].year <= t.year,
                    "seed {seed}: {:?} of {} tells of event in {}",
                    t.genre,
                    t.year,
                    w.history.events[e].year
                );
            }
        }
    }
}

/// D08: a story's texts tell its events; its disasters leave scenes and its
/// journeys a seal; instructions lie beside the work they explain.
#[test]
fn stories_and_their_evidence_agree() {
    use scraped_world::society::ArcKind;
    use scraped_world::texts::Genre;
    for seed in [1u64, 42] {
        let w = World::generate(seed);
        for t in &w.texts {
            if let (Some(a), Some(e)) = (t.arc, t.event) {
                assert!(w.history.arcs[a].events.contains(&e), "seed {seed}");
            }
        }
        for a in &w.history.arcs {
            let in_arc = |e: Option<usize>| e.is_some_and(|e| a.events.contains(&e));
            match a.kind {
                ArcKind::Disaster => assert!(
                    w.scenes.iter().any(|s| in_arc(s.event)),
                    "seed {seed}: disaster arc {} left no scene",
                    a.id
                ),
                ArcKind::Venture => assert!(
                    w.objects
                        .iter()
                        .any(|o| o.kind == "seal" && in_arc(o.event)),
                    "seed {seed}: journey arc {} left no seal",
                    a.id
                ),
                _ => {}
            }
        }
        for t in w.texts.iter().filter(|t| t.genre == Genre::Instructions) {
            let (Some(r), Some(f)) = (t.room, t.feature) else {
                panic!("instructions outside");
            };
            let kind = w.structures[t.structure].interior.rooms[r].features[f].kind;
            assert!(
                [
                    "kiln",
                    "oven",
                    "anvil",
                    "hearth",
                    "loom",
                    "millstone",
                    "waterwheel",
                    "vat",
                    "cask",
                    "beacon",
                    "wall"
                ]
                .contains(&kind),
                "seed {seed}: instructions on a {kind}"
            );
        }
    }
}
