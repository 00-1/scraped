//! `scraped-lang fair …`: the solvability batch (M14's release gate).

use scraped_game::fairness::{check, fair_seed, PRESETS};
use scraped_game::site::Site;

pub const USAGE: &str = "\
fairness commands:
  scraped-lang fair --seeds A-B [--difficulty D | --all] [--json]
      checks every seed from A to B: whether its world is fair as made, and
      that a fair world is found for it (the release gate needs all of them)
  scraped-lang fair --seed N [--difficulty D]   one world's goals and metrics";

/// Runs a fairness command; `Err` carries the message to print (and fails
/// the process).
pub fn run(args: &[String]) -> Result<String, String> {
    let mut range = None;
    let mut one = None;
    let mut presets = vec!["standard".to_string()];
    let mut json = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--seeds" => {
                let v = it.next().ok_or("--seeds needs A-B")?;
                let (a, b) = v.split_once('-').ok_or("--seeds needs A-B")?;
                range = Some((
                    a.parse::<u64>().map_err(|e| e.to_string())?,
                    b.parse::<u64>().map_err(|e| e.to_string())?,
                ));
            }
            "--seed" => {
                one = Some(
                    it.next()
                        .and_then(|v| v.parse::<u64>().ok())
                        .ok_or("--seed needs a number")?,
                )
            }
            "--difficulty" => {
                presets = vec![it.next().ok_or("--difficulty needs a preset")?.clone()]
            }
            "--all" => presets = PRESETS.iter().map(|p| p.to_string()).collect(),
            "--json" => json = true,
            _ => return Err(USAGE.to_string()),
        }
    }
    for p in &presets {
        if !PRESETS.contains(&p.as_str()) {
            return Err(format!("unknown difficulty {p}"));
        }
    }
    if let Some(seed) = one {
        let r = check(&Site::create(seed, &presets[0], None), &presets[0]);
        return Ok(serde_json::to_string_pretty(&r).expect("report") + "\n");
    }
    let (a, b) = range.ok_or(USAGE)?;
    let mut out = String::new();
    let mut failed = Vec::new();
    let mut rows = Vec::new();
    for p in &presets {
        let mut raw = 0;
        for seed in a..=b {
            let r = check(&Site::create(seed, p, None), p);
            if r.ok {
                raw += 1;
            }
            let fair = if r.ok { seed } else { fair_seed(seed, p) };
            let ok = r.ok || check(&Site::create(fair, p, None), p).ok;
            if !ok {
                failed.push(format!("{p} {seed}"));
            }
            rows.push(serde_json::json!({ "difficulty": p, "seed": seed, "fair_as_made": r.ok, "plays_as": fair, "ok": ok, "missing": r.goals.iter().filter(|g| !g.ok).map(|g| &g.goal).collect::<Vec<_>>() }));
        }
        out.push_str(&format!(
            "{p:<14} {raw}/{} fair as made; {} without a fair world\n",
            b - a + 1,
            failed.iter().filter(|f| f.starts_with(p.as_str())).count()
        ));
    }
    if json {
        out = serde_json::to_string_pretty(&rows).expect("rows") + "\n";
    }
    if failed.is_empty() {
        Ok(out)
    } else {
        Err(format!("{out}no fair world for: {}", failed.join(", ")))
    }
}
