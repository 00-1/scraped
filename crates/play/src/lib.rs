//! The player's side of the game: a line-based session that the terminal
//! client and the JSON-lines agent protocol both drive.
//!
//! Session commands (save, load, transcript, quit) are handled here; every
//! other line goes to the game. All text shown comes from the game's
//! content slots.

use std::path::{Path, PathBuf};

use scraped_content::Pack;
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
}

/// What a line produced.
pub struct Reply {
    pub output: Output,
}

impl Session {
    pub fn new(seed: u64, pack: Pack, spoil: bool) -> (Self, Output) {
        let mut game = Game::new(seed, pack.clone());
        game.spoil = spoil;
        let first = game.start();
        (
            Session {
                game,
                pack,
                transcript: None,
                done: false,
            },
            first,
        )
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
                match std::fs::write(
                    &path,
                    serde_json::to_string_pretty(&save).expect("save serialises"),
                ) {
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
                    .and_then(|t| serde_json::from_str::<Save>(&t).map_err(|e| e.to_string()));
                match loaded {
                    Ok(save) => {
                        let spoil = self.game.spoil;
                        let (game, changed) = Game::load(&save, self.pack.clone());
                        self.game = game;
                        self.game.spoil = spoil;
                        let mut text = self.game.message("say.loaded");
                        if changed {
                            text.push_str("\n\n");
                            text.push_str(&self.game.message("say.pack_changed"));
                        }
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
            _ => self.game.step(input),
        };
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

/// One protocol response line.
pub fn protocol_line(o: &Output) -> String {
    let mut v = json!({ "text": o.text, "state": o.state });
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
            assert!(v["state"].is_object());
            assert!(v.get("truth").is_none() || o.truth.is_some());
        }
        assert!(!s.done);
        s.line("quit");
        assert!(s.done);
    }
}
