//! Replaying a saved game stretch by stretch (C01), each stretch on the
//! build it was played on: this build plays its own; other versions'
//! player programs play theirs through their `replay-stretch` subcommand.
//! Used by `scraped-lang replay`.

use std::path::PathBuf;

use scraped_content::Pack;
use scraped_game::saves::{replay_stretch, same_play, seal, unseal, Stretch, ENGINE};
use scraped_game::Save;

/// Reads a sealed save, or a save-corpus entry that wraps one.
pub fn read_save(text: &str) -> Result<(String, Save), String> {
    let mut text = text.to_string();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(s) = v.get("save").and_then(|s| s.as_str()) {
            text = s.to_string();
        }
    }
    let save = serde_json::from_str(&unseal(&text)?).map_err(|e| e.to_string())?;
    Ok((text, save))
}

/// Replays `text` (a sealed save) stretch by stretch; `builds` names the
/// player program for each other version. Returns a report, or an error
/// if a build is missing or the replay doesn't reach the saved state.
// DEBUG-TEXT: a developer tool's report.
pub fn replay(text: &str, builds: &[(String, PathBuf)], pack: Pack) -> Result<String, String> {
    let (text, save) = read_save(text)?;
    let text = &text;
    let mut out = String::new();
    let mut prev: Option<Save> = None;
    let tmp = std::env::temp_dir().join(format!("scraped-replay-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let history = if save.history.is_empty() {
        vec![Stretch {
            engine: save.engine.clone(),
            pack: save.pack_version.clone(),
            from: 0,
            to: save.turn,
        }]
    } else {
        save.history.clone()
    };
    for s in &history {
        let next = if s.engine == ENGINE {
            replay_stretch(&save, s.from, s.to, prev.as_ref(), pack.clone())?
        } else {
            let bin = builds
                .iter()
                .find(|(v, _)| *v == s.engine)
                .map(|(_, p)| p.clone())
                .ok_or(format!(
                    "moves {}–{} were played on {}: name its player program with --build {}=PATH",
                    s.from, s.to, s.engine, s.engine
                ))?;
            let save_path = tmp.join("save");
            std::fs::write(&save_path, text).map_err(|e| e.to_string())?;
            let mut cmd = std::process::Command::new(&bin);
            cmd.arg("replay-stretch")
                .arg(&save_path)
                .arg(s.from.to_string())
                .arg(s.to.to_string());
            if let Some(p) = &prev {
                let start = tmp.join("start");
                std::fs::write(&start, seal(&serde_json::to_string(p).expect("save")))
                    .map_err(|e| e.to_string())?;
                cmd.arg(&start);
            }
            let o = cmd
                .output()
                .map_err(|e| format!("{}: {e}", bin.display()))?;
            if !o.status.success() {
                return Err(String::from_utf8_lossy(&o.stderr).to_string());
            }
            let sealed = String::from_utf8_lossy(&o.stdout).trim().to_string();
            serde_json::from_str(&unseal(&sealed)?).map_err(|e| e.to_string())?
        };
        out.push_str(&format!(
            "moves {}–{} on {}: replayed\n",
            s.from, s.to, s.engine
        ));
        prev = Some(next);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    let end = prev.ok_or("nothing to replay")?;
    if same_play(&end, &save) {
        out.push_str("the replay reaches the saved state\n");
        Ok(out)
    } else {
        Err(format!("{out}the replay does not reach the saved state\n"))
    }
}
