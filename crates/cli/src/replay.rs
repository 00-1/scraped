//! `scraped-lang replay SAVE …` (C01): replays a saved game stretch by
//! stretch, each on the build it was played on, and checks the end matches
//! the save. Builds of other versions are player programs named with
//! `--build VERSION=PATH` (every release's is downloadable from the Pages
//! site); this build plays its own stretches.

use std::path::PathBuf;

use scraped_content::Pack;
use scraped_game::saves::{replay_stretch, same_play, seal, unseal, ENGINE};
use scraped_game::Save;

const USAGE: &str = "\
usage: scraped-lang replay SAVE [--build VERSION=PATH ...] [--content DIR]
  replays SAVE stretch by stretch, each on its own build (this one for its
  own version; a player program for each other version), and checks the
  final state matches the save";

pub fn run(args: &[String]) -> Result<String, String> {
    let mut file: Option<PathBuf> = None;
    let mut builds: Vec<(String, PathBuf)> = Vec::new();
    let mut content = PathBuf::from("content");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--build" => {
                let v = it.next().ok_or(USAGE)?;
                let (ver, path) = v.split_once('=').ok_or(USAGE)?;
                builds.push((ver.to_string(), PathBuf::from(path)));
            }
            "--content" => content = PathBuf::from(it.next().ok_or(USAGE)?),
            other => file = Some(PathBuf::from(other)),
        }
    }
    let file = file.ok_or(USAGE)?;
    let pack = Pack::load(&crate::content::read_pack(&content)?).0;
    let mut text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    // A save-corpus entry wraps the sealed save.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
        if let Some(s) = v.get("save").and_then(|s| s.as_str()) {
            text = s.to_string();
        }
    }
    let save: Save = serde_json::from_str(&unseal(&text)?).map_err(|e| e.to_string())?;
    let mut out = String::new();
    let mut prev: Option<Save> = None;
    let tmp = std::env::temp_dir().join(format!("scraped-replay-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let history = if save.history.is_empty() {
        vec![scraped_game::saves::Stretch {
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
            std::fs::write(&save_path, &text).map_err(|e| e.to_string())?;
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
