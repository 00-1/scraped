//! `scraped-player`: the program for playing (C01). The text is baked in,
//! the spoilers are compiled out (build without the `spoilers` feature),
//! and it plays alone, over JSON lines, as an MCP server, or a world
//! shared with another player.
//!
//! ```text
//! scraped-player [--seed N | --code CODE] [--difficulty D] [--json]
//! scraped-player --mcp
//! scraped-player --world FILE [--as WHO] [--json]      play a shared world
//! scraped-player --new-world FILE --id ID [--seed N] [--as WHO]
//! scraped-player merge OURS THEIRS                     after a sync conflict
//! ```

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use scraped_play::world::{merge, Merged, Shared};
use scraped_play::{baked_pack, mcp::Server, protocol_line, wrap, Session, FAIR_PLAY};

// DEBUG-TEXT: usage and errors are for players' programs and agents.
const USAGE: &str = "\
usage: scraped-player [--seed N | --code CODE] [--difficulty D] [--json] [--width W]
       scraped-player --mcp
       scraped-player --world FILE [--as WHO] [--json]
       scraped-player --new-world FILE --id ID [--seed N] [--difficulty D] [--as WHO] [--json]
       scraped-player merge OURS THEIRS
       scraped-player --version

in a shared world: talk TEXT (a message to your partner), talk since N, quit";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("merge") {
        return match (args.get(1), args.get(2)) {
            (Some(a), Some(b)) => match merge(&PathBuf::from(a), &PathBuf::from(b)) {
                Ok(Merged::Kept(side)) => {
                    println!("kept {side:?}");
                    ExitCode::SUCCESS
                }
                Ok(Merged::Branched(p)) => {
                    println!("the world split; the other line is now {}", p.display());
                    ExitCode::SUCCESS
                }
                Err(e) => fail(&e),
            },
            _ => fail("merge needs two world files"),
        };
    }
    let mut seed = 1u64;
    let mut preset = "standard".to_string();
    let mut code: Option<String> = None;
    let mut json = false;
    let mut mcp = false;
    let mut width = 80usize;
    let mut world: Option<PathBuf> = None;
    let mut new_world: Option<PathBuf> = None;
    let mut id: Option<String> = None;
    let mut who = "jb".to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seed" => match it.next().and_then(|v| v.parse().ok()) {
                Some(s) => seed = s,
                None => return fail("--seed needs a number"),
            },
            "--code" => code = it.next().cloned(),
            "--difficulty" => match it.next() {
                Some(d) if scraped_game::fairness::PRESETS.contains(&d.as_str()) => {
                    preset = d.clone()
                }
                _ => return fail("--difficulty is gentle, standard or archaeologist"),
            },
            "--width" => match it.next().and_then(|v| v.parse().ok()) {
                Some(w) => width = w,
                None => return fail("--width needs a number"),
            },
            "--json" => json = true,
            "--mcp" => mcp = true,
            "--world" => world = it.next().map(PathBuf::from),
            "--new-world" => new_world = it.next().map(PathBuf::from),
            "--id" => id = it.next().cloned(),
            "--as" => match it.next() {
                Some(w) => who = w.clone(),
                None => return fail("--as needs a name"),
            },
            "--version" => {
                println!("scraped-player {}", scraped_game::saves::ENGINE);
                return ExitCode::SUCCESS;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            other => return fail(&format!("unexpected argument: {other}")),
        }
    }
    let pack = baked_pack();
    if mcp {
        return serve(Server::new(pack));
    }
    if let Some(c) = &code {
        match scraped_game::seedcode::decode(c) {
            Some(sc) => {
                seed = sc.seed;
                preset = sc.preset;
            }
            None => return fail("that seed code doesn't read; check it for typos"),
        }
    } else {
        seed = scraped_game::fairness::fair_seed(seed, &preset);
    }
    let mut out = std::io::stdout();
    let mut first_reply = true;
    let mut show = |out: &mut std::io::Stdout, o: &scraped_game::Output| {
        if json {
            let mut line: serde_json::Value =
                serde_json::from_str(&protocol_line(o)).expect("protocol JSON");
            if first_reply {
                line["fair_play"] = serde_json::Value::from(FAIR_PLAY);
                first_reply = false;
            }
            let _ = writeln!(out, "{line}");
        } else if !o.text.is_empty() {
            let _ = writeln!(out, "{}\n", wrap(&o.text, width));
        }
        let _ = out.flush();
    };
    // A shared world: every move and word of talk goes into its file.
    if world.is_some() || new_world.is_some() {
        let opened = match (&world, &new_world) {
            (Some(path), _) => Shared::open(path, &who, pack),
            (None, Some(path)) => match &id {
                Some(id) => Shared::create(path, id, &who, seed, &preset, pack),
                None => return fail("--new-world needs an --id"),
            },
            _ => unreachable!(),
        };
        let (mut shared, first) = match opened {
            Ok(x) => x,
            Err(e) => return fail(&e),
        };
        show(&mut out, &first);
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let line = input(&line);
            if line.is_empty() {
                continue;
            }
            if matches!(line.as_str(), "quit" | "q" | "exit") {
                break;
            }
            let reply = if let Some(rest) = line.strip_prefix("talk since") {
                let n = rest.trim().parse().unwrap_or(0);
                let talk = shared.talk_since(n);
                serde_json::json!({ "talk": talk, "next": n.max(shared.file.talk.len()) })
            } else if let Some(text) = line.strip_prefix("talk ") {
                match shared.talk(text.trim()) {
                    Ok(()) => serde_json::json!({ "talk": shared.file.talk.len() }),
                    Err(e) => return fail(&e),
                }
            } else {
                match shared.play(&line) {
                    Ok(o) => {
                        show(&mut out, &o);
                        continue;
                    }
                    Err(e) => return fail(&e),
                }
            };
            if json {
                let _ = writeln!(out, "{reply}");
            } else {
                let _ = writeln!(
                    out,
                    "{}\n",
                    serde_json::to_string_pretty(&reply).unwrap_or_default()
                );
            }
            let _ = out.flush();
        }
        return ExitCode::SUCCESS;
    }
    // Alone.
    let (mut session, first) = Session::create(seed, &preset, pack, false, None);
    show(&mut out, &first);
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
        let o = session.line(&input(&line));
        show(&mut out, &o);
        if session.done {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// A bare command, or `{"cmd": "..."}` from an agent.
fn input(line: &str) -> String {
    serde_json::from_str::<serde_json::Value>(line.trim())
        .ok()
        .and_then(|v| v["cmd"].as_str().map(str::to_string))
        .unwrap_or_else(|| line.trim().to_string())
}

fn serve(mut server: Server) -> ExitCode {
    let mut out = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(serde_json::json!({
                "jsonrpc": "2.0", "id": null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            })),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
    ExitCode::SUCCESS
}

fn fail(message: &str) -> ExitCode {
    eprintln!("error: {message}\n\n{USAGE}");
    ExitCode::FAILURE
}
