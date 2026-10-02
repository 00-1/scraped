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
fn grammar_sheets_and_scripts() {
    for seed in SEEDS {
        for lang in Language::generate(seed).eras() {
            let sheet = GrammarSheet::new(&lang);
            let era = lang.era;
            check(
                &format!("seed-{seed}-era-{era}-grammar.txt"),
                &sheet.to_text(),
            );
            check(
                &format!("seed-{seed}-era-{era}-script.txt"),
                &sheet.script_text(),
            );
        }
    }
}

#[test]
fn corpora() {
    for seed in SEEDS {
        for lang in Language::generate(seed).eras() {
            let corpus = Corpus::generate(&lang, 40);
            let era = lang.era;
            check(
                &format!("seed-{seed}-era-{era}-corpus.txt"),
                &corpus.to_text(false),
            );
            check(
                &format!("seed-{seed}-era-{era}-corpus-spoil.txt"),
                &corpus.to_text(true),
            );
            check(
                &format!("seed-{seed}-era-{era}-corpus-glyphs.txt"),
                &corpus.to_glyph_text(),
            );
        }
    }
}

#[test]
fn json_corpus() {
    let lang = Language::generate(42).at_era(1);
    let corpus = Corpus::generate(&lang, 10);
    let json = serde_json::to_string_pretty(&corpus.to_json(true)).unwrap();
    check("seed-42-era-1-corpus.json", &json);
}
