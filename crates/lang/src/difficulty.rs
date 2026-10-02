//! Difficulty dials: presentation and complexity choices that change how
//! hard a language is to decipher, without changing what it means.

use serde::Serialize;

use crate::script::ScriptKind;

/// How words are separated on the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Separation {
    Spaces,
    /// A word-divider sign between words.
    Dots,
    /// Scriptio continua: no breaks at all.
    None,
}

/// Whether personal names are flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NameMarking {
    None,
    /// A determinative sign before every name, as in cuneiform.
    Determinative,
}

/// How regular the inflection is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Regularity {
    /// One affix per category, always (M01).
    Regular,
    /// A few affix combinations fuse into a single new form.
    Fused,
}

/// All difficulty dials for one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Difficulty {
    pub separation: Separation,
    pub names: NameMarking,
    /// Force a script type instead of choosing per seed.
    pub script: Option<ScriptKind>,
    /// Number of historical eras, 1–5.
    pub eras: u32,
    pub regularity: Regularity,
}

impl Default for Difficulty {
    /// The proposed defaults from the M02 spec.
    // DESIGN-Q: defaults are spaces, unmarked names, script chosen by seed,
    // three eras, regular morphology. Jb to confirm.
    fn default() -> Self {
        Difficulty {
            separation: Separation::Spaces,
            names: NameMarking::None,
            script: None,
            eras: 3,
            regularity: Regularity::Regular,
        }
    }
}

impl Difficulty {
    /// Clamps the era count to the supported range.
    pub fn eras(&self) -> u32 {
        self.eras.clamp(1, 5)
    }
}
