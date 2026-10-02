//! Word-order parameters chosen per seed.

use serde::Serialize;

use crate::rng::{Rng, Stream};

/// Order of subject, object and verb in a clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WordOrder {
    #[serde(rename = "SOV")]
    Sov,
    #[serde(rename = "SVO")]
    Svo,
    #[serde(rename = "VSO")]
    Vso,
}

impl WordOrder {
    /// Verb-final languages put objects (and adverbs) before the verb.
    pub fn verb_final(self) -> bool {
        self == WordOrder::Sov
    }

    pub fn label(self) -> &'static str {
        match self {
            WordOrder::Sov => "SOV",
            WordOrder::Svo => "SVO",
            WordOrder::Vso => "VSO",
        }
    }
}

/// Whether a modifier comes before or after the noun it modifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Before,
    After,
}

/// The ordering rules of one language.
#[derive(Debug, Clone, Serialize)]
pub struct Syntax {
    pub word_order: WordOrder,
    /// Adjectives, numerals and the demonstrative relative to the noun.
    pub modifiers: Side,
    /// The possessor relative to the possessed noun.
    pub genitive: Side,
}

impl Syntax {
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed, Stream::Syntax);
        let word_order = rng.weighted(&[
            (WordOrder::Sov, 45),
            (WordOrder::Svo, 40),
            (WordOrder::Vso, 15),
        ]);
        let modifiers = rng.weighted(&[(Side::Before, 50), (Side::After, 50)]);
        // Possessor order tends to follow head direction, as in real languages.
        let genitive = if word_order.verb_final() {
            rng.weighted(&[(Side::Before, 80), (Side::After, 20)])
        } else {
            rng.weighted(&[(Side::Before, 30), (Side::After, 70)])
        };
        Syntax {
            word_order,
            modifiers,
            genitive,
        }
    }

    /// Human-readable summary lines for the grammar sheet.
    // DESIGN-Q: numerals and "this" are placed like adjectives, and adverbs
    // and datives follow head direction (before the verb/object in SOV,
    // clause-final otherwise). The milestone fixes only S/O/V, adjective and
    // genitive order; these are the simplest regular extensions.
    pub fn describe(&self) -> Vec<String> {
        let side = |s: Side, a: &str, b: &str| match s {
            Side::Before => format!("{a} before {b}"),
            Side::After => format!("{a} after {b}"),
        };
        vec![
            format!("clause order: {}", self.word_order.label()),
            format!(
                "dative (beneficiary): {}",
                if self.word_order.verb_final() {
                    "just before the object"
                } else {
                    "after the object"
                }
            ),
            format!(
                "adverbs: {}",
                if self.word_order.verb_final() {
                    "just before the verb"
                } else {
                    "at the end of the clause"
                }
            ),
            "commands: plain non-past verb with no subject".to_string(),
            match self.modifiers {
                Side::Before => "noun phrase: this – number – adjectives – noun".to_string(),
                Side::After => "noun phrase: noun – adjectives – number – this".to_string(),
            },
            side(self.genitive, "possessor", "possessed noun"),
            "appositions (titles, 'child of X') follow the name".to_string(),
        ]
    }
}
