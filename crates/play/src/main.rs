//! `scraped`: play in a terminal, or drive the game over JSON lines.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use scraped_play::{load_pack, protocol_line, wrap, Session};

// DEBUG-TEXT: command-line usage is for developers and agents, not players.
const USAGE: &str = "\
usage: scraped [--seed N | --code CODE] [--difficulty D] [--raw] [--content DIR]
               [--json] [--spoil] [--width W] [--load FILE] [--legacy [FILE]]

  --seed N       world seed (default 1); a seed whose world fails the
                 fairness check is replaced by a fair one derived from it
  --code CODE    play the world a seed code names (shown at the start)
  --difficulty D gentle, standard (default) or archaeologist
  --raw          use the seed as given, without the fairness check
  --content DIR  content pack folder (default ./content)
  --json         JSON-lines protocol: one command per input line, one JSON
                 object per output line: {text, state} (+ truth with --spoil)
  --spoil        include ground truth in --json output
  --width W      wrap terminal text at W columns (default 80, 0 = off)
  --load FILE    resume a saved game
  --legacy FILE  carry the last run's final inscription into this world, and
                 keep this run's when it ends (default scraped.legacy.json)

in play: save [FILE], load [FILE], transcript on [FILE], transcript off,
         export [PREFIX] (notebook: transcript, named places, run record), quit";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed = 1u64;
    let mut content = PathBuf::from("content");
    let mut json = false;
    let mut spoil = false;
    let mut width = 80usize;
    let mut load: Option<String> = None;
    let mut legacy: Option<PathBuf> = None;
    let mut preset = "standard".to_string();
    let mut code: Option<String> = None;
    let mut raw = false;
    let mut it = args.iter().peekable();
    while let Some(a) = it.next() {
        let value = |it: &mut std::iter::Peekable<std::slice::Iter<String>>| it.next().cloned();
        match a.as_str() {
            "--seed" => match value(&mut it).and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return fail("--seed needs a number"),
            },
            "--content" => match value(&mut it) {
                Some(d) => content = PathBuf::from(d),
                None => return fail("--content needs a folder"),
            },
            "--width" => match value(&mut it).and_then(|v| v.parse().ok()) {
                Some(w) => width = w,
                None => return fail("--width needs a number"),
            },
            "--load" => load = value(&mut it),
            "--code" => code = value(&mut it),
            "--raw" => raw = true,
            "--difficulty" => match value(&mut it) {
                Some(d) if scraped_game::fairness::PRESETS.contains(&d.as_str()) => preset = d,
                _ => return fail("--difficulty is gentle, standard or archaeologist"),
            },
            "--legacy" => {
                let file = match it.peek() {
                    Some(v) if !v.starts_with("--") => value(&mut it),
                    _ => None,
                };
                legacy = Some(PathBuf::from(
                    file.unwrap_or_else(|| "scraped.legacy.json".to_string()),
                ));
            }
            "--json" => json = true,
            "--spoil" => spoil = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => return fail(&format!("unexpected argument: {other}")),
        }
    }
    let pack = match load_pack(&content) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    if let Some(c) = &code {
        match scraped_game::seedcode::decode(c) {
            Some(sc) => {
                seed = sc.seed;
                preset = sc.preset;
                raw = true;
                if !pack.version().starts_with(&sc.pack) {
                    // DEBUG-TEXT: developer-facing warning on stderr.
                    eprintln!(
                        "note: this code was made with different text; the world is the same"
                    );
                }
            }
            None => return fail("that seed code doesn't read; check it for typos"),
        }
    }
    if !raw {
        seed = scraped_game::fairness::fair_seed(seed, &preset);
    }
    let (mut session, first) = Session::create(seed, &preset, pack, spoil, legacy);
    let mut out = std::io::stdout();
    let show = |out: &mut std::io::Stdout, o: &scraped_game::Output| {
        if json {
            let _ = writeln!(out, "{}", protocol_line(o));
        } else if !o.text.is_empty() {
            let _ = writeln!(out, "{}\n", wrap(&o.text, width));
        }
        let _ = out.flush();
    };
    show(&mut out, &first);
    if let Some(file) = load {
        let o = session.line(&format!("load {file}"));
        show(&mut out, &o);
    }
    let stdin = std::io::stdin();
    loop {
        if !json {
            let _ = write!(out, "> ");
            let _ = out.flush();
        }
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        // Agents may send either a bare command or {"cmd": "..."}.
        let input = serde_json::from_str::<serde_json::Value>(line.trim())
            .ok()
            .and_then(|v| v["cmd"].as_str().map(str::to_string))
            .unwrap_or_else(|| line.trim().to_string());
        let o = session.line(&input);
        show(&mut out, &o);
        if session.done {
            break;
        }
    }
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}\n\n{USAGE}");
    ExitCode::FAILURE
}
