//! `scraped-lang world …`: debug views of a generated world (spoilers).

use scraped_world::{debug, World};

pub const USAGE: &str = "\
world commands (all spoilers, so --spoil is required):
  scraped-lang world --seed N map --spoil [--png FILE] [--scale K]
  scraped-lang world --seed N history --spoil
  scraped-lang world --seed N site S --spoil      (S = settlement number)
  scraped-lang world --seed N json --spoil";

/// Runs a world command; `Err` carries the message to print.
pub fn run(args: &[String]) -> Result<String, String> {
    let mut seed = None;
    let mut spoil = false;
    let mut png = None;
    let mut scale = 3usize;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seed" => {
                seed = Some(
                    it.next()
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or("--seed needs a number")?,
                )
            }
            "--spoil" => spoil = true,
            "--png" => png = Some(it.next().ok_or("--png needs a file name")?.clone()),
            "--scale" => {
                scale = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|&s| (1..=8).contains(&s))
                    .ok_or("--scale must be 1 to 8")?
            }
            _ => rest.push(a.clone()),
        }
    }
    let seed = seed.ok_or_else(|| format!("missing --seed\n\n{USAGE}"))?;
    if !spoil {
        return Err(format!(
            "world views show the whole map and history; pass --spoil to see them\n\n{USAGE}"
        ));
    }
    let w = World::generate(seed);
    match rest.first().map(String::as_str) {
        Some("map") => match png {
            Some(file) => {
                std::fs::write(&file, debug::png(&w, scale))
                    .map_err(|e| format!("cannot write {file}: {e}"))?;
                Ok(format!("wrote {file}\n"))
            }
            None => Ok(debug::ascii_map(&w, 2)),
        },
        Some("history") => Ok(debug::timeline(&w)),
        Some("site") => {
            let s: usize = rest
                .get(1)
                .and_then(|v| v.parse().ok())
                .ok_or("site needs a settlement number")?;
            if s >= w.history.settlements.len() {
                return Err(format!(
                    "this world has settlements 0 to {}",
                    w.history.settlements.len() - 1
                ));
            }
            Ok(debug::site(&w, s))
        }
        Some("json") => {
            let mut s = serde_json::to_string(&w).expect("world serialises");
            s.push('\n');
            Ok(s)
        }
        _ => Err(USAGE.to_string()),
    }
}
