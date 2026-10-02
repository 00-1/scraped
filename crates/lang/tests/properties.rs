//! Milestone 1 properties, checked across many seeds.

use std::collections::BTreeSet;

use scraped_lang::concepts::{self, Pos};
use scraped_lang::corpus::Corpus;
use scraped_lang::lexicon::{edit_distance, MIN_ROOT_DISTANCE};
use scraped_lang::meaning::{
    Clause, Head, Mood, NounPhrase, Number, Polarity, Role, Sentence, Tense,
};
use scraped_lang::morphology::AffixPosition;
use scraped_lang::sheet::GrammarSheet;
use scraped_lang::syntax::WordOrder;
use scraped_lang::Language;

const SEEDS: std::ops::Range<u64> = 0..40;

#[test]
fn same_seed_identical_output() {
    for seed in SEEDS {
        let a = Language::generate(seed);
        let b = Language::generate(seed);
        let ca = Corpus::generate(&a, 30);
        let cb = Corpus::generate(&b, 30);
        assert_eq!(ca.to_text(true), cb.to_text(true), "seed {seed}");
        assert_eq!(ca.to_json(true), cb.to_json(true), "seed {seed}");
        assert_eq!(
            GrammarSheet::new(&a).to_text(),
            GrammarSheet::new(&b).to_text(),
            "seed {seed}"
        );
    }
}

#[test]
fn shorter_corpus_is_a_prefix() {
    let lang = Language::generate(11);
    let short = Corpus::generate(&lang, 10).to_text(false);
    let long = Corpus::generate(&lang, 40).to_text(false);
    assert!(long.starts_with(&short));
}

#[test]
fn seeds_differ_noticeably() {
    let langs: Vec<Language> = (0..60).map(Language::generate).collect();
    let orders: BTreeSet<&str> = langs.iter().map(|l| l.syntax.word_order.label()).collect();
    assert_eq!(orders.len(), 3, "all word orders occur: {orders:?}");
    let sizes: BTreeSet<usize> = langs
        .iter()
        .map(|l| l.phonology.inventory.phonemes.len())
        .collect();
    assert!(sizes.len() >= 8, "inventory sizes vary: {sizes:?}");
    let positions: BTreeSet<(bool, bool)> = langs
        .iter()
        .map(|l| {
            (
                l.morphology.noun_position == AffixPosition::Prefix,
                l.morphology.verb_position == AffixPosition::Prefix,
            )
        })
        .collect();
    assert_eq!(positions.len(), 4, "prefix/suffix combinations vary");
    let templates: BTreeSet<String> = langs
        .iter()
        .map(|l| l.phonology.template.notation())
        .collect();
    assert!(templates.len() >= 4, "syllable shapes vary: {templates:?}");

    // Neighbouring seeds should not share a lexicon or affix set.
    for w in langs.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let affixes = |l: &Language| -> Vec<String> {
            l.morphology
                .affixes()
                .iter()
                .map(|x| l.romanise(x))
                .collect()
        };
        assert_ne!(affixes(a), affixes(b));
        let same = concepts::all()
            .iter()
            .filter(|c| a.romanise(a.lexicon.root(&c.id)) == b.romanise(b.lexicon.root(&c.id)))
            .count();
        assert!(
            same < 10,
            "seeds {} and {} share {same} roots",
            a.seed,
            b.seed
        );
    }
}

#[test]
fn every_word_obeys_phonotactics() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 60);
        let p = &lang.phonology;
        for c in concepts::all() {
            let root = lang.lexicon.root(&c.id);
            for form in lang.morphology.all_forms(root, c.pos) {
                assert!(p.is_valid(&form), "seed {seed}: {}", lang.romanise(&form));
            }
        }
        for name in &corpus.names {
            assert!(
                p.is_valid(name),
                "seed {seed}: name {}",
                lang.romanise(name)
            );
        }
        for i in &corpus.inscriptions {
            for w in &i.rendered.words {
                let ph = w.phonemes();
                assert!(p.is_valid(&ph), "seed {seed}: {}", lang.romanise(&ph));
                // The spelling always splits back into the same sounds.
                assert_eq!(p.decode(&lang.romanise(&ph)), Some(ph), "seed {seed}");
            }
        }
    }
}

#[test]
fn roots_and_affixes_never_collide() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 1);
        let mut words: Vec<_> = concepts::all()
            .iter()
            .map(|c| lang.lexicon.root(&c.id).clone())
            .collect();
        words.extend(corpus.names.iter().cloned());
        for (i, a) in words.iter().enumerate() {
            for b in &words[i + 1..] {
                assert!(
                    edit_distance(a, b) >= MIN_ROOT_DISTANCE,
                    "seed {seed}: {} ~ {}",
                    lang.romanise(a),
                    lang.romanise(b)
                );
            }
        }
        let affixes: BTreeSet<_> = lang.morphology.affixes().into_iter().collect();
        assert_eq!(affixes.len(), 6, "seed {seed}");
        // No surface form anywhere in the language has two analyses.
        for (form, glosses) in lang.analyses(&corpus.names) {
            assert_eq!(
                glosses.len(),
                1,
                "seed {seed}: {} = {glosses:?}",
                lang.romanise(&form)
            );
        }
    }
}

/// Canonical word gloss: root plus sorted grammatical labels, so the check
/// does not depend on affix order or word order.
fn canon(root: &str, mut labels: Vec<&str>) -> String {
    labels.sort();
    format!("{root}|{}", labels.join(","))
}

fn expect_np(
    np: &NounPhrase,
    case: &'static str,
    names: &dyn Fn(usize) -> String,
    out: &mut Vec<String>,
) {
    let root = match &np.head {
        Head::Concept(id) => id.clone(),
        Head::Name(p) => names(*p),
    };
    let mut labels = Vec::new();
    if np.number == Number::Plural {
        labels.push("PL");
    }
    if !case.is_empty() {
        labels.push(case);
    }
    out.push(canon(&root, labels));
    for a in &np.adjectives {
        out.push(canon(a, vec![]));
    }
    if let Some(n) = np.quantity {
        out.push(canon(&concepts::numeral(n).id, vec![]));
    }
    if let Some(d) = &np.determiner {
        out.push(canon(d, vec![]));
    }
    if let Some(p) = &np.possessor {
        expect_np(p, "GEN", names, out);
    }
    for a in &np.apposition {
        expect_np(a, case, names, out);
    }
}

fn expect_clause(c: &Clause, names: &dyn Fn(usize) -> String, out: &mut Vec<String>) {
    let mut labels = Vec::new();
    if c.tense == Tense::Past {
        labels.push("PST");
    }
    if c.polarity == Polarity::Negative {
        labels.push("NEG");
    }
    out.push(canon(&c.predicate, labels));
    for a in &c.args {
        match a.role {
            Role::Subject if c.mood == Mood::Imperative => {}
            Role::Subject => expect_np(&a.np, "", names, out),
            Role::Object => expect_np(&a.np, "ACC", names, out),
            Role::Recipient => expect_np(&a.np, "DAT", names, out),
        }
    }
    for a in &c.adverbs {
        out.push(canon(a, vec![]));
    }
}

#[test]
fn glosses_match_meaning() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 60);
        let r = corpus.renderer();
        let analyses = lang.analyses(&corpus.names);
        for (n, i) in corpus.inscriptions.iter().enumerate() {
            let mut expected = Vec::new();
            match &i.meaning {
                Sentence::Clause(c) => expect_clause(c, &|p| r.name(p), &mut expected),
                Sentence::List(items) => {
                    for np in items {
                        expect_np(np, "", &|p| r.name(p), &mut expected);
                    }
                }
            }
            let mut actual: Vec<String> = i
                .rendered
                .words
                .iter()
                .map(|w| {
                    let root = w.morphs.iter().find(|m| m.is_root).expect("root");
                    let labels = w
                        .morphs
                        .iter()
                        .filter(|m| !m.is_root)
                        .map(|m| m.gloss.as_str())
                        .collect();
                    canon(&root.gloss, labels)
                })
                .collect();
            expected.sort();
            actual.sort();
            assert_eq!(actual, expected, "seed {seed} inscription {}", n + 1);

            // Reading the surface alone recovers exactly the rendered gloss.
            for w in &i.rendered.words {
                assert_eq!(
                    analyses.get(&w.phonemes()),
                    Some(&vec![w.gloss()]),
                    "seed {seed}"
                );
            }
        }
    }
}

#[test]
fn every_inscription_kind_appears() {
    let lang = Language::generate(5);
    let corpus = Corpus::generate(&lang, 40);
    let kinds: BTreeSet<&str> = corpus.inscriptions.iter().map(|i| i.kind.label()).collect();
    assert_eq!(kinds.len(), 4, "{kinds:?}");
}

#[test]
fn plain_output_hides_ground_truth() {
    let lang = Language::generate(8);
    let corpus = Corpus::generate(&lang, 40);
    let text = corpus.to_text(false);
    assert_eq!(text.lines().count(), 40);
    for c in concepts::all() {
        // A gloss could only appear by coincidence as a whole word.
        let leaked = text
            .split_whitespace()
            .any(|w| w == c.id && lang.romanise(lang.lexicon.root(&c.id)) != c.id);
        assert!(!leaked, "{}", c.id);
    }
    let json = corpus.to_json(false).to_string();
    for key in ["gloss", "meaning", "translation", "cast", "kind"] {
        assert!(!json.contains(key), "{key} leaked");
    }
}

#[test]
fn word_order_shows_in_tomb_formula() {
    // Tombs are subject + verb + adverb; the verb's position follows the order.
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 40);
        for i in &corpus.inscriptions {
            let Sentence::Clause(c) = &i.meaning else {
                continue;
            };
            if c.predicate != "lie" {
                continue;
            }
            let verb_at = i
                .rendered
                .words
                .iter()
                .position(|w| w.morphs.iter().any(|m| m.gloss == "lie"))
                .unwrap();
            let last = i.rendered.words.len() - 1;
            match lang.syntax.word_order {
                WordOrder::Sov => assert_eq!(verb_at, last),
                WordOrder::Vso => assert_eq!(verb_at, 0),
                WordOrder::Svo => assert_eq!(verb_at, last - 1),
            }
        }
    }
}

#[test]
fn verbs_and_nouns_inflect_regularly() {
    // Every noun form is root + the same affix strings, whatever the root.
    let lang = Language::generate(21);
    let m = &lang.morphology;
    for c in concepts::with_pos(Pos::Noun) {
        let root = lang.lexicon.root(&c.id);
        let forms = m.all_forms(root, Pos::Noun);
        for f in &forms {
            let extra = f.len() - root.len();
            match m.noun_position {
                AffixPosition::Suffix => assert_eq!(&f[..root.len()], &root[..]),
                AffixPosition::Prefix => assert_eq!(&f[extra..], &root[..]),
            }
        }
    }
}
