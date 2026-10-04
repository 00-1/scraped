//! C01: every save in the corpus (`tests/saves/<version>/`) from this major
//! version loads from its snapshot and carries on. Whether it also replays
//! identically decides patch or minor: `scraped-lang saves check`.

use scraped_content::Pack;
use scraped_game::saves::{unseal, ENGINE};
use scraped_game::{Game, Save};

fn pack() -> Pack {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");
    let mut files: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n"),
            )
        })
        .collect();
    files.sort();
    Pack::load(&files).0
}

#[test]
fn corpus_saves_load_and_carry_on() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/saves");
    let major = |v: &str| v.split('.').next().unwrap_or("").to_string();
    let pack = pack();
    let mut n = 0;
    for dir in std::fs::read_dir(root).unwrap().flatten() {
        for f in std::fs::read_dir(dir.path()).unwrap().flatten() {
            let entry: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(f.path()).unwrap()).unwrap();
            let save: Save =
                serde_json::from_str(&unseal(entry["save"].as_str().unwrap()).unwrap()).unwrap();
            if major(&save.engine) != major(ENGINE) {
                continue;
            }
            let (mut g, _) = Game::load(&save, pack.clone());
            for c in ["look", "wait", "inventory"] {
                assert!(!g.step(c).text.is_empty(), "{}: {c}", f.path().display());
            }
            n += 1;
        }
    }
    assert!(n >= 3, "corpus saves checked: {n}");
}
