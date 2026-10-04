//! Structured meaning: what a sentence says, before it has any form.
//!
//! Every generated text starts here and is rendered through the grammar, so
//! the engine can always report the true meaning of what it produced. Later,
//! parsing player text means going the other way: surface → `Sentence`.

use serde::{Deserialize, Serialize};

pub use crate::morphology::{Number, Polarity, Tense};

/// What a noun phrase is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Head {
    /// A concept from the lexicon, by id.
    Concept(String),
    /// A named person from the corpus cast, by index.
    Name(usize),
}

/// A noun phrase with its modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NounPhrase {
    pub head: Head,
    pub number: Number,
    /// A counted quantity (0–999), rendered as numeral words.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u16>,
    /// A rank (D07): "the third year", and with "part" a fraction ("a
    /// third part of the grain").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u16>,
    /// A determiner concept, e.g. "this".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determiner: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjectives: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub possessor: Option<Box<NounPhrase>>,
    /// Phrases renaming the head: titles, "child of X".
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apposition: Vec<NounPhrase>,
    /// A comparison on one adjective: "greater (than the tower)" (D07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degree: Option<Box<Degree>>,
    /// "the man who built the gate": a clause with one argument left out,
    /// filled by this noun (D07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<Box<Relative>>,
}

/// How an adjective compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Compare {
    /// greater (than X)
    More,
    /// the greatest
    Most,
    /// as great (as X)
    As,
}

/// An adjective with a degree, and what it is measured against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Degree {
    pub compare: Compare,
    pub adjective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<NounPhrase>,
}

/// A relative clause: `clause` lacks the argument in role `gap`, which the
/// noun it modifies fills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relative {
    pub gap: Role,
    pub clause: Clause,
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
            ordinal: None,
            determiner: None,
            adjectives: Vec::new(),
            possessor: None,
            apposition: Vec::new(),
            degree: None,
            relative: None,
        }
    }

    /// Sets a counted quantity; plural follows from it.
    /// With a determiner ("this", "all", "no"…).
    pub fn det(mut self, d: &str) -> Self {
        self.determiner = Some(d.to_string());
        self
    }

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Subject,
    Object,
    /// The one something is done for.
    Recipient,
    /// When (D07): "in the third year of the reign of X".
    Time,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argument {
    pub role: Role,
    pub np: NounPhrase,
}

/// Statement, command, wish, condition or question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    Declarative,
    /// Addressed to the reader; has no subject.
    Imperative,
    /// A claim in the potent register: framed by fixed formulae and marked
    /// with a particle. What it does is decided in M08.
    Potent,
    /// "May it…" (D07).
    Optative,
    /// "It would…" (D07).
    Conditional,
    /// A question (D07).
    Interrogative,
}

impl Mood {
    /// Moods marked on the verb by their own affix (D07).
    pub const MARKED: [Mood; 3] = [Mood::Optative, Mood::Conditional, Mood::Interrogative];
}

/// Aspect, beyond tense (D07). Each language marks at most one of the
/// three; `Simple` is unmarked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    #[default]
    Simple,
    /// The whole event, done.
    Perfective,
    /// Ongoing.
    Imperfective,
    /// Usual, repeated.
    Habitual,
}

impl Aspect {
    pub fn is_simple(&self) -> bool {
        *self == Aspect::Simple
    }
}

/// How a subordinate clause links to its main clause (D07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Link {
    When,
    Because,
    If,
    Until,
    Before,
    After,
    SoThat,
    Although,
    /// An exception (D09): "let no one pass unless the priest comes".
    Unless,
}

impl Link {
    pub const ALL: [Link; 9] = [
        Link::When,
        Link::Because,
        Link::If,
        Link::Until,
        Link::Before,
        Link::After,
        Link::SoThat,
        Link::Although,
        Link::Unless,
    ];

    /// The function word's concept id.
    pub fn concept(self) -> &'static str {
        match self {
            Link::When => "sub.when",
            Link::Because => "sub.because",
            Link::If => "sub.if",
            Link::Until => "sub.until",
            Link::Before => "sub.before",
            Link::After => "sub.after",
            Link::SoThat => "sub.sothat",
            Link::Although => "sub.although",
            Link::Unless => "sub.unless",
        }
    }
}

/// How clauses of equal standing join (D07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Conj {
    And,
    But,
    Or,
    Then,
}

impl Conj {
    pub const ALL: [Conj; 4] = [Conj::And, Conj::But, Conj::Or, Conj::Then];

    pub fn concept(self) -> &'static str {
        match self {
            Conj::And => "conj.and",
            Conj::But => "conj.but",
            Conj::Or => "conj.or",
            Conj::Then => "conj.then",
        }
    }
}

/// A clause subordinate to another: "when the king died, …".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subordinate {
    pub link: Link,
    pub clause: Clause,
}

/// What a speech verb reports: "said that…" or a quotation (D07).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Complement {
    /// A direct quotation rather than reported speech.
    pub direct: bool,
    pub content: Sentence,
}

/// A clause: one predicate and its arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clause {
    pub predicate: String,
    pub mood: Mood,
    pub tense: Tense,
    pub polarity: Polarity,
    pub args: Vec<Argument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adverbs: Vec<String>,
    #[serde(default, skip_serializing_if = "Aspect::is_simple")]
    pub aspect: Aspect,
    /// Adverbial clauses: when, because, if… (D07).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subordinate: Vec<Subordinate>,
    /// What a speech verb reports (D07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complement: Option<Box<Complement>>,
}

impl Clause {
    pub fn arg(&self, role: Role) -> Option<&NounPhrase> {
        self.args.iter().find(|a| a.role == role).map(|a| &a.np)
    }

    /// A plain present clause: `subject predicate`.
    pub fn plain(predicate: &str, subject: NounPhrase) -> Self {
        Clause {
            predicate: predicate.to_string(),
            mood: Mood::Declarative,
            tense: Tense::NonPast,
            polarity: Polarity::Positive,
            args: vec![Argument {
                role: Role::Subject,
                np: subject,
            }],
            adverbs: Vec::new(),
            aspect: Aspect::default(),
            subordinate: Vec::new(),
            complement: None,
        }
    }

    /// A spell (D09): "let `subject` `predicate`".
    pub fn potent(predicate: &str, subject: NounPhrase) -> Self {
        Clause {
            mood: Mood::Potent,
            ..Clause::plain(predicate, subject)
        }
    }

    pub fn with_object(mut self, object: NounPhrase) -> Self {
        self.args.push(Argument {
            role: Role::Object,
            np: object,
        });
        self
    }

    pub fn with_adverb(mut self, adverb: &str) -> Self {
        self.adverbs.push(adverb.to_string());
        self
    }

    pub fn denied(mut self) -> Self {
        self.polarity = Polarity::Negative;
        self
    }

    pub fn with_clause(mut self, link: Link, clause: Clause) -> Self {
        self.subordinate.push(Subordinate { link, clause });
        self
    }
}

/// The meaning of one inscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "type", content = "value")]
pub enum Sentence {
    Clause(Clause),
    /// A verbless list of items, as on a ledger or label.
    List(Vec<NounPhrase>),
    /// Several sentences in a row, as in a letter.
    Text(Vec<Sentence>),
    /// Clauses joined by a conjunction: "…and…", "…but…" (D07).
    Joined(Conj, Vec<Clause>),
}

impl Sentence {
    /// Every clause in it, nested ones included (joined parts, adverbial
    /// clauses, reported speech, relative clauses), outermost first.
    pub fn clauses(&self) -> Vec<&Clause> {
        let mut out = Vec::new();
        match self {
            Sentence::Clause(c) => c.walk(&mut out),
            Sentence::List(items) => items.iter().for_each(|n| n.walk(&mut out)),
            Sentence::Text(parts) => parts.iter().for_each(|p| out.extend(p.clauses())),
            Sentence::Joined(_, parts) => parts.iter().for_each(|c| c.walk(&mut out)),
        }
        out
    }

    /// Every noun phrase in it, at any depth.
    pub fn noun_phrases(&self) -> Vec<&NounPhrase> {
        let mut out = Vec::new();
        if let Sentence::List(items) = self {
            items.iter().for_each(|n| n.collect(&mut out));
        }
        for c in self.clauses() {
            for a in &c.args {
                a.np.collect(&mut out);
            }
        }
        out
    }

    /// The function words its structure uses (D07): conjunctions, linking
    /// words, quotation frames, relative and degree words.
    pub fn function_words(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if let Sentence::Joined(conj, _) = self {
            out.push(conj.concept());
        }
        if let Sentence::Text(parts) = self {
            for p in parts {
                if let Sentence::Joined(conj, _) = p {
                    out.push(conj.concept());
                }
            }
        }
        for c in self.clauses() {
            for s in &c.subordinate {
                out.push(s.link.concept());
            }
            if let Some(comp) = &c.complement {
                if comp.direct {
                    out.extend(["quote.open", "quote.close"]);
                } else {
                    out.push("comp.that");
                }
                if let Sentence::Joined(conj, _) = &comp.content {
                    out.push(conj.concept());
                }
            }
            for a in &c.args {
                if a.role == Role::Time {
                    out.push("time.at");
                }
                let mut nps = Vec::new();
                a.np.collect(&mut nps);
                for n in nps {
                    if n.ordinal.is_some() {
                        out.push("ord");
                    }
                    if n.relative.is_some() {
                        out.push("rel");
                    }
                    if let Some(d) = &n.degree {
                        out.push(match d.compare {
                            Compare::More => "cmp.more",
                            Compare::Most => "cmp.most",
                            Compare::As => "cmp.as",
                        });
                        if d.standard.is_some() {
                            out.push(if d.compare == Compare::As {
                                "cmp.like"
                            } else {
                                "cmp.than"
                            });
                        }
                    }
                }
            }
        }
        out
    }

    /// Whether any clause in it is in the potent register.
    pub fn is_potent(&self) -> bool {
        self.clauses().iter().any(|c| c.mood == Mood::Potent)
    }
}

impl Clause {
    fn walk<'a>(&'a self, out: &mut Vec<&'a Clause>) {
        out.push(self);
        for a in &self.args {
            a.np.walk(out);
        }
        for s in &self.subordinate {
            s.clause.walk(out);
        }
        if let Some(comp) = &self.complement {
            out.extend(comp.content.clauses());
        }
    }
}

impl NounPhrase {
    /// Clauses inside this phrase (relative clauses, at any depth).
    fn walk<'a>(&'a self, out: &mut Vec<&'a Clause>) {
        if let Some(p) = &self.possessor {
            p.walk(out);
        }
        for a in &self.apposition {
            a.walk(out);
        }
        if let Some(st) = self.degree.as_ref().and_then(|d| d.standard.as_ref()) {
            st.walk(out);
        }
        if let Some(r) = &self.relative {
            r.clause.walk(out);
        }
    }

    /// This phrase and every phrase inside it (not inside its relative
    /// clause, which `Sentence::clauses` reaches).
    fn collect<'a>(&'a self, out: &mut Vec<&'a NounPhrase>) {
        out.push(self);
        if let Some(p) = &self.possessor {
            p.collect(out);
        }
        for a in &self.apposition {
            a.collect(out);
        }
        if let Some(st) = self.degree.as_ref().and_then(|d| d.standard.as_ref()) {
            st.collect(out);
        }
    }
}
