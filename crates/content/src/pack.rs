//! The content pack: Jb's templates, in plain TOML files under `content/`.
//!
//! The format is one `[[variant]]` table per variant, so files diff well
//! and can be edited by hand or by the authoring tool. Loading takes file
//! texts, never paths, so it works in a browser.

use serde::{Deserialize, Serialize};

/// One way of filling a slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub slot: String,
    pub text: String,
    /// Condition on the slot's variables; the variant is only used when it
    /// holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Relative likelihood among the variants that apply.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub weight: u32,
    /// An agent-written example showing the intended shape. Counts as a
    /// placeholder: the release check refuses packs that rely on examples.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub example: bool,
}

fn one() -> u32 {
    1
}

fn is_one(n: &u32) -> bool {
    *n == 1
}

/// One file of the pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackFile {
    /// Path relative to the `content/` folder, e.g. `glyphs.toml`.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub variants: Vec<Variant>,
}

/// A problem reading a pack file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackError {
    pub path: String,
    pub message: String,
}

#[derive(Deserialize)]
struct FileToml {
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    variant: Vec<Variant>,
}

impl PackFile {
    /// Parses one file's text.
    pub fn parse(path: &str, text: &str) -> Result<Self, PackError> {
        let parsed: FileToml = toml::from_str(text).map_err(|e| PackError {
            path: path.to_string(),
            message: e.to_string(),
        })?;
        Ok(PackFile {
            path: path.to_string(),
            notes: parsed.notes,
            variants: parsed.variant,
        })
    }

    /// The file as TOML, in the canonical layout the authoring tool writes.
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        if let Some(notes) = &self.notes {
            out.push_str(&format!("notes = {}\n", toml_string(notes)));
        }
        for v in &self.variants {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("[[variant]]\n");
            out.push_str(&format!("slot = {}\n", toml_string(&v.slot)));
            out.push_str(&format!("text = {}\n", toml_string(&v.text)));
            if let Some(w) = &v.when {
                out.push_str(&format!("when = {}\n", toml_string(w)));
            }
            if v.weight != 1 {
                out.push_str(&format!("weight = {}\n", v.weight));
            }
            if v.example {
                out.push_str("example = true\n");
            }
        }
        out
    }
}

/// A TOML string: multi-line literal for multi-line text, so templates stay
/// readable; otherwise a basic string with escapes.
fn toml_string(s: &str) -> String {
    if s.contains('\n') && !s.contains("'''") && !s.ends_with('\'') {
        format!("'''\n{s}'''")
    } else {
        serde_json::to_string(s).expect("string serialises")
    }
}

/// All of Jb's content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pack {
    pub files: Vec<PackFile>,
}

impl Pack {
    /// Loads a pack from (path, text) pairs. Files that fail to parse are
    /// reported and skipped.
    pub fn load(files: &[(String, String)]) -> (Self, Vec<PackError>) {
        let mut pack = Pack::default();
        let mut errors = Vec::new();
        let mut sorted: Vec<&(String, String)> = files.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, text) in sorted {
            match PackFile::parse(path, text) {
                Ok(f) => pack.files.push(f),
                Err(e) => errors.push(e),
            }
        }
        (pack, errors)
    }

    /// Every variant for a slot, in file order.
    pub fn variants(&self, slot: &str) -> Vec<&Variant> {
        self.files
            .iter()
            .flat_map(|f| f.variants.iter())
            .filter(|v| v.slot == slot)
            .collect()
    }

    /// Every variant with the file it lives in.
    pub fn all_variants(&self) -> impl Iterator<Item = (&PackFile, usize, &Variant)> {
        self.files
            .iter()
            .flat_map(|f| f.variants.iter().enumerate().map(move |(i, v)| (f, i, v)))
    }

    /// A short hash identifying this exact content. A run records it, so a
    /// replay can tell whether the text it was played with has changed.
    pub fn version(&self) -> String {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for f in &self.files {
            for b in f
                .path
                .bytes()
                .chain([0])
                .chain(f.to_toml().bytes())
                .chain([0])
            {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        format!("{h:016x}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
notes = "Glyph descriptions."

[[variant]]
slot = "glyph.stroke"
text = "a {stroke}"

[[variant]]
slot = "glyph.stroke"
text = '''
a {stroke}, turned {turn}'''
when = "turn != 'none'"
weight = 3
example = true
"#;

    #[test]
    fn parses_and_round_trips() {
        let f = PackFile::parse("glyphs.toml", SAMPLE).unwrap();
        assert_eq!(f.variants.len(), 2);
        assert_eq!(f.variants[1].weight, 3);
        assert!(f.variants[1].example);
        assert_eq!(f.variants[1].text, "a {stroke}, turned {turn}");
        let again = PackFile::parse("glyphs.toml", &f.to_toml()).unwrap();
        assert_eq!(again, f);
    }

    #[test]
    fn multi_line_text_survives() {
        let f = PackFile {
            path: "x.toml".into(),
            notes: None,
            variants: vec![Variant {
                slot: "a.b".into(),
                text: "line one\nline 'two'\n".into(),
                when: None,
                weight: 1,
                example: false,
            }],
        };
        assert_eq!(PackFile::parse("x.toml", &f.to_toml()).unwrap(), f);
        let mut odd = f.clone();
        odd.variants[0].text = "ends with quote'\nand '''triple'''".into();
        assert_eq!(PackFile::parse("x.toml", &odd.to_toml()).unwrap(), odd);
    }

    #[test]
    fn bad_files_are_reported_not_fatal() {
        let (pack, errors) = Pack::load(&[
            ("a.toml".into(), SAMPLE.into()),
            ("b.toml".into(), "[[variant]\nbroken".into()),
        ]);
        assert_eq!(pack.files.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path, "b.toml");
    }

    #[test]
    fn version_changes_with_content() {
        let (a, _) = Pack::load(&[("a.toml".into(), SAMPLE.into())]);
        let (b, _) = Pack::load(&[(
            "a.toml".into(),
            SAMPLE.replace("a {stroke}\"", "an {stroke}\""),
        )]);
        assert_ne!(a.version(), b.version());
        assert_eq!(a.version(), a.clone().version());
    }
}
