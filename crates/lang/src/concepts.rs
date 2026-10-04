//! The starter list of concepts every language names, loaded from data.
//!
//! Concepts live in `data/concepts.toml` so the list can grow without code
//! changes. The file is embedded at compile time; no filesystem access.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const CONCEPTS_TOML: &str = include_str!("../data/concepts.toml");
const LEXICON_TXT: &str = include_str!("../data/lexicon.txt");

/// Broad grouping, used to lay out the grammar sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Domain {
    People,
    Nature,
    Places,
    Objects,
    Actions,
    Qualities,
    Deixis,
    Numbers,
    /// Function words: particles with no lexical meaning of their own.
    Grammar,
}

/// Part of speech: decides which inflections and positions a word takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pos {
    Noun,
    Verb,
    Adj,
    Num,
    Det,
    Adv,
    /// An uninflected function word, glossed in capitals.
    Particle,
}

/// One meaning the language has a root for.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Concept {
    pub id: String,
    pub domain: Domain,
    pub pos: Pos,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub objects: Vec<String>,
    #[serde(default)]
    pub applies: Vec<String>,
    #[serde(default)]
    pub value: Option<u16>,
    #[serde(default)]
    pub needs: Vec<String>,
    #[serde(default)]
    pub en_plural: Option<String>,
    #[serde(default)]
    pub en_3sg: Option<String>,
    #[serde(default)]
    pub en_past: Option<String>,
    /// The semantic field (D07): "core" for the starter list, else "body",
    /// "kin", "sea", "fauna"…
    #[serde(default = "core_field")]
    pub field: String,
    /// Semantic relations (D07): a kind of, a part of, the opposite of.
    #[serde(default)]
    pub kind_of: Option<String>,
    #[serde(default)]
    pub part_of: Option<String>,
    #[serde(default)]
    pub opposite: Option<String>,
}

fn core_field() -> String {
    "core".to_string()
}

/// Fields every language names (D07); the rest are the culture's choice,
/// and species are named where they live.
pub const CORE_FIELDS: &[&str] = &[
    "core", "body", "kin", "land", "nature", "sky", "weather", "time", "act", "quality",
];

/// Fields a culture may care about.
pub const CULTURE_FIELDS: &[&str] = &[
    "crafts", "trade", "law", "religion", "sea", "herding", "farming", "war", "learning",
];

impl Concept {
    /// Whether it is built from other concepts (D07): a derived word
    /// ("build+agt") or a compound ("river~stone"), with no root of its own.
    pub fn is_built(&self) -> bool {
        self.id.contains('+') || self.id.contains('~')
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Whether this verb can take `noun` as its object.
    pub fn accepts_object(&self, noun: &Concept) -> bool {
        self.objects.iter().any(|t| noun.has_tag(t))
    }

    /// Whether this adjective sensibly modifies `noun`.
    pub fn applies_to(&self, noun: &Concept) -> bool {
        self.applies.iter().any(|t| noun.has_tag(t))
    }
}

#[derive(Deserialize)]
struct ConceptFile {
    concept: Vec<Concept>,
}

/// The full concept list, parsed once: the starter list, then the
/// cultural lexicon (D07).
pub fn all() -> &'static [Concept] {
    static CONCEPTS: OnceLock<Vec<Concept>> = OnceLock::new();
    CONCEPTS.get_or_init(|| {
        let mut v = toml::from_str::<ConceptFile>(CONCEPTS_TOML)
            .expect("data/concepts.toml is valid")
            .concept;
        v.extend(LEXICON_TXT.lines().filter_map(lexicon_line));
        let built = built_concepts(&v);
        v.extend(built);
        v
    })
}

fn index() -> &'static std::collections::BTreeMap<&'static str, usize> {
    static INDEX: OnceLock<std::collections::BTreeMap<&'static str, usize>> = OnceLock::new();
    INDEX.get_or_init(|| {
        all()
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect()
    })
}

/// Heads of compounds and the tags their modifiers carry (D07).
// DESIGN-Q: compounds are formed on a few heads with modifiers of fitting
// kinds ("river-stone", "city-road", "potter-house", "bird-song"); a
// language has every such compound whose parts it names.
const COMPOUNDS: &[(&str, &[&str])] = &[
    ("stone", &["landform", "water", "settlement"]),
    ("road", &["settlement", "landform"]),
    ("gate", &["settlement", "landform", "heaven"]),
    ("house", &["trade", "rank", "food"]),
    ("field", &["food"]),
    ("water", &["landform", "heaven"]),
    ("song", &["bird", "heaven", "event"]),
    ("day", &["event", "heaven"]),
];

/// Derived words and compounds, built from the base concepts (D07).
fn built_concepts(base: &[Concept]) -> Vec<Concept> {
    let mut out = Vec::new();
    let make = |id: String, pos: Pos, domain: Domain, tags: Vec<&str>, field: &str| {
        let tags: Vec<String> = tags.into_iter().map(str::to_string).collect();
        Concept {
            id,
            domain,
            pos,
            tags: if pos == Pos::Noun {
                tags.clone()
            } else {
                Vec::new()
            },
            objects: Vec::new(),
            applies: if pos == Pos::Adj { tags } else { Vec::new() },
            value: None,
            needs: Vec::new(),
            en_plural: None,
            en_3sg: None,
            en_past: None,
            field: field.to_string(),
            kind_of: None,
            part_of: None,
            opposite: None,
        }
    };
    let words =
        |c: &Concept| c.domain != Domain::Grammar && c.domain != Domain::Numbers && !c.is_built();
    for c in base.iter().filter(|c| words(c)) {
        let f = c.field.as_str();
        let species = f == "fauna" || f == "flora";
        match c.pos {
            Pos::Verb => {
                out.push(make(
                    format!("{}+agt", c.id),
                    Pos::Noun,
                    Domain::People,
                    vec!["person", "doer"],
                    f,
                ));
                out.push(make(
                    format!("{}+place", c.id),
                    Pos::Noun,
                    Domain::Places,
                    vec!["building", "workplace"],
                    f,
                ));
                out.push(make(
                    format!("{}+instr", c.id),
                    Pos::Noun,
                    Domain::Objects,
                    vec!["thing", "tool"],
                    f,
                ));
            }
            Pos::Noun => {
                if !species {
                    out.push(make(
                        format!("{}+abst", c.id),
                        Pos::Noun,
                        Domain::Qualities,
                        vec!["state"],
                        f,
                    ));
                }
                out.push(make(
                    format!("{}+dim", c.id),
                    Pos::Noun,
                    c.domain,
                    vec!["small"],
                    f,
                ));
                out.push(make(
                    format!("{}+adjz", c.id),
                    Pos::Adj,
                    Domain::Qualities,
                    vec![
                        "thing", "person", "landform", "building", "animal", "plant", "material",
                        "ground",
                    ],
                    f,
                ));
            }
            Pos::Adj => {
                out.push(make(
                    format!("{}+abst", c.id),
                    Pos::Noun,
                    Domain::Qualities,
                    vec!["state"],
                    f,
                ));
            }
            _ => {}
        }
    }
    for (head, tags) in COMPOUNDS {
        let Some(h) = base.iter().find(|c| c.id == *head) else {
            continue;
        };
        for m in base
            .iter()
            .filter(|c| words(c) && c.pos == Pos::Noun && c.id != *head)
        {
            if tags.iter().any(|t| m.has_tag(t)) {
                let field = if crate::concepts::CORE_FIELDS.contains(&m.field.as_str()) {
                    h.field.as_str()
                } else {
                    m.field.as_str()
                };
                out.push(make(
                    format!("{}~{}", m.id, h.id),
                    Pos::Noun,
                    h.domain,
                    vec!["compound"],
                    field,
                ));
            }
        }
    }
    out
}

/// A plain English word for a concept, for spoiler output (D07): derived
/// words and compounds spelled out from their parts.
// PLACEHOLDER-PROSE: mechanical English for debugging only.
pub fn english(id: &str) -> String {
    if let Some((base, d)) = id.split_once('+') {
        let b = english(base);
        return match d {
            "agt" => format!("{b}er"),
            "place" => format!("{b}-place"),
            "instr" => format!("{b}ing-tool"),
            "abst" => format!("{b}ness"),
            "dim" => format!("little {b}"),
            "adjz" => format!("{b}-like"),
            _ => b,
        };
    }
    if let Some((m, h)) = id.split_once('~') {
        return format!("{}-{}", english(m), english(h));
    }
    id.replace('_', " ")
}

/// One line of `data/lexicon.txt`.
fn lexicon_line(line: &str) -> Option<Concept> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let cols: Vec<&str> = line.split('|').map(str::trim).collect();
    assert!(cols.len() >= 4, "lexicon line {line:?}");
    let pos = match cols[1] {
        "noun" => Pos::Noun,
        "verb" => Pos::Verb,
        "adj" => Pos::Adj,
        "adv" => Pos::Adv,
        "det" => Pos::Det,
        "particle" => Pos::Particle,
        p => panic!("lexicon: unknown part of speech {p:?}"),
    };
    let field = cols[2].to_string();
    let tags: Vec<String> = cols[3]
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect();
    let domain = match (pos, field.as_str()) {
        (Pos::Verb, _) => Domain::Actions,
        (Pos::Adj, _) => Domain::Qualities,
        (Pos::Adv | Pos::Det, _) => Domain::Deixis,
        (Pos::Particle, _) => Domain::Grammar,
        (_, "kin" | "body" | "law" | "religion") => Domain::People,
        (_, "land") => Domain::Places,
        (_, "crafts" | "trade" | "war" | "learning") => Domain::Objects,
        _ => Domain::Nature,
    };
    let mut c = Concept {
        id: cols[0].to_string(),
        domain,
        pos,
        tags: Vec::new(),
        objects: Vec::new(),
        applies: Vec::new(),
        value: None,
        needs: Vec::new(),
        en_plural: None,
        en_3sg: None,
        en_past: None,
        field,
        kind_of: None,
        part_of: None,
        opposite: None,
    };
    match pos {
        Pos::Verb => c.objects = tags,
        Pos::Adj => c.applies = tags,
        _ => c.tags = tags,
    }
    for kv in cols.get(4).copied().unwrap_or("").split_whitespace() {
        let (k, v) = kv
            .split_once('=')
            .unwrap_or_else(|| panic!("lexicon: {kv:?}"));
        let v = Some(v.to_string());
        match k {
            "kind" => c.kind_of = v,
            "part" => c.part_of = v,
            "opp" => c.opposite = v,
            "pl" => c.en_plural = v,
            "past" => c.en_past = v,
            "s" => c.en_3sg = v,
            _ => panic!("lexicon: unknown extra {k:?}"),
        }
    }
    Some(c)
}

/// Concepts that are a kind of `id` (D07).
pub fn kinds_of(id: &str) -> Vec<&'static Concept> {
    all()
        .iter()
        .filter(|c| c.kind_of.as_deref() == Some(id))
        .collect()
}

/// Concepts that are a part of `id` (D07).
pub fn parts_of(id: &str) -> Vec<&'static Concept> {
    all()
        .iter()
        .filter(|c| c.part_of.as_deref() == Some(id))
        .collect()
}

/// A concept's opposite, if it has one (D07).
pub fn opposite(id: &str) -> Option<&'static Concept> {
    let c = get(id);
    c.opposite
        .as_deref()
        .and_then(|o| all().iter().find(|x| x.id == o))
}

/// Looks up a concept by id. Panics on unknown ids: they are programmer errors.
pub fn get(id: &str) -> &'static Concept {
    index()
        .get(id)
        .map(|&i| &all()[i])
        .unwrap_or_else(|| panic!("unknown concept {id:?}"))
}

/// All concepts with the given part of speech.
pub fn with_pos(pos: Pos) -> impl Iterator<Item = &'static Concept> {
    all().iter().filter(move |c| c.pos == pos)
}

/// All nouns carrying `tag`.
pub fn nouns_tagged(tag: &str) -> Vec<&'static Concept> {
    with_pos(Pos::Noun).filter(|c| c.has_tag(tag)).collect()
}

/// The numeral concept with value `n` (0–11 and the powers).
pub fn numeral(n: u16) -> &'static Concept {
    with_pos(Pos::Num)
        .find(|c| c.value == Some(n))
        .unwrap_or_else(|| panic!("no numeral {n}"))
}

/// The gloss shown for a concept: particles in capitals, Leipzig style;
/// derived words as `build-AGT`, compounds as `river+stone` (D07).
pub fn gloss(id: &str) -> String {
    if let Some((base, d)) = id.split_once('+') {
        return format!("{base}-{}", d.to_uppercase());
    }
    if let Some((m, h)) = id.split_once('~') {
        return format!("{m}+{h}");
    }
    if get(id).pos == Pos::Particle {
        id.to_uppercase()
    } else {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn list_loads_with_expected_size() {
        let n = all().len();
        assert!((100..=1500).contains(&n), "{n} concepts");
    }

    #[test]
    fn ids_are_unique_and_clean() {
        let ids: BTreeSet<&str> = all().iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids.len(), all().len());
        for id in ids {
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || matches!(c, '.' | '_' | '+' | '~')),
                "{id}"
            );
        }
    }

    #[test]
    fn relations_point_at_known_concepts() {
        let ids: BTreeSet<&str> = all().iter().map(|c| c.id.as_str()).collect();
        for c in all() {
            for r in [&c.kind_of, &c.part_of, &c.opposite].into_iter().flatten() {
                assert!(ids.contains(r.as_str()), "{} -> {r}", c.id);
            }
            if let Some(o) = &c.opposite {
                assert_eq!(
                    get(o).opposite.as_deref(),
                    Some(c.id.as_str()),
                    "{} <-> {o}",
                    c.id
                );
            }
        }
        assert!(!kinds_of("tree").is_empty() && !parts_of("house").is_empty());
    }

    #[test]
    fn numbers_one_to_ten() {
        for n in 1..=10 {
            assert_eq!(numeral(n).domain, Domain::Numbers);
        }
    }

    #[test]
    fn every_verb_object_tag_is_used_by_some_noun() {
        for v in with_pos(Pos::Verb) {
            for t in &v.objects {
                assert!(!nouns_tagged(t).is_empty(), "{}: {t}", v.id);
            }
        }
        for a in with_pos(Pos::Adj) {
            assert!(with_pos(Pos::Noun).any(|n| a.applies_to(n)), "{}", a.id);
        }
    }
}
