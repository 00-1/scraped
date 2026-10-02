//! `scraped-lang content …`: lint, coverage, previews and the release check
//! for the content pack, from the terminal.

use std::path::Path;

use scraped_content::{coverage, lint, release_check, Pack, Renderer, Severity};
use scraped_game::slots::registry;
use scraped_lang::slots::LangHooks;
use scraped_lang::Language;

pub const USAGE: &str = "\
content commands (default folder: ./content, or --content DIR):
  scraped-lang content lint [--json]
  scraped-lang content coverage [--json]
  scraped-lang content preview SLOT [--seed N] [--count K]
  scraped-lang content registry
  scraped-lang content release-check";

/// Reads every `.toml` file in the content folder.
pub fn read_pack(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for e in entries.flatten() {
        let path = e.path();
        if path.extension().is_some_and(|x| x == "toml") {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            files.push((name, text));
        }
    }
    files.sort();
    Ok(files)
}

fn pretty(v: &impl serde::Serialize) -> String {
    let mut s = serde_json::to_string_pretty(v).expect("serialises");
    s.push('\n');
    s
}

/// Runs a content command. `Err` means failure (bad usage, lint errors, or a
/// failed release check), with the text to print.
pub fn run(args: &[String]) -> Result<String, String> {
    let mut dir = "content".to_string();
    let mut json = false;
    let mut seed = 42u64;
    let mut count = 5usize;
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--content" => dir = it.next().ok_or("--content needs a folder")?.clone(),
            "--json" => json = true,
            "--seed" => {
                seed = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--seed needs a number")?;
            }
            "--count" => {
                count = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--count needs a number")?;
            }
            _ => rest.push(a.clone()),
        }
    }
    let reg = registry();
    let files = read_pack(Path::new(&dir))?;
    let (pack, errors) = Pack::load(&files);
    let lang = Language::generate(seed);
    let hooks = LangHooks { lang: &lang };
    let issues = lint(&reg, &pack, &errors, &hooks);
    match rest.first().map(String::as_str) {
        Some("lint") => {
            if json {
                return Ok(pretty(&issues));
            }
            let mut out = String::new();
            for i in &issues {
                let place = match (&i.file, i.variant) {
                    (Some(f), Some(v)) => format!("{f} variant {}", v + 1),
                    (Some(f), None) => f.clone(),
                    _ => String::new(),
                };
                out.push_str(&format!(
                    "{:<7} {:<16} {place:<22} {}\n",
                    format!("{:?}", i.severity).to_lowercase(),
                    i.slot,
                    i.message
                ));
            }
            let errors = issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .count();
            out.push_str(&format!(
                "{} issue(s), {errors} error(s); pack version {}\n",
                issues.len(),
                pack.version()
            ));
            if errors > 0 {
                Err(out)
            } else {
                Ok(out)
            }
        }
        Some("coverage") => {
            let cov = coverage(&reg, &pack, &issues);
            if json {
                return Ok(pretty(&cov));
            }
            let mut out = format!(
                "{:<20} {:<13} {:>8} {:>10}\n",
                "slot", "status", "written", "uncovered"
            );
            for c in &cov {
                out.push_str(&format!(
                    "{:<20} {:<13} {:>4} / {:<2} {:>6} / {}\n",
                    c.slot,
                    serde_json::to_value(c.status)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default(),
                    c.written,
                    c.needed,
                    c.uncovered,
                    c.samples
                ));
            }
            Ok(out)
        }
        Some("preview") => {
            let slot = rest.get(1).ok_or("preview needs a slot id")?;
            let def = reg.get(slot).ok_or(format!("no slot called '{slot}'"))?;
            let samples = def.samples(&[seed]);
            let mut r = Renderer::new(&reg, &pack, seed, &hooks);
            let mut out = String::new();
            for (_, ctx) in samples.iter().take(count) {
                let vars: Vec<String> = ctx
                    .iter()
                    .map(|(k, v)| format!("{k}={}", v.text()))
                    .collect();
                out.push_str(&format!(
                    "  [{}]\n  {}\n\n",
                    vars.join("  "),
                    r.render(slot, ctx)
                ));
            }
            Ok(out)
        }
        Some("registry") => Ok(pretty(&reg)),
        Some("release-check") => {
            let problems = release_check(&reg, &pack, &issues);
            if problems.is_empty() {
                Ok(format!(
                    "ready for release (pack version {})\n",
                    pack.version()
                ))
            } else {
                Err(format!(
                    "not ready for release:\n  {}\n",
                    problems.join("\n  ")
                ))
            }
        }
        _ => Err(USAGE.to_string()),
    }
}
