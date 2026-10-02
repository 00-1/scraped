//! Seed snapshots: the exact output for a few seeds is pinned in files.
//!
//! Any change to generation shows up here as a diff. If the change is
//! intended, regenerate with `UPDATE_SNAPSHOTS=1 cargo test` and review
//! the diff before committing.

use std::path::PathBuf;

use scraped_lang::corpus::Corpus;
use scraped_lang::sheet::GrammarSheet;
use scraped_lang::Language;

const SEEDS: [u64; 3] = [1, 42, 9001];

fn check(name: &str, actual: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing snapshot {name}; run with UPDATE_SNAPSHOTS=1"));
    assert!(
        expected == actual,
        "snapshot {name} changed; rerun with UPDATE_SNAPSHOTS=1 if intended"
    );
}

#[test]
fn grammar_sheets() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        check(
            &format!("seed-{seed}-grammar.txt"),
            &GrammarSheet::new(&lang).to_text(),
        );
    }
}

#[test]
fn corpora() {
    for seed in SEEDS {
        let lang = Language::generate(seed);
        let corpus = Corpus::generate(&lang, 40);
        check(&format!("seed-{seed}-corpus.txt"), &corpus.to_text(false));
        check(
            &format!("seed-{seed}-corpus-spoil.txt"),
            &corpus.to_text(true),
        );
    }
}

#[test]
fn json_corpus() {
    let lang = Language::generate(42);
    let corpus = Corpus::generate(&lang, 10);
    let json = serde_json::to_string_pretty(&corpus.to_json(true)).unwrap();
    check("seed-42-corpus.json", &json);
}
