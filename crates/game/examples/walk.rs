//! Plays a fixed script against the real content pack, for a quick look.
use scraped_content::Pack;
fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let mut files = Vec::new();
    for e in std::fs::read_dir("content").unwrap().flatten() {
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
    let mut g = scraped_game::Game::new(seed, pack);
    println!("{}\n", g.start().text);
    for cmd in std::env::args().skip(2) {
        println!("> {cmd}\n{}\n", g.step(&cmd).text);
    }
}
