//! Sound change: how a language turns into its later eras.
//!
//! Each era is derived from the one before by a short, ordered list of
//! regular sound changes drawn from a catalogue of attested change types.
//! Every rule applies to every word without exception, so a reader comparing
//! old and new inscriptions can work the changes out, as historical
//! linguists do. Rules also say how they change the syllable rules, so each
//! era has phonotactics of its own.

use serde::Serialize;

use crate::phonology::{phoneme, Manner, Sets};
use crate::rng::{Rng, Stream};

/// One regular sound change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Rule {
    /// `from` becomes `to` everywhere. A merger when `to` already exists.
    Shift {
        from: &'static str,
        to: &'static str,
    },
    /// `from` becomes `to` between vowels (lenition).
    Intervocalic {
        from: &'static str,
        to: &'static str,
    },
    /// `from` becomes `to` before a front vowel (palatalisation).
    BeforeFront {
        from: &'static str,
        to: &'static str,
    },
    /// Onset clusters lose their second consonant: `pla` → `pa`.
    ClusterLoss,
    /// A word-final vowel is lost after a single consonant in words of two or
    /// more syllables: `kana` → `kan`.
    Apocope,
}

const FRONT_VOWELS: &[&str] = &["i", "e", "ɛ"];

/// Unconditioned changes: mostly mergers of rarer sounds into common ones.
const SHIFTS: &[(&str, &str)] = &[
    ("z", "s"),
    ("v", "f"),
    ("ʒ", "ʃ"),
    ("ʃ", "s"),
    ("ɲ", "n"),
    ("θ", "t"),
    ("ð", "d"),
    ("x", "h"),
    ("ɣ", "g"),
    ("q", "k"),
    ("ts", "s"),
    ("dʒ", "ʒ"),
    ("ɬ", "l"),
    ("ŋ", "n"),
    ("ʔ", "h"),
];

/// Vowel shifts between neighbouring vowels.
const VOWEL_SHIFTS: &[(&str, &str)] = &[
    ("a", "o"),
    ("a", "ə"),
    ("e", "i"),
    ("e", "ɛ"),
    ("i", "e"),
    ("o", "u"),
    ("o", "ɔ"),
    ("u", "o"),
    ("u", "ɨ"),
    ("ɛ", "e"),
    ("ɔ", "o"),
    ("ə", "a"),
    ("ɨ", "i"),
];

/// Lenition between vowels: voicing or spirantisation.
const VOICING: &[(&str, &str)] = &[("p", "b"), ("t", "d"), ("k", "g")];
const SPIRANT: &[(&str, &str)] = &[
    ("p", "f"),
    ("t", "θ"),
    ("k", "x"),
    ("b", "v"),
    ("d", "ð"),
    ("g", "ɣ"),
];
const PALATAL: &[(&str, &str)] = &[("k", "tʃ"), ("g", "dʒ"), ("t", "ts")];

fn is_vowel(s: &str) -> bool {
    phoneme(s).is_vowel()
}

impl Rule {
    /// Applies the rule to one word.
    pub fn apply(&self, word: &[&'static str]) -> Vec<&'static str> {
        let n = word.len();
        match *self {
            Rule::Shift { from, to } => word
                .iter()
                .map(|&s| if s == from { to } else { s })
                .collect(),
            Rule::Intervocalic { from, to } => (0..n)
                .map(|i| {
                    let between =
                        i > 0 && i + 1 < n && is_vowel(word[i - 1]) && is_vowel(word[i + 1]);
                    if word[i] == from && between {
                        to
                    } else {
                        word[i]
                    }
                })
                .collect(),
            Rule::BeforeFront { from, to } => (0..n)
                .map(|i| {
                    if word[i] == from && i + 1 < n && FRONT_VOWELS.contains(&word[i + 1]) {
                        to
                    } else {
                        word[i]
                    }
                })
                .collect(),
            Rule::ClusterLoss => {
                let mut out = Vec::new();
                for i in 0..n {
                    let second_of_cluster = i > 0
                        && i + 1 < n
                        && is_cluster(word[i - 1], word[i])
                        && is_vowel(word[i + 1]);
                    if !second_of_cluster {
                        out.push(word[i]);
                    }
                }
                out
            }
            Rule::Apocope => {
                let syllables = word.iter().filter(|s| is_vowel(s)).count();
                if syllables >= 2
                    && n >= 3
                    && is_vowel(word[n - 1])
                    && !is_vowel(word[n - 2])
                    && is_vowel(word[n - 3])
                {
                    word[..n - 1].to_vec()
                } else {
                    word.to_vec()
                }
            }
        }
    }

    /// How the rule changes the syllable rules, so derived words stay legal.
    pub fn apply_sets(&self, sets: &Sets) -> Sets {
        let mut out = sets.clone();
        match *self {
            Rule::Shift { from, to } => {
                let map = |s: &&'static str| if *s == from { to } else { *s };
                out.sounds = sets.sounds.iter().map(map).collect();
                out.onsets = sets
                    .onsets
                    .iter()
                    .map(|o| o.iter().map(map).collect())
                    .collect();
                out.codas = sets.codas.iter().map(map).collect();
            }
            Rule::Intervocalic { to, .. } | Rule::BeforeFront { to, .. } => {
                out.sounds.insert(to);
                out.onsets.insert(vec![to]);
            }
            Rule::ClusterLoss => {
                out.onsets.retain(|o| o.len() == 1);
                out.template.onset_max = 1;
            }
            Rule::Apocope => {
                for o in &sets.onsets {
                    if o.len() == 1 {
                        out.codas.insert(o[0]);
                    }
                }
                out.template.coda_max = 1;
            }
        }
        out
    }

    /// Conventional notation, e.g. `t > d / V_V`.
    pub fn notation(&self) -> String {
        match self {
            Rule::Shift { from, to } => format!("{from} > {to}"),
            Rule::Intervocalic { from, to } => format!("{from} > {to} / V_V"),
            Rule::BeforeFront { from, to } => format!("{from} > {to} / _[front vowel]"),
            Rule::ClusterLoss => "CC > C at the start of a syllable (pl > p)".to_string(),
            Rule::Apocope => "final V > ∅ / VC_# in words of 2+ syllables".to_string(),
        }
    }
}

fn is_cluster(a: &str, b: &str) -> bool {
    let (pa, pb) = (phoneme(a), phoneme(b));
    pa.manner() == Some(Manner::Stop)
        && matches!(
            pb.manner(),
            Some(Manner::Lateral | Manner::Rhotic | Manner::Glide)
        )
}

/// The sound changes between one era and the next, applied in order.
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    /// The era these changes produce.
    pub era: u32,
    pub rules: Vec<Rule>,
}

impl Step {
    /// Applies every rule in order to a word.
    pub fn apply(&self, word: &[&'static str]) -> Vec<&'static str> {
        self.rules.iter().fold(word.to_vec(), |w, r| r.apply(&w))
    }

    pub fn apply_sets(&self, sets: &Sets) -> Sets {
        self.rules
            .iter()
            .fold(sets.clone(), |s, r| r.apply_sets(&s))
    }

    /// Generates the changes leading to `era`, given the sounds of the era
    /// before and the rules already applied. Draws two or three changes of
    /// different kinds, never repeating or undoing an earlier rule.
    pub fn generate(seed: u64, era: u32, before: &Sets, earlier: &[Rule]) -> Self {
        let fresh = |r: &Rule| {
            !earlier.contains(r)
                && !matches!(r, Rule::Shift { from, to } if earlier.contains(&Rule::Shift { from: to, to: from }))
        };
        let mut rng = Rng::new(seed, Stream::SoundChange(era));
        let has = |s: &str| before.sounds.contains(s);
        let fronts = FRONT_VOWELS.iter().any(|v| has(v));

        #[derive(Clone, Copy, PartialEq)]
        enum Kind {
            Lenition,
            Palatal,
            Merger,
            Vowel,
            Cluster,
            Apocope,
        }
        let mut kinds = vec![(Kind::Lenition, 30), (Kind::Merger, 30), (Kind::Vowel, 30)];
        if fronts && PALATAL.iter().any(|(f, _)| has(f)) {
            kinds.push((Kind::Palatal, 20));
        }
        if before.template.onset_max > 1 && !earlier.contains(&Rule::ClusterLoss) {
            kinds.push((Kind::Cluster, 20));
        }
        // Final vowel loss is a one-off: once codas are everywhere it would
        // keep eating words, so it only happens while some vowels are final.
        if !earlier.contains(&Rule::Apocope) && (before.template.coda_max == 0 || rng.chance(40)) {
            kinds.push((Kind::Apocope, 15));
        }

        let count = rng.range(2, 3);
        let mut rules = Vec::new();
        let mut sets = before.clone();
        let mut used = Vec::new();
        let mut groups = 0;
        while groups < count {
            let options: Vec<(Kind, u32)> = kinds
                .iter()
                .copied()
                .filter(|(k, _)| !used.contains(k))
                .collect();
            if options.is_empty() {
                break;
            }
            let kind = rng.weighted(&options);
            used.push(kind);
            let has = |s: &str| sets.sounds.contains(s);
            let new: Vec<Rule> = match kind {
                Kind::Lenition => {
                    let table = if rng.chance(50) { VOICING } else { SPIRANT };
                    table
                        .iter()
                        .filter(|(f, _)| has(f))
                        .map(|&(from, to)| Rule::Intervocalic { from, to })
                        .collect()
                }
                Kind::Palatal => PALATAL
                    .iter()
                    .filter(|(f, _)| has(f))
                    .take(1)
                    .map(|&(from, to)| Rule::BeforeFront { from, to })
                    .collect(),
                Kind::Merger => pick_pair(&mut rng, SHIFTS, &has),
                Kind::Vowel => pick_pair(&mut rng, VOWEL_SHIFTS, &has),
                Kind::Cluster => vec![Rule::ClusterLoss],
                Kind::Apocope => vec![Rule::Apocope],
            };
            let mut added = false;
            for r in new.into_iter().filter(|r| fresh(r)) {
                sets = r.apply_sets(&sets);
                rules.push(r);
                added = true;
            }
            groups += u32::from(added);
        }
        Step { era, rules }
    }
}

fn pick_pair(
    rng: &mut Rng,
    table: &[(&'static str, &'static str)],
    has: &dyn Fn(&str) -> bool,
) -> Vec<Rule> {
    let options: Vec<&(&str, &str)> = table.iter().filter(|(f, _)| has(f)).collect();
    if options.is_empty() {
        return Vec::new();
    }
    let &&(from, to) = rng.pick(&options);
    vec![Rule::Shift { from, to }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &[&'static str]) -> Vec<&'static str> {
        s.to_vec()
    }

    #[test]
    fn intervocalic_only_between_vowels() {
        let r = Rule::Intervocalic { from: "t", to: "d" };
        assert_eq!(r.apply(&w(&["t", "a", "t", "a"])), w(&["t", "a", "d", "a"]));
        assert_eq!(r.apply(&w(&["a", "t"])), w(&["a", "t"]));
    }

    #[test]
    fn palatalisation_before_front_vowels() {
        let r = Rule::BeforeFront {
            from: "k",
            to: "tʃ",
        };
        assert_eq!(
            r.apply(&w(&["k", "i", "k", "a"])),
            w(&["tʃ", "i", "k", "a"])
        );
    }

    #[test]
    fn cluster_loss_keeps_first_consonant() {
        assert_eq!(
            Rule::ClusterLoss.apply(&w(&["p", "l", "a"])),
            w(&["p", "a"])
        );
        assert_eq!(
            Rule::ClusterLoss.apply(&w(&["a", "l", "a"])),
            w(&["a", "l", "a"])
        );
    }

    #[test]
    fn apocope_needs_two_syllables_and_a_single_consonant() {
        assert_eq!(
            Rule::Apocope.apply(&w(&["k", "a", "n", "a"])),
            w(&["k", "a", "n"])
        );
        assert_eq!(Rule::Apocope.apply(&w(&["k", "a"])), w(&["k", "a"]));
        assert_eq!(
            Rule::Apocope.apply(&w(&["k", "a", "n", "t", "a"])),
            w(&["k", "a", "n", "t", "a"])
        );
    }

    #[test]
    fn steps_are_deterministic_and_nonempty() {
        for seed in 0..50 {
            let sets = crate::phonology::Phonology::generate(seed).sets();
            let a = Step::generate(seed, 1, &sets, &[]);
            let b = Step::generate(seed, 1, &sets, &[]);
            assert_eq!(a.rules, b.rules);
            assert!(!a.rules.is_empty(), "seed {seed}");
        }
    }

    #[test]
    fn later_steps_never_repeat_rules() {
        for seed in 0..50 {
            let lang = crate::Language::generate_with(
                seed,
                crate::difficulty::Difficulty {
                    eras: 5,
                    ..Default::default()
                },
            );
            let last = lang.at_era(4);
            let all: Vec<&Rule> = last.history.iter().flat_map(|s| &s.rules).collect();
            for (i, r) in all.iter().enumerate() {
                assert!(!all[..i].contains(r), "seed {seed}: {r:?} repeated");
            }
            assert!(
                last.history.iter().all(|s| !s.rules.is_empty()),
                "seed {seed}: empty step"
            );
        }
    }
}
