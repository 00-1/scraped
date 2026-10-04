//! Milestone 1 and 2 properties, checked across many seeds and every era.

use std::collections::BTreeSet;

use scraped_lang::concepts::{self, Pos};
use scraped_lang::corpus::{Corpus, Kind};
use scraped_lang::difficulty::{Difficulty, NameMarking, Regularity, Separation};
use scraped_lang::lexicon::{edit_distance, MIN_ROOT_DISTANCE};
use scraped_lang::meaning::{
    Clause, Head, Mood, NounPhrase, Number, Polarity, Role, Sentence, Tense,
};
use scraped_lang::morphology::{AffixPosition, Morphology};
use scraped_lang::script::{GlyphKey, ScriptKind};
use scraped_lang::sheet::GrammarSheet;
use scraped_lang::syntax::WordOrder;
use scraped_lang::Language;

const SEEDS: std::ops::Range<u64> = 0..30;

/// Every era of every test seed.
fn all_eras() -> Vec<Language> {
    SEEDS.flat_map(|s| Language::generate(s).eras()).collect()
}

#[test]
fn same_seed_identical_output() {
    for seed in 0..15 {
        for era in 0..3 {
            let a = Language::generate(seed).at_era(era);
            let b = Language::generate(seed).at_era(era);
            let ca = Corpus::generate(&a, 30);
            let cb = Corpus::generate(&b, 30);
            assert_eq!(ca.to_text(true), cb.to_text(true), "seed {seed}");
            assert_eq!(ca.to_json(true), cb.to_json(true), "seed {seed}");
            assert_eq!(
                GrammarSheet::new(&a).to_text(),
                GrammarSheet::new(&b).to_text()
            );
        }
    }
}

#[test]
fn at_era_does_not_depend_on_the_starting_era() {
    let lang = Language::generate(4);
    let direct = lang.at_era(2);
    let via = lang.at_era(1).at_era(2);
    let back = direct.at_era(1);
    assert_eq!(
        GrammarSheet::new(&direct).to_text(),
        GrammarSheet::new(&via).to_text()
    );
    assert_eq!(
        GrammarSheet::new(&back).to_text(),
        GrammarSheet::new(&lang.at_era(1)).to_text()
    );
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
    let scripts: BTreeSet<String> = langs
        .iter()
        .map(|l| format!("{:?}", l.script.kind))
        .collect();
    assert_eq!(scripts.len(), 3, "all script kinds occur: {scripts:?}");
    let bases: BTreeSet<String> = langs
        .iter()
        .map(|l| format!("{:?}", l.numerals.base))
        .collect();
    assert_eq!(bases.len(), 4, "all numeral bases occur: {bases:?}");

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
    }
}

#[test]
fn every_word_obeys_its_eras_phonotactics() {
    for lang in all_eras() {
        let corpus = Corpus::generate(&lang, 40);
        let p = &lang.phonology;
        let tag = format!("seed {} era {}", lang.seed, lang.era);
        for (id, root) in &lang.lexicon.roots {
            for form in lang.morphology.all_forms(root, concepts::get(id).pos) {
                assert!(p.is_valid(&form), "{tag}: {id} {}", lang.romanise(&form));
            }
        }
        for a in lang.morphology.particles() {
            assert!(p.is_valid(&a.form), "{tag}: particle {}", a.gloss);
        }
        for name in &corpus.names {
            assert!(p.is_valid(name), "{tag}: name {}", lang.romanise(name));
        }
        for i in &corpus.inscriptions {
            for w in &i.rendered.words {
                let ph = w.phonemes();
                assert!(p.is_valid(&ph), "{tag}: {}", lang.romanise(&ph));
                assert_eq!(p.decode(&lang.romanise(&ph)), Some(ph), "{tag}");
            }
        }
    }
}

#[test]
fn sound_change_derives_every_word_exactly() {
    for seed in SEEDS {
        let eras = Language::generate(seed).eras();
        for pair in eras.windows(2) {
            let (old, new) = (&pair[0], &pair[1]);
            let step = new.history.last().expect("a step per era");
            for (id, root) in &old.lexicon.roots {
                if new.changes.replaced.contains(id) {
                    continue;
                }
                let expected = step.apply(&old.phonology.to_ipa(root));
                let actual = new.phonology.to_ipa(&new.lexicon.root(id));
                assert_eq!(actual, expected, "seed {seed} era {}: {id}", new.era);
            }
            for (a, b) in old
                .morphology
                .affix_list()
                .iter()
                .zip(new.morphology.affix_list())
            {
                if b.particle && !a.particle {
                    continue; // eroded this era: a new word
                }
                let expected = step.apply(&old.phonology.to_ipa(&a.form));
                assert_eq!(
                    new.phonology.to_ipa(&b.form),
                    expected,
                    "seed {seed}: {}",
                    a.gloss
                );
            }
        }
    }
}

#[test]
fn replacements_are_rare() {
    let mut replaced = 0;
    let mut total = 0;
    for lang in all_eras().into_iter().filter(|l| l.era > 0) {
        replaced += lang.changes.replaced.len();
        total += lang.lexicon.roots.len();
    }
    assert!(replaced * 5 < total, "{replaced} of {total} words replaced");
}

#[test]
fn eras_actually_change_words() {
    for seed in SEEDS {
        let eras = Language::generate(seed).eras();
        let changed = concepts::all()
            .iter()
            .filter(|c| eras[0].lexicon.has(&c.id))
            .filter(|c| {
                eras[0].romanise(&eras[0].lexicon.root(&c.id))
                    != eras[2].romanise(&eras[2].lexicon.root(&c.id))
            })
            .count();
        assert!(
            changed >= 10,
            "seed {seed}: only {changed} words changed over two eras"
        );
    }
}

#[test]
fn roots_and_affixes_never_collide() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 1);
        let mut words: Vec<_> = lang.lexicon.roots.values().cloned().collect();
        words.extend(corpus.names.iter().cloned());
        for (i, a) in words.iter().enumerate() {
            for b in &words[i + 1..] {
                assert!(edit_distance(a, b) >= MIN_ROOT_DISTANCE, "seed {seed}");
            }
        }
        assert!(lang.morphology.affixes_are_distinct());
    }
    // Sound change may bring words close, but never makes them identical.
    for lang in all_eras() {
        let corpus = Corpus::generate(&lang, 1);
        assert!(lang.morphology.affixes_are_distinct());
        for (form, glosses) in lang.analyses(&corpus.names) {
            assert_eq!(
                glosses.len(),
                1,
                "seed {} era {}: {} = {glosses:?}",
                lang.seed,
                lang.era,
                lang.romanise(&form)
            );
        }
    }
}

/// Canonical word gloss: root plus sorted grammatical labels, so the check
/// does not depend on affix order or word order.
fn canon(root: &str, mut labels: Vec<String>) -> String {
    labels.sort();
    format!("{root}|{}", labels.join(","))
}

struct Expect<'a> {
    m: &'a Morphology,
    lang: &'a Language,
    names: &'a dyn Fn(usize) -> String,
    out: Vec<String>,
}

impl Expect<'_> {
    /// Splits grammatical labels between the word and separate particles.
    fn inflected(&mut self, root: String, labels: Vec<&'static str>) {
        let particle = |g: &str| {
            self.m
                .affix_list()
                .iter()
                .any(|a| a.gloss == g && a.particle)
        };
        let (parts, attached): (Vec<&str>, Vec<&str>) =
            labels.into_iter().partition(|g| particle(g));
        for p in parts {
            self.out.push(canon(p, vec![]));
        }
        // Fused combinations show up as one dotted label.
        let joined: Vec<String> = match self.m.fusions.iter().find(|f| f.glosses == attached) {
            Some(f) => vec![f.glosses.join(".")],
            None => attached.iter().map(|s| s.to_string()).collect(),
        };
        self.out.push(canon(&root, joined));
    }

    fn plain(&mut self, id: &str) {
        self.out.push(canon(&concepts::gloss(id), vec![]));
    }

    fn np(&mut self, np: &NounPhrase, case: &'static str) {
        let root = match &np.head {
            Head::Concept(id) => id.clone(),
            Head::Name(p) => (self.names)(*p),
        };
        let mut labels = Vec::new();
        if np.number == Number::Plural {
            labels.push("PL");
        }
        if !case.is_empty() {
            labels.push(case);
        }
        self.inflected(root, labels);
        for a in &np.adjectives {
            self.plain(a);
        }
        if let Some(n) = np.quantity {
            for w in self.lang.numerals.words(n) {
                self.plain(w);
            }
        }
        if let Some(d) = &np.determiner {
            self.plain(d);
        }
        if let Some(p) = &np.possessor {
            self.np(p, "GEN");
        }
        for a in &np.apposition {
            self.np(a, case);
        }
    }

    fn clause(&mut self, c: &Clause) {
        let mut labels = Vec::new();
        if c.tense == Tense::Past {
            labels.push("PST");
        }
        if c.polarity == Polarity::Negative {
            labels.push("NEG");
        }
        self.inflected(c.predicate.clone(), labels);
        if c.mood == Mood::Potent {
            for p in ["pot", "pot.open", "pot.close"] {
                self.plain(p);
            }
        }
        for a in &c.args {
            match a.role {
                Role::Subject if c.mood == Mood::Imperative => {}
                Role::Subject => self.np(&a.np, ""),
                Role::Object => self.np(&a.np, "ACC"),
                Role::Recipient => self.np(&a.np, "DAT"),
            }
        }
        for a in &c.adverbs {
            self.plain(a);
        }
    }

    fn sentence(&mut self, s: &Sentence) {
        match s {
            Sentence::Clause(c) => self.clause(c),
            Sentence::List(items) => items.iter().for_each(|np| self.np(np, "")),
            Sentence::Text(parts) => {
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        self.plain("sent.end");
                    }
                    self.sentence(p);
                }
            }
            Sentence::Joined(conj, parts) => {
                for (i, c) in parts.iter().enumerate() {
                    if i > 0 {
                        self.plain(conj.concept());
                    }
                    self.clause(c);
                }
            }
        }
    }
}

#[test]
fn glosses_match_meaning_in_every_era() {
    let mut langs = all_eras();
    let fused = Difficulty {
        regularity: Regularity::Fused,
        ..Difficulty::default()
    };
    langs.extend((0..10).flat_map(|s| Language::generate_with(s, fused).eras()));
    for lang in langs {
        let corpus = Corpus::generate(&lang, 40);
        let r = corpus.renderer();
        let analyses = lang.analyses(&corpus.names);
        for (n, i) in corpus.inscriptions.iter().enumerate() {
            let mut e = Expect {
                m: &lang.morphology,
                lang: &lang,
                names: &|p| r.name(p),
                out: Vec::new(),
            };
            e.sentence(&i.meaning);
            let mut expected = e.out;
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
                        .map(|m| m.gloss.clone())
                        .collect();
                    canon(&root.gloss, labels)
                })
                .collect();
            expected.sort();
            actual.sort();
            let tag = format!("seed {} era {} inscription {}", lang.seed, lang.era, n + 1);
            assert_eq!(actual, expected, "{tag}");
            // Reading the surface alone recovers exactly the rendered gloss.
            for w in &i.rendered.words {
                assert_eq!(analyses.get(&w.phonemes()), Some(&vec![w.gloss()]), "{tag}");
            }
        }
    }
}

#[test]
fn ledger_totals_add_up() {
    let mut checked = 0;
    for lang in all_eras() {
        let corpus = Corpus::generate(&lang, 60);
        for i in &corpus.inscriptions {
            let Sentence::List(items) = &i.meaning else {
                continue;
            };
            let Some(total) = items
                .iter()
                .find(|np| np.head == Head::Concept("total".into()))
            else {
                continue;
            };
            let sum: u16 = items
                .iter()
                .filter(|np| np.head != total.head)
                .filter_map(|np| np.quantity)
                .sum();
            assert_eq!(total.quantity, Some(sum));
            checked += 1;
        }
    }
    assert!(checked > 20, "only {checked} totals");
}

#[test]
fn numeral_words_are_in_the_lexicon_and_parse() {
    for lang in all_eras() {
        for n in 0..=999u16 {
            let words = lang.numerals.words(n);
            for w in &words {
                assert!(lang.lexicon.has(w), "seed {}: no word for {w}", lang.seed);
            }
            if n % 37 == 0 {
                assert_eq!(lang.numerals.parse(&words), Some(n));
            }
        }
    }
}

#[test]
fn every_kind_and_both_registers_appear() {
    let lang = Language::generate(5);
    let corpus = Corpus::generate(&lang, 120);
    let kinds: BTreeSet<&str> = corpus.inscriptions.iter().map(|i| i.kind.label()).collect();
    assert_eq!(kinds.len(), 7, "{kinds:?}");
    let potent = corpus
        .inscriptions
        .iter()
        .find(|i| i.kind == Kind::Potent)
        .unwrap();
    // The potent frame opens and closes the inscription.
    let words = &potent.rendered.words;
    assert_eq!(words.first().unwrap().gloss(), "POT.OPEN");
    assert_eq!(words.last().unwrap().gloss(), "POT.CLOSE");
}

#[test]
fn scripts_write_every_word_in_every_era() {
    for lang in all_eras() {
        let glyphs: BTreeSet<_> = lang.script.glyphs.iter().map(|(_, g)| g.clone()).collect();
        assert_eq!(
            glyphs.len(),
            lang.script.glyphs.len(),
            "seed {} era {}",
            lang.seed,
            lang.era
        );
        let corpus = Corpus::generate(&lang, 30);
        let r = corpus.renderer();
        for i in &corpus.inscriptions {
            for key in r.glyphs(&i.rendered).into_iter().flatten() {
                lang.script.index(&key); // panics if the script lacks it
            }
        }
    }
}

#[test]
fn script_kind_can_be_forced() {
    for kind in [
        ScriptKind::Alphabet,
        ScriptKind::Abjad,
        ScriptKind::Syllabary,
    ] {
        let d = Difficulty {
            script: Some(kind),
            ..Difficulty::default()
        };
        for seed in 0..5 {
            for lang in Language::generate_with(seed, d).eras() {
                assert_eq!(lang.script.kind, kind);
            }
        }
    }
}

#[test]
fn separation_and_name_dials_change_presentation_only() {
    let base = Language::generate(9);
    let dotted = Language::generate_with(
        9,
        Difficulty {
            separation: Separation::Dots,
            names: NameMarking::Determinative,
            ..Difficulty::default()
        },
    );
    let joined = Language::generate_with(
        9,
        Difficulty {
            separation: Separation::None,
            ..Difficulty::default()
        },
    );
    let (a, b, c) = (
        Corpus::generate(&base, 30),
        Corpus::generate(&dotted, 30),
        Corpus::generate(&joined, 30),
    );
    let mut saw_name = false;
    for ((x, y), z) in a
        .inscriptions
        .iter()
        .zip(&b.inscriptions)
        .zip(&c.inscriptions)
    {
        assert_eq!(x.meaning, y.meaning);
        let plain = a.text(x);
        assert_eq!(c.text(z), plain.replace(' ', ""));
        let marked = b.text(y);
        assert_eq!(
            marked
                .replace(scraped_lang::render::DETERMINATIVE, "")
                .replace('·', " "),
            plain
        );
        saw_name |= marked.contains(scraped_lang::render::DETERMINATIVE);
        assert!(b.renderer().glyphs(&y.rendered).iter().all(|g| g.is_some()));
        assert!(b.renderer().glyphs(&y.rendered).len() > 1);
    }
    assert!(saw_name);
    assert!(b.inscriptions.iter().any(|i| b
        .renderer()
        .glyphs(&i.rendered)
        .contains(&Some(GlyphKey::Determinative))));
}

#[test]
fn eras_count_is_configurable() {
    for eras in 1..=5 {
        let d = Difficulty {
            eras,
            ..Difficulty::default()
        };
        assert_eq!(Language::generate_with(2, d).eras().len(), eras as usize);
    }
}

#[test]
fn plain_output_hides_ground_truth() {
    let lang = Language::generate(8);
    let corpus = Corpus::generate(&lang, 40);
    let text = corpus.to_text(false);
    assert_eq!(text.lines().count(), 40);
    // Keys, not text: a romanised word may happen to spell "kind".
    let json = corpus.to_json(false);
    let mut keys = Vec::new();
    fn walk(v: &serde_json::Value, keys: &mut Vec<String>) {
        match v {
            serde_json::Value::Object(m) => {
                for (k, x) in m {
                    keys.push(k.clone());
                    walk(x, keys);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| walk(x, keys)),
            _ => {}
        }
    }
    walk(&json, &mut keys);
    for key in ["gloss", "meaning", "translation", "cast", "kind"] {
        assert!(!keys.iter().any(|k| k == key), "{key} leaked");
    }
}

#[test]
fn word_order_shows_in_tomb_formula() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 40);
        for i in &corpus.inscriptions {
            let Sentence::Clause(c) = &i.meaning else {
                continue;
            };
            if c.predicate != "lie" || !lang.morphology.particles().is_empty() {
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
fn nouns_inflect_regularly() {
    let lang = Language::generate(21);
    let m = &lang.morphology;
    for c in concepts::with_pos(Pos::Noun) {
        let root = lang.lexicon.root(&c.id);
        for f in m.all_forms(&root, Pos::Noun) {
            let extra = f.len() - root.len();
            match m.noun_position {
                AffixPosition::Suffix => assert_eq!(&f[..root.len()], &root[..]),
                AffixPosition::Prefix => assert_eq!(&f[extra..], &root[..]),
            }
        }
    }
}

#[test]
fn dials_leave_the_words_alone() {
    let mut fused_same = 0;
    for seed in 0..20 {
        let base = Language::generate(seed);
        for d in [
            Difficulty {
                script: Some(ScriptKind::Syllabary),
                ..Difficulty::default()
            },
            Difficulty {
                separation: Separation::None,
                names: NameMarking::Determinative,
                ..Difficulty::default()
            },
            Difficulty {
                eras: 5,
                ..Difficulty::default()
            },
        ] {
            let other = Language::generate_with(seed, d);
            assert_eq!(base.lexicon.roots, other.lexicon.roots, "seed {seed} {d:?}");
        }
        let fused = Language::generate_with(
            seed,
            Difficulty {
                regularity: Regularity::Fused,
                ..Difficulty::default()
            },
        );
        fused_same += usize::from(base.lexicon.roots == fused.lexicon.roots);
    }
    assert!(
        fused_same >= 16,
        "fusion reshuffled {} of 20 lexicons",
        20 - fused_same
    );
}
