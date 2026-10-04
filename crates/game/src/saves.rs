//! Saves that survive updates (C01): a snapshot of play, restored on load
//! rather than replayed; the builds each stretch of play ran on, so a game
//! can be replayed exactly; a turn counter and a chain over the moves, so
//! an older copy of a world can't quietly continue; and a sealed form a
//! person can't read or edit by eye (this discourages, it doesn't secure).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Game, State, Travelled};

/// The engine's version, recorded in every save.
pub const ENGINE: &str = env!("CARGO_PKG_VERSION");

/// Everything about play that the world, made again from its seed, does
/// not hold. The player's state and the world's lasting changes both live
/// in [`State`] (kept apart there, for multiplayer later).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub state: State,
    /// What the templates remember having said.
    pub memory: BTreeMap<String, String>,
    pub transcript: Vec<(String, String)>,
    pub hooks_seen: BTreeSet<String>,
    pub summarised: bool,
    pub threshold: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_link: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_travel: Option<Travelled>,
    #[serde(default)]
    pub reading_now: bool,
    #[serde(default)]
    pub listening: bool,
    #[serde(default)]
    pub attentive: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_scrape: Option<usize>,
    #[serde(default)]
    pub scrape_felt: bool,
}

/// A stretch of play on one build: engine version and content pack, from
/// one turn to another (turn = commands played).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stretch {
    pub engine: String,
    pub pack: String,
    pub from: usize,
    pub to: usize,
}

/// Extends the history with play on this build up to `turn`.
pub fn extend(history: &mut Vec<Stretch>, engine: &str, pack: &str, turn: usize) {
    match history.last_mut() {
        Some(s) if s.engine == engine && s.pack == pack => s.to = turn,
        last => {
            let from = last.map_or(0, |s| s.to);
            history.push(Stretch {
                engine: engine.to_string(),
                pack: pack.to_string(),
                from,
                to: turn,
            });
        }
    }
}

/// The chain over a world's moves: each move folded into the one before,
/// so two copies agree on it only if they played the same moves.
pub fn chain(moves: &[String]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for m in moves {
        for b in m.bytes().chain(std::iter::once(0)) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h = mix(h);
    }
    format!("{h:016x}")
}

fn mix(mut v: u64) -> u64 {
    v ^= v >> 31;
    v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    v ^= v >> 29;
    v
}

/// Whether a copy of a world at `turn` with `chain` may continue, given the
/// newest copy of it seen (`seen`: turn and chain). A copy from earlier on
/// the same line is a rewind; a copy on another line is a split.
pub fn continues(turn: usize, chain: &str, seen: Option<(usize, &str)>) -> Continuity {
    match seen {
        None => Continuity::Fine,
        Some((t, c)) if t == turn && c == chain => Continuity::Fine,
        Some((t, _)) if turn > t => Continuity::Fine,
        Some((t, _)) if turn < t => Continuity::Rewind,
        _ => Continuity::Split,
    }
}

/// What carrying on from a copy of a world would be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Continuity {
    Fine,
    /// An older copy than one already played on.
    Rewind,
    /// The same number of moves, but different ones: the world split.
    Split,
}

// ---------- versions ----------

/// The smallest version bump a change needs (see docs/VERSIONING.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Bump {
    /// Saved worlds replay identically.
    Patch,
    /// Saved worlds load from their snapshot and carry on, but replay
    /// differently.
    Minor,
    /// Saved worlds don't load, or their worlds generate differently.
    Major,
}

/// What this build makes of a saved world: whether its world still
/// generates the same (`world`: the fingerprint recorded with the save),
/// whether its snapshot loads and carries on, and whether its moves replay
/// to the same state.
pub fn verdict(save: &crate::Save, world: &str, pack: scraped_content::Pack) -> (Bump, String) {
    let fresh = Game::create(
        save.seed,
        pack.clone(),
        &save.difficulty,
        save.legacy.clone(),
    );
    if fresh.site.world.fingerprint() != world {
        return (Bump::Major, "its world generates differently".into());
    }
    let Some(snap) = &save.snapshot else {
        return (Bump::Major, "it has no snapshot to load".into());
    };
    let (mut loaded, _) = Game::load(save, pack.clone());
    loaded.step("look");
    let mut replay = fresh;
    replay.start();
    for c in &save.commands {
        replay.step(c);
    }
    // Wording is not rules: what was said, and what the templates
    // remember saying, may change in a patch.
    let rules = |s: &Snapshot| {
        let mut s = s.clone();
        s.transcript.clear();
        s.memory.clear();
        s
    };
    if rules(&replay.snapshot()) == rules(snap) {
        (Bump::Patch, "it replays identically".into())
    } else {
        (
            Bump::Minor,
            "it loads and carries on, but replays differently".into(),
        )
    }
}

/// Replays one stretch of a saved game on this build: moves `from..to`,
/// starting from the seed (`start` none, `from` 0) or from the save the
/// previous stretch's build left. Returns the save at `to`.
pub fn replay_stretch(
    save: &crate::Save,
    from: usize,
    to: usize,
    start: Option<&crate::Save>,
    pack: scraped_content::Pack,
) -> Result<crate::Save, String> {
    let to = to.min(save.commands.len());
    let mut g = match start {
        Some(prev) => {
            if prev.turn != from {
                return Err(format!(
                    "the previous stretch ended at move {}, not {from}",
                    prev.turn
                ));
            }
            Game::load(prev, pack).0
        }
        None => {
            if from != 0 {
                return Err("only the first stretch starts from the seed".into());
            }
            let mut g = Game::create(save.seed, pack, &save.difficulty, save.legacy.clone());
            g.start();
            g
        }
    };
    for c in &save.commands[from..to] {
        g.step(c);
    }
    Ok(g.save())
}

/// Whether two saves reached the same play (wording aside).
pub fn same_play(a: &crate::Save, b: &crate::Save) -> bool {
    let rules = |s: &crate::Save| {
        s.snapshot.as_ref().map(|s| {
            let mut s = (**s).clone();
            s.transcript.clear();
            s.memory.clear();
            s
        })
    };
    a.commands == b.commands && rules(a) == rules(b)
}

/// Whether this build may open a world last played on `engine` (C01): never
/// one from another major version (a newer one needs this program updated;
/// an older one needs a new world), and noting the step up from an older
/// minor version. `Ok(true)` means an upgrade to note.
// DEBUG-TEXT: errors for players' programs and agents.
pub fn may_open(engine: &str) -> Result<bool, String> {
    may_open_on(engine, ENGINE)
}

fn may_open_on(engine: &str, ours: &str) -> Result<bool, String> {
    let part = |v: &str, i: usize| -> u64 {
        v.trim_start_matches('v')
            .split('.')
            .nth(i)
            .and_then(|p| p.parse().ok())
            .unwrap_or(0)
    };
    let major = |v: &str| part(v, 0);
    let minor = |v: &str| part(v, 1);
    if engine.is_empty() {
        return Ok(false);
    }
    match bump_between(engine, ours) {
        Some(Bump::Major) if major(engine) > major(ours) => Err(format!(
            "this world was played on version {engine}, newer than this program ({ours}): update the player program"
        )),
        Some(Bump::Major) => Err(format!(
            "this world was played on version {engine}, which this program ({ours}) cannot carry on: start a new world"
        )),
        Some(Bump::Minor) => Ok(minor(engine) < minor(ours)),
        _ => Ok(false),
    }
}

/// The bump between two versions ("0.1.0" to "0.2.0" is minor).
pub fn bump_between(old: &str, new: &str) -> Option<Bump> {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut it = v.trim_start_matches('v').split('.').map(|p| p.parse().ok());
        Some((it.next()??, it.next()??, it.next()??))
    };
    let (a, b) = (parse(old)?, parse(new)?);
    Some(if b.0 != a.0 {
        Bump::Major
    } else if b.1 != a.1 {
        Bump::Minor
    } else {
        Bump::Patch
    })
}

// ---------- sealing ----------

/// The prefix of a sealed save.
const SEAL: &str = "SCRAPED1:";
/// The key built into the program.
// DESIGN-Q: a fixed key in the source: enough to stop reading by eye.
const KEY: u64 = 0x5c4a_9e1d_7b23_f00d;

fn keystream(n: usize) -> impl Iterator<Item = u8> {
    let mut s = KEY;
    (0..n).map(move |i| {
        if i % 8 == 0 {
            s = mix(s.wrapping_add(0x9e37_79b9_7f4a_7c15));
        }
        (s >> ((i % 8) * 8)) as u8
    })
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// A save's JSON, scrambled and written in a plain alphabet.
pub fn seal(json: &str) -> String {
    let bytes: Vec<u8> = json
        .bytes()
        .zip(keystream(json.len()))
        .map(|(b, k)| b ^ k)
        .collect();
    let mut out = String::from(SEAL);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    out
}

/// The JSON of a sealed save; plain JSON passes through, for old saves.
pub fn unseal(text: &str) -> Result<String, String> {
    let Some(body) = text.trim().strip_prefix(SEAL) else {
        return Ok(text.to_string());
    };
    let mut bytes = Vec::with_capacity(body.len() * 3 / 4);
    let digits: Vec<u32> = body
        .bytes()
        .map(|c| {
            B64.iter()
                .position(|&b| b == c)
                .map(|p| p as u32)
                .ok_or("not a save")
        })
        .collect::<Result<_, _>>()?;
    for chunk in digits.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, &d)| n | d << (18 - 6 * i));
        for i in 0..chunk.len().saturating_sub(1) {
            bytes.push((n >> (16 - 8 * i)) as u8);
        }
    }
    let plain: Vec<u8> = bytes
        .iter()
        .zip(keystream(bytes.len()))
        .map(|(b, k)| b ^ k)
        .collect();
    String::from_utf8(plain).map_err(|_| "not a save".to_string())
}

impl Game {
    /// The state of play, for a save.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state.clone(),
            memory: self.memory.clone(),
            transcript: self.transcript.clone(),
            hooks_seen: self.hooks_seen.clone(),
            summarised: self.summarised,
            threshold: self.threshold,
            last_link: self.last_link,
            last_travel: self.last_travel.clone(),
            reading_now: self.reading_now,
            listening: self.listening,
            attentive: self.attentive,
            last_scrape: self.last_scrape,
            scrape_felt: self.scrape_felt,
        }
    }

    /// Puts play back as it was, in a world made again from its seed.
    pub(crate) fn restore(&mut self, s: &Snapshot, log: &[String]) {
        self.state = s.state.clone();
        self.memory = s.memory.clone();
        self.transcript = s.transcript.clone();
        self.hooks_seen = s.hooks_seen.clone();
        self.summarised = s.summarised;
        self.threshold = s.threshold;
        self.last_link = s.last_link;
        self.last_travel = s.last_travel.clone();
        self.reading_now = s.reading_now;
        self.listening = s.listening;
        self.attentive = s.attentive;
        self.last_scrape = s.last_scrape;
        self.scrape_felt = s.scrape_felt;
        self.log = log.to_vec();
        self.sync_made();
        self.sync_written();
        self.recompute_claims();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps_between_versions() {
        assert_eq!(bump_between("0.1.0", "0.1.1"), Some(Bump::Patch));
        assert_eq!(bump_between("0.1.3", "0.2.0"), Some(Bump::Minor));
        assert_eq!(bump_between("v0.9.0", "1.0.0"), Some(Bump::Major));
        assert!(Bump::Patch < Bump::Minor && Bump::Minor < Bump::Major);
        assert_eq!(may_open_on("0.9.3", "0.10.0"), Ok(true));
        assert_eq!(may_open_on("0.10.1", "0.10.0"), Ok(false));
        assert!(may_open_on("2.0.0", "1.4.0").is_err());
        assert!(may_open_on("1.0.0", "2.0.0").is_err());
    }

    #[test]
    fn sealing_round_trips_and_hides() {
        let json = r#"{"seed":42,"commands":["look","go north"],"é":"ü"}"#;
        let sealed = seal(json);
        assert!(!sealed.contains("seed"));
        assert_eq!(unseal(&sealed).unwrap(), json);
        assert_eq!(unseal(json).unwrap(), json);
        for n in 0..10 {
            let s: String = "x".repeat(n);
            assert_eq!(unseal(&seal(&s)).unwrap(), s);
        }
    }

    #[test]
    fn history_starts_a_stretch_per_build() {
        let mut h = Vec::new();
        extend(&mut h, "0.1.0", "p", 5);
        extend(&mut h, "0.1.0", "p", 9);
        extend(&mut h, "0.2.0", "p", 12);
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].from, h[0].to, h[1].from, h[1].to), (0, 9, 9, 12));
    }

    #[test]
    fn rewinds_and_splits_are_told_apart() {
        let a = chain(&["look".into(), "north".into()]);
        let b = chain(&["look".into(), "south".into()]);
        assert_ne!(a, b);
        assert_eq!(continues(2, &a, Some((2, &a))), Continuity::Fine);
        assert_eq!(continues(3, &a, Some((2, &a))), Continuity::Fine);
        assert_eq!(continues(1, &a, Some((2, &a))), Continuity::Rewind);
        assert_eq!(continues(2, &b, Some((2, &a))), Continuity::Split);
    }
}
