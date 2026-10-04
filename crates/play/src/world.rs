//! Playing a shared world file (C01): open it as one of the players,
//! refuse an older copy than this machine has seen, record every move and
//! word of table talk into it, and merge two copies after a sync.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use scraped_content::Pack;
use scraped_game::saves::Continuity;
use scraped_game::shared::{may_continue, merge_files, Merge, Move, Side, Talk, WorldFile};
use scraped_game::{Game, Output};

/// A shared world being played.
pub struct Shared {
    pub path: PathBuf,
    pub file: WorldFile,
    pub game: Game,
    pub who: String,
}

/// Seconds since 1970 by this machine's clock.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Where this machine records the newest copy of each world it has seen.
fn seen_path() -> PathBuf {
    let base = std::env::var_os("SCRAPED_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".scraped")))
        .unwrap_or_else(|| PathBuf::from(".scraped"));
    base.join("seen.json")
}

type Seen = BTreeMap<String, (usize, String)>;

fn read_seen() -> Seen {
    std::fs::read_to_string(seen_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn key(f: &WorldFile) -> String {
    format!("{}#{}", f.id, f.branch)
}

fn record_seen(f: &WorldFile) {
    let mut seen = read_seen();
    seen.insert(key(f), (f.turn, f.chain.clone()));
    let path = seen_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, serde_json::to_string(&seen).unwrap_or_default());
}

fn read_file(path: &Path) -> Result<WorldFile, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: not a world file: {e}", path.display()))
}

fn write_file(path: &Path, f: &WorldFile) -> Result<(), String> {
    let text = serde_json::to_string_pretty(f).expect("a world file serialises");
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

// DEBUG-TEXT: errors for players' programs and agents.
impl Shared {
    /// Begins a new shared world and writes it.
    pub fn create(
        path: &Path,
        id: &str,
        who: &str,
        seed: u64,
        preset: &str,
        pack: Pack,
    ) -> Result<(Self, Output), String> {
        let mut game = Game::create(seed, pack, preset, None);
        let first = game.start();
        let file = WorldFile::new(id, &game);
        write_file(path, &file)?;
        record_seen(&file);
        Ok((
            Shared {
                path: path.to_path_buf(),
                file,
                game,
                who: who.to_string(),
            },
            first,
        ))
    }

    /// Opens a shared world to play on. Refuses an older copy than this
    /// machine has played (no undo), and a copy that split from it.
    pub fn open(path: &Path, who: &str, pack: Pack) -> Result<(Self, Output), String> {
        let file = read_file(path)?;
        let seen = read_seen();
        let mine = seen.get(&key(&file)).map(|(t, c)| (*t, c.as_str()));
        match may_continue(&file, mine) {
            Continuity::Fine => {}
            Continuity::Rewind => {
                return Err(format!(
                    "this copy of the world is older (move {}) than one already played here (move {}): sync first",
                    file.turn,
                    mine.map_or(0, |m| m.0)
                ))
            }
            Continuity::Split => {
                return Err("this copy split from the one played here: merge them first".into())
            }
        }
        let save = file.open()?;
        let (mut game, said) = Game::open(&save, pack)?;
        let mut look = game.step_quiet();
        look.text = said;
        record_seen(&file);
        Ok((
            Shared {
                path: path.to_path_buf(),
                file,
                game,
                who: who.to_string(),
            },
            look,
        ))
    }

    /// Plays one move, records it, and writes the world.
    pub fn play(&mut self, command: &str) -> Result<Output, String> {
        let o = self.game.step(command);
        self.file.moves.push(Move {
            who: self.who.clone(),
            at: now(),
            command: command.to_string(),
            text: o.text.clone(),
        });
        self.file.store(&self.game);
        write_file(&self.path, &self.file)?;
        record_seen(&self.file);
        Ok(o)
    }

    /// Leaves a message across the table, and writes the world.
    pub fn talk(&mut self, text: &str) -> Result<(), String> {
        self.file.say(&self.who, now(), text);
        write_file(&self.path, &self.file)
    }

    /// Messages after the first `since`.
    pub fn talk_since(&self, since: usize) -> Vec<Talk> {
        self.file.talk_since(since).to_vec()
    }
}

/// What a merge did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    /// Kept one copy (with both copies' table talk).
    Kept(Side),
    /// The world split: ours stays, theirs is written beside it as a
    /// branch.
    Branched(PathBuf),
}

/// Merges another copy of a world into ours (for a sync that found both
/// changed): the newer copy wins, table talk is joined, and a split keeps
/// both lines, theirs as a branch file beside ours.
pub fn merge(ours_path: &Path, theirs_path: &Path) -> Result<Merged, String> {
    let ours = read_file(ours_path)?;
    let theirs = read_file(theirs_path)?;
    let (how, kept, branch) = merge_files(&ours, &theirs)?;
    write_file(ours_path, &kept)?;
    match (how, branch) {
        (_, Some(branch)) => {
            let name = format!(
                "{}.{}.world",
                ours_path
                    .file_stem()
                    .map_or("world".into(), |s| s.to_string_lossy()),
                branch.branch
            );
            let path = ours_path.with_file_name(name);
            write_file(&path, &branch)?;
            Ok(Merged::Branched(path))
        }
        (
            Merge::Newer {
                newest: Side::Theirs,
            },
            None,
        ) => Ok(Merged::Kept(Side::Theirs)),
        _ => Ok(Merged::Kept(Side::Ours)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Pack {
        crate::load_pack(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
            .expect("content")
    }

    #[test]
    fn two_players_take_turns_rewinds_are_refused_and_splits_branch() {
        let dir = std::env::temp_dir().join(format!("scraped-world-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("SCRAPED_HOME", dir.join("home-jb"));
        let path = dir.join("w.world");
        let (mut jb, _) = Shared::create(&path, "w1", "jb", 42, "standard", pack()).unwrap();
        jb.play("look").unwrap();
        jb.talk("try the temple").unwrap();
        let old = std::fs::read_to_string(&path).unwrap();
        jb.play("wait").unwrap();
        // The AI, on another machine, carries on from the newest copy.
        std::env::set_var("SCRAPED_HOME", dir.join("home-ai"));
        let (mut ai, _) = Shared::open(&path, "ai", pack()).unwrap();
        assert_eq!(ai.talk_since(0).len(), 1);
        ai.play("listen").unwrap();
        assert_eq!(ai.file.last_mover(), Some("ai"));
        // Back on Jb's machine, an older copy is refused.
        std::env::set_var("SCRAPED_HOME", dir.join("home-jb"));
        let stale = dir.join("stale.world");
        std::fs::write(&stale, &old).unwrap();
        assert!(Shared::open(&stale, "jb", pack()).is_err());
        // Both play on from the same copy: a split, kept as a branch.
        let mine = dir.join("mine.world");
        std::fs::copy(&path, &mine).unwrap();
        std::env::set_var("SCRAPED_HOME", dir.join("home-x"));
        let (mut a, _) = Shared::open(&mine, "jb", pack()).unwrap();
        a.play("sleep").unwrap();
        std::env::set_var("SCRAPED_HOME", dir.join("home-y"));
        let (mut b, _) = Shared::open(&path, "ai", pack()).unwrap();
        b.play("drink").unwrap();
        match merge(&mine, &path).unwrap() {
            Merged::Branched(p) => {
                // Each line carries on, each on its own player's machine.
                assert!(Shared::open(&p, "ai", pack()).is_ok());
                std::env::set_var("SCRAPED_HOME", dir.join("home-x"));
                assert!(Shared::open(&mine, "jb", pack()).is_ok());
                // And the other line, here, is a split to merge first.
                assert!(Shared::open(&path, "jb", pack()).is_err());
            }
            other => panic!("expected a branch, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
