//! `scraped-lang replay SAVE …` (C01): replays a saved game stretch by
//! stretch, each on the build it was played on, and checks the end matches
//! the save. Builds of other versions are player programs named with
//! `--build VERSION=PATH` (every release's is downloadable from the Pages
//! site); this build plays its own stretches.

use std::path::PathBuf;

use scraped_content::Pack;

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
    let text = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
    scraped_play::replay::replay(&text, &builds, pack)
}
