//! Milestone 5 properties: replay determinism, save/load, and no prose in
//! code.

use scraped_content::Pack;
use scraped_game::Game;
use scraped_lang::rng::{Rng, Stream};

fn pack() -> Pack {
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
    let (pack, errors) = Pack::load(&files);
    assert!(errors.is_empty(), "{errors:?}");
    pack
}

const WORDS: &[&str] = &[
    "look",
    "n",
    "s",
    "e",
    "w",
    "up",
    "down",
    "out",
    "go temple",
    "go house",
    "go second house",
    "go tomb",
    "open door",
    "open north",
    "x wall",
    "x altar",
    "read wall",
    "read stele",
    "read it",
    "more",
    "take jar",
    "take tablet",
    "drop it",
    "i",
    "wait",
    "define 2 as ka",
    "define 1 as ru",
    "first",
    "second",
    "dance",
    "",
    "help",
    "examine statue",
    "go store",
];

fn script(seed: u64, len: usize) -> Vec<String> {
    let mut rng = Rng::new(seed, Stream::Inscription(99));
    (0..len).map(|_| rng.pick(WORDS).to_string()).collect()
}

fn play(seed: u64, commands: &[String]) -> Vec<String> {
    let mut g = Game::new(seed, pack());
    let mut out = vec![g.start().text];
    out.extend(commands.iter().map(|c| g.step(c).text));
    out
}

#[test]
fn replays_are_identical() {
    for seed in [1, 42] {
        let cmds = script(seed, 120);
        assert_eq!(play(seed, &cmds), play(seed, &cmds), "seed {seed}");
    }
}

#[test]
fn save_and_load_restore_the_same_state() {
    for seed in [3, 42] {
        let p = pack();
        let mut g = Game::new(seed, p.clone());
        g.start();
        for c in script(seed + 7, 80) {
            g.step(&c);
        }
        let save = g.save();
        let json = serde_json::to_string(&save).unwrap();
        let (loaded, changed) = Game::load(&serde_json::from_str(&json).unwrap(), p);
        assert!(!changed);
        assert_eq!(loaded.state, g.state, "seed {seed}");
    }
}

#[test]
fn no_text_is_ever_empty_or_an_error() {
    let mut g = Game::new(42, pack());
    let first = g.start();
    assert!(!first.text.is_empty());
    for c in script(5, 200) {
        let o = g.step(&c);
        assert!(!o.text.contains("⟦error"), "{c}: {}", o.text);
        assert!(
            !o.text.contains("⟦slot"),
            "{c}: missing content: {}",
            o.text
        );
    }
}

#[test]
fn reading_and_labels_work() {
    let mut g = Game::new(42, pack());
    g.start();
    // Find something readable here or in the first building.
    for c in ["go temple", "read stele"] {
        g.step(c);
    }
    let before = g.step("read stele").text;
    let defined = g.step("define 1 as zo").text;
    assert!(defined.contains("zo"), "{defined}");
    let after = g.step("read stele").text;
    assert_ne!(before, after);
    assert!(after.contains("«zo»") || after.contains("zo"));
}

/// Player-visible English must come from content slots. String literals in
/// the game and play crates that look like prose (three or more words) are
/// errors, except in slot declarations (descriptions for Jb), tests and
/// lines marked DEBUG-TEXT.
#[test]
fn no_prose_in_code() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    let files = [
        "game/src/lib.rs",
        "game/src/site.rs",
        "game/src/parser.rs",
        "play/src/lib.rs",
        "play/src/main.rs",
    ];
    let mut found = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(format!("{root}/{f}")).unwrap();
        let mut debug_block = false;
        for (n, line) in text.lines().enumerate() {
            if line.contains("#[cfg(test)]") {
                break;
            }
            let trimmed = line.trim_start();
            // A comment line marking the next string as debug text.
            if trimmed.starts_with("// DEBUG-TEXT") {
                debug_block = true;
                continue;
            }
            if debug_block {
                if line.trim_end().ends_with("\";") {
                    debug_block = false;
                }
                continue;
            }
            // Developer-facing: comments, panics, file and argument errors,
            // and lines marked DEBUG-TEXT.
            let developer = [
                "expect(",
                "unreachable!",
                "panic!",
                "map_err",
                "Err(format!",
                "fail(",
                "DEBUG-TEXT",
            ];
            if trimmed.starts_with("//") || developer.iter().any(|d| line.contains(d)) {
                continue;
            }
            for lit in line.split('"').skip(1).step_by(2) {
                let words = lit
                    .split_whitespace()
                    .filter(|w| w.chars().any(char::is_alphabetic))
                    .count();
                if words >= 3 {
                    found.push(format!("{f}:{}: \"{lit}\"", n + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "prose in code (use a content slot):\n{}",
        found.join("\n")
    );
}
