//! A forgiving command parser: a verb (from `data/verbs.toml`) and a noun
//! phrase, resolved against what the player can perceive.
//!
//! Nouns match the words of each thing's displayed name, which comes from
//! Jb's content, so whatever he calls a thing is what the player can type.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

/// Whether a verb takes an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectRule {
    None,
    Optional,
    Required,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Verb {
    pub id: String,
    pub words: Vec<String>,
    pub object: ObjectRule,
}

#[derive(Deserialize)]
struct VerbFile {
    verb: Vec<Verb>,
}

/// Every verb, from data.
pub fn verbs() -> &'static [Verb] {
    static VERBS: OnceLock<Vec<Verb>> = OnceLock::new();
    VERBS.get_or_init(|| {
        toml::from_str::<VerbFile>(include_str!("../data/verbs.toml"))
            .expect("data/verbs.toml is valid")
            .verb
    })
}

/// A parsed command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub verb: String,
    /// The rest of the input, lowercased, split into words.
    pub words: Vec<String>,
}

/// Why a command could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    UnknownVerb(String),
}

pub(crate) fn tokens(input: &str) -> Vec<String> {
    input
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '.' | '!' | ';'))
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Finds the verb (longest matching form first) and the words after it.
pub fn parse(input: &str) -> Result<Command, ParseError> {
    let toks = tokens(input);
    if toks.is_empty() {
        return Err(ParseError::Empty);
    }
    let mut best: Option<(usize, &Verb)> = None;
    for v in verbs() {
        for form in &v.words {
            let f: Vec<&str> = form.split(' ').collect();
            if toks.len() >= f.len()
                && toks[..f.len()].iter().zip(&f).all(|(a, b)| a == b)
                && best.is_none_or(|(n, _)| f.len() > n)
            {
                best = Some((f.len(), v));
            }
        }
    }
    match best {
        Some((n, v)) => Ok(Command {
            verb: v.id.clone(),
            words: toks[n..].to_vec(),
        }),
        None => Err(ParseError::UnknownVerb(toks[0].clone())),
    }
}

/// Little words that never pick out a thing.
const FILLER: &[&str] = &[
    "the", "a", "an", "to", "into", "through", "at", "on", "in", "of", "with", "among",
];

const ORDINALS: &[&str] = &[
    "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth",
];

/// Something the player can refer to, with the words that name it.
#[derive(Debug, Clone)]
pub struct Candidate<T> {
    pub target: T,
    pub words: BTreeSet<String>,
    /// Words that name it only loosely ("west" for something to the
    /// north-west): when several things match, the one named without loose
    /// words wins.
    pub loose: BTreeSet<String>,
    /// The words of its displayed name: when names nest ("the hilltop
    /// mountain", "the twin hilltop mountain"), saying all of one name
    /// picks that one.
    pub name: BTreeSet<String>,
}

impl<T> Candidate<T> {
    /// Names a target by the words of its displayed name plus extra words
    /// (such as its kind id).
    pub fn new(target: T, name: &str, extra: &[&str]) -> Self {
        let name: BTreeSet<String> = tokens(name)
            .into_iter()
            .filter(|w| !FILLER.contains(&w.as_str()))
            .collect();
        let mut words = name.clone();
        for e in extra {
            words.extend(tokens(e));
        }
        Candidate {
            target,
            words,
            loose: BTreeSet::new(),
            name,
        }
    }

    /// Adds words that name it only loosely.
    pub fn loosely(mut self, extra: &[&str]) -> Self {
        for e in extra {
            for t in tokens(e) {
                if !self.words.contains(&t) {
                    self.words.insert(t.clone());
                    self.loose.insert(t);
                }
            }
        }
        self
    }
}

/// The result of resolving a noun phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution<T> {
    One(T),
    None,
    /// Several things match; the player must choose.
    Many(Vec<T>),
}

/// Resolves a noun phrase against candidates. Supports ordinals ("the
/// second door", "door 2") and "it".
pub fn resolve<T: Clone + PartialEq>(
    words: &[String],
    cands: &[Candidate<T>],
    it: Option<&T>,
) -> Resolution<T> {
    let mut ordinal: Option<usize> = None;
    let mut wanted: Vec<&str> = Vec::new();
    for w in words {
        if FILLER.contains(&w.as_str()) {
            continue;
        }
        if let Some(i) = ORDINALS.iter().position(|o| o == w) {
            ordinal = Some(i);
        } else if let Ok(n) = w.parse::<usize>() {
            ordinal = n.checked_sub(1);
        } else {
            wanted.push(w);
        }
    }
    if wanted == ["it"] || wanted == ["them"] {
        return match it {
            Some(t) if cands.iter().any(|c| &c.target == t) => Resolution::One(t.clone()),
            _ => Resolution::None,
        };
    }
    // A bare ordinal ("first", "the second") picks among all of them, as
    // when answering "which do you mean?".
    let bare = wanted.is_empty() && ordinal.is_some();
    let all: Vec<&Candidate<T>> = cands
        .iter()
        .filter(|c| bare || (!wanted.is_empty() && wanted.iter().all(|w| c.words.contains(*w))))
        .collect();
    // Prefer the things named most exactly.
    let score = |c: &Candidate<T>| {
        let loose = wanted.iter().filter(|w| c.loose.contains(**w)).count();
        let unsaid = c
            .name
            .iter()
            .filter(|n| !wanted.contains(&n.as_str()))
            .count();
        (loose, unsaid)
    };
    let best = all.iter().map(|c| score(c)).min().unwrap_or((0, 0));
    let matches: Vec<T> = all
        .iter()
        .filter(|c| ordinal.is_some() || score(c) == best)
        .map(|c| c.target.clone())
        .collect();
    match (matches.len(), ordinal) {
        (0, _) => Resolution::None,
        (_, Some(i)) => matches
            .get(i)
            .cloned()
            .map_or(Resolution::None, Resolution::One),
        (1, None) => Resolution::One(matches[0].clone()),
        _ => Resolution::Many(matches),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(s: &str) -> Vec<String> {
        tokens(s)
    }

    #[test]
    fn verbs_and_synonyms() {
        assert_eq!(
            parse("x altar").unwrap(),
            Command {
                verb: "examine".into(),
                words: words("altar")
            }
        );
        assert_eq!(parse("look at the altar").unwrap().verb, "examine");
        assert_eq!(parse("look").unwrap().verb, "look");
        assert_eq!(parse("pick up the jar").unwrap().words, words("the jar"));
        assert_eq!(parse("N").unwrap().verb, "north");
        assert_eq!(parse("i").unwrap().verb, "inventory");
        assert_eq!(parse("  "), Err(ParseError::Empty));
        assert_eq!(
            parse("dance wildly"),
            Err(ParseError::UnknownVerb("dance".into()))
        );
        for v in verbs() {
            for w in &v.words {
                assert_eq!(parse(w).unwrap().verb, v.id, "{w}");
            }
        }
    }

    #[test]
    fn nouns_ordinals_pronouns_and_ambiguity() {
        let cands = vec![
            Candidate::new(1, "an iron door", &["door"]),
            Candidate::new(2, "a cedar door", &["door"]),
            Candidate::new(3, "a stone altar", &["altar"]),
        ];
        assert_eq!(resolve(&words("altar"), &cands, None), Resolution::One(3));
        assert_eq!(
            resolve(&words("the cedar door"), &cands, None),
            Resolution::One(2)
        );
        assert_eq!(
            resolve(&words("door"), &cands, None),
            Resolution::Many(vec![1, 2])
        );
        assert_eq!(
            resolve(&words("second door"), &cands, None),
            Resolution::One(2)
        );
        assert_eq!(resolve(&words("door 1"), &cands, None), Resolution::One(1));
        assert_eq!(resolve(&words("it"), &cands, Some(&3)), Resolution::One(3));
        assert_eq!(resolve(&words("it"), &cands, None), Resolution::None);
        assert_eq!(resolve(&words("throne"), &cands, None), Resolution::None);
        assert_eq!(
            resolve(&words("fifth door"), &cands, None),
            Resolution::None
        );
        // Answering "which do you mean?" with an ordinal alone.
        assert_eq!(resolve(&words("first"), &cands, None), Resolution::One(1));
        assert_eq!(
            resolve(&words("the third"), &cands, None),
            Resolution::One(3)
        );
        // Saying all of one name picks it over a longer one.
        let nested = vec![
            Candidate::new(1, "the hilltop mountain", &[]),
            Candidate::new(2, "the twin hilltop mountain", &[]),
        ];
        assert_eq!(
            resolve(&words("hilltop mountain"), &nested, None),
            Resolution::One(1)
        );
        assert_eq!(
            resolve(&words("twin mountain"), &nested, None),
            Resolution::One(2)
        );
        // Loose words lose to exact ones.
        let ways = vec![
            Candidate::new(1, "the town", &["west"]),
            Candidate::new(2, "the town", &["northwest"]).loosely(&["north", "west"]),
        ];
        assert_eq!(
            resolve(&words("town to the west"), &ways, None),
            Resolution::One(1)
        );
        assert_eq!(
            resolve(&words("north town"), &ways, None),
            Resolution::One(2)
        );
    }
}
