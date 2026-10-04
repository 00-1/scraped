//! The writing system: glyphs built from strokes, never shown, only described.
//!
//! Each glyph is a small set of marks (a stroke, its orientation and its
//! position in the glyph square). Players draw glyphs from descriptions, so
//! the structure must be drawable and distinct. A seed's "script logic" can
//! make related sounds share marks (a voiced stop is its voiceless partner
//! plus a dot), which is a rule a reader can discover.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Serialize, Serializer};

use crate::numerals::Numerals;
use crate::phonology::{Manner, Phoneme, Phonology};
use crate::rng::Rng;

/// What kind of writing system a language uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptKind {
    /// One glyph per sound.
    Alphabet,
    /// Consonants only; a carrier sign marks a word-initial vowel.
    // DESIGN-Q: real abjads vary in how (or whether) they write initial
    // vowels. One carrier sign is the simplest rule a reader can learn.
    Abjad,
    /// One glyph per consonant+vowel pair, built from a consonant shape and a
    /// vowel mark; a "dead" mark writes a consonant with no vowel.
    Syllabary,
}

/// Reading direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    /// Alternate lines run in opposite directions, first line left to right.
    // DESIGN-Q: historical boustrophedon also mirrors the glyphs on reversed
    // lines. Not done here; the glyph shapes stay the same.
    Boustrophedon,
}

/// The stroke vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stroke {
    Bar,
    Hook,
    Dot,
    Ring,
    Arc,
    Wedge,
    Tail,
    Cross,
    Zigzag,
}

const STROKES: [Stroke; 9] = [
    Stroke::Bar,
    Stroke::Hook,
    Stroke::Dot,
    Stroke::Ring,
    Stroke::Arc,
    Stroke::Wedge,
    Stroke::Tail,
    Stroke::Cross,
    Stroke::Zigzag,
];

/// Which way a stroke points or opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Turn {
    Up,
    Right,
    Down,
    Left,
}

const TURNS: [Turn; 4] = [Turn::Up, Turn::Right, Turn::Down, Turn::Left];

/// Where in the glyph square a mark sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Spot {
    Centre,
    Top,
    Bottom,
    Left,
    Right,
}

const SPOTS: [Spot; 5] = [
    Spot::Centre,
    Spot::Top,
    Spot::Bottom,
    Spot::Left,
    Spot::Right,
];

/// One stroke placed in a glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Mark {
    pub stroke: Stroke,
    pub turn: Turn,
    pub spot: Spot,
}

impl Mark {
    /// Symmetric strokes look the same whichever way they turn, so their
    /// turn is normalised: two marks are equal exactly when they look equal.
    pub fn new(stroke: Stroke, turn: Turn, spot: Spot) -> Self {
        let turn = match (stroke, turn) {
            (Stroke::Dot | Stroke::Ring | Stroke::Cross, _) => Turn::Up,
            (Stroke::Bar, Turn::Down) => Turn::Up,
            (Stroke::Bar, Turn::Left) => Turn::Right,
            // A zigzag looks the same turned half about (S02).
            (Stroke::Zigzag, Turn::Down) => Turn::Up,
            (Stroke::Zigzag, Turn::Left) => Turn::Right,
            (_, t) => t,
        };
        Mark { stroke, turn, spot }
    }

    fn random(rng: &mut Rng) -> Self {
        Mark::new(*rng.pick(&STROKES), *rng.pick(&TURNS), *rng.pick(&SPOTS))
    }

    /// Mechanical description, for debug output only. Player-facing glyph
    /// descriptions are content slots (M03).
    pub fn describe(&self) -> String {
        let stroke = match self.stroke {
            Stroke::Bar => "bar",
            Stroke::Hook => "hook",
            Stroke::Dot => "dot",
            Stroke::Ring => "ring",
            Stroke::Arc => "arc",
            Stroke::Wedge => "wedge",
            Stroke::Tail => "tail",
            Stroke::Cross => "cross",
            Stroke::Zigzag => "zigzag",
        };
        let turn = match (self.stroke, self.turn) {
            (Stroke::Dot | Stroke::Ring | Stroke::Cross, _) => String::new(),
            (Stroke::Bar, Turn::Up) => " upright".to_string(),
            (Stroke::Bar, _) => " level".to_string(),
            (_, t) => format!(
                " {}",
                serde_json::to_value(t)
                    .expect("enum")
                    .as_str()
                    .unwrap_or("")
            ),
        };
        let spot = match self.spot {
            Spot::Centre => "in the middle",
            Spot::Top => "at the top",
            Spot::Bottom => "at the bottom",
            Spot::Left => "on the left",
            Spot::Right => "on the right",
        };
        format!("{stroke}{turn} {spot}")
    }
}

/// A glyph: a set of marks, kept sorted so equal shapes compare equal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Glyph {
    pub marks: Vec<Mark>,
}

impl Glyph {
    fn from_marks(mut marks: Vec<Mark>) -> Self {
        marks.sort();
        marks.dedup();
        Glyph { marks }
    }

    fn with(&self, extra: Option<Mark>) -> Self {
        let mut marks = self.marks.clone();
        marks.extend(extra);
        Glyph::from_marks(marks)
    }

    fn random(rng: &mut Rng) -> Self {
        let n = rng.weighted(&[(1, 25), (2, 50), (3, 25)]);
        Glyph::from_marks((0..n).map(|_| Mark::random(rng)).collect())
    }

    /// Mechanical description, for debug output only.
    pub fn describe(&self) -> String {
        self.marks
            .iter()
            .map(Mark::describe)
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// An SVG drawing of the glyph, for debugging and the authoring tool.
    /// Never used in play: the player only ever gets descriptions.
    pub fn svg(&self, size: u32) -> String {
        let mut body = String::new();
        for m in &self.marks {
            let (x, y, w) = match m.spot {
                Spot::Centre => (30.0, 30.0, 40.0),
                Spot::Top => (30.0, 4.0, 40.0),
                Spot::Bottom => (30.0, 66.0, 30.0),
                Spot::Left => (4.0, 30.0, 30.0),
                Spot::Right => (66.0, 30.0, 30.0),
            };
            let w = if matches!(m.spot, Spot::Top) { 30.0 } else { w };
            let x = if matches!(m.spot, Spot::Top | Spot::Bottom) {
                35.0
            } else {
                x
            };
            let rot = match m.turn {
                Turn::Up => 0,
                Turn::Right => 90,
                Turn::Down => 180,
                Turn::Left => 270,
            };
            let shape = match m.stroke {
                Stroke::Bar => "<path d='M50 5 L50 95'/>",
                Stroke::Hook => "<path d='M50 95 L50 30 Q50 5 80 12'/>",
                Stroke::Dot => "<circle cx='50' cy='50' r='13' class='fill'/>",
                Stroke::Ring => "<circle cx='50' cy='50' r='34'/>",
                Stroke::Arc => "<path d='M10 80 Q50 -10 90 80'/>",
                Stroke::Wedge => "<path d='M15 10 L85 50 L15 90'/>",
                Stroke::Tail => "<path d='M50 5 L50 60 Q50 95 15 88'/>",
                Stroke::Cross => "<path d='M50 8 L50 92 M8 50 L92 50'/>",
                Stroke::Zigzag => "<path d='M15 15 L85 38 L15 62 L85 85'/>",
            };
            let s = w / 100.0;
            body.push_str(&format!(
                "<g transform='translate({x} {y}) scale({s}) rotate({rot} 50 50)'>{shape}</g>"
            ));
        }
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100' width='{size}' height='{size}' \
             fill='none' stroke='currentColor' stroke-width='9' stroke-linecap='round' stroke-linejoin='round'>\
             <style>.fill{{fill:currentColor;stroke:none}}</style>{body}</svg>"
        )
    }
}

/// What a glyph writes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GlyphKey {
    /// A sound (alphabet letters, abjad consonants).
    Sound(&'static str),
    /// The abjad's initial-vowel carrier.
    Carrier,
    /// A syllabary sign: optional consonant plus vowel.
    Syllable(Option<&'static str>, &'static str),
    /// A syllabary consonant with no vowel.
    Dead(&'static str),
    Numeral(u16),
    /// Word divider.
    Divider,
    /// Name determinative.
    Determinative,
}

impl GlyphKey {
    /// Short label in IPA, for spoiler tables.
    pub fn label(&self) -> String {
        match self {
            GlyphKey::Sound(s) => s.to_string(),
            GlyphKey::Carrier => "(vowel carrier)".to_string(),
            GlyphKey::Syllable(c, v) => format!("{}{v}", c.unwrap_or("")),
            GlyphKey::Dead(c) => format!("{c} (no vowel)"),
            GlyphKey::Numeral(n) => format!("numeral {n}"),
            GlyphKey::Divider => "(word divider)".to_string(),
            GlyphKey::Determinative => "(name marker)".to_string(),
        }
    }
}

impl Serialize for GlyphKey {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.label())
    }
}

/// Which sound features the script marks with a shared stroke.
#[derive(Debug, Clone, Serialize)]
pub struct Logic {
    /// Voiced consonant = voiceless partner + this mark.
    pub voicing: Option<Mark>,
    /// Fricative = stop at the same place + this mark.
    pub fricative: Option<Mark>,
    /// Nasal = stop at the same place + this mark.
    pub nasal: Option<Mark>,
}

/// A complete writing system for one era of a language.
#[derive(Debug, Clone, Serialize)]
pub struct Script {
    pub kind: ScriptKind,
    pub direction: Direction,
    pub logic: Logic,
    /// Syllabary vowel marks; the inherent vowel has none.
    pub vowel_marks: BTreeMap<&'static str, Option<Mark>>,
    /// Syllabary mark for a consonant with no vowel.
    pub dead: Option<Mark>,
    /// Shapes of glyphs not derived from others, keyed by what they write.
    #[serde(skip)]
    bases: BTreeMap<GlyphKey, Glyph>,
    /// Every glyph, in table order.
    pub glyphs: Vec<(GlyphKey, Glyph)>,
}

/// A random mark different from all of `avoid`.
fn fresh_mark(rng: &mut Rng, avoid: &[Mark]) -> Mark {
    loop {
        let m = Mark::random(rng);
        if !avoid.contains(&m) {
            return m;
        }
    }
}

/// Largest syllabary chosen by seed; bigger inventories get an alphabet.
const MAX_SYLLABARY: usize = 100;

impl Script {
    /// Generates a script for a language's first era.
    pub fn generate(
        rng: &mut Rng,
        phonology: &Phonology,
        numerals: &Numerals,
        forced: Option<ScriptKind>,
    ) -> Self {
        let consonants = phonology.inventory.consonants().count();
        let vowels = phonology.inventory.vowels().count();
        let kind = forced.unwrap_or_else(|| {
            let k = rng.weighted(&[
                (ScriptKind::Alphabet, 50),
                (ScriptKind::Abjad, 25),
                (ScriptKind::Syllabary, 25),
            ]);
            if k == ScriptKind::Syllabary && (consonants + 1) * vowels + consonants > MAX_SYLLABARY
            {
                ScriptKind::Alphabet
            } else {
                k
            }
        });
        let direction = rng.weighted(&[
            (Direction::LeftToRight, 45),
            (Direction::RightToLeft, 35),
            (Direction::Boustrophedon, 20),
        ]);
        // Feature marks must differ from each other, or two derived glyphs
        // (b = p + mark, f = p + mark) would always coincide.
        let mut used: Vec<Mark> = Vec::new();
        let mut feature = |chance| {
            rng.chance(chance).then(|| {
                let m = fresh_mark(rng, &used);
                used.push(m);
                m
            })
        };
        let logic = Logic {
            voicing: feature(60),
            fricative: feature(40),
            nasal: feature(30),
        };
        let mut script = Script {
            kind,
            direction,
            logic,
            vowel_marks: BTreeMap::new(),
            dead: None,
            bases: BTreeMap::new(),
            glyphs: Vec::new(),
        };
        script.fit(rng, phonology, numerals, &BTreeMap::new());
        script
    }

    /// The script of the next era: same logic and direction, some glyphs
    /// simplified (a mark dropped), new glyphs for new sounds.
    pub fn evolve(&self, rng: &mut Rng, phonology: &Phonology, numerals: &Numerals) -> Self {
        let mut inherited = BTreeMap::new();
        for (key, glyph) in &self.bases {
            let mut g = glyph.clone();
            if g.marks.len() >= 2 && rng.chance(30) {
                let i = rng.index(g.marks.len());
                g.marks.remove(i);
            }
            inherited.insert(key.clone(), g);
        }
        let mut next = Script {
            bases: BTreeMap::new(),
            glyphs: Vec::new(),
            ..self.clone()
        };
        next.vowel_marks
            .retain(|v, _| phonology.inventory.by_ipa(v).is_some());
        if !next.fit(rng, phonology, numerals, &inherited) {
            // Simplification made two glyphs identical: keep the old shapes.
            next.fit(rng, phonology, numerals, &self.bases);
        }
        next
    }

    /// Chooses missing shapes and builds the glyph table, retrying until all
    /// glyphs are distinct. Shapes in `keep` are reused where they still
    /// fit. Returns false if `keep` itself could not be made to fit.
    fn fit(
        &mut self,
        rng: &mut Rng,
        phonology: &Phonology,
        numerals: &Numerals,
        keep: &BTreeMap<GlyphKey, Glyph>,
    ) -> bool {
        let sounds: Vec<&Phoneme> = phonology.inventory.phonemes.iter().collect();
        if self.kind == ScriptKind::Syllabary {
            let vowels: Vec<&'static str> = sounds
                .iter()
                .filter(|p| p.is_vowel())
                .map(|p| p.ipa)
                .collect();
            let inherent = if vowels.contains(&"a") {
                "a"
            } else {
                vowels[0]
            };
            self.vowel_marks.entry(inherent).or_insert(None);
            if self.dead.is_none() {
                self.dead = Some(fresh_mark(rng, &self.fixed_marks()));
            }
        }
        // Vowel marks that already exist are kept like inherited shapes.
        let fixed_marks = self.vowel_marks.clone();
        for attempt in 0..400 {
            let keep_now = attempt < 200;
            let mut bases = BTreeMap::new();
            for key in self.root_keys(&sounds, numerals) {
                let g = match keep.get(&key) {
                    Some(g) if keep_now => g.clone(),
                    _ => Glyph::random(rng),
                };
                bases.insert(key, g);
            }
            if self.kind == ScriptKind::Syllabary {
                let mut taken = self.fixed_marks();
                for p in sounds.iter().filter(|p| p.is_vowel()) {
                    let mark = match fixed_marks.get(p.ipa) {
                        Some(None) => None,
                        Some(Some(m)) if keep_now => Some(*m),
                        _ => Some(fresh_mark(rng, &taken)),
                    };
                    taken.extend(mark);
                    self.vowel_marks.insert(p.ipa, mark);
                }
            }
            self.bases = bases;
            self.glyphs = self.build(&sounds, numerals);
            let distinct: BTreeSet<&Glyph> = self.glyphs.iter().map(|(_, g)| g).collect();
            if distinct.len() == self.glyphs.len()
                && self.glyphs.iter().all(|(_, g)| !g.marks.is_empty())
            {
                return keep_now || keep.is_empty();
            }
        }
        panic!("could not build a script with distinct glyphs");
    }

    /// Logic and dead-consonant marks, which other marks must avoid.
    fn fixed_marks(&self) -> Vec<Mark> {
        [
            self.logic.voicing,
            self.logic.fricative,
            self.logic.nasal,
            self.dead,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Glyphs whose shape is chosen freely rather than derived.
    fn root_keys(&self, sounds: &[&Phoneme], numerals: &Numerals) -> Vec<GlyphKey> {
        let mut keys = Vec::new();
        for p in sounds {
            if p.is_vowel() {
                if self.kind == ScriptKind::Alphabet {
                    keys.push(GlyphKey::Sound(p.ipa));
                }
            } else if self.parent(p, sounds).is_none() {
                keys.push(GlyphKey::Sound(p.ipa));
            }
        }
        if self.kind != ScriptKind::Alphabet {
            keys.push(GlyphKey::Carrier);
        }
        keys.extend(numerals.sign_values().into_iter().map(GlyphKey::Numeral));
        keys.push(GlyphKey::Divider);
        keys.push(GlyphKey::Determinative);
        keys
    }

    /// The sound a consonant's glyph is derived from, and the added mark.
    fn parent(&self, p: &Phoneme, sounds: &[&Phoneme]) -> Option<(&'static str, Mark)> {
        let find = |f: &dyn Fn(&Phoneme) -> bool| {
            sounds.iter().find(|q| !q.is_vowel() && f(q)).map(|q| q.ipa)
        };
        let manner = p.manner()?;
        if let Some(m) = self.logic.voicing {
            if p.voiced() && matches!(manner, Manner::Stop | Manner::Fricative | Manner::Affricate)
            {
                if let Some(t) =
                    find(&|q| q.place() == p.place() && q.manner() == Some(manner) && !q.voiced())
                {
                    return Some((t, m));
                }
            }
        }
        if let Some(m) = self.logic.fricative {
            if manner == Manner::Fricative {
                if let Some(t) = find(&|q| {
                    q.place() == p.place()
                        && q.manner() == Some(Manner::Stop)
                        && q.voiced() == p.voiced()
                }) {
                    return Some((t, m));
                }
            }
        }
        if let Some(m) = self.logic.nasal {
            if manner == Manner::Nasal {
                if let Some(t) = find(&|q| {
                    q.place() == p.place() && q.manner() == Some(Manner::Stop) && !q.voiced()
                }) {
                    return Some((t, m));
                }
            }
        }
        None
    }

    fn consonant_glyph(&self, ipa: &'static str, sounds: &[&Phoneme]) -> Glyph {
        let p = sounds.iter().find(|p| p.ipa == ipa).expect("known sound");
        match self.parent(p, sounds) {
            Some((parent, mark)) => self.consonant_glyph(parent, sounds).with(Some(mark)),
            None => self.bases[&GlyphKey::Sound(ipa)].clone(),
        }
    }

    fn build(&self, sounds: &[&Phoneme], numerals: &Numerals) -> Vec<(GlyphKey, Glyph)> {
        let mut out = Vec::new();
        let consonants: Vec<&'static str> = sounds
            .iter()
            .filter(|p| !p.is_vowel())
            .map(|p| p.ipa)
            .collect();
        let vowels: Vec<&'static str> = sounds
            .iter()
            .filter(|p| p.is_vowel())
            .map(|p| p.ipa)
            .collect();
        match self.kind {
            ScriptKind::Alphabet | ScriptKind::Abjad => {
                for &c in &consonants {
                    out.push((GlyphKey::Sound(c), self.consonant_glyph(c, sounds)));
                }
                if self.kind == ScriptKind::Alphabet {
                    for &v in &vowels {
                        out.push((GlyphKey::Sound(v), self.bases[&GlyphKey::Sound(v)].clone()));
                    }
                } else {
                    out.push((GlyphKey::Carrier, self.bases[&GlyphKey::Carrier].clone()));
                }
            }
            ScriptKind::Syllabary => {
                for &v in &vowels {
                    let carrier = self.bases[&GlyphKey::Carrier].with(self.vowel_marks[v]);
                    out.push((GlyphKey::Syllable(None, v), carrier));
                }
                for &c in &consonants {
                    let base = self.consonant_glyph(c, sounds);
                    for &v in &vowels {
                        out.push((
                            GlyphKey::Syllable(Some(c), v),
                            base.with(self.vowel_marks[v]),
                        ));
                    }
                    out.push((GlyphKey::Dead(c), base.with(self.dead)));
                }
            }
        }
        for v in numerals.sign_values() {
            out.push((
                GlyphKey::Numeral(v),
                self.bases[&GlyphKey::Numeral(v)].clone(),
            ));
        }
        out.push((GlyphKey::Divider, self.bases[&GlyphKey::Divider].clone()));
        out.push((
            GlyphKey::Determinative,
            self.bases[&GlyphKey::Determinative].clone(),
        ));
        out
    }

    /// The glyphs that write one word, given as IPA.
    pub fn spell(&self, word: &[&'static str]) -> Vec<GlyphKey> {
        let vowel = |s: &str| crate::phonology::phoneme(s).is_vowel();
        let mut out = Vec::new();
        match self.kind {
            ScriptKind::Alphabet => out.extend(word.iter().map(|s| GlyphKey::Sound(s))),
            ScriptKind::Abjad => {
                if word.first().is_some_and(|s| vowel(s)) {
                    out.push(GlyphKey::Carrier);
                }
                out.extend(
                    word.iter()
                        .filter(|s| !vowel(s))
                        .map(|s| GlyphKey::Sound(s)),
                );
            }
            ScriptKind::Syllabary => {
                let mut i = 0;
                while i < word.len() {
                    let s = word[i];
                    if vowel(s) {
                        out.push(GlyphKey::Syllable(None, s));
                        i += 1;
                    } else if i + 1 < word.len() && vowel(word[i + 1]) {
                        out.push(GlyphKey::Syllable(Some(s), word[i + 1]));
                        i += 2;
                    } else {
                        out.push(GlyphKey::Dead(s));
                        i += 1;
                    }
                }
            }
        }
        out
    }

    /// Position of a glyph in the table, used as its reference number.
    pub fn index(&self, key: &GlyphKey) -> usize {
        self.glyphs
            .iter()
            .position(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("no glyph for {key:?}"))
    }

    /// Lays a stream of words out in lines of at most `width` glyphs, in
    /// physical left-to-right order as they would sit on the surface.
    /// `None` entries are gaps between words.
    pub fn layout(&self, tokens: &[Option<GlyphKey>], width: usize) -> Vec<Vec<Option<GlyphKey>>> {
        let mut lines: Vec<Vec<Option<GlyphKey>>> = Vec::new();
        let mut line = Vec::new();
        for t in tokens {
            if line.len() >= width && t.is_none() || line.len() >= width * 2 {
                lines.push(std::mem::take(&mut line));
                if t.is_none() {
                    continue;
                }
            }
            line.push(t.clone());
        }
        if !line.is_empty() {
            lines.push(line);
        }
        for (i, l) in lines.iter_mut().enumerate() {
            let reverse = match self.direction {
                Direction::LeftToRight => false,
                Direction::RightToLeft => true,
                Direction::Boustrophedon => i % 2 == 1,
            };
            if reverse {
                l.reverse();
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Stream;

    fn script(seed: u64, kind: Option<ScriptKind>) -> (Phonology, Script) {
        let p = Phonology::generate(seed);
        let n = Numerals::generate(seed);
        let mut rng = Rng::new(seed, Stream::Script);
        let s = Script::generate(&mut rng, &p, &n, kind);
        (p, s)
    }

    #[test]
    fn glyphs_are_distinct_for_every_kind() {
        for seed in 0..40 {
            for kind in [
                ScriptKind::Alphabet,
                ScriptKind::Abjad,
                ScriptKind::Syllabary,
            ] {
                let (_, s) = script(seed, Some(kind));
                let set: BTreeSet<&Glyph> = s.glyphs.iter().map(|(_, g)| g).collect();
                assert_eq!(set.len(), s.glyphs.len(), "seed {seed} {kind:?}");
            }
        }
    }

    #[test]
    fn voicing_logic_adds_one_mark() {
        for seed in 0..60 {
            let (p, s) = script(seed, Some(ScriptKind::Alphabet));
            let (Some(m), Some(_), Some(_)) = (
                s.logic.voicing,
                p.inventory.by_ipa("p"),
                p.inventory.by_ipa("b"),
            ) else {
                continue;
            };
            let g = |k| s.glyphs[s.index(&GlyphKey::Sound(k))].1.clone();
            assert_eq!(g("b"), g("p").with(Some(m)), "seed {seed}");
            return;
        }
        panic!("no seed with voicing logic and p/b");
    }

    #[test]
    fn spelling_covers_every_sound() {
        for seed in 0..20 {
            for kind in [
                ScriptKind::Alphabet,
                ScriptKind::Abjad,
                ScriptKind::Syllabary,
            ] {
                let (p, s) = script(seed, Some(kind));
                let mut rng = Rng::new(seed, Stream::Lexicon);
                for _ in 0..30 {
                    let w = p.random_word(&mut rng, 3);
                    for key in s.spell(&p.to_ipa(&w)) {
                        s.index(&key);
                    }
                }
            }
        }
    }

    #[test]
    fn abjad_drops_inner_vowels() {
        let (_, mut s) = script(1, Some(ScriptKind::Abjad));
        s.kind = ScriptKind::Abjad;
        assert_eq!(
            s.spell(&["a", "k", "a", "n"]),
            [
                GlyphKey::Carrier,
                GlyphKey::Sound("k"),
                GlyphKey::Sound("n")
            ]
        );
    }

    #[test]
    fn layout_reverses_lines_by_direction() {
        let (_, mut s) = script(2, None);
        let toks: Vec<Option<GlyphKey>> =
            vec![Some(GlyphKey::Divider), Some(GlyphKey::Determinative)];
        s.direction = Direction::RightToLeft;
        assert_eq!(s.layout(&toks, 10)[0][0], Some(GlyphKey::Determinative));
        s.direction = Direction::LeftToRight;
        assert_eq!(s.layout(&toks, 10)[0][0], Some(GlyphKey::Divider));
    }

    #[test]
    fn svg_is_well_formed() {
        let (_, s) = script(3, None);
        for (_, g) in &s.glyphs {
            let svg = g.svg(32);
            assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        }
    }
}
