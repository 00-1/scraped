//! `scraped-lang depth | bots | samples …`: the D01 instruments.

use std::path::{Path, PathBuf};

use scraped_content::Pack;
use scraped_game::bots::{play, DEPTH_BOTS};
use scraped_game::depth::{report, table};

use crate::content::read_pack;

pub const USAGE: &str = "\
depth instruments:
  scraped-lang depth --seeds A-B [--hours H] [--json]
      measures each world, and an explorer's H hours in it (default 24)
  scraped-lang bots --seeds A-B [--bot explorer|scholar] [--hours H] [--json]
      plays the depth bots and reports how far they got
  scraped-lang samples MILESTONE [--hours H] [--inside]
      writes explorer transcripts for seeds 1, 42 and 9001 (default 3 hours)
      to docs/samples/MILESTONE/; --inside starts at the largest great
      interior's entrance with a lamp
  --content DIR    the content pack (default: content)";

struct Opts {
    seeds: Vec<u64>,
    hours: Option<f64>,
    json: bool,
    inside: bool,
    bot: Option<String>,
    content: PathBuf,
    rest: Vec<String>,
}

fn opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        seeds: Vec::new(),
        hours: None,
        json: false,
        inside: false,
        bot: None,
        content: PathBuf::from("content"),
        rest: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seeds" => {
                let v = it.next().ok_or("--seeds needs A-B")?;
                let (a, b) = v.split_once('-').unwrap_or((v, v));
                let a: u64 = a.parse().map_err(|_| "--seeds needs A-B")?;
                let b: u64 = b.parse().map_err(|_| "--seeds needs A-B")?;
                o.seeds = (a..=b).collect();
            }
            "--hours" => {
                o.hours = Some(
                    it.next()
                        .and_then(|v| v.parse().ok())
                        .ok_or("--hours needs a number")?,
                )
            }
            "--bot" => o.bot = Some(it.next().ok_or("--bot needs a name")?.clone()),
            "--content" => o.content = PathBuf::from(it.next().ok_or("--content needs a folder")?),
            "--json" => o.json = true,
            "--inside" => o.inside = true,
            other if other.starts_with("--") => return Err(USAGE.to_string()),
            other => o.rest.push(other.to_string()),
        }
    }
    Ok(o)
}

fn pack(dir: &Path) -> Result<Pack, String> {
    Ok(Pack::load(&read_pack(dir)?).0)
}

/// `scraped-lang depth`.
pub fn depth(args: &[String]) -> Result<String, String> {
    let o = opts(args)?;
    if o.seeds.is_empty() {
        return Err(USAGE.to_string());
    }
    let r = report(&pack(&o.content)?, &o.seeds, o.hours.unwrap_or(24.0));
    Ok(if o.json {
        serde_json::to_string_pretty(&r).expect("report") + "\n"
    } else {
        table(&r)
    })
}

/// `scraped-lang bots`.
pub fn bots(args: &[String]) -> Result<String, String> {
    let o = opts(args)?;
    if o.seeds.is_empty() {
        return Err(USAGE.to_string());
    }
    let p = pack(&o.content)?;
    let kinds: Vec<&'static str> = match o.bot.as_deref() {
        None => DEPTH_BOTS.to_vec(),
        Some(b) => vec![*DEPTH_BOTS
            .iter()
            .find(|k| **k == b)
            .ok_or(format!("unknown bot {b}"))?],
    };
    let mut out = String::new();
    let mut runs = Vec::new();
    for &kind in &kinds {
        // The scholar is measured over a season, the explorer over days.
        let hours = o
            .hours
            .unwrap_or(if kind == "scholar" { 2160.0 } else { 72.0 });
        for &seed in &o.seeds {
            let r = play(&p, seed, kind, hours, 200_000);
            out.push_str(&format!(
                "{kind:<9} seed {seed:>5}: {:>6.1} h, {:>6} commands, {:>3} buildings, {:>3} texts read, {:>3} new things, died {}, great {}, deepest {}, first release {}\n", // DEBUG-TEXT
                r.hours,
                r.commands.len(),
                r.buildings,
                r.texts_read,
                r.novelty.len(),
                r.died.as_deref().unwrap_or("no"),
                r.read_great,
                r.read_deepest,
                r.first_release
                    .as_ref()
                    .map_or("never".to_string(), |(h, c)| format!("{h:.1} h ({c})"))
            ));
            runs.push(serde_json::json!({
                "bot": kind,
                "seed": seed,
                "hours": r.hours,
                "commands": r.commands.len(),
                "buildings": r.buildings,
                "texts_read": r.texts_read,
                "novelty": r.novelty,
                "died": r.died,
                "read_great": r.read_great,
                "read_deepest": r.read_deepest,
            }));
        }
    }
    Ok(if o.json {
        serde_json::to_string_pretty(&runs).expect("runs") + "\n"
    } else {
        out
    })
}

/// The seeds sample transcripts are written for.
pub const SAMPLE_SEEDS: [u64; 3] = [1, 42, 9001];

/// A response as a fenced block that always closes (S03): the fence is
/// longer than any run of backticks inside, and an empty reply is marked.
fn block(text: &str) -> String {
    let text = text.trim_end();
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    let body = if text.trim().is_empty() {
        "(no reply)" // DEBUG-TEXT
    } else {
        text
    };
    format!("{fence}\n{body}\n{fence}\n")
}

/// `scraped-lang samples`: spoiler-free explorer transcripts, for Jb to
/// read what play feels like now.
pub fn samples(args: &[String]) -> Result<String, String> {
    let o = opts(args)?;
    let milestone = o.rest.first().ok_or(USAGE)?.clone();
    let p = pack(&o.content)?;
    let dir = PathBuf::from("docs/samples").join(&milestone);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let hours = o.hours.unwrap_or(3.0);
    let mut out = String::new();
    for seed in SAMPLE_SEEDS {
        let r = if o.inside {
            scraped_game::bots::play_with(&p, seed, "explorer", hours, 5_000, |g| {
                g.begin_inside_great()
            })
        } else {
            play(&p, seed, "explorer", hours, 5_000)
        };
        let mut md = format!(
            "# {milestone}: the curious explorer, seed {seed}\n\n{} commands, {:.1} hours of game time. Example text only; spoiler-free.\n\n", // DEBUG-TEXT
            r.commands.len(),
            r.hours
        );
        md.push_str(&block(&r.texts[0]));
        for (c, t) in r.commands.iter().zip(r.texts.iter().skip(1)) {
            md.push_str(&format!("\n**> {c}**\n\n{}", block(t)));
        }
        let path = dir.join(format!("seed-{seed}.md"));
        std::fs::write(&path, md).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        out.push_str(&format!("wrote {}\n", path.display())); // DEBUG-TEXT
    }
    Ok(out)
}
