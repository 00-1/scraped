//! A world shared between two players (C01): Jb in the app, and an AI
//! through the player program, taking turns whenever either likes, with no
//! server. The world file holds the sealed save, every move with what it
//! showed, and table talk (messages that aren't moves), each tagged with
//! who made it and when. Whoever has the newest copy plays; if both played
//! on from the same point, the world has split, and both lines can be kept
//! as branches.
//!
//! Pure data and rules: the clients read and write the files and supply
//! the clock.

use serde::{Deserialize, Serialize};

use crate::saves::{self, Continuity};
use crate::{Game, Save};

/// Who made a move or said something.
pub type Who = String;

/// One move: the command, what it showed, who made it and when (seconds
/// since 1970, from the client's clock).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Move {
    pub who: Who,
    pub at: u64,
    pub command: String,
    pub text: String,
}

/// A message across the table, not a move. `turn` is the move it follows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Talk {
    pub who: Who,
    pub at: u64,
    pub turn: usize,
    pub text: String,
}

/// A shared world, as written to the shared location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldFile {
    /// Stays the same for the world's whole life, across copies.
    pub id: String,
    /// Which line of the world this is ("" for the first; a branch is
    /// named when the world splits).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch: String,
    /// The sealed save (see [`saves::seal`]).
    pub save: String,
    pub turn: usize,
    pub chain: String,
    pub moves: Vec<Move>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub talk: Vec<Talk>,
}

impl WorldFile {
    /// A new shared world from a game just begun.
    pub fn new(id: &str, g: &Game) -> Self {
        let mut w = WorldFile {
            id: id.to_string(),
            branch: String::new(),
            save: String::new(),
            turn: 0,
            chain: String::new(),
            moves: Vec::new(),
            talk: Vec::new(),
        };
        w.store(g);
        w
    }

    /// Writes the game's state into the file.
    pub fn store(&mut self, g: &Game) {
        let save = g.save();
        self.turn = save.turn;
        self.chain = save.chain.clone();
        self.save = saves::seal(&serde_json::to_string(&save).expect("a save serialises"));
    }

    /// The save inside.
    pub fn open(&self) -> Result<Save, String> {
        let json = saves::unseal(&self.save)?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }

    /// Plays one move on `g` (loaded from this file), recording it.
    pub fn play(&mut self, g: &mut Game, who: &str, at: u64, command: &str) -> String {
        let text = g.step(command).text;
        self.moves.push(Move {
            who: who.to_string(),
            at,
            command: command.to_string(),
            text: text.clone(),
        });
        self.store(g);
        text
    }

    /// Leaves a message across the table.
    pub fn say(&mut self, who: &str, at: u64, text: &str) {
        self.talk.push(Talk {
            who: who.to_string(),
            at,
            turn: self.turn,
            text: text.to_string(),
        });
    }

    /// Messages after the first `since`.
    pub fn talk_since(&self, since: usize) -> &[Talk] {
        &self.talk[since.min(self.talk.len())..]
    }

    /// Who moved last.
    pub fn last_mover(&self) -> Option<&str> {
        self.moves.last().map(|m| m.who.as_str())
    }
}

/// What two copies of the same world are to each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Merge {
    /// The same moves.
    Same,
    /// One copy is the other played on: keep the newer.
    Newer { newest: Side },
    /// Both played on from `turn`: keep one line, or both as branches.
    Split { turn: usize },
}

/// Which of two copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Ours,
    Theirs,
}

/// Compares two copies of one world by their moves. Table talk never
/// splits a world: [`merge_talk`] joins it.
pub fn compare(ours: &WorldFile, theirs: &WorldFile) -> Merge {
    let common = ours
        .moves
        .iter()
        .zip(&theirs.moves)
        .take_while(|(a, b)| a.command == b.command && a.who == b.who)
        .count();
    let (a, b) = (ours.moves.len(), theirs.moves.len());
    if common == a && common == b {
        Merge::Same
    } else if common == b {
        Merge::Newer { newest: Side::Ours }
    } else if common == a {
        Merge::Newer {
            newest: Side::Theirs,
        }
    } else {
        Merge::Split { turn: common }
    }
}

/// The table talk of both copies, in order, without repeats.
pub fn merge_talk(ours: &[Talk], theirs: &[Talk]) -> Vec<Talk> {
    let mut all: Vec<Talk> = ours.to_vec();
    for t in theirs {
        if !all.contains(t) {
            all.push(t.clone());
        }
    }
    all.sort_by_key(|t| (t.at, t.turn));
    all
}

/// Whether a player program that has seen this world at (`turn`, `chain`)
/// may carry on from `file`: never from an older copy (no undo).
pub fn may_continue(file: &WorldFile, seen: Option<(usize, &str)>) -> Continuity {
    saves::continues(file.turn, &file.chain, seen)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(moves: &[(&str, &str)]) -> WorldFile {
        WorldFile {
            id: "w".into(),
            branch: String::new(),
            save: String::new(),
            turn: moves.len(),
            chain: String::new(),
            moves: moves
                .iter()
                .map(|(w, c)| Move {
                    who: w.to_string(),
                    at: 0,
                    command: c.to_string(),
                    text: String::new(),
                })
                .collect(),
            talk: Vec::new(),
        }
    }

    #[test]
    fn newer_copies_and_splits() {
        let base = file(&[("jb", "look"), ("ai", "north")]);
        let on = file(&[("jb", "look"), ("ai", "north"), ("ai", "read")]);
        let other = file(&[("jb", "look"), ("ai", "north"), ("jb", "sleep")]);
        assert_eq!(compare(&base, &base), Merge::Same);
        assert_eq!(compare(&on, &base), Merge::Newer { newest: Side::Ours });
        assert_eq!(
            compare(&base, &on),
            Merge::Newer {
                newest: Side::Theirs
            }
        );
        assert_eq!(compare(&on, &other), Merge::Split { turn: 2 });
    }

    #[test]
    fn talk_joins_without_repeats() {
        let t = |who: &str, at| Talk {
            who: who.into(),
            at,
            turn: 0,
            text: "hi".into(),
        };
        let all = merge_talk(&[t("jb", 1), t("ai", 3)], &[t("ai", 3), t("ai", 2)]);
        assert_eq!(all.iter().map(|t| t.at).collect::<Vec<_>>(), vec![1, 2, 3]);
    }
}
