//! Sound inventory, syllable structure and romanisation.
//!
//! A language's sounds are drawn from a weighted pool of cross-linguistically
//! common phonemes with simple implicational rules (no /b/ without /p/), so
//! inventories look like real ones rather than random letter bags.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::rng::{Rng, Stream};

/// Index into [`Inventory::phonemes`].
pub type PhonemeId = u8;

/// A word as a sequence of phonemes. Romanised only when shown.
pub type Phonemes = Vec<PhonemeId>;

/// How a consonant is produced; drives coda and cluster rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Manner {
    Stop,
    Affricate,
    Fricative,
    Nasal,
    Lateral,
    Rhotic,
    Glide,
}

/// Where in the mouth a consonant is made. Scripts use it to give related
/// sounds related glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    Labial,
    Dental,
    Alveolar,
    Postalveolar,
    Palatal,
    Velar,
    Uvular,
    Glottal,
}

/// Consonant or vowel, with the features rules need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase", tag = "kind")]
pub enum PhonemeKind {
    Consonant {
        place: Place,
        manner: Manner,
        voiced: bool,
    },
    Vowel,
}

/// One phoneme in a generated inventory.
#[derive(Debug, Clone, Serialize)]
pub struct Phoneme {
    /// IPA symbol (spoiler-level information).
    pub ipa: &'static str,
    /// Latin spelling used in all player-facing output.
    pub roman: String,
    #[serde(flatten)]
    pub kind: PhonemeKind,
    /// Relative frequency when building roots.
    pub weight: u32,
}

impl Phoneme {
    pub fn is_vowel(&self) -> bool {
        self.kind == PhonemeKind::Vowel
    }

    pub fn manner(&self) -> Option<Manner> {
        match self.kind {
            PhonemeKind::Consonant { manner, .. } => Some(manner),
            PhonemeKind::Vowel => None,
        }
    }

    pub fn place(&self) -> Option<Place> {
        match self.kind {
            PhonemeKind::Consonant { place, .. } => Some(place),
            PhonemeKind::Vowel => None,
        }
    }

    pub fn voiced(&self) -> bool {
        match self.kind {
            PhonemeKind::Consonant { voiced, .. } => voiced,
            PhonemeKind::Vowel => true,
        }
    }
}

struct ConsonantSpec {
    ipa: &'static str,
    place: Place,
    manner: Manner,
    voiced: bool,
    /// Likelihood of appearing in an inventory, and of use within words.
    weight: u32,
    /// Implicational universal: only present if this phoneme is present.
    requires: Option<&'static str>,
    /// Spellings in order of preference. The last is always a character no
    /// other phoneme uses, so a uniquely decodable choice always exists.
    spellings: &'static [&'static str],
}

const fn c(
    ipa: &'static str,
    place: Place,
    manner: Manner,
    voiced: bool,
    weight: u32,
    requires: Option<&'static str>,
    spellings: &'static [&'static str],
) -> ConsonantSpec {
    ConsonantSpec {
        ipa,
        place,
        manner,
        voiced,
        weight,
        requires,
        spellings,
    }
}

use Manner::*;
use Place::*;

const CONSONANTS: &[ConsonantSpec] = &[
    c("p", Labial, Stop, false, 90, None, &["p"]),
    c("t", Alveolar, Stop, false, 98, None, &["t"]),
    c("k", Velar, Stop, false, 98, None, &["k"]),
    c("b", Labial, Stop, true, 60, Some("p"), &["b"]),
    c("d", Alveolar, Stop, true, 60, Some("t"), &["d"]),
    c("g", Velar, Stop, true, 50, Some("k"), &["g"]),
    c("q", Uvular, Stop, false, 12, Some("k"), &["q"]),
    c("ʔ", Glottal, Stop, false, 35, None, &["'"]),
    c("m", Labial, Nasal, true, 96, None, &["m"]),
    c("n", Alveolar, Nasal, true, 97, None, &["n"]),
    c("ŋ", Velar, Nasal, true, 45, Some("k"), &["ng", "ŋ"]),
    c("ɲ", Palatal, Nasal, true, 25, None, &["ny", "ñ"]),
    c("f", Labial, Fricative, false, 45, None, &["f"]),
    c("v", Labial, Fricative, true, 30, Some("f"), &["v"]),
    c("s", Alveolar, Fricative, false, 88, None, &["s"]),
    c("z", Alveolar, Fricative, true, 30, Some("s"), &["z"]),
    c(
        "ʃ",
        Postalveolar,
        Fricative,
        false,
        45,
        None,
        &["sh", "x", "š"],
    ),
    c(
        "ʒ",
        Postalveolar,
        Fricative,
        true,
        15,
        Some("ʃ"),
        &["zh", "ž"],
    ),
    c("h", Glottal, Fricative, false, 60, None, &["h"]),
    c("x", Velar, Fricative, false, 25, None, &["kh", "x", "ĥ"]),
    c("ɣ", Velar, Fricative, true, 10, Some("x"), &["gh", "ğ"]),
    c("θ", Dental, Fricative, false, 10, None, &["th", "þ"]),
    c("ð", Dental, Fricative, true, 8, Some("θ"), &["dh", "ð"]),
    c(
        "ts",
        Alveolar,
        Affricate,
        false,
        20,
        None,
        &["ts", "c", "ċ"],
    ),
    c(
        "tʃ",
        Postalveolar,
        Affricate,
        false,
        40,
        None,
        &["ch", "c", "č"],
    ),
    c(
        "dʒ",
        Postalveolar,
        Affricate,
        true,
        30,
        Some("tʃ"),
        &["j", "dj", "ǰ"],
    ),
    c("l", Alveolar, Lateral, true, 85, None, &["l"]),
    c("ɬ", Alveolar, Lateral, false, 8, Some("l"), &["lh", "ł"]),
    c("r", Alveolar, Rhotic, true, 70, None, &["r"]),
    c("j", Palatal, Glide, true, 80, None, &["y"]),
    c("w", Labial, Glide, true, 70, None, &["w"]),
];

/// Always present: every language has at least these.
const CORE_CONSONANTS: &[&str] = &["t", "k", "m", "n"];

struct VowelSpec {
    ipa: &'static str,
    weight: u32,
    roman: &'static str,
}

const VOWELS: &[VowelSpec] = &[
    VowelSpec {
        ipa: "a",
        weight: 30,
        roman: "a",
    },
    VowelSpec {
        ipa: "e",
        weight: 15,
        roman: "e",
    },
    VowelSpec {
        ipa: "i",
        weight: 22,
        roman: "i",
    },
    VowelSpec {
        ipa: "o",
        weight: 15,
        roman: "o",
    },
    VowelSpec {
        ipa: "u",
        weight: 18,
        roman: "u",
    },
    VowelSpec {
        ipa: "ə",
        weight: 8,
        roman: "ë",
    },
    VowelSpec {
        ipa: "ɨ",
        weight: 6,
        roman: "ï",
    },
    VowelSpec {
        ipa: "ɛ",
        weight: 8,
        roman: "è",
    },
    VowelSpec {
        ipa: "ɔ",
        weight: 8,
        roman: "ò",
    },
];

/// Attested vowel systems and how often to pick each.
const VOWEL_SYSTEMS: &[(&[&str], u32)] = &[
    (&["a", "i", "u"], 10),
    (&["a", "e", "i", "u"], 4),
    (&["a", "i", "o", "u"], 4),
    (&["a", "i", "u", "ə"], 3),
    (&["a", "e", "i", "o", "u"], 35),
    (&["a", "e", "i", "o", "u", "ə"], 12),
    (&["a", "e", "i", "o", "u", "ɨ"], 6),
    (&["a", "e", "i", "o", "u", "ɛ", "ɔ"], 10),
    (&["a", "e", "i", "o", "u", "ə", "ɨ"], 6),
];

/// The phonemes of one language. Consonants come first, then vowels.
#[derive(Debug, Clone, Serialize)]
pub struct Inventory {
    pub phonemes: Vec<Phoneme>,
}

impl Inventory {
    pub fn consonants(&self) -> impl Iterator<Item = PhonemeId> + '_ {
        self.ids().filter(|&id| !self.get(id).is_vowel())
    }

    pub fn vowels(&self) -> impl Iterator<Item = PhonemeId> + '_ {
        self.ids().filter(|&id| self.get(id).is_vowel())
    }

    pub fn get(&self, id: PhonemeId) -> &Phoneme {
        &self.phonemes[usize::from(id)]
    }

    pub fn by_ipa(&self, ipa: &str) -> Option<PhonemeId> {
        self.ids().find(|&id| self.get(id).ipa == ipa)
    }

    pub fn ids(&self) -> impl Iterator<Item = PhonemeId> {
        (0..self.phonemes.len()).map(|i| i as PhonemeId)
    }
}

/// Syllable shape, e.g. `(C)V(C)`: how many consonants may open and close a
/// syllable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Template {
    pub onset_min: u8,
    pub onset_max: u8,
    pub coda_max: u8,
}

impl Template {
    /// Conventional notation, e.g. `C(C)V(C)`.
    pub fn notation(&self) -> String {
        let mut s = String::new();
        for i in 0..self.onset_max {
            s.push_str(if i < self.onset_min { "C" } else { "(C)" });
        }
        s.push('V');
        for _ in 0..self.coda_max {
            s.push_str("(C)");
        }
        s
    }
}

/// Sounds and the rules for combining them.
#[derive(Debug, Clone, Serialize)]
pub struct Phonology {
    pub inventory: Inventory,
    pub template: Template,
    /// Every permitted syllable onset (single consonants and clusters).
    pub onsets: Vec<Phonemes>,
    /// Consonants permitted to close a syllable.
    pub codas: Vec<PhonemeId>,
}

impl Phonology {
    /// Generates the sound system for `seed`.
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed, Stream::Phonology);
        let consonant_specs = pick_consonants(&mut rng);
        let vowel_system = VOWEL_SYSTEMS
            [rng.weighted_index(&VOWEL_SYSTEMS.iter().map(|&(_, w)| w).collect::<Vec<_>>())]
        .0;

        let mut phonemes: Vec<Phoneme> = consonant_specs.iter().map(|s| phoneme(s.ipa)).collect();
        phonemes.extend(
            VOWELS
                .iter()
                .filter(|v| vowel_system.contains(&v.ipa))
                .map(|v| phoneme(v.ipa)),
        );
        assign_spellings(&mut phonemes);
        let inventory = Inventory { phonemes };

        let template = Template {
            onset_min: if rng.chance(50) { 1 } else { 0 },
            onset_max: if rng.chance(30) { 2 } else { 1 },
            coda_max: if rng.chance(65) { 1 } else { 0 },
        };

        let ban_initial_velar_nasal = rng.chance(70);
        let mut onsets: Vec<Phonemes> = inventory
            .consonants()
            .filter(|&id| !(ban_initial_velar_nasal && inventory.get(id).ipa == "ŋ"))
            .map(|id| vec![id])
            .collect();
        if template.onset_max >= 2 {
            onsets.extend(clusters(&inventory));
        }
        // A template that promises clusters but finds none is simply (C)V.
        let template = Template {
            onset_max: if onsets.iter().any(|o| o.len() == 2) {
                template.onset_max
            } else {
                1
            },
            ..template
        };

        let codas = if template.coda_max > 0 {
            pick_codas(&mut rng, &inventory)
        } else {
            Vec::new()
        };
        // Every sound must be usable somewhere: an /ŋ/ banned from onsets
        // needs to be a possible coda, or the ban is lifted.
        if let Some(ng) = inventory.by_ipa("ŋ") {
            if !codas.contains(&ng) && !onsets.contains(&vec![ng]) {
                onsets.push(vec![ng]);
            }
        }

        Phonology {
            inventory,
            template,
            onsets,
            codas,
        }
    }

    /// Romanises a phoneme sequence.
    pub fn romanise(&self, word: &[PhonemeId]) -> String {
        word.iter()
            .map(|&id| self.inventory.get(id).roman.as_str())
            .collect()
    }

    /// Splits romanised text back into phonemes, if it is spellable at all.
    /// Romanisation is uniquely decodable, so the answer is never ambiguous.
    pub fn decode(&self, text: &str) -> Option<Phonemes> {
        if text.is_empty() {
            return Some(Vec::new());
        }
        self.inventory.ids().find_map(|id| {
            let rest = text.strip_prefix(self.inventory.get(id).roman.as_str())?;
            let mut tail = self.decode(rest)?;
            tail.insert(0, id);
            Some(tail)
        })
    }

    /// Whether `word` can be split into syllables matching the template.
    pub fn is_valid(&self, word: &[PhonemeId]) -> bool {
        let n = word.len();
        let mut reachable = vec![false; n + 1];
        reachable[0] = true;
        for start in 0..n {
            if !reachable[start] {
                continue;
            }
            for end in self.syllable_ends(word, start) {
                reachable[end] = true;
            }
        }
        n > 0 && reachable[n]
    }

    /// All positions where a syllable starting at `start` could end.
    fn syllable_ends(&self, word: &[PhonemeId], start: usize) -> Vec<usize> {
        let mut nuclei = Vec::new();
        if self.template.onset_min == 0 {
            nuclei.push(start);
        }
        for onset in &self.onsets {
            if word[start..].starts_with(onset) {
                nuclei.push(start + onset.len());
            }
        }
        let mut ends = Vec::new();
        for v in nuclei {
            if v < word.len() && self.inventory.get(word[v]).is_vowel() {
                ends.push(v + 1);
                if self.template.coda_max > 0
                    && v + 1 < word.len()
                    && self.codas.contains(&word[v + 1])
                {
                    ends.push(v + 2);
                }
            }
        }
        ends
    }

    /// One random syllable. Word-internal syllables always take an onset so
    /// roots avoid vowel hiatus.
    pub fn random_syllable(&self, rng: &mut Rng, initial: bool) -> Phonemes {
        let mut syl = Vec::new();
        let onset_optional = initial && self.template.onset_min == 0;
        if !(onset_optional && rng.chance(25)) {
            let clusters: Vec<&Phonemes> = self.onsets.iter().filter(|o| o.len() > 1).collect();
            if !clusters.is_empty() && rng.chance(15) {
                syl.extend(rng.pick(&clusters).iter());
            } else {
                let singles: Vec<PhonemeId> = self
                    .onsets
                    .iter()
                    .filter(|o| o.len() == 1)
                    .map(|o| o[0])
                    .collect();
                syl.push(self.weighted_phoneme(rng, &singles));
            }
        }
        let vowels: Vec<PhonemeId> = self.inventory.vowels().collect();
        syl.push(self.weighted_phoneme(rng, &vowels));
        if !self.codas.is_empty() && rng.chance(35) {
            syl.push(self.weighted_phoneme(rng, &self.codas));
        }
        syl
    }

    /// A random word of `syllables` syllables.
    pub fn random_word(&self, rng: &mut Rng, syllables: u32) -> Phonemes {
        (0..syllables)
            .flat_map(|i| self.random_syllable(rng, i == 0))
            .collect()
    }

    /// A short consonant-initial form for affixes: `CV`, or `CVC` when codas
    /// exist. Starting with a consonant means affixes never create hiatus
    /// and always attach without breaking the template.
    pub fn random_affix(&self, rng: &mut Rng) -> Phonemes {
        let singles: Vec<PhonemeId> = self
            .onsets
            .iter()
            .filter(|o| o.len() == 1)
            .map(|o| o[0])
            .collect();
        let vowels: Vec<PhonemeId> = self.inventory.vowels().collect();
        let mut affix = vec![
            self.weighted_phoneme(rng, &singles),
            self.weighted_phoneme(rng, &vowels),
        ];
        if !self.codas.is_empty() && rng.chance(30) {
            affix.push(self.weighted_phoneme(rng, &self.codas));
        }
        affix
    }

    fn weighted_phoneme(&self, rng: &mut Rng, from: &[PhonemeId]) -> PhonemeId {
        let weights: Vec<u32> = from
            .iter()
            .map(|&id| self.inventory.get(id).weight)
            .collect();
        from[rng.weighted_index(&weights)]
    }
}

/// Builds a phoneme from its IPA symbol, using the pool's features. Panics on
/// symbols outside the pool: sound changes only ever target pool sounds.
pub fn phoneme(ipa: &str) -> Phoneme {
    if let Some(s) = CONSONANTS.iter().find(|s| s.ipa == ipa) {
        return Phoneme {
            ipa: s.ipa,
            roman: String::new(),
            kind: PhonemeKind::Consonant {
                place: s.place,
                manner: s.manner,
                voiced: s.voiced,
            },
            weight: s.weight,
        };
    }
    let v = VOWELS
        .iter()
        .find(|v| v.ipa == ipa)
        .unwrap_or_else(|| panic!("unknown phoneme {ipa:?}"));
    Phoneme {
        ipa: v.ipa,
        roman: v.roman.to_string(),
        kind: PhonemeKind::Vowel,
        weight: v.weight,
    }
}

/// Position of a sound in the pool, so inventories always list sounds in
/// the same order whatever era they come from.
fn pool_rank(ipa: &str) -> usize {
    CONSONANTS
        .iter()
        .position(|s| s.ipa == ipa)
        .or_else(|| VOWELS.iter().position(|v| v.ipa == ipa).map(|i| 100 + i))
        .unwrap_or_else(|| panic!("unknown phoneme {ipa:?}"))
}

/// Spellings for a set of sounds, chosen together so that the whole set is
/// uniquely decodable. A language family shares one table across its eras,
/// so a sound is spelled the same way in every era.
pub fn spelling_for(ipas: &BTreeSet<&'static str>) -> BTreeMap<&'static str, String> {
    let mut list: Vec<&'static str> = ipas.iter().copied().collect();
    list.sort_by_key(|i| pool_rank(i));
    let mut phonemes: Vec<Phoneme> = list.iter().map(|i| phoneme(i)).collect();
    assign_spellings(&mut phonemes);
    phonemes.into_iter().map(|p| (p.ipa, p.roman)).collect()
}

/// A phonology described by IPA symbols instead of ids. Sound changes work
/// at this level, since ids are only meaningful within one inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sets {
    pub sounds: BTreeSet<&'static str>,
    pub template: Template,
    pub onsets: BTreeSet<Vec<&'static str>>,
    pub codas: BTreeSet<&'static str>,
}

impl Phonology {
    /// This phonology as IPA sets.
    pub fn sets(&self) -> Sets {
        let ipa = |id: &PhonemeId| self.inventory.get(*id).ipa;
        Sets {
            sounds: self.inventory.phonemes.iter().map(|p| p.ipa).collect(),
            template: self.template,
            onsets: self
                .onsets
                .iter()
                .map(|o| o.iter().map(ipa).collect())
                .collect(),
            codas: self.codas.iter().map(ipa).collect(),
        }
    }

    /// Rebuilds a phonology from IPA sets, spelled with `spelling`.
    pub fn from_sets(sets: &Sets, spelling: &BTreeMap<&'static str, String>) -> Self {
        let mut list: Vec<&'static str> = sets.sounds.iter().copied().collect();
        list.sort_by_key(|i| pool_rank(i));
        let phonemes = list
            .iter()
            .map(|i| {
                let mut p = phoneme(i);
                p.roman = spelling[i].clone();
                p
            })
            .collect();
        let inventory = Inventory { phonemes };
        let id = |i: &&'static str| inventory.by_ipa(i).expect("sound in inventory");
        let mut onsets: Vec<Phonemes> = sets
            .onsets
            .iter()
            .map(|o| o.iter().map(id).collect())
            .collect();
        onsets.sort();
        let mut codas: Vec<PhonemeId> = sets.codas.iter().map(id).collect();
        codas.sort();
        Phonology {
            template: sets.template,
            onsets,
            codas,
            inventory,
        }
    }

    /// Respells every phoneme from a shared table.
    pub fn respell(&mut self, spelling: &BTreeMap<&'static str, String>) {
        for p in &mut self.inventory.phonemes {
            p.roman = spelling[p.ipa].clone();
        }
    }

    /// A word as IPA symbols.
    pub fn to_ipa(&self, word: &[PhonemeId]) -> Vec<&'static str> {
        word.iter().map(|&id| self.inventory.get(id).ipa).collect()
    }

    /// A word from IPA symbols, if every sound is in the inventory.
    pub fn from_ipa(&self, word: &[&str]) -> Option<Phonemes> {
        word.iter().map(|i| self.inventory.by_ipa(i)).collect()
    }
}

fn pick_consonants(rng: &mut Rng) -> Vec<&'static ConsonantSpec> {
    let target = rng.range(12, 22) as usize;
    let mut chosen: BTreeSet<&'static str> = CORE_CONSONANTS.iter().copied().collect();
    // Every language gets at least one liquid.
    chosen.insert(if rng.chance(55) { "l" } else { "r" });
    while chosen.len() < target {
        let eligible: Vec<&ConsonantSpec> = CONSONANTS
            .iter()
            .filter(|s| !chosen.contains(s.ipa))
            .filter(|s| s.requires.is_none_or(|r| chosen.contains(r)))
            .collect();
        if eligible.is_empty() {
            break;
        }
        let weights: Vec<u32> = eligible.iter().map(|s| s.weight).collect();
        chosen.insert(eligible[rng.weighted_index(&weights)].ipa);
    }
    // Keep pool order so inventories always list sounds the same way.
    CONSONANTS
        .iter()
        .filter(|s| chosen.contains(s.ipa))
        .collect()
}

/// Stop + liquid clusters (pl, tr, kl…) and kw/gw, where both sounds exist.
fn clusters(inv: &Inventory) -> Vec<Phonemes> {
    let mut out = Vec::new();
    for first in inv.consonants() {
        let f = inv.get(first);
        if f.manner() != Some(Stop) || f.ipa == "ʔ" || f.ipa == "q" {
            continue;
        }
        for second in inv.consonants() {
            let s = inv.get(second);
            let ok = match s.manner() {
                Some(Lateral) => s.ipa == "l" && !matches!(f.ipa, "t" | "d"),
                Some(Rhotic) => true,
                Some(Glide) => s.ipa == "w" && matches!(f.ipa, "k" | "g"),
                _ => false,
            };
            if ok {
                out.push(vec![first, second]);
            }
        }
    }
    out
}

fn pick_codas(rng: &mut Rng, inv: &Inventory) -> Vec<PhonemeId> {
    #[derive(Clone, Copy)]
    enum CodaSet {
        Nasals,
        NasalsLiquids,
        NasalsLiquidsS,
        MostConsonants,
    }
    let set = rng.weighted(&[
        (CodaSet::Nasals, 25),
        (CodaSet::NasalsLiquids, 30),
        (CodaSet::NasalsLiquidsS, 20),
        (CodaSet::MostConsonants, 25),
    ]);
    inv.consonants()
        .filter(|&id| {
            let p = inv.get(id);
            let manner = p.manner().expect("consonant");
            let nasal = manner == Nasal && p.ipa != "ɲ";
            let liquid = matches!(manner, Lateral | Rhotic);
            match set {
                CodaSet::Nasals => nasal,
                CodaSet::NasalsLiquids => nasal || liquid,
                CodaSet::NasalsLiquidsS => nasal || liquid || p.ipa == "s",
                CodaSet::MostConsonants => manner != Glide && p.ipa != "h",
            }
        })
        .collect()
}

/// Chooses each consonant's spelling so that the whole romanisation stays
/// uniquely decodable: any romanised word splits into sounds exactly one way.
// DESIGN-Q: when a digraph would be ambiguous (n+g vs ng) this falls back to
// a single non-ASCII letter (ŋ, ñ, š, ĥ…). The alternative is ASCII-only
// spelling with a separator like `n'g`, which is noisier but easier to type.
fn assign_spellings(phonemes: &mut [Phoneme]) {
    let spec = |ipa: &str| CONSONANTS.iter().find(|s| s.ipa == ipa).expect("known");
    let mut chosen: Vec<String> = phonemes
        .iter()
        .filter(|p| p.is_vowel())
        .map(|p| p.roman.clone())
        .collect();
    // Single letters first: digraphs must then dodge them, never vice versa.
    let mut order: Vec<usize> = (0..phonemes.len())
        .filter(|&i| !phonemes[i].is_vowel())
        .collect();
    order.sort_by_key(|&i| spec(phonemes[i].ipa).spellings[0].chars().count() > 1);
    for i in order {
        let options = spec(phonemes[i].ipa).spellings;
        let pick = options
            .iter()
            .find(|cand| {
                let mut trial = chosen.clone();
                trial.push(cand.to_string());
                uniquely_decodable(&trial)
            })
            .expect("last spelling is always unique");
        chosen.push(pick.to_string());
        phonemes[i].roman = pick.to_string();
    }
}

/// Sardinas–Patterson test: can every concatenation of `code` words be split
/// back in only one way?
pub fn uniquely_decodable(code: &[String]) -> bool {
    let words: BTreeSet<&str> = code.iter().map(String::as_str).collect();
    if words.len() != code.len() || words.contains("") {
        return false;
    }
    let dangling = |a: &str, b: &str| -> Option<String> {
        (b.len() > a.len() && b.starts_with(a)).then(|| b[a.len()..].to_string())
    };
    let mut current: BTreeSet<String> = BTreeSet::new();
    for a in &words {
        for b in &words {
            if let Some(s) = dangling(a, b) {
                current.insert(s);
            }
        }
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    while !current.is_empty() {
        if current.iter().any(|s| words.contains(s.as_str())) {
            return false;
        }
        let mut next = BTreeSet::new();
        for s in &current {
            for w in &words {
                if let Some(d) = dangling(s, w) {
                    next.insert(d);
                }
                if let Some(d) = dangling(w, s) {
                    next.insert(d);
                }
            }
        }
        seen.extend(current);
        current = next.difference(&seen).cloned().collect();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn sardinas_patterson() {
        assert!(uniquely_decodable(&s(&["a", "b", "sh"])));
        assert!(!uniquely_decodable(&s(&["s", "h", "sh"])));
        assert!(!uniquely_decodable(&s(&["a", "a"])));
        assert!(uniquely_decodable(&s(&["0", "10", "110"])));
        assert!(!uniquely_decodable(&s(&["a", "ab", "bc", "c"])));
    }

    #[test]
    fn inventory_sizes_in_range() {
        for seed in 0..200 {
            let p = Phonology::generate(seed);
            let c = p.inventory.consonants().count();
            let v = p.inventory.vowels().count();
            assert!((12..=22).contains(&c), "seed {seed}: {c} consonants");
            assert!((3..=7).contains(&v), "seed {seed}: {v} vowels");
        }
    }

    #[test]
    fn implicational_rules_hold() {
        for seed in 0..200 {
            let p = Phonology::generate(seed);
            for ph in &p.inventory.phonemes {
                if let Some(spec) = CONSONANTS.iter().find(|s| s.ipa == ph.ipa) {
                    if let Some(req) = spec.requires {
                        assert!(
                            p.inventory.by_ipa(req).is_some(),
                            "seed {seed}: {} without {req}",
                            ph.ipa
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn romanisation_round_trips() {
        for seed in 0..100 {
            let p = Phonology::generate(seed);
            let romans: Vec<String> = p
                .inventory
                .phonemes
                .iter()
                .map(|x| x.roman.clone())
                .collect();
            assert!(uniquely_decodable(&romans), "seed {seed}: {romans:?}");
            let mut rng = Rng::new(seed, Stream::Lexicon);
            for _ in 0..50 {
                let w = p.random_word(&mut rng, 3);
                assert_eq!(p.decode(&p.romanise(&w)), Some(w));
            }
        }
    }

    #[test]
    fn random_words_obey_phonotactics() {
        for seed in 0..100 {
            let p = Phonology::generate(seed);
            let mut rng = Rng::new(seed, Stream::Lexicon);
            for n in 1..4 {
                for _ in 0..30 {
                    let w = p.random_word(&mut rng, n);
                    assert!(p.is_valid(&w), "seed {seed}: {}", p.romanise(&w));
                }
            }
        }
    }

    #[test]
    fn every_phoneme_is_usable() {
        for seed in 0..200 {
            let p = Phonology::generate(seed);
            for c in p.inventory.consonants() {
                let in_onset = p.onsets.iter().any(|o| o.contains(&c));
                assert!(
                    in_onset || p.codas.contains(&c),
                    "seed {seed}: {}",
                    p.inventory.get(c).ipa
                );
            }
        }
    }

    #[test]
    fn validator_rejects_bad_shapes() {
        let p = Phonology::generate(1);
        let v = p.inventory.vowels().next().unwrap();
        let cons: Vec<PhonemeId> = p.inventory.consonants().collect();
        // Three consonants in a row can never be one syllable boundary when
        // codas are at most one and onsets at most two... but four never fit.
        assert!(!p.is_valid(&[cons[0], cons[1], cons[2], cons[3], v]));
        assert!(!p.is_valid(&[]));
        // A lone consonant is never a word.
        assert!(!p.is_valid(&[cons[0]]));
    }
}
