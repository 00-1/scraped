//! Quick look at a world: `cargo run -p scraped-world --example ascii -- SEED`.
fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let t = std::time::Instant::now();
    let w = scraped_world::World::generate(seed);
    eprintln!(
        "generated in {:?}: {} settlements, {} structures, {} texts, {} events",
        t.elapsed(),
        w.history.settlements.len(),
        w.structures.len(),
        w.texts.len(),
        w.history.events.len()
    );
    print!("{}", scraped_world::debug::ascii_map(&w, 2));
    print!("{}", scraped_world::debug::timeline(&w));
    print!("{}", scraped_world::debug::site(&w, 0));
}
