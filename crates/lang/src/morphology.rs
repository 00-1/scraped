//! One regular, agglutinative inflection system.
//!
//! Each grammatical category has exactly one affix and unmarked values take
//! none, so a reader comparing forms can peel affixes off one at a time.
//! There are no irregular forms and no sandhi: a word is literally
//! root + affixes, in a fixed order.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::concepts::Pos;
use crate::difficulty::Regularity;
use crate::meaning::{Aspect, Mood};
use crate::phonology::{Phonemes, Phonology};
use crate::rng::{Rng, Stream};

/// Grammatical role of a noun phrase, marked on its head noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    /// Unmarked: the doer, or the one who is.
    Subject,
    /// The thing acted on.
    Object,
    /// The owner in "the king's gate", "child of X".
    Genitive,
    /// The beneficiary in "made this for X".
    // DESIGN-Q: the milestone asks for 2–3 cases, but the dedication formula
    // needs "for [person/place]". A dative case is the simplest regular way
    // to express it; an adposition would be the alternative.
    Dative,
}

impl Case {
    pub const ALL: [Case; 4] = [Case::Subject, Case::Object, Case::Genitive, Case::Dative];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Number {
    Singular,
    Plural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tense {
    #[default]
    NonPast,
    Past,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Polarity {
    #[default]
    Positive,
    Negative,
}

/// Every inflectional choice of a verb (D07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct VerbForm {
    pub aspect: Aspect,
    pub tense: Tense,
    /// Only the marked moods take an affix; the rest are unmarked here.
    pub mood: Option<Mood>,
    pub polarity: Polarity,
}

impl VerbForm {
    pub fn plain(tense: Tense, polarity: Polarity) -> Self {
        VerbForm {
            tense,
            polarity,
            ..Default::default()
        }
    }
}

/// Whether a word class puts its affixes before or after the root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AffixPosition {
    Prefix,
    Suffix,
}

/// A grammatical marker and its Leipzig-style gloss label.
#[derive(Debug, Clone, Serialize)]
pub struct Affix {
    pub gloss: &'static str,
    pub form: Phonemes,
    /// A separate word next to its host instead of part of it. Affixes
    /// erode into particles when sound change wears them down (M02).
    pub particle: bool,
}

/// Several affixes merged into one unanalysable form, as in Latin `-orum`
/// for genitive plural. Only with the `Fused` difficulty dial.
#[derive(Debug, Clone, Serialize)]
pub struct Fusion {
    /// The affixes it replaces, from the root outward.
    pub glosses: Vec<&'static str>,
    pub form: Phonemes,
}

/// The affixes of one language and where they go.
#[derive(Debug, Clone, Serialize)]
pub struct Morphology {
    pub noun_position: AffixPosition,
    pub verb_position: AffixPosition,
    pub plural: Affix,
    pub object: Affix,
    pub genitive: Affix,
    pub dative: Affix,
    pub past: Affix,
    pub negative: Affix,
    pub fusions: Vec<Fusion>,
    /// The one aspect this language marks, if any (D07).
    pub aspect: Aspect,
    pub aspect_affix: Affix,
    /// Optative, conditional and interrogative (D07).
    pub moods: Vec<Affix>,
    /// Derivational affixes (D07), in the order of `DERIVATIONS`; on the
    /// same side as noun affixes, next to the root.
    pub derivations: Vec<Affix>,
    /// Compounds put their head last ("river-stone") or first.
    pub compound_head_last: bool,
}

/// The derivations (D07): gloss, what they derive from, what they make.
/// AGT: who does it (builder); PLACE: where it is done (bakery); INSTR:
/// what it is done with (opener); ABST: the state or quality (kingship,
/// greatness); DIM: a small one (little house); ADJZ: like it (stony).
pub const DERIVATIONS: [(&str, Pos, Pos); 7] = [
    ("AGT", Pos::Verb, Pos::Noun),
    ("PLACE", Pos::Verb, Pos::Noun),
    ("INSTR", Pos::Verb, Pos::Noun),
    ("ABST", Pos::Noun, Pos::Noun),
    ("ABST", Pos::Adj, Pos::Noun),
    ("DIM", Pos::Noun, Pos::Noun),
    ("ADJZ", Pos::Noun, Pos::Adj),
];

/// The distinct derivational glosses, in order.
pub const DERIVATION_GLOSSES: [&str; 6] = ["AGT", "PLACE", "INSTR", "ABST", "DIM", "ADJZ"];

/// One piece of a word: a root or an affix, with its gloss.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Morph {
    pub form: Phonemes,
    pub gloss: String,
    pub is_root: bool,
}

impl Morphology {
    /// Generates affixes for a language. Affixes are distinct, and every
    /// combination within a paradigm spells out differently.
    pub fn generate(seed: u64, phonology: &Phonology, regularity: Regularity) -> Self {
        Self::generate_with(
            seed,
            phonology,
            regularity,
            crate::difficulty::Markers::Mixed,
        )
    }

    /// Generates affixes with the aspect and mood markers set by the
    /// difficulty dial (D07).
    pub fn generate_with(
        seed: u64,
        phonology: &Phonology,
        regularity: Regularity,
        markers: crate::difficulty::Markers,
    ) -> Self {
        let mut rng = Rng::new(seed, Stream::Morphology);
        let noun_position = position(&mut rng);
        let verb_position = position(&mut rng);
        loop {
            let mut make = |gloss| Affix {
                gloss,
                form: phonology.random_affix(&mut rng),
                particle: false,
            };
            let mut m = Morphology {
                noun_position,
                verb_position,
                plural: make("PL"),
                object: make("ACC"),
                genitive: make("GEN"),
                dative: make("DAT"),
                past: make("PST"),
                negative: make("NEG"),
                fusions: Vec::new(),
                aspect: Aspect::Simple,
                aspect_affix: Affix {
                    gloss: "ASP",
                    form: Vec::new(),
                    particle: false,
                },
                moods: Vec::new(),
                derivations: Vec::new(),
                compound_head_last: true,
            };
            if m.affixes_are_distinct() {
                m.add_moods_and_aspect(seed, phonology, markers);
                m.add_derivations(seed, phonology);
                if regularity == Regularity::Fused {
                    m.fuse_some(seed, phonology);
                }
                return m;
            }
        }
    }

    /// Adds the D07 affixes from their own stream, so the older affixes
    /// keep their forms. Each is an affix or, about one time in three, a
    /// separate particle word.
    // DESIGN-Q: aspect systems are tense only (a quarter of languages), or
    // tense plus one marked aspect: perfective, imperfective or habitual.
    // Moods are optative, conditional and interrogative, each with its own
    // marker.
    fn add_moods_and_aspect(
        &mut self,
        seed: u64,
        phonology: &Phonology,
        markers: crate::difficulty::Markers,
    ) {
        let mut rng = Rng::new(seed, Stream::Grammar);
        self.aspect = rng.weighted(&[
            (Aspect::Simple, 25),
            (Aspect::Perfective, 30),
            (Aspect::Imperfective, 30),
            (Aspect::Habitual, 15),
        ]);
        let base = self.clone();
        loop {
            let mut make = |gloss: &'static str| {
                let form = phonology.random_affix(&mut rng);
                // The draw is made whatever the dial, so forms stay put.
                let chance = rng.chance(30);
                Affix {
                    gloss,
                    form,
                    particle: match markers {
                        crate::difficulty::Markers::Words => true,
                        crate::difficulty::Markers::Mixed => chance,
                        crate::difficulty::Markers::Attached => false,
                    },
                }
            };
            let aspect_gloss = match self.aspect {
                Aspect::Perfective => "PFV",
                Aspect::Imperfective => "IPFV",
                Aspect::Habitual => "HAB",
                Aspect::Simple => "ASP",
            };
            let mut m = base.clone();
            m.aspect_affix = make(aspect_gloss);
            if m.aspect == Aspect::Simple {
                m.aspect_affix.form = Vec::new();
                m.aspect_affix.particle = false;
            }
            m.moods = vec![make("OPT"), make("COND"), make("Q")];
            // Particles need forms distinct from each other as words.
            let words: BTreeSet<&Phonemes> = m.particles().iter().map(|a| &a.form).collect();
            if m.affixes_are_distinct() && words.len() == m.particles().len() {
                *self = m;
                return;
            }
        }
    }

    /// Adds one fused noun form and one fused verb form. Drawn from a
    /// separate stream, so turning the dial leaves the rest of the language
    /// alone (bar the rare root that would collide with a fused form).
    fn fuse_some(&mut self, seed: u64, phonology: &Phonology) {
        let mut rng = Rng::new(seed, Stream::Fusion);
        let case = *rng.pick(&["ACC", "GEN", "DAT"]);
        loop {
            self.fusions = [vec!["PL", case], vec!["PST", "NEG"]]
                .into_iter()
                .map(|glosses| Fusion {
                    glosses,
                    form: phonology.random_affix(&mut rng),
                })
                .collect();
            if self.affixes_are_distinct() {
                return;
            }
        }
    }

    /// Forms of affixes that attach to words (not particles), and fusions.
    pub fn affixes(&self) -> Vec<&Phonemes> {
        self.affix_list()
            .into_iter()
            .filter(|a| !a.particle)
            .map(|a| &a.form)
            .chain(self.derivations.iter().map(|a| &a.form))
            .chain(self.fusions.iter().map(|f| &f.form))
            .collect()
    }

    /// Forms of the regular attached affixes, without fusions.
    pub fn plain_affixes(&self) -> Vec<&Phonemes> {
        self.affix_list()
            .into_iter()
            .filter(|a| !a.particle)
            .map(|a| &a.form)
            .chain(self.derivations.iter().map(|a| &a.form))
            .collect()
    }

    /// Adds the derivational affixes from their own stream (D07), and the
    /// order of compounds.
    // DESIGN-Q: six derivations, always affixes (never separate words),
    // on the noun side of the root; compounds head-last in verb-final
    // languages and head-first otherwise is the real tendency, but here the
    // order is drawn freely.
    fn add_derivations(&mut self, seed: u64, phonology: &Phonology) {
        let mut rng = Rng::new(seed, Stream::Derivation);
        self.compound_head_last = rng.chance(60);
        let base = self.clone();
        loop {
            let mut m = base.clone();
            m.derivations = DERIVATION_GLOSSES
                .iter()
                .map(|g| Affix {
                    gloss: g,
                    form: phonology.random_affix(&mut rng),
                    particle: false,
                })
                .collect();
            if m.affixes_are_distinct() {
                *self = m;
                return;
            }
        }
    }

    /// A root with a derivational affix (D07).
    pub fn derive(&self, root: &Phonemes, gloss: &str) -> Phonemes {
        let Some(a) = self.derivations.iter().find(|a| a.gloss == gloss) else {
            return root.clone();
        };
        match self.noun_position {
            AffixPosition::Suffix => root.iter().chain(&a.form).copied().collect(),
            AffixPosition::Prefix => a.form.iter().chain(root).copied().collect(),
        }
    }

    /// Two roots compounded, modifier and head in the language's order.
    pub fn compound(&self, modifier: &Phonemes, head: &Phonemes) -> Phonemes {
        if self.compound_head_last {
            modifier.iter().chain(head).copied().collect()
        } else {
            head.iter().chain(modifier).copied().collect()
        }
    }

    /// Affixes that have become separate words.
    pub fn particles(&self) -> Vec<&Affix> {
        self.affix_list()
            .into_iter()
            .filter(|a| a.particle)
            .collect()
    }

    /// Mutable access to all affixes, in the same order as `affix_list`.
    pub fn affix_list_mut(&mut self) -> Vec<&mut Affix> {
        let marked = self.aspect != Aspect::Simple;
        let mut v = vec![
            &mut self.plural,
            &mut self.object,
            &mut self.genitive,
            &mut self.dative,
            &mut self.past,
            &mut self.negative,
        ];
        if marked {
            v.push(&mut self.aspect_affix);
        }
        v.extend(self.moods.iter_mut());
        v
    }

    /// All affixes with glosses, in a fixed order: the six of M01, then
    /// the marked aspect and the moods (D07).
    pub fn affix_list(&self) -> Vec<&Affix> {
        let mut v = vec![
            &self.plural,
            &self.object,
            &self.genitive,
            &self.dative,
            &self.past,
            &self.negative,
        ];
        if self.aspect != Aspect::Simple {
            v.push(&self.aspect_affix);
        }
        v.extend(self.moods.iter());
        v
    }

    /// Every verb form this language distinguishes.
    pub fn verb_forms(&self) -> Vec<VerbForm> {
        let mut aspects = vec![Aspect::Simple];
        if self.aspect != Aspect::Simple {
            aspects.push(self.aspect);
        }
        let mut moods = vec![None];
        moods.extend(Mood::MARKED.iter().take(self.moods.len()).map(|m| Some(*m)));
        let mut out = Vec::new();
        for &aspect in &aspects {
            for tense in [Tense::NonPast, Tense::Past] {
                for &mood in &moods {
                    for polarity in [Polarity::Positive, Polarity::Negative] {
                        out.push(VerbForm {
                            aspect,
                            tense,
                            mood,
                            polarity,
                        });
                    }
                }
            }
        }
        out
    }

    /// Whether every affix is distinct and every combination in a paradigm
    /// spells out differently (particles aside: they are separate words).
    pub fn affixes_are_distinct(&self) -> bool {
        let forms = self.affixes();
        let set: BTreeSet<&Phonemes> = forms.iter().copied().collect();
        if set.len() != forms.len() {
            return false;
        }
        // Within each word class, cells with different attached affixes must
        // spell differently. Cells that differ only by a particle may share a
        // word form: the particle word tells them apart.
        let dummy: Phonemes = vec![];
        let mut cells: Vec<(Phonemes, String)> = Vec::new();
        for number in [Number::Singular, Number::Plural] {
            for case in Case::ALL {
                let m = self.noun(&dummy, "", number, case);
                cells.push((join(&m), format!("n:{}", gloss_labels(&m))));
            }
        }
        for f in self.verb_forms() {
            let m = self.verb_form(&dummy, "", f);
            cells.push((join(&m), format!("v:{}", gloss_labels(&m))));
        }
        let mut seen: std::collections::BTreeMap<(char, Phonemes), String> = Default::default();
        for (form, label) in cells {
            let class = label.chars().next().expect("class");
            if let Some(prev) = seen.insert((class, form), label.clone()) {
                if prev != label {
                    return false;
                }
            }
        }
        true
    }

    fn case_affix(&self, case: Case) -> Option<&Affix> {
        match case {
            Case::Subject => None,
            Case::Object => Some(&self.object),
            Case::Genitive => Some(&self.genitive),
            Case::Dative => Some(&self.dative),
        }
    }

    fn noun_markers(&self, number: Number, case: Case) -> Vec<&Affix> {
        let mut inner = Vec::new();
        if number == Number::Plural {
            inner.push(&self.plural);
        }
        inner.extend(self.case_affix(case));
        inner
    }

    /// Verb markers from the root outward: aspect, tense, mood, negation.
    fn verb_markers(&self, f: VerbForm) -> Vec<&Affix> {
        let mut inner = Vec::new();
        if f.aspect != Aspect::Simple && f.aspect == self.aspect {
            inner.push(&self.aspect_affix);
        }
        if f.tense == Tense::Past {
            inner.push(&self.past);
        }
        if let Some(i) = f
            .mood
            .and_then(|m| Mood::MARKED.iter().position(|x| *x == m))
        {
            if let Some(a) = self.moods.get(i) {
                inner.push(a);
            }
        }
        if f.polarity == Polarity::Negative {
            inner.push(&self.negative);
        }
        inner
    }

    /// Inflects a noun. Number sits next to the root, case outside it.
    pub fn noun(&self, root: &Phonemes, gloss: &str, number: Number, case: Case) -> Vec<Morph> {
        let inner = self.fuse(self.noun_markers(number, case));
        assemble(root, gloss, &inner, self.noun_position)
    }

    /// Inflects a verb. Tense sits next to the root, negation outside it.
    pub fn verb(
        &self,
        root: &Phonemes,
        gloss: &str,
        tense: Tense,
        polarity: Polarity,
    ) -> Vec<Morph> {
        self.verb_form(root, gloss, VerbForm::plain(tense, polarity))
    }

    /// Inflects a verb in any form (D07): aspect next to the root, then
    /// tense, mood and negation.
    pub fn verb_form(&self, root: &Phonemes, gloss: &str, f: VerbForm) -> Vec<Morph> {
        let inner = self.fuse(self.verb_markers(f));
        assemble(root, gloss, &inner, self.verb_position)
    }

    /// Particle words that go with a noun form, nearest first.
    pub fn noun_particles(&self, number: Number, case: Case) -> Vec<Morph> {
        particle_morphs(self.noun_markers(number, case))
    }

    /// Particle words that go with a verb form, nearest first.
    pub fn verb_particles(&self, f: VerbForm) -> Vec<Morph> {
        particle_morphs(self.verb_markers(f))
    }

    /// Drops particles and replaces fused combinations with their form.
    /// Returns (form, gloss label) pairs from the root outward.
    fn fuse(&self, markers: Vec<&Affix>) -> Vec<(Phonemes, String)> {
        let attached: Vec<&Affix> = markers.into_iter().filter(|a| !a.particle).collect();
        let glosses: Vec<&str> = attached.iter().map(|a| a.gloss).collect();
        if let Some(f) = self.fusions.iter().find(|f| f.glosses == glosses) {
            return vec![(f.form.clone(), f.glosses.join("."))];
        }
        attached
            .into_iter()
            .map(|a| (a.form.clone(), a.gloss.to_string()))
            .collect()
    }

    /// Every inflected form of a root, as morphs.
    pub fn paradigm(&self, root: &Phonemes, gloss: &str, pos: Pos) -> Vec<Vec<Morph>> {
        let mut out = Vec::new();
        match pos {
            Pos::Noun => {
                for number in [Number::Singular, Number::Plural] {
                    for case in Case::ALL {
                        out.push(self.noun(root, gloss, number, case));
                    }
                }
            }
            Pos::Verb => {
                for f in self.verb_forms() {
                    out.push(self.verb_form(root, gloss, f));
                }
            }
            _ => out.push(assemble(root, gloss, &[], AffixPosition::Suffix)),
        }
        out
    }

    /// Every surface form a root can take, its derived words' forms
    /// included (D07), for collision checks.
    pub fn all_forms(&self, root: &Phonemes, pos: Pos) -> Vec<Phonemes> {
        let mut out: Vec<Phonemes> = self
            .paradigm(root, "", pos)
            .iter()
            .map(|m| join(m))
            .collect();
        if !self.derivations.is_empty() {
            for (gloss, from, to) in DERIVATIONS {
                if from == pos {
                    let d = self.derive(root, gloss);
                    out.extend(self.paradigm(&d, "", to).iter().map(|m| join(m)));
                }
            }
        }
        out
    }
}

fn position(rng: &mut Rng) -> AffixPosition {
    // Suffixing is far more common in the world's languages.
    rng.weighted(&[(AffixPosition::Suffix, 70), (AffixPosition::Prefix, 30)])
}

fn particle_morphs(markers: Vec<&Affix>) -> Vec<Morph> {
    markers
        .into_iter()
        .filter(|a| a.particle)
        .map(|a| Morph {
            form: a.form.clone(),
            gloss: a.gloss.to_string(),
            is_root: false,
        })
        .collect()
}

/// Places affixes around a root. `inner` lists (form, gloss) pairs from the
/// root outward, so prefixing languages mirror suffixing ones.
fn assemble(
    root: &Phonemes,
    gloss: &str,
    inner: &[(Phonemes, String)],
    pos: AffixPosition,
) -> Vec<Morph> {
    let root = Morph {
        form: root.clone(),
        gloss: gloss.to_string(),
        is_root: true,
    };
    let affixes = inner.iter().map(|(form, gloss)| Morph {
        form: form.clone(),
        gloss: gloss.clone(),
        is_root: false,
    });
    match pos {
        AffixPosition::Suffix => std::iter::once(root).chain(affixes).collect(),
        AffixPosition::Prefix => {
            let mut v: Vec<Morph> = affixes.collect();
            v.reverse();
            v.push(root);
            v
        }
    }
}

/// The affix labels of a word, root excluded.
fn gloss_labels(morphs: &[Morph]) -> String {
    morphs
        .iter()
        .filter(|m| !m.is_root)
        .map(|m| m.gloss.as_str())
        .collect::<Vec<_>>()
        .join("-")
}

/// Concatenates morphs into one phoneme sequence.
pub fn join(morphs: &[Morph]) -> Phonemes {
    morphs.iter().flat_map(|m| m.form.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affixes_never_collide() {
        for seed in 0..100 {
            let p = Phonology::generate(seed);
            let m = Morphology::generate(seed, &p, Regularity::Regular);
            assert!(m.affixes_are_distinct(), "seed {seed}");
            for a in m.affixes() {
                assert!(p.is_valid(a), "seed {seed}");
            }
        }
    }

    #[test]
    fn suffix_order_is_root_number_case() {
        let p = Phonology::generate(3);
        let mut m = Morphology::generate(3, &p, Regularity::Regular);
        m.noun_position = AffixPosition::Suffix;
        let root = vec![0, p.inventory.vowels().next().unwrap()];
        let glosses: Vec<String> = m
            .noun(&root, "gate", Number::Plural, Case::Object)
            .into_iter()
            .map(|x| x.gloss)
            .collect();
        assert_eq!(glosses, ["gate", "PL", "ACC"]);
        m.noun_position = AffixPosition::Prefix;
        let glosses: Vec<String> = m
            .noun(&root, "gate", Number::Plural, Case::Object)
            .into_iter()
            .map(|x| x.gloss)
            .collect();
        assert_eq!(glosses, ["ACC", "PL", "gate"]);
    }

    #[test]
    fn unmarked_forms_are_bare_roots() {
        let p = Phonology::generate(5);
        let m = Morphology::generate(5, &p, Regularity::Regular);
        let root = vec![0, p.inventory.vowels().next().unwrap()];
        assert_eq!(
            join(&m.noun(&root, "x", Number::Singular, Case::Subject)),
            root
        );
        assert_eq!(
            join(&m.verb(&root, "x", Tense::NonPast, Polarity::Positive)),
            root
        );
    }
}
