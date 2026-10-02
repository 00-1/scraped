//! Numbers: how a language counts in words and writes numeral signs.
//!
//! Numerals are a classic decipherment anchor: ledgers with totals let a
//! reader check a guess by arithmetic. Each seed picks a base and a way of
//! combining words, and every number from 0 to 999 has exactly one spelling
//! that parses back to it.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::rng::{Rng, Stream};

/// Largest number the system must express.
pub const MAX: u16 = 999;

/// Counting base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Base {
    /// Tens and hundreds.
    Decimal,
    /// Dozens and grosses, with words up to eleven.
    Duodecimal,
    /// Scores and four-hundreds; 11–19 are "ten one" … "ten nine".
    Vigesimal,
    /// Decimal, but 6–9 are "five one" … "five four".
    Quinary,
}

impl Base {
    /// Tag used by `needs` in concepts.toml.
    pub fn tag(self) -> &'static str {
        match self {
            Base::Decimal => "decimal",
            Base::Duodecimal => "duodecimal",
            Base::Vigesimal => "vigesimal",
            Base::Quinary => "quinary",
        }
    }

    /// The base and its square, with their words.
    fn powers(self) -> [(&'static str, u16); 2] {
        match self {
            Base::Decimal | Base::Quinary => [("hundred", 100), ("ten", 10)],
            Base::Duodecimal => [("gross", 144), ("dozen", 12)],
            Base::Vigesimal => [("fourhundred", 400), ("score", 20)],
        }
    }

    pub fn radix(self) -> u16 {
        self.powers()[1].1
    }
}

/// How numeral signs are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignStyle {
    /// Repeated signs for 1, (5), the base and its square, like tally marks.
    Additive,
    /// One digit sign per place, including a zero.
    Positional,
}

/// Whether the biggest part of a number is said first or last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    BigFirst,
    SmallFirst,
}

/// A language's number system.
#[derive(Debug, Clone, Serialize)]
pub struct Numerals {
    pub base: Base,
    pub signs: SignStyle,
    pub order: Order,
    /// "hundred" rather than "one hundred".
    pub bare_power: bool,
    /// An "and" word between the parts of a number.
    pub linker: bool,
}

const ATOMS: [&str; 11] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven",
];

impl Numerals {
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed, Stream::Numerals);
        let base = rng.weighted(&[
            (Base::Decimal, 45),
            (Base::Vigesimal, 20),
            (Base::Duodecimal, 15),
            (Base::Quinary, 20),
        ]);
        let signs = rng.weighted(&[(SignStyle::Additive, 60), (SignStyle::Positional, 40)]);
        let order = rng.weighted(&[(Order::BigFirst, 70), (Order::SmallFirst, 30)]);
        let mut n = Numerals {
            base,
            signs,
            order,
            bare_power: rng.chance(60),
            linker: rng.chance(25),
        };
        // Some combinations are ambiguous without a linker ("five one ten"
        // could be 16 or 60); real languages solve this the same way.
        if !n.is_unambiguous() {
            n.linker = true;
        }
        debug_assert!(n.is_unambiguous());
        n
    }

    /// Whether a concept is part of this system's vocabulary.
    pub fn needs(&self, concept: &str, needs: &[String]) -> bool {
        if concept == "and" {
            return self.linker;
        }
        needs.is_empty() || needs.iter().any(|t| t == self.base.tag())
    }

    /// The words for a coefficient smaller than the base.
    fn coefficient(&self, c: u16) -> Vec<&'static str> {
        let c = usize::from(c);
        match self.base {
            Base::Vigesimal if c > 10 => vec!["ten", ATOMS[c - 11]],
            Base::Quinary if c > 5 => vec!["five", ATOMS[c - 6]],
            _ => vec![ATOMS[c - 1]],
        }
    }

    /// The words for `n` (0–999), as concept ids in spoken order.
    pub fn words(&self, n: u16) -> Vec<&'static str> {
        assert!(n <= MAX, "{n} is out of range");
        if n == 0 {
            return vec!["zero"];
        }
        let mut terms: Vec<Vec<&'static str>> = Vec::new();
        let mut rest = n;
        for (word, value) in self.base.powers() {
            let c = rest / value;
            rest %= value;
            if c > 0 {
                let mut t = if c == 1 && self.bare_power {
                    Vec::new()
                } else {
                    self.coefficient(c)
                };
                t.push(word);
                terms.push(t);
            }
        }
        if rest > 0 {
            terms.push(self.coefficient(rest));
        }
        if self.order == Order::SmallFirst {
            terms.reverse();
        }
        let mut out = Vec::new();
        for (i, t) in terms.into_iter().enumerate() {
            if i > 0 && self.linker {
                out.push("and");
            }
            out.extend(t);
        }
        out
    }

    /// Reads a number back from its words.
    pub fn parse(&self, words: &[&str]) -> Option<u16> {
        (0..=MAX).find(|&n| self.words(n) == words)
    }

    fn is_unambiguous(&self) -> bool {
        let mut seen = BTreeMap::new();
        (0..=MAX).all(|n| seen.insert(self.words(n), n).is_none())
    }

    /// Values that have a numeral sign of their own.
    // DESIGN-Q: every script has numeral signs, but inscriptions still write
    // numbers as words. Ledgers in signs would be a stronger anchor (sums
    // checkable before any word is known); words teach more vocabulary.
    pub fn sign_values(&self) -> Vec<u16> {
        let b = self.base.radix();
        match self.signs {
            SignStyle::Positional => (0..b).collect(),
            SignStyle::Additive => {
                let mut v = vec![1];
                if self.base == Base::Quinary {
                    v.push(5);
                }
                v.extend([b, b * b]);
                v
            }
        }
    }

    /// `n` written as numeral signs, biggest first: each entry is the value
    /// of one sign.
    pub fn signs(&self, n: u16) -> Vec<u16> {
        let b = self.base.radix();
        match self.signs {
            SignStyle::Positional => {
                let mut digits = Vec::new();
                let mut rest = n;
                loop {
                    digits.push(rest % b);
                    rest /= b;
                    if rest == 0 {
                        break;
                    }
                }
                digits.reverse();
                digits
            }
            SignStyle::Additive => {
                let mut out = Vec::new();
                let mut rest = n;
                for v in self.sign_values().into_iter().rev() {
                    while rest >= v {
                        out.push(v);
                        rest -= v;
                    }
                }
                out
            }
        }
    }

    /// Reads numeral signs back.
    pub fn read_signs(&self, signs: &[u16]) -> u16 {
        match self.signs {
            SignStyle::Positional => signs.iter().fold(0, |acc, d| acc * self.base.radix() + d),
            SignStyle::Additive => signs.iter().sum(),
        }
    }

    /// Summary lines for the grammar sheet.
    pub fn describe(&self) -> Vec<String> {
        let [(sq, _), (b, _)] = self.base.powers();
        vec![
            format!("base: {} ({b}, {sq})", self.base.tag()),
            format!(
                "order: {}",
                match self.order {
                    Order::BigFirst => "biggest part first",
                    Order::SmallFirst => "smallest part first",
                }
            ),
            format!(
                "one {b}: {}",
                if self.bare_power {
                    format!("'{b}' alone")
                } else {
                    format!("'one {b}'")
                }
            ),
            format!(
                "linker between parts: {}",
                if self.linker { "yes ('and')" } else { "no" }
            ),
            format!(
                "numeral signs: {}",
                match self.signs {
                    SignStyle::Additive => "additive (repeat signs and add)",
                    SignStyle::Positional => "positional (one digit per place, with zero)",
                }
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_systems() -> Vec<Numerals> {
        let mut out = Vec::new();
        for base in [
            Base::Decimal,
            Base::Duodecimal,
            Base::Vigesimal,
            Base::Quinary,
        ] {
            for signs in [SignStyle::Additive, SignStyle::Positional] {
                for order in [Order::BigFirst, Order::SmallFirst] {
                    for bare_power in [false, true] {
                        let mut n = Numerals {
                            base,
                            signs,
                            order,
                            bare_power,
                            linker: false,
                        };
                        if !n.is_unambiguous() {
                            n.linker = true;
                        }
                        out.push(n);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn words_round_trip_for_every_system() {
        for sys in all_systems() {
            for n in 0..=MAX {
                let w = sys.words(n);
                assert_eq!(sys.parse(&w), Some(n), "{sys:?} {n} {w:?}");
            }
        }
    }

    #[test]
    fn signs_round_trip_for_every_system() {
        for sys in all_systems() {
            for n in 1..=MAX {
                assert_eq!(sys.read_signs(&sys.signs(n)), n, "{sys:?} {n}");
            }
        }
    }

    #[test]
    fn decimal_examples() {
        let sys = Numerals {
            base: Base::Decimal,
            signs: SignStyle::Additive,
            order: Order::BigFirst,
            bare_power: true,
            linker: false,
        };
        assert_eq!(sys.words(7), ["seven"]);
        assert_eq!(sys.words(10), ["ten"]);
        assert_eq!(sys.words(342), ["three", "hundred", "four", "ten", "two"]);
        assert_eq!(sys.words(110), ["hundred", "ten"]);
        assert_eq!(sys.signs(23), [10, 10, 1, 1, 1]);
    }

    #[test]
    fn vigesimal_and_quinary_compose_small_numbers() {
        let mut sys = Numerals {
            base: Base::Vigesimal,
            signs: SignStyle::Positional,
            order: Order::BigFirst,
            bare_power: false,
            linker: false,
        };
        assert_eq!(sys.words(37), ["one", "score", "ten", "seven"]);
        assert_eq!(sys.signs(37), [1, 17]);
        sys.base = Base::Quinary;
        assert_eq!(sys.words(8), ["five", "three"]);
    }

    #[test]
    fn ambiguous_systems_get_a_linker() {
        let sys = all_systems()
            .into_iter()
            .find(|s| s.base == Base::Quinary && s.order == Order::SmallFirst)
            .unwrap();
        assert!(sys.linker);
    }
}
