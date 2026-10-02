//! The full grammar of a language, for spoiler/debug output.

use serde::Serialize;

use crate::concepts::{self, Domain};
use crate::morphology::AffixPosition;
use crate::Language;

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
}

#[derive(Debug, Clone, Serialize)]
pub struct LexiconEntry {
    pub concept: String,
    pub domain: Domain,
    pub root: String,
}

/// Everything about a language in readable form. Spoiler-level.
#[derive(Debug, Clone, Serialize)]
pub struct GrammarSheet {
    pub seed: u64,
    pub era: u32,
    pub consonants: Vec<Sound>,
    pub vowels: Vec<Sound>,
    pub syllable: String,
    pub onset_clusters: Vec<String>,
    pub codas: Vec<String>,
    pub affixes: Vec<AffixEntry>,
    pub morphology_notes: Vec<String>,
    pub word_order: crate::syntax::Syntax,
    pub syntax_notes: Vec<String>,
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
        };
        let mut lexicon: Vec<LexiconEntry> = concepts::all()
            .iter()
            .map(|c| LexiconEntry {
                concept: c.id.clone(),
                domain: c.domain,
                root: lang.romanise(lang.lexicon.root(&c.id)),
            })
            .collect();
        // Stable sort keeps file order within each domain.
        lexicon.sort_by_key(|e| e.domain);
        let order = |p: AffixPosition, inner: &str, outer: &str| match p {
            AffixPosition::Suffix => format!("root-{inner}-{outer}"),
            AffixPosition::Prefix => format!("{outer}-{inner}-root"),
        };
        GrammarSheet {
            seed: lang.seed,
            era: lang.era,
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
            affixes: vec![
                affix(&m.plural, "noun", m.noun_position),
                affix(&m.object, "noun", m.noun_position),
                affix(&m.genitive, "noun", m.noun_position),
                affix(&m.dative, "noun", m.noun_position),
                affix(&m.past, "verb", m.verb_position),
                affix(&m.negative, "verb", m.verb_position),
            ],
            morphology_notes: vec![
                format!("nouns: {}", order(m.noun_position, "NUMBER", "CASE")),
                format!("verbs: {}", order(m.verb_position, "TENSE", "NEG")),
                "unmarked: singular, subject case, non-past, positive".to_string(),
                "adjectives, numerals, 'this' and 'here' never inflect".to_string(),
                "a counted noun is plural when the number is above one".to_string(),
            ],
            word_order: lang.syntax.clone(),
            syntax_notes: lang.syntax.describe(),
            lexicon,
        }
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
            &format!("GRAMMAR SHEET — seed {} (era {})", self.seed, self.era),
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
        line(
            &mut s,
            &format!(
                "  codas: {}",
                if self.codas.is_empty() {
                    "none".to_string()
                } else {
                    self.codas.join(" ")
                }
            ),
        );
        line(&mut s, "");
        line(&mut s, "MORPHOLOGY");
        for a in &self.affixes {
            let shown = match a.position {
                AffixPosition::Suffix => format!("-{}", a.form),
                AffixPosition::Prefix => format!("{}-", a.form),
            };
            line(
                &mut s,
                &format!("  {:<4} {:<8} ({})", a.gloss, shown, a.applies_to),
            );
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
        line(&mut s, "LEXICON");
        let mut current = None;
        for e in &self.lexicon {
            if current != Some(e.domain) {
                current = Some(e.domain);
                line(
                    &mut s,
                    &format!(
                        "  [{}]",
                        serde_json::to_value(e.domain)
                            .expect("enum")
                            .as_str()
                            .unwrap_or("")
                    ),
                );
            }
            line(&mut s, &format!("    {:<10} {}", e.concept, e.root));
        }
        s
    }
}
