//! Structured meaning: what a sentence says, before it has any form.
//!
//! Every generated text starts here and is rendered through the grammar, so
//! the engine can always report the true meaning of what it produced. Later,
//! parsing player text means going the other way: surface → `Sentence`.

use serde::Serialize;

pub use crate::morphology::{Number, Polarity, Tense};

/// What a noun phrase is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Head {
    /// A concept from the lexicon, by id.
    Concept(String),
    /// A named person from the corpus cast, by index.
    Name(usize),
}

/// A noun phrase with its modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NounPhrase {
    pub head: Head,
    pub number: Number,
    /// A counted quantity (0–999), rendered as numeral words.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u16>,
    /// A determiner concept, e.g. "this".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub determiner: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub adjectives: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub possessor: Option<Box<NounPhrase>>,
    /// Phrases renaming the head: titles, "child of X".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub apposition: Vec<NounPhrase>,
}

impl NounPhrase {
    /// A bare singular concept noun.
    pub fn concept(id: &str) -> Self {
        Self::bare(Head::Concept(id.to_string()))
    }

    /// A bare named person.
    pub fn name(person: usize) -> Self {
        Self::bare(Head::Name(person))
    }

    fn bare(head: Head) -> Self {
        NounPhrase {
            head,
            number: Number::Singular,
            quantity: None,
            determiner: None,
            adjectives: Vec::new(),
            possessor: None,
            apposition: Vec::new(),
        }
    }

    /// Sets a counted quantity; plural follows from it.
    pub fn counted(mut self, n: u16) -> Self {
        self.quantity = Some(n);
        self.number = if n > 1 {
            Number::Plural
        } else {
            Number::Singular
        };
        self
    }

    pub fn with_possessor(mut self, p: NounPhrase) -> Self {
        self.possessor = Some(Box::new(p));
        self
    }
}

/// What part an argument plays in its clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Subject,
    Object,
    /// The one something is done for.
    Recipient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Argument {
    pub role: Role,
    pub np: NounPhrase,
}

/// Statement or command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Declarative,
    /// Addressed to the reader; has no subject.
    Imperative,
    /// A claim in the potent register: framed by fixed formulae and marked
    /// with a particle. What it does is decided in M08.
    Potent,
}

/// A clause: one predicate and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Clause {
    pub predicate: String,
    pub mood: Mood,
    pub tense: Tense,
    pub polarity: Polarity,
    pub args: Vec<Argument>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub adverbs: Vec<String>,
}

impl Clause {
    pub fn arg(&self, role: Role) -> Option<&NounPhrase> {
        self.args.iter().find(|a| a.role == role).map(|a| &a.np)
    }
}

/// The meaning of one inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "value")]
pub enum Sentence {
    Clause(Clause),
    /// A verbless list of items, as on a ledger or label.
    List(Vec<NounPhrase>),
    /// Several sentences in a row, as in a letter.
    Text(Vec<Sentence>),
}
