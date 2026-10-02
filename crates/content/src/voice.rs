//! Voice tools and pack diffs: Jb's recurring words and phrases, phrases
//! repeated across slots, length and rhythm by family, and what changed in
//! the pack since a given version.
//!
//! Only Jb's own variants count for voice; agent-written examples are
//! placeholders and say nothing about his voice.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::lint::{Issue, Severity};
use crate::pack::{Pack, Storylet, Variant};

/// Words too common to make a phrase distinctive.
const COMMON: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "of", "to", "in", "on", "at", "by", "for", "with",
    "from", "is", "are", "was", "were", "be", "it", "its", "this", "that", "you", "your", "there",
    "here", "as", "into", "up", "down", "out", "not", "no", "so", "if", "than", "then", "has",
    "have",
];

/// The words of a template, with variables, helpers and `[if]` markers
/// breaking the text into runs (a phrase never spans one).
fn runs(text: &str) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new()];
    let mut word = String::new();
    let mut depth = 0i32;
    let flush = |word: &mut String, out: &mut Vec<Vec<String>>| {
        if !word.is_empty() {
            out.last_mut().expect("a run").push(std::mem::take(word));
        }
    };
    for c in text.chars() {
        match c {
            '{' | '[' => {
                flush(&mut word, &mut out);
                depth += 1;
                out.push(Vec::new());
            }
            '}' | ']' => {
                depth -= 1;
            }
            _ if depth > 0 => {}
            c if c.is_alphabetic() || c == '\'' => word.extend(c.to_lowercase()),
            c if c.is_whitespace() || c == '-' => flush(&mut word, &mut out),
            _ => {
                flush(&mut word, &mut out);
                out.push(Vec::new());
            }
        }
    }
    flush(&mut word, &mut out);
    out.retain(|r| !r.is_empty());
    out
}

fn jb_variants(pack: &Pack) -> impl Iterator<Item = &Variant> {
    pack.files
        .iter()
        .flat_map(|f| f.variants.iter())
        .filter(|v| !v.example)
}

/// A recurring word or phrase in Jb's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Phrase {
    pub text: String,
    pub count: usize,
    /// Slots it appears in.
    pub slots: Vec<String>,
}

fn distinctive(words: &[String]) -> bool {
    words
        .iter()
        .any(|w| !COMMON.contains(&w.as_str()) && w.len() > 2)
}

/// Every phrase (n words long) in Jb's variants, with where it appears.
fn phrases(pack: &Pack, n: usize) -> BTreeMap<String, (usize, BTreeSet<String>)> {
    let mut out: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
    for v in jb_variants(pack) {
        for run in runs(&v.text) {
            for w in run.windows(n) {
                if !distinctive(w)
                    || (n > 1
                        && COMMON.contains(&w[0].as_str())
                        && COMMON.contains(&w[n - 1].as_str()))
                {
                    continue;
                }
                let e = out.entry(w.join(" ")).or_default();
                e.0 += 1;
                e.1.insert(v.slot.clone());
            }
        }
    }
    out
}

/// Jb's recurring words and phrases (at least `min` uses), most used
/// first: a glossary of his voice.
pub fn glossary(pack: &Pack, min: usize) -> Vec<Phrase> {
    let mut out: Vec<Phrase> = Vec::new();
    for n in 1..=4 {
        for (text, (count, slots)) in phrases(pack, n) {
            if count >= min && (n > 1 || !COMMON.contains(&text.as_str())) {
                out.push(Phrase {
                    text,
                    count,
                    slots: slots.into_iter().collect(),
                });
            }
        }
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.text.cmp(&b.text)));
    out
}

/// How long a distinctive phrase must be before using it in two places
/// is worth a warning.
// DESIGN-Q: four words.
pub const ECHO_WORDS: usize = 4;

/// Warnings for variants that repeat a distinctive phrase used in another
/// slot's text.
pub fn echoes(pack: &Pack) -> Vec<Issue> {
    let shared: BTreeMap<String, BTreeSet<String>> = phrases(pack, ECHO_WORDS)
        .into_iter()
        .filter(|(_, (_, slots))| slots.len() > 1)
        .map(|(p, (_, slots))| (p, slots))
        .collect();
    let mut out = Vec::new();
    for f in &pack.files {
        for (i, v) in f.variants.iter().enumerate() {
            if v.example {
                continue;
            }
            // One warning per other slot echoed, quoting the first phrase.
            let mut said: BTreeSet<&String> = BTreeSet::new();
            for run in runs(&v.text) {
                for w in run.windows(ECHO_WORDS) {
                    let p = w.join(" ");
                    let Some(slots) = shared.get(&p) else {
                        continue;
                    };
                    let others: Vec<&String> = slots
                        .iter()
                        .filter(|s| **s != v.slot && !said.contains(s))
                        .collect();
                    if others.is_empty() {
                        continue;
                    }
                    said.extend(others.iter().copied());
                    out.push(Issue {
                        severity: Severity::Warning,
                        slot: v.slot.clone(),
                        file: Some(f.path.clone()),
                        variant: Some(i),
                        kind: "repeated-phrase",
                        message: format!(
                            "“{p}…” also appears in {}; a player may notice the echo",
                            others
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    });
                }
            }
        }
    }
    out
}

/// Length and rhythm of one slot family's text.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FamilyStats {
    pub family: String,
    pub variants: usize,
    pub mean_chars: f64,
    pub median_chars: usize,
    pub sentences: usize,
    /// Mean words per sentence.
    pub words_per_sentence: f64,
}

/// Length and rhythm by family, over Jb's variants.
pub fn stats(pack: &Pack) -> Vec<FamilyStats> {
    let mut by: BTreeMap<String, Vec<&Variant>> = BTreeMap::new();
    for v in jb_variants(pack) {
        let fam = v.slot.split('.').next().unwrap_or("").to_string();
        by.entry(fam).or_default().push(v);
    }
    by.into_iter()
        .map(|(family, vs)| {
            let mut lens: Vec<usize> = vs.iter().map(|v| v.text.chars().count()).collect();
            lens.sort_unstable();
            let mut sentences = 0;
            let mut words = 0;
            for v in &vs {
                let s = v
                    .text
                    .split(['.', '!', '?'])
                    .filter(|s| s.split_whitespace().count() > 0)
                    .count()
                    .max(1);
                sentences += s;
                words += v.text.split_whitespace().count();
            }
            FamilyStats {
                family,
                variants: vs.len(),
                mean_chars: lens.iter().sum::<usize>() as f64 / lens.len() as f64,
                median_chars: lens[lens.len() / 2],
                sentences,
                words_per_sentence: words as f64 / sentences.max(1) as f64,
            }
        })
        .collect()
}

/// One variant's place in a diff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub slot: String,
    /// Position among the slot's variants.
    pub index: usize,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// What changed between two versions of the pack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct PackDiff {
    pub added: Vec<Change>,
    pub removed: Vec<Change>,
    pub changed: Vec<Change>,
    /// Storylets added, removed or changed (other than their text).
    pub storylets: Vec<String>,
    /// Whether saves made with the old pack will replay differently: text
    /// changes never do, storylet changes can (where and when they happen,
    /// and what they do).
    pub breaks_saves: bool,
}

/// The difference between two packs, variant by variant and storylet by
/// storylet.
// DESIGN-Q: any change to a storylet's rules or effects is treated as
// breaking saves; text never is.
pub fn diff(old: &Pack, new: &Pack) -> PackDiff {
    let by_slot = |p: &Pack| {
        let mut m: BTreeMap<String, Vec<Variant>> = BTreeMap::new();
        for v in p.files.iter().flat_map(|f| f.variants.iter()) {
            m.entry(v.slot.clone()).or_default().push(v.clone());
        }
        m
    };
    let (a, b) = (by_slot(old), by_slot(new));
    let mut d = PackDiff::default();
    let slots: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    let empty = Vec::new();
    for slot in slots {
        let (va, vb) = (a.get(slot).unwrap_or(&empty), b.get(slot).unwrap_or(&empty));
        for i in 0..va.len().max(vb.len()) {
            let c = Change {
                slot: slot.clone(),
                index: i,
                before: va.get(i).map(|v| v.text.clone()),
                after: vb.get(i).map(|v| v.text.clone()),
            };
            match (va.get(i), vb.get(i)) {
                (Some(x), Some(y)) if x != y => d.changed.push(c),
                (None, Some(_)) => d.added.push(c),
                (Some(_), None) => d.removed.push(c),
                _ => {}
            }
        }
    }
    let st = |p: &Pack| -> BTreeMap<String, Storylet> {
        p.storylets().map(|s| (s.id.clone(), s.clone())).collect()
    };
    let (sa, sb) = (st(old), st(new));
    let ids: BTreeSet<&String> = sa.keys().chain(sb.keys()).collect();
    for id in ids {
        if sa.get(id) != sb.get(id) {
            d.storylets.push(id.clone());
        }
    }
    d.breaks_saves = !d.storylets.is_empty();
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackFile;

    fn pack(text: &str) -> Pack {
        Pack {
            files: vec![PackFile::parse("a.toml", text).unwrap()],
        }
    }

    const P: &str = r#"
[[variant]]
slot = "a.one"
text = "The wind keens over the broken stones of {place}."

[[variant]]
slot = "b.two"
text = "Again the wind keens over the broken stones, louder."

[[variant]]
slot = "c.three"
text = "[if cold]The wind keens.[end] Stones."
example = true
"#;

    #[test]
    fn runs_skip_template_parts() {
        assert_eq!(
            runs("A {b} c[if x] d[end]."),
            vec![
                vec!["a".to_string()],
                vec!["c".to_string()],
                vec!["d".to_string()]
            ]
        );
    }

    #[test]
    fn glossary_and_echoes() {
        let p = pack(P);
        let g = glossary(&p, 2);
        let wind = g
            .iter()
            .find(|x| x.text == "wind keens")
            .expect("phrase found");
        assert_eq!(wind.count, 2, "examples don't count");
        let e = echoes(&p);
        assert_eq!(e.len(), 2);
        assert!(e.iter().all(|i| i.kind == "repeated-phrase"));
    }

    #[test]
    fn family_stats() {
        let s = stats(&pack(P));
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].variants, 1);
        assert!(s[0].words_per_sentence > 5.0);
    }

    #[test]
    fn diffs() {
        let a = pack(P);
        let mut b = a.clone();
        b.files[0].variants[0].text.push('!');
        b.files[0].variants.push(Variant {
            slot: "a.one".into(),
            text: "new".into(),
            when: None,
            weight: 1,
            example: false,
        });
        let d = diff(&a, &b);
        assert_eq!(d.changed.len(), 1);
        assert_eq!(d.added.len(), 1);
        assert!(!d.breaks_saves);
        b.files[0].storylets.push(
            PackFile::parse("s.toml", "[[storylet]]\nid = \"x\"\nabout = \"y\"\n")
                .unwrap()
                .storylets
                .remove(0),
        );
        let d = diff(&a, &b);
        assert_eq!(d.storylets, ["x"]);
        assert!(d.breaks_saves);
    }
}
