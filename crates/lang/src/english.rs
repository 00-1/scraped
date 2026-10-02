//! Placeholder English translations of meanings, for spoiler output only.
//!
//! PLACEHOLDER-PROSE: this is a mechanical gloss-to-English renderer for
//! testing, not the game's voice. It is never shown without `--spoil`.

use crate::concepts::{self, Concept};
use crate::meaning::{Clause, Head, Mood, NounPhrase, Number, Polarity, Role, Sentence, Tense};

/// Renders a meaning as an English sentence. `name` gives display names.
pub fn translate(s: &Sentence, name: &dyn Fn(usize) -> String) -> String {
    let text = match s {
        Sentence::Clause(c) => clause(c, name),
        Sentence::List(items) => items
            .iter()
            .map(|np| noun_phrase(np, name))
            .collect::<Vec<_>>()
            .join(", "),
    };
    format!("{}.", upper_first(text.trim_end_matches(',')))
}

fn clause(c: &Clause, name: &dyn Fn(usize) -> String) -> String {
    let verb = concepts::get(&c.predicate);
    let mut parts = Vec::new();
    match c.mood {
        Mood::Imperative => {
            if c.polarity == Polarity::Negative {
                parts.push("do not".to_string());
            }
            parts.push(verb.id.clone());
        }
        Mood::Declarative => {
            let subject = c.arg(Role::Subject).expect("statements have subjects");
            parts.push(noun_phrase(subject, name));
            parts.push(finite(verb, c.tense, c.polarity, subject.number));
        }
    }
    if let Some(o) = c.arg(Role::Object) {
        parts.push(noun_phrase(o, name));
    }
    if let Some(r) = c.arg(Role::Recipient) {
        parts.push(format!("for {}", noun_phrase(r, name)));
    }
    parts.extend(c.adverbs.iter().cloned());
    parts.join(" ")
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
        words.push(concepts::numeral(n).id.clone());
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
