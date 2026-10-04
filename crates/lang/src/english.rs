//! Placeholder English translations of meanings, for spoiler output only.
//!
//! PLACEHOLDER-PROSE: this is a mechanical gloss-to-English renderer for
//! testing, not the game's voice. It is never shown without `--spoil`.

use crate::concepts::{self, Concept};
use crate::meaning::{
    Aspect, Clause, Compare, Conj, Head, Link, Mood, NounPhrase, Number, Polarity, Role, Sentence,
    Tense,
};

/// Renders a meaning as an English sentence. `name` gives display names.
pub fn translate(s: &Sentence, name: &dyn Fn(usize) -> String) -> String {
    match s {
        Sentence::Text(parts) => parts
            .iter()
            .map(|p| translate(p, name))
            .collect::<Vec<_>>()
            .join(" "),
        _ => {
            let text = match s {
                Sentence::Clause(c) => clause(c, name),
                Sentence::Joined(conj, parts) => {
                    let word = match conj {
                        Conj::And => ", and ",
                        Conj::But => ", but ",
                        Conj::Or => ", or ",
                        Conj::Then => ", then ",
                    };
                    parts
                        .iter()
                        .map(|c| clause(c, name).trim_end_matches(':').to_string())
                        .collect::<Vec<_>>()
                        .join(word)
                }
                Sentence::List(items) => items
                    .iter()
                    .map(|np| noun_phrase(np, name))
                    .collect::<Vec<_>>()
                    .join(", "),
                Sentence::Text(_) => unreachable!(),
            };
            let text = upper_first(text.trim_end_matches(','));
            if text.ends_with(':') {
                text
            } else {
                format!("{text}.")
            }
        }
    }
}

/// Number in English: words up to ten, digits beyond.
fn number(n: u16) -> String {
    if n <= 10 {
        concepts::numeral(n).id.clone()
    } else {
        n.to_string()
    }
}

fn clause(c: &Clause, name: &dyn Fn(usize) -> String) -> String {
    let core = clause_without(c, None, name);
    if c.subordinate.is_empty() {
        return core;
    }
    let subs: Vec<String> = c
        .subordinate
        .iter()
        .map(|s| {
            let link = match s.link {
                Link::When => "when",
                Link::Because => "because",
                Link::If => "if",
                Link::Until => "until",
                Link::Before => "before",
                Link::After => "after",
                Link::SoThat => "so that",
                Link::Although => "although",
            };
            format!("{link} {}", clause(&s.clause, name))
        })
        .collect();
    format!("{core} {}", subs.join(" "))
}

/// A clause, with the argument in `gap` left out (relative clauses).
fn clause_without(c: &Clause, gap: Option<Role>, name: &dyn Fn(usize) -> String) -> String {
    let verb = concepts::get(&c.predicate);
    let mut parts = Vec::new();
    if c.mood == Mood::Potent {
        parts.push("[potent] let".to_string());
    }
    let subject = c.arg(Role::Subject).filter(|_| gap != Some(Role::Subject));
    let number = c.arg(Role::Subject).map_or(Number::Singular, |s| s.number);
    let aspect = match c.aspect {
        Aspect::Simple => "",
        Aspect::Perfective => "[done] ",
        Aspect::Imperfective => "[ongoing] ",
        Aspect::Habitual => "[usually] ",
    };
    match c.mood {
        Mood::Imperative => {
            if c.polarity == Polarity::Negative {
                parts.push("do not".to_string());
            }
            parts.push(format!("{aspect}{}", verb.id));
        }
        Mood::Potent => {
            if let Some(s) = subject {
                parts.push(noun_phrase(s, name));
            }
            if c.polarity == Polarity::Negative {
                parts.push("not".to_string());
            }
            parts.push(format!("{aspect}{}", verb.id));
        }
        Mood::Optative | Mood::Conditional => {
            parts.push(
                if c.mood == Mood::Optative {
                    "may"
                } else {
                    "would"
                }
                .to_string(),
            );
            if let Some(s) = subject {
                parts.push(noun_phrase(s, name));
            }
            if c.polarity == Polarity::Negative {
                parts.push("not".to_string());
            }
            // The past here is "have" and the past form (a spoiler gloss:
            // close enough where the participle differs, as "went").
            let verb = if c.tense == Tense::Past {
                format!(
                    "have {aspect}{}",
                    finite(verb, c.tense, Polarity::Positive, number)
                )
            } else {
                format!("{aspect}{}", verb.id)
            };
            parts.push(verb);
        }
        Mood::Declarative | Mood::Interrogative => {
            if c.mood == Mood::Interrogative {
                parts.push("[question]".to_string());
            }
            if let Some(s) = subject {
                parts.push(noun_phrase(s, name));
            }
            parts.push(format!(
                "{aspect}{}",
                finite(verb, c.tense, c.polarity, number)
            ));
        }
    }
    if let Some(o) = c.arg(Role::Object).filter(|_| gap != Some(Role::Object)) {
        parts.push(noun_phrase(o, name));
    }
    if let Some(r) = c
        .arg(Role::Recipient)
        .filter(|_| gap != Some(Role::Recipient))
    {
        let to = if c.predicate == "say" { "to" } else { "for" };
        parts.push(format!("{to} {}", noun_phrase(r, name)));
    }
    if let Some(t) = c.arg(Role::Time) {
        parts.push(format!("in {}", noun_phrase(t, name)));
    }
    parts.extend(c.adverbs.iter().cloned());
    if let Some(comp) = &c.complement {
        let inner = translate(&comp.content, name);
        if comp.direct {
            parts.push(format!("\"{}\"", inner.trim_end_matches('.')));
        } else {
            parts.push(format!(
                "that {}",
                inner.trim_end_matches('.').to_lowercase()
            ));
        }
    }
    let s = parts.join(" ");
    if c.predicate == "say" && c.complement.is_none() && gap.is_none() {
        format!("{s}:")
    } else {
        s
    }
}

fn finite(verb: &Concept, tense: Tense, polarity: Polarity, subject: Number) -> String {
    let singular = subject == Number::Singular;
    match (tense, polarity) {
        (Tense::Past, Polarity::Positive) => verb
            .en_past
            .clone()
            .unwrap_or_else(|| format!("{}ed", verb.id.trim_end_matches('e'))),
        (Tense::Past, Polarity::Negative) => format!("did not {}", verb.id),
        (Tense::NonPast, Polarity::Positive) if singular => verb
            .en_3sg
            .clone()
            .unwrap_or_else(|| format!("{}s", verb.id)),
        (Tense::NonPast, Polarity::Positive) => verb.id.clone(),
        (Tense::NonPast, Polarity::Negative) if singular => format!("does not {}", verb.id),
        (Tense::NonPast, Polarity::Negative) => format!("do not {}", verb.id),
    }
}

fn noun_phrase(np: &NounPhrase, name: &dyn Fn(usize) -> String) -> String {
    if np.head == Head::Concept("total".to_string()) {
        return format!("{} in all", number(np.quantity.unwrap_or(0)));
    }
    let mut words = Vec::new();
    let is_name = matches!(np.head, Head::Name(_));
    match (&np.determiner, np.quantity) {
        (Some(d), _) if d == "this" => words.push(
            if np.number == Number::Plural {
                "these"
            } else {
                "this"
            }
            .to_string(),
        ),
        (Some(d), _) => words.push(d.clone()),
        (None, None) if !is_name => words.push("the".to_string()),
        _ => {}
    }
    if let Some(n) = np.quantity {
        words.push(number(n));
    }
    if let Some(n) = np.ordinal {
        words.push(format!("{n}th"));
    }
    if let Some(d) = &np.degree {
        words.push(match d.compare {
            Compare::More => format!("more {}", d.adjective),
            Compare::Most => format!("most {}", d.adjective),
            Compare::As => format!("as {}", d.adjective),
        });
    }
    words.extend(np.adjectives.iter().cloned());
    words.push(match &np.head {
        Head::Name(p) => name(*p),
        Head::Concept(id) => {
            let c = concepts::get(id);
            if np.number == Number::Plural {
                c.en_plural.clone().unwrap_or_else(|| format!("{id}s"))
            } else {
                id.clone()
            }
        }
    });
    if let Some(p) = &np.possessor {
        words.push(format!("of {}", noun_phrase(p, name)));
    }
    if let Some(st) = np
        .degree
        .as_ref()
        .and_then(|d| d.standard.as_ref().map(|s| (d.compare, s)))
    {
        let w = if st.0 == Compare::As { "as" } else { "than" };
        words.push(format!("{w} {}", noun_phrase(st.1, name)));
    }
    if let Some(rel) = &np.relative {
        let who = if matches!(np.head, Head::Name(_)) {
            "who"
        } else {
            "that"
        };
        words.push(format!(
            "{who} {}",
            clause_without(&rel.clause, Some(rel.gap), name)
        ));
    }
    let s = words.join(" ");
    if np.apposition.is_empty() {
        return s;
    }
    let appositions: Vec<String> = np
        .apposition
        .iter()
        .map(|a| {
            let a = noun_phrase(a, name);
            // "the child of X" reads better as "child of X" in apposition.
            a.strip_prefix("the ").map(str::to_string).unwrap_or(a)
        })
        .collect();
    format!("{s}, {},", appositions.join(", "))
}

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(i: usize) -> String {
        ["Akan", "Mira"][i].to_string()
    }

    #[test]
    fn tomb_formula() {
        let mut subject = NounPhrase::name(0);
        subject
            .apposition
            .push(NounPhrase::concept("child").with_possessor(NounPhrase::name(1)));
        let s = Sentence::Clause(Clause {
            predicate: "lie".into(),
            mood: Mood::Declarative,
            tense: Tense::NonPast,
            polarity: Polarity::Positive,
            args: vec![crate::meaning::Argument {
                role: Role::Subject,
                np: subject,
            }],
            adverbs: vec!["here".into()],
            aspect: Default::default(),
            subordinate: Vec::new(),
            complement: None,
        });
        assert_eq!(translate(&s, &n), "Akan, child of Mira, lies here.");
    }

    #[test]
    fn warning() {
        let s = Sentence::Clause(Clause {
            predicate: "open".into(),
            mood: Mood::Imperative,
            tense: Tense::NonPast,
            polarity: Polarity::Negative,
            args: vec![crate::meaning::Argument {
                role: Role::Object,
                np: NounPhrase::concept("gate"),
            }],
            adverbs: vec![],
            aspect: Default::default(),
            subordinate: Vec::new(),
            complement: None,
        });
        assert_eq!(translate(&s, &n), "Do not open the gate.");
    }

    #[test]
    fn ledger_list() {
        let s = Sentence::List(vec![
            NounPhrase::concept("sheep").counted(3),
            NounPhrase::concept("loaf").counted(1),
        ]);
        assert_eq!(translate(&s, &n), "Three sheep, one loaf.");
    }
}
