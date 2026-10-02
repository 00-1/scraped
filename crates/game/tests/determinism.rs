//! Cross-platform determinism: a scripted run's transcript and final state
//! hash to the same value on every platform (CI runs this on Linux, macOS
//! and Windows; tools/smoke/determinism.cjs checks WebAssembly against the
//! same file). Regenerate with `UPDATE_SNAPSHOTS=1 cargo test` after an
//! intended change.

use scraped_content::Pack;
use scraped_game::coverage::transcript_hash;

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
                // Line endings differ by checkout; the text must not.
                std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n"),
            )
        })
        .collect();
    files.sort();
    Pack::load(&files).0
}

pub const CASES: [(u64, &str); 4] = [
    (1, "standard"),
    (42, "gentle"),
    (9001, "archaeologist"),
    (7, "standard"),
];

#[test]
fn transcripts_match_on_every_platform() {
    let p = pack();
    let got: String = CASES
        .iter()
        .map(|&(seed, preset)| {
            format!(
                "{seed} {preset} {}\n",
                transcript_hash(&p, seed, preset, 40)
            )
        })
        .collect();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/transcripts.txt");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(path, &got).unwrap();
    }
    let want = std::fs::read_to_string(path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        got, want,
        "transcripts differ; rerun with UPDATE_SNAPSHOTS=1 if intended"
    );
}
