//! The player's side of the game: a line-based session that the terminal
//! client and the JSON-lines agent protocol both drive.
//!
//! Session commands (save, load, transcript, export, quit) and the legacy
//! file are handled here; every
//! other line goes to the game. All text shown comes from the game's
//! content slots.

pub mod mcp;

mod baked {
    include!(concat!(env!("OUT_DIR"), "/pack.rs"));
}

/// The fair-play note shown to any agent that plays (C01).
pub const FAIR_PLAY: &str = include_str!("../../../docs/coop/FAIR-PLAY.md");

/// The content pack baked into the program (C01: the player program
/// carries its text with it, and no authoring notes).
pub fn baked_pack() -> Pack {
    let files: Vec<(String, String)> = baked::FILES
        .iter()
        .map(|(n, t)| (n.to_string(), t.to_string()))
        .collect();
    Pack::load(&files).0
}
pub mod world;

use std::path::{Path, PathBuf};

use scraped_content::Pack;
use scraped_game::ending::Legacy;
use scraped_game::{Game, Output, Save};
use serde_json::json;

/// Reads every `.toml` file in a content folder.
pub fn load_pack(dir: &Path) -> Result<Pack, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read content folder {}: {e}", dir.display()))?;
    let mut files = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "toml") {
            let text = std::fs::read_to_string(&p)
                .map_err(|e| format!("cannot read {}: {e}", p.display()))?;
            files.push((
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                text,
            ));
        }
    }
    let (pack, errors) = Pack::load(&files);
    if let Some(e) = errors.first() {
        return Err(format!("{}: {}", e.path, e.message));
    }
    Ok(pack)
}

/// One player's session.
pub struct Session {
    pub game: Game,
    pack: Pack,
    transcript: Option<PathBuf>,
    pub done: bool,
    /// Where the legacy file lives, when legacy is enabled.
    legacy: Option<PathBuf>,
    /// Whether the end of the run has been dealt with (legacy kept, export
    /// offered).
    ended: bool,
}

/// What a line produced.
pub struct Reply {
    pub output: Output,
}

impl Session {
    pub fn new(seed: u64, pack: Pack, spoil: bool) -> (Self, Output) {
        Self::with_legacy(seed, pack, spoil, None)
    }

    /// A session with legacy enabled: the world carries the final
    /// inscription in `legacy` (if the file exists), and this run's final
    /// inscription replaces it when the run ends.
    pub fn with_legacy(
        seed: u64,
        pack: Pack,
        spoil: bool,
        legacy: Option<PathBuf>,
    ) -> (Self, Output) {
        Self::create(seed, "standard", pack, spoil, legacy)
    }

    /// A session at a difficulty preset, with legacy if a file is given.
    pub fn create(
        seed: u64,
        preset: &str,
        pack: Pack,
        spoil: bool,
        legacy: Option<PathBuf>,
    ) -> (Self, Output) {
        let carried: Option<Legacy> = legacy
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok());
        let mut game = Game::create(seed, pack.clone(), preset, carried);
        game.spoil = spoil;
        let mut first = game.start();
        let code = game.seed_code();
        let note = game.say_code(&code);
        first.text.push_str("\n\n");
        first.text.push_str(&note);
        (
            Session {
                game,
                pack,
                transcript: None,
                done: false,
                legacy,
                ended: false,
            },
            first,
        )
    }

    /// Writes the notebook: transcript, named places and the run record.
    fn export(&mut self, prefix: &str) -> Output {
        let (transcript, places) = self.game.notebook_files();
        let record = serde_json::to_string_pretty(&self.game.record()).expect("record serialises");
        let files = [
            (format!("{prefix}.transcript.txt"), transcript),
            (format!("{prefix}.places.txt"), places),
            (format!("{prefix}.record.json"), record),
        ];
        for (path, body) in &files {
            if let Err(e) = std::fs::write(path, body) {
                return self.wrap_output(format!("[{path}: {e}]"));
            }
        }
        self.note("say.exported")
    }

    /// Once a run ends: keep its legacy, and offer the notebook.
    fn at_end(&mut self, out: &mut Output) {
        if self.ended || self.game.ending().is_none() {
            return;
        }
        self.ended = true;
        if let (Some(path), Some(l)) = (&self.legacy, self.game.legacy()) {
            let body = serde_json::to_string_pretty(&l).expect("legacy serialises");
            if std::fs::write(path, body).is_ok() {
                out.text.push_str("\n\n");
                out.text.push_str(&self.game.message("say.legacy_kept"));
            }
        }
        out.text.push_str("\n\n");
        out.text.push_str(&self.game.message("say.export_offer"));
    }

    fn default_save(&self) -> PathBuf {
        PathBuf::from(format!("scraped-{}.save.json", self.game.seed()))
    }

    fn note(&mut self, slot: &str) -> Output {
        let text = self.game.message(slot);
        self.wrap_output(text)
    }

    fn wrap_output(&mut self, text: String) -> Output {
        let mut o = self.game.step_quiet();
        o.text = text;
        o
    }

    /// Handles one line of input.
    pub fn line(&mut self, input: &str) -> Output {
        let words: Vec<&str> = input.split_whitespace().collect();
        let out = match words.as_slice() {
            ["quit"] | ["q"] | ["exit"] => {
                self.done = true;
                self.wrap_output(String::new())
            }
            ["save", rest @ ..] => {
                let path = rest
                    .first()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.default_save());
                let save = self.game.save();
                // Sealed (C01): not for reading or editing by eye.
                let sealed = scraped_game::saves::seal(
                    &serde_json::to_string(&save).expect("save serialises"),
                );
                match std::fs::write(&path, sealed) {
                    Ok(()) => self.note("say.saved"),
                    Err(e) => self.wrap_output(format!("[{}: {e}]", path.display())),
                }
            }
            ["load", rest @ ..] => {
                let path = rest
                    .first()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.default_save());
                let loaded = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|t| scraped_game::saves::unseal(&t))
                    .and_then(|t| serde_json::from_str::<Save>(&t).map_err(|e| e.to_string()));
                match loaded.and_then(|save| Game::open(&save, self.pack.clone())) {
                    Ok((game, mut text)) => {
                        let spoil = self.game.spoil;
                        self.game = game;
                        self.ended = self.game.ending().is_some();
                        self.game.spoil = spoil;
                        let look = self.game.step("look");
                        text.push_str("\n\n");
                        text.push_str(&look.text);
                        self.wrap_output(text)
                    }
                    Err(e) => self.wrap_output(format!("[{}: {e}]", path.display())),
                }
            }
            ["transcript", "off"] => {
                self.transcript = None;
                self.wrap_output(String::new())
            }
            ["transcript", rest @ ..] => {
                let path = rest
                    .iter()
                    .find(|w| **w != "on")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(format!("scraped-{}.transcript.txt", self.game.seed()))
                    });
                self.transcript = Some(path);
                self.wrap_output(String::new())
            }
            ["code"] => {
                let code = self.game.seed_code();
                let t = self.game.say_code(&code);
                self.wrap_output(t)
            }
            ["export", rest @ ..] => {
                let prefix = rest
                    .first()
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| format!("scraped-{}", self.game.seed()));
                self.export(&prefix)
            }
            _ => self.game.step(input),
        };
        let mut out = out;
        self.at_end(&mut out);
        if let Some(path) = &self.transcript {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(f, "> {input}\n{}\n", out.text);
            }
        }
        out
    }
}

/// The JSON-lines protocol's version. Bump it when a response changes
/// shape in a way agents must notice (see docs/PROTOCOL.md).
pub const PROTOCOL: u32 = 1;

/// One protocol response line.
pub fn protocol_line(o: &Output) -> String {
    let mut v = json!({ "protocol": PROTOCOL, "text": o.text, "state": o.state });
    if let Some(t) = &o.truth {
        v["truth"] = t.clone();
    }
    v.to_string()
}

/// Wraps text to `width` columns, keeping existing line breaks.
pub fn wrap(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }
    text.split('\n')
        .map(|line| {
            let mut out = String::new();
            let mut col = 0;
            for word in line.split(' ') {
                let len = word.chars().count();
                if col > 0 && col + 1 + len > width {
                    out.push('\n');
                    col = 0;
                } else if col > 0 {
                    out.push(' ');
                    col += 1;
                }
                out.push_str(word);
                col += len;
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapping_keeps_breaks_and_width() {
        let w = wrap("one two three four five\n\nsix", 9);
        assert_eq!(w, "one two\nthree\nfour five\n\nsix");
        assert_eq!(wrap("x y", 0), "x y");
    }

    fn content() -> Pack {
        load_pack(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"))
            .expect("content loads")
    }

    #[test]
    fn protocol_lines_are_json_with_the_session_text() {
        let (mut s, first) = Session::new(7, content(), true);
        let mut outs = vec![first];
        for line in ["look", "help", "inventory", "dance", "transcript off"] {
            outs.push(s.line(line));
        }
        for o in &outs {
            let line = protocol_line(o);
            assert!(!line.contains('\n'), "one line per response");
            let v: serde_json::Value = serde_json::from_str(&line).expect("valid JSON");
            assert_eq!(v["text"], o.text.as_str());
            assert_eq!(
                v["protocol"], PROTOCOL,
                "every line carries the protocol version"
            );
            for key in [
                "place", "things", "carried", "exits", "minutes", "body", "light", "load",
            ] {
                assert!(
                    v["state"].get(key).is_some(),
                    "state.{key} is part of protocol 1"
                );
            }
            assert!(v["state"].is_object());
            assert!(v.get("truth").is_none() || o.truth.is_some());
        }
        assert!(!s.done);
        s.line("quit");
        assert!(s.done);
    }

    #[test]
    fn legacy_files_and_the_notebook() {
        let dir = std::env::temp_dir().join(format!("scraped-play-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("legacy.json");
        let carried = r#"{"from_seed":1,"ending":"left","meaning":{"type":"clause","value":{"predicate":"depart","mood":"potent","tense":"nonpast","polarity":"positive","args":[{"role":"subject","np":{"head":{"concept":"self"},"number":"singular"}}]}}}"#;
        std::fs::write(&file, carried).unwrap();
        let (mut s, _) = Session::with_legacy(42, content(), false, Some(file.clone()));
        assert!(s.game.site.writing.legacy.is_some(), "legacy placed");
        let (plain, _) = Session::new(42, content(), false);
        assert!(plain.game.site.writing.legacy.is_none());
        // Run out of water; the end offers the notebook once.
        let mut offered = 0;
        for _ in 0..20 {
            let o = s.line("wait 1 day");
            if o.text.contains(&s.game.message("say.export_offer")) {
                offered += 1;
            }
        }
        assert_eq!(offered, 1);
        // Nothing written: the old legacy stays.
        assert_eq!(std::fs::read_to_string(&file).unwrap(), carried);
        let prefix = dir.join("notebook");
        s.line(&format!("export {}", prefix.display()));
        for ext in ["transcript.txt", "places.txt", "record.json"] {
            let p = format!("{}.{ext}", prefix.display());
            assert!(std::fs::metadata(&p).is_ok(), "{p}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
