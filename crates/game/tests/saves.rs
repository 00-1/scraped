//! C01: saves load from their snapshot and carry on exactly as straight
//! play would.

use scraped_content::Pack;
use scraped_game::saves::{seal, unseal};
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

const FIRST: &[&str] = &[
    "look",
    "take all",
    "go in",
    "look closer",
    "out",
    "head north",
    "listen",
    "wait",
];
const THEN: &[&str] = &[
    "head east",
    "look",
    "read",
    "read closely",
    "sleep",
    "drink",
    "head south",
    "inventory",
];

#[test]
fn a_snapshot_carries_on_as_straight_play() {
    let pack = pack();
    for seed in [1u64, 42] {
        let mut straight = Game::new(seed, pack.clone());
        straight.start();
        for c in FIRST {
            straight.step(c);
        }
        let save = straight.save();
        let sealed = seal(&serde_json::to_string(&save).unwrap());
        let back: Save = serde_json::from_str(&unseal(&sealed).unwrap()).unwrap();
        assert_eq!(back, save);
        let (mut loaded, changed) = Game::load(&back, pack.clone());
        assert!(!changed);
        for c in THEN {
            let a = straight.step(c);
            let b = loaded.step(c);
            assert_eq!(a.text, b.text, "seed {seed}: after {c}");
        }
        assert_eq!(straight.save(), loaded.save(), "seed {seed}");
    }
}

#[test]
fn saves_record_turns_chain_and_builds() {
    let pack = pack();
    let mut g = Game::new(7, pack.clone());
    g.start();
    g.step("look");
    g.step("wait");
    let s = g.save();
    assert_eq!(s.turn, 2);
    assert_eq!(s.chain.len(), 16);
    assert_eq!(s.history.len(), 1);
    assert_eq!((s.history[0].from, s.history[0].to), (0, 2));
    assert_eq!(s.engine, s.created);
    // An old save (moves only) still loads by replaying.
    let mut old = s.clone();
    old.snapshot = None;
    let (mut a, _) = Game::load(&old, pack.clone());
    let (mut b, _) = Game::load(&s, pack);
    assert_eq!(a.step("look").text, b.step("look").text);
}
