//! `scraped-lang saves …`: the save corpus behind the versioning policy
//! (C01, docs/VERSIONING.md). Each corpus entry is a real save and the
//! fingerprint of its world; checking the corpus tells which version bump
//! a change needs.

use std::path::{Path, PathBuf};

use scraped_content::Pack;
use scraped_game::saves::{bump_between, seal, unseal, verdict, Bump, ENGINE};
use scraped_game::{Game, Save};

const USAGE: &str = "\
usage:
  scraped-lang saves add NAME [--seed N] [--hours H] [--dir tests/saves]
      plays the explorer bot for H hours (default 6) on seed N (default 1)
      and adds its save to the corpus for this engine version
  scraped-lang saves check [--dir tests/saves] [--against VERSION]
      what each corpus save makes of this build, and the bump the change
      needs; with --against (the last release), fails if the version in
      Cargo.toml is not bumped far enough
  --content DIR    the content pack (default: content)";

struct Opts {
    dir: PathBuf,
    content: PathBuf,
    seed: u64,
    hours: f64,
    against: Option<String>,
    rest: Vec<String>,
}

fn opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        dir: PathBuf::from("tests/saves"),
        content: PathBuf::from("content"),
        seed: 1,
        hours: 6.0,
        against: None,
        rest: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().cloned().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--dir" => o.dir = PathBuf::from(val()?),
            "--content" => o.content = PathBuf::from(val()?),
            "--seed" => o.seed = val()?.parse().map_err(|_| "--seed needs a number")?,
            "--hours" => o.hours = val()?.parse().map_err(|_| "--hours needs a number")?,
            "--against" => o.against = Some(val()?),
            other => o.rest.push(other.to_string()),
        }
    }
    Ok(o)
}

/// One corpus entry as stored.
#[derive(serde::Serialize, serde::Deserialize)]
struct Entry {
    world: String,
    save: String,
}

/// `scraped-lang saves`.
pub fn run(args: &[String]) -> Result<String, String> {
    let o = opts(args)?;
    let pack = Pack::load(&crate::content::read_pack(&o.content)?).0;
    match o.rest.first().map(String::as_str) {
        Some("add") => {
            let name = o.rest.get(1).ok_or(USAGE)?;
            let run = scraped_game::bots::play(&pack, o.seed, "explorer", o.hours, 2_000);
            let mut g = Game::new(o.seed, pack.clone());
            g.start();
            for c in &run.commands {
                g.step(c);
            }
            let save = g.save();
            let entry = Entry {
                world: g.site.world.fingerprint(),
                save: seal(&serde_json::to_string(&save).expect("save")),
            };
            let dir = o.dir.join(ENGINE);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(format!("{name}.json"));
            std::fs::write(&path, serde_json::to_string_pretty(&entry).expect("entry"))
                .map_err(|e| e.to_string())?;
            Ok(format!("wrote {} ({} moves)\n", path.display(), save.turn))
        }
        Some("check") => check(&o, &pack),
        _ => Err(USAGE.to_string()),
    }
}

fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for v in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        for f in std::fs::read_dir(v.path()).into_iter().flatten().flatten() {
            if f.path().extension().is_some_and(|x| x == "json") {
                out.push(f.path());
            }
        }
    }
    out.sort();
    out
}

fn check(o: &Opts, pack: &Pack) -> Result<String, String> {
    let mut out = String::new();
    let mut needed = Bump::Patch;
    // Only saves from the release being compared against (or, without one,
    // from this major) must carry on.
    let major = |v: &str| {
        v.trim_start_matches('v')
            .split('.')
            .next()
            .map(str::to_string)
    };
    let base = o.against.clone().unwrap_or_else(|| ENGINE.to_string());
    for path in entries(&o.dir) {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let entry: Entry =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let (bump, why) = match unseal(&entry.save)
            .and_then(|j| serde_json::from_str::<Save>(&j).map_err(|e| e.to_string()))
        {
            Ok(save) => {
                if major(&save.engine) != major(&base) {
                    out.push_str(&format!(
                        "{}: from an older major version; skipped\n",
                        path.display()
                    ));
                    continue;
                }
                verdict(&save, &entry.world, pack.clone())
            }
            Err(e) => (Bump::Major, format!("it no longer reads ({e})")),
        };
        out.push_str(&format!("{}: {bump:?}: {why}\n", path.display()));
        needed = needed.max(bump);
    }
    out.push_str(&format!(
        "this change needs at least a {needed:?} release\n"
    ));
    if let Some(last) = &o.against {
        let declared = bump_between(last, ENGINE).ok_or("versions don't read")?;
        if last.trim_start_matches('v') == ENGINE || declared < needed {
            return Err(format!(
                "{out}version {ENGINE} after {last} is a {declared:?} bump; this change needs a {needed:?} one\n"
            ));
        }
    }
    Ok(out)
}
