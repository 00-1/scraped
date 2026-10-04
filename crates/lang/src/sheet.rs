//! The full grammar of a language at one era, for spoiler/debug output.

use serde::Serialize;

use crate::concepts::{self, Domain};
use crate::history::Step;
use crate::morphology::AffixPosition;
use crate::numerals::Numerals;
use crate::script::{Direction, Logic, ScriptKind};
use crate::{EraChanges, Language};

#[derive(Debug, Clone, Serialize)]
pub struct Sound {
    pub ipa: String,
    pub roman: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AffixEntry {
    pub gloss: String,
    pub form: String,
    pub applies_to: &'static str,
    pub position: AffixPosition,
    /// Written as a separate word rather than attached.
    pub particle: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LexiconEntry {
    pub concept: String,
    pub domain: Domain,
    pub root: String,
}

/// One glyph in the script table.
#[derive(Debug, Clone, Serialize)]
pub struct GlyphEntry {
    /// Reference number, as used in glyph-text output.
    pub number: usize,
    /// What it writes, in IPA.
    pub writes: String,
    /// What it writes, romanised.
    pub roman: String,
    /// Mechanical description (debug; real descriptions are content slots).
    pub description: String,
    pub marks: Vec<crate::script::Mark>,
    pub svg: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptSheet {
    pub kind: ScriptKind,
    pub direction: Direction,
    pub logic: Logic,
    pub glyphs: Vec<GlyphEntry>,
}

/// Everything about a language at one era, in readable form. Spoiler-level.
#[derive(Debug, Clone, Serialize)]
pub struct GrammarSheet {
    pub seed: u64,
    pub era: u32,
    pub eras: u32,
    pub consonants: Vec<Sound>,
    pub vowels: Vec<Sound>,
    pub syllable: String,
    pub onset_clusters: Vec<String>,
    pub codas: Vec<String>,
    pub affixes: Vec<AffixEntry>,
    pub fusions: Vec<(String, String)>,
    pub morphology_notes: Vec<String>,
    pub word_order: crate::syntax::Syntax,
    pub syntax_notes: Vec<String>,
    pub numerals: Numerals,
    pub numeral_notes: Vec<String>,
    pub script: ScriptSheet,
    pub history: Vec<Step>,
    pub changes: EraChanges,
    pub lexicon: Vec<LexiconEntry>,
}

impl GrammarSheet {
    pub fn new(lang: &Language) -> Self {
        let inv = &lang.phonology.inventory;
        let sound = |id| {
            let p = inv.get(id);
            Sound {
                ipa: p.ipa.to_string(),
                roman: p.roman.clone(),
            }
        };
        let m = &lang.morphology;
        let affix = |a: &crate::morphology::Affix, applies_to, position| AffixEntry {
            gloss: a.gloss.to_string(),
            form: lang.romanise(&a.form),
            applies_to,
            position,
            particle: a.particle,
        };
        let mut lexicon: Vec<LexiconEntry> = concepts::all()
            .iter()
            .filter(|c| lang.lexicon.has(&c.id))
            .map(|c| LexiconEntry {
                concept: c.id.clone(),
                domain: c.domain,
                root: lang.romanise(&lang.lexicon.root(&c.id)),
            })
            .collect();
        // Stable sort keeps file order within each domain.
        lexicon.sort_by_key(|e| e.domain);
        let order = |p: AffixPosition, inner: &str, outer: &str| match p {
            AffixPosition::Suffix => format!("root-{inner}-{outer}"),
            AffixPosition::Prefix => format!("{outer}-{inner}-root"),
        };
        let mut morphology_notes = vec![
            format!("nouns: {}", order(m.noun_position, "NUMBER", "CASE")),
            format!(
                "verbs: {}",
                match m.verb_position {
                    AffixPosition::Suffix => "root-ASPECT-TENSE-MOOD-NEG",
                    AffixPosition::Prefix => "NEG-MOOD-TENSE-ASPECT-root",
                }
            ),
            format!(
                "aspect marked: {}",
                match m.aspect {
                    crate::meaning::Aspect::Simple => "none (tense only)",
                    crate::meaning::Aspect::Perfective => "perfective (a whole, finished event)",
                    crate::meaning::Aspect::Imperfective => "imperfective (ongoing)",
                    crate::meaning::Aspect::Habitual => "habitual (usual, repeated)",
                }
            ),
            "moods marked on the verb: optative (may…), conditional (would…), question; commands have no subject".to_string(),
            "unmarked: singular, subject case, non-past, positive".to_string(),
            "adjectives, numerals, 'this' and 'here' never inflect".to_string(),
            "a counted noun is plural when the number is above one".to_string(),
        ];
        if !m.particles().is_empty() {
            morphology_notes.push(
                "particles are separate words right after their host (before it when affixes are prefixes)".to_string(),
            );
        }
        let script = &lang.script;
        let glyphs = script
            .glyphs
            .iter()
            .enumerate()
            .map(|(number, (key, glyph))| GlyphEntry {
                number,
                writes: key.label(),
                roman: roman_label(lang, key),
                description: glyph.describe(),
                marks: glyph.marks.clone(),
                svg: glyph.svg(48),
            })
            .collect();
        GrammarSheet {
            seed: lang.seed,
            era: lang.era,
            eras: lang.difficulty.eras(),
            consonants: inv.consonants().map(sound).collect(),
            vowels: inv.vowels().map(sound).collect(),
            syllable: lang.phonology.template.notation(),
            onset_clusters: lang
                .phonology
                .onsets
                .iter()
                .filter(|o| o.len() > 1)
                .map(|o| lang.romanise(o))
                .collect(),
            codas: lang
                .phonology
                .codas
                .iter()
                .map(|&c| sound(c).roman)
                .collect(),
            affixes: m
                .affix_list()
                .into_iter()
                .enumerate()
                .map(|(i, a)| {
                    if i < 4 {
                        affix(a, "noun", m.noun_position)
                    } else {
                        affix(a, "verb", m.verb_position)
                    }
                })
                .collect(),
            fusions: m
                .fusions
                .iter()
                .map(|f| (f.glosses.join("."), lang.romanise(&f.form)))
                .collect(),
            morphology_notes,
            word_order: lang.syntax.clone(),
            syntax_notes: lang.syntax.describe(),
            numerals: lang.numerals.clone(),
            numeral_notes: lang.numerals.describe(),
            script: ScriptSheet {
                kind: script.kind,
                direction: script.direction,
                logic: script.logic.clone(),
                glyphs,
            },
            history: lang.history.clone(),
            changes: lang.changes.clone(),
            lexicon,
        }
    }

    /// The script table alone, as text.
    pub fn script_text(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "SCRIPT — seed {} era {}: {} written {}\n",
            self.seed,
            self.era,
            label(&self.script.kind),
            label(&self.script.direction).replace('_', " ")
        ));
        let logic = &self.script.logic;
        for (name, m) in [
            ("voiced", logic.voicing),
            ("fricative", logic.fricative),
            ("nasal", logic.nasal),
        ] {
            if let Some(m) = m {
                s.push_str(&format!("  {name} sounds add: {}\n", m.describe()));
            }
        }
        for g in &self.script.glyphs {
            s.push_str(&format!(
                "  #{:<3} {:<16} {:<8} {}\n",
                g.number, g.writes, g.roman, g.description
            ));
        }
        s
    }

    /// Plain-text rendering for the terminal.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        let line = |s: &mut String, t: &str| {
            s.push_str(t);
            s.push('\n');
        };
        line(
            &mut s,
            &format!(
                "GRAMMAR SHEET — seed {} (era {} of {})",
                self.seed,
                self.era,
                self.eras - 1
            ),
        );
        line(&mut s, "");
        line(&mut s, "SOUNDS (ipa = spelling)");
        let sounds = |xs: &[Sound]| {
            xs.iter()
                .map(|x| {
                    if x.ipa == x.roman {
                        x.roman.clone()
                    } else {
                        format!("{}={}", x.ipa, x.roman)
                    }
                })
                .collect::<Vec<_>>()
                .join("  ")
        };
        line(
            &mut s,
            &format!(
                "  consonants ({}): {}",
                self.consonants.len(),
                sounds(&self.consonants)
            ),
        );
        line(
            &mut s,
            &format!("  vowels ({}): {}", self.vowels.len(), sounds(&self.vowels)),
        );
        line(&mut s, &format!("  syllable: {}", self.syllable));
        if !self.onset_clusters.is_empty() {
            line(
                &mut s,
                &format!("  onset clusters: {}", self.onset_clusters.join(" ")),
            );
        }
        let codas = if self.codas.is_empty() {
            "none".to_string()
        } else {
            self.codas.join(" ")
        };
        line(&mut s, &format!("  codas: {codas}"));
        if !self.history.is_empty() {
            line(&mut s, "");
            line(&mut s, "SOUND CHANGES SINCE ERA 0");
            for step in &self.history {
                for r in &step.rules {
                    line(&mut s, &format!("  era {}: {}", step.era, r.notation()));
                }
            }
            if !self.changes.replaced.is_empty() {
                line(
                    &mut s,
                    &format!("  new words this era: {}", self.changes.replaced.join(", ")),
                );
            }
            if !self.changes.eroded.is_empty() {
                line(
                    &mut s,
                    &format!(
                        "  became particles this era: {}",
                        self.changes.eroded.join(", ")
                    ),
                );
            }
            if self.changes.unfused > 0 {
                line(
                    &mut s,
                    &format!("  fused forms levelled this era: {}", self.changes.unfused),
                );
            }
        }
        line(&mut s, "");
        line(&mut s, "MORPHOLOGY");
        for a in &self.affixes {
            let shown = match (a.particle, a.position) {
                (true, _) => format!("{} (word)", a.form),
                (false, AffixPosition::Suffix) => format!("-{}", a.form),
                (false, AffixPosition::Prefix) => format!("{}-", a.form),
            };
            line(
                &mut s,
                &format!("  {:<4} {:<12} ({})", a.gloss, shown, a.applies_to),
            );
        }
        for (g, f) in &self.fusions {
            line(&mut s, &format!("  {g:<7} {f:<9} (fused)"));
        }
        for n in &self.morphology_notes {
            line(&mut s, &format!("  {n}"));
        }
        line(&mut s, "");
        line(&mut s, "SYNTAX");
        for n in &self.syntax_notes {
            line(&mut s, &format!("  {n}"));
        }
        line(&mut s, "");
        line(&mut s, "NUMERALS");
        for n in &self.numeral_notes {
            line(&mut s, &format!("  {n}"));
        }
        line(&mut s, "");
        line(
            &mut s,
            &format!(
                "SCRIPT: {}, {}, {} glyphs (see `script --spoil`)",
                label(&self.script.kind),
                label(&self.script.direction).replace('_', " "),
                self.script.glyphs.len()
            ),
        );
        line(&mut s, "");
        line(&mut s, "LEXICON");
        let mut current = None;
        for e in &self.lexicon {
            if current != Some(e.domain) {
                current = Some(e.domain);
                line(&mut s, &format!("  [{}]", label(&e.domain)));
            }
            line(&mut s, &format!("    {:<11} {}", e.concept, e.root));
        }
        s
    }
}

/// A serde enum's lowercase name.
fn label<T: Serialize>(x: &T) -> String {
    serde_json::to_value(x)
        .expect("enum")
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// What a glyph writes, romanised.
fn roman_label(lang: &Language, key: &crate::script::GlyphKey) -> String {
    use crate::script::GlyphKey;
    let r = |ipa: &str| lang.spelling.get(ipa).cloned().unwrap_or_default();
    match key {
        GlyphKey::Sound(s) => r(s),
        GlyphKey::Syllable(c, v) => format!("{}{}", c.map(r).unwrap_or_default(), r(v)),
        GlyphKey::Dead(c) => r(c),
        GlyphKey::Numeral(n) => n.to_string(),
        GlyphKey::Carrier => "-".to_string(),
        GlyphKey::Divider => crate::render::separator_glyph().to_string(),
        GlyphKey::Determinative => crate::render::DETERMINATIVE.to_string(),
    }
}
