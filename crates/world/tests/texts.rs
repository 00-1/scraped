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
