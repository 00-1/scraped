//! Content slots the language engine needs, and the language hooks that
//! let Jb's templates include generated language.
//!
//! Glyph descriptions are the first slot family: the player never sees a
//! glyph, only its description, so the wording is Jb's to write.

use scraped_content::{Context, Hooks, Registry, Renderer, SlotDef, Value, VarType};

use crate::impression::{impressions, Impression, RESEMBLANCES};
use crate::script::{Glyph, Mark, Script};
use crate::Language;

fn enum_of(values: &[&str]) -> VarType {
    VarType::Enum {
        values: values.iter().map(|s| s.to_string()).collect(),
    }
}

const STROKES: [&str; 9] = [
    "bar", "hook", "dot", "ring", "arc", "wedge", "tail", "cross", "zigzag",
];
const TURNS: [&str; 5] = ["up", "right", "down", "left", "none"];
const SPOTS: [&str; 5] = ["centre", "top", "bottom", "left", "right"];
const SPOTS_TOLD: [&str; 6] = ["centre", "top", "bottom", "left", "right", "none"];
const OUTLINES: [&str; 6] = ["round", "tall", "wide", "angular", "slight", "plain"];

/// Every slot the language engine declares.
pub fn slots() -> Vec<SlotDef> {
    vec![
        SlotDef::new(
            "glyph.stroke",
            "One stroke of a glyph in the ancient script, as the player sees it. Used inside \
             glyph.describe, so write a phrase, not a sentence (\"a hook opening to the right, \
             at the top\"). It must be exact enough to draw: which stroke, which way it points \
             or opens, and where it sits in the glyph's square. Never mention sounds or meanings.",
        )
        .var("stroke", enum_of(&STROKES), "Which stroke.")
        .var(
            "turn",
            enum_of(&TURNS),
            "Which way the stroke points or opens. 'none' for dots, rings and crosses, which look \
             the same any way round. For a bar, 'up' means upright and 'right' means level.",
        )
        .var(
            "spot",
            enum_of(&SPOTS),
            "Where in the glyph's square the stroke sits.",
        )
        .min_variants(1)
        .max_len(90)
        .sampler(stroke_samples),
        SlotDef::new(
            "glyph.describe",
            "A whole glyph, built from its strokes, as the player sees it on a surface. The \
             player must be able to draw it from this alone, and two different glyphs must never \
             read the same. Never mention sounds or meanings. In previews, the strokes use the \
             engine's plain debug wording; in play they come from glyph.stroke.",
        )
        .var(
            "strokes",
            VarType::List,
            "Each stroke, already described by glyph.stroke.",
        )
        .var(
            "count",
            VarType::Number,
            "How many strokes the glyph has (1–5).",
        )
        .max_len(320)
        .sampler(glyph_samples),
        SlotDef::new(
            "glyph.impression",
            "A sign as a person sees and remembers it at a glance, in a close reading (S01): \
             its outline, the stroke that carries it, a mark or two that stand out, and what it \
             looks like (\"tall and hooked\", \"a ring like an eye\", \"a squat cross with a dot \
             beside it\"). Not exact enough to draw (tracing gives that). The same sign always \
             gives the same impression, so the player can recognise it again, and no two signs \
             of a script may read the same: the engine gives only as much detail as is needed to \
             tell them apart, so use every variable it fills. A sign that is another sign plus a \
             mark comes with 'like' (that sign's impression) and 'added': say it that way (\"like \
             the hooked sign, with a dot below\"), so related signs read as related. A phrase, not \
             a sentence. Never mention sounds or meanings.",
        )
        .var("outline", enum_of(&OUTLINES), "Its overall outline.")
        .var(
            "main",
            enum_of(&STROKES),
            "The stroke that carries the sign.",
        )
        .var(
            "main_turn",
            enum_of(&TURNS),
            "Which way the main stroke points or opens ('none' if it looks the same any way \
             round).",
        )
        .var(
            "main_spot",
            enum_of(&SPOTS_TOLD),
            "Where the main stroke sits, when that is needed to tell it apart ('none' otherwise).",
        )
        .var(
            "others",
            VarType::List,
            "Other marks worth telling, most distinctive first, each from glyph.stroke (its turn \
             'none' unless needed). Often empty.",
        )
        .var(
            "resembles",
            enum_of(RESEMBLANCES),
            "What it looks like, where something fits ('' if nothing does).",
        )
        .var("count", VarType::Number, "How many strokes it has.")
        .var(
            "like",
            VarType::Text,
            "For a sign that is another plus a mark or two: that sign's impression ('' otherwise).",
        )
        .var(
            "added",
            VarType::List,
            "What this sign adds to the 'like' sign, each from glyph.stroke.",
        )
        .max_len(160)
        .sampler(impression_samples),
    ]
}

fn impression_samples(seed: u64) -> Vec<Context> {
    let lang = Language::generate(seed);
    let imps = impressions(&lang.script, false);
    let plain = |m: &Mark| Value::from(m.describe());
    imps.iter()
        .take(30)
        .map(|imp| {
            let like = imp
                .like
                .map(|j| format!("a {} sign", imps[j].outline))
                .unwrap_or_default();
            impression_context(
                imp,
                imp.others.iter().map(plain).collect(),
                like,
                imp.added.iter().map(plain).collect(),
            )
        })
        .collect()
}

fn impression_context(
    imp: &Impression,
    others: Vec<Value>,
    like: String,
    added: Vec<Value>,
) -> Context {
    let name = |x: &dyn erased::Named| x.name();
    let symmetric = matches!(
        imp.main.stroke,
        crate::script::Stroke::Dot | crate::script::Stroke::Ring | crate::script::Stroke::Cross
    );
    [
        ("outline".to_string(), Value::from(imp.outline)),
        ("main".to_string(), Value::from(name(&imp.main.stroke))),
        (
            "main_turn".to_string(),
            Value::from(if symmetric {
                "none".to_string()
            } else {
                name(&imp.main.turn)
            }),
        ),
        (
            "main_spot".to_string(),
            Value::from(if imp.main_spot {
                name(&imp.main.spot)
            } else {
                "none".to_string()
            }),
        ),
        ("others".to_string(), Value::List(others)),
        ("resembles".to_string(), Value::from(imp.resembles)),
        ("count".to_string(), Value::Number(imp.count as i64)),
        ("like".to_string(), Value::from(like)),
        ("added".to_string(), Value::List(added)),
    ]
    .into_iter()
    .collect()
}

/// A mark through `glyph.stroke`, its turn left out unless `turn`.
fn stroke_phrase(r: &mut Renderer, m: &Mark, turn: bool) -> Value {
    let mut c = stroke_context(m);
    if !turn {
        c.insert("turn".to_string(), Value::from("none"));
    }
    Value::from(r.render("glyph.stroke", &c))
}

/// Every sign of a script as it is seen at a glance, through Jb's
/// templates (`glyph.impression`, with marks from `glyph.stroke`), in
/// table order. `confusable` leaves a few alike (archaeologist).
pub fn impression_texts(r: &mut Renderer, script: &Script, confusable: bool) -> Vec<String> {
    let imps = impressions(script, confusable);
    let mut texts: Vec<Option<String>> = vec![None; imps.len()];
    // Signs told by their own look first: related signs use them.
    for pass in 0..2 {
        for (i, imp) in imps.iter().enumerate() {
            if (pass == 0) != imp.like.is_none() {
                continue;
            }
            let others = imp
                .others
                .iter()
                .map(|m| stroke_phrase(r, m, imp.turns))
                .collect();
            let added = imp
                .added
                .iter()
                .map(|m| stroke_phrase(r, m, true))
                .collect();
            let like = imp.like.and_then(|j| texts[j].clone()).unwrap_or_default();
            let c = impression_context(imp, others, like, added);
            texts[i] = Some(r.render("glyph.impression", &c));
        }
    }
    texts.into_iter().map(Option::unwrap_or_default).collect()
}

/// The registry of every slot the engine declares so far.
pub fn registry() -> Registry {
    Registry::new(slots())
}

fn sample_glyphs(seed: u64) -> Vec<Glyph> {
    let lang = Language::generate(seed);
    lang.script
        .glyphs
        .iter()
        .map(|(_, g)| g.clone())
        .take(24)
        .collect()
}

/// Variables for one stroke.
pub fn stroke_context(m: &Mark) -> Context {
    let name = |x: &dyn erased::Named| x.name();
    let symmetric = matches!(
        m.stroke,
        crate::script::Stroke::Dot | crate::script::Stroke::Ring | crate::script::Stroke::Cross
    );
    [
        ("stroke".to_string(), Value::from(name(&m.stroke))),
        (
            "turn".to_string(),
            Value::from(if symmetric {
                "none".to_string()
            } else {
                name(&m.turn)
            }),
        ),
        ("spot".to_string(), Value::from(name(&m.spot))),
    ]
    .into_iter()
    .collect()
}

fn stroke_samples(seed: u64) -> Vec<Context> {
    let mut out: Vec<Context> = Vec::new();
    for g in sample_glyphs(seed) {
        for m in &g.marks {
            let c = stroke_context(m);
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}

fn glyph_samples(seed: u64) -> Vec<Context> {
    sample_glyphs(seed)
        .iter()
        .map(|g| {
            let strokes = g.marks.iter().map(|m| Value::from(m.describe())).collect();
            glyph_context(strokes, g.marks.len())
        })
        .collect()
}

fn glyph_context(strokes: Vec<Value>, count: usize) -> Context {
    [
        ("strokes".to_string(), Value::List(strokes)),
        ("count".to_string(), Value::Number(count as i64)),
    ]
    .into_iter()
    .collect()
}

/// A glyph described through Jb's templates: each stroke via
/// `glyph.stroke`, then the whole via `glyph.describe`.
pub fn describe_glyph(r: &mut Renderer, g: &Glyph) -> String {
    let strokes = g
        .marks
        .iter()
        .map(|m| Value::from(r.render("glyph.stroke", &stroke_context(m))))
        .collect();
    r.render("glyph.describe", &glyph_context(strokes, g.marks.len()))
}

/// One preview rendering.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PreviewRow {
    pub vars: Context,
    pub text: String,
    /// A drawing of what the text describes, when there is one (spoiler:
    /// the player never sees it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
}

/// Sample renderings of a slot for previews. Glyph descriptions are built
/// from real glyphs, with each stroke rendered through `glyph.stroke`,
/// exactly as in play.
pub fn preview(r: &mut Renderer, slot: &str, seed: u64, count: usize) -> Vec<PreviewRow> {
    if slot == "glyph.describe" {
        return sample_glyphs(seed)
            .iter()
            .take(count)
            .map(|g| {
                let strokes: Vec<Value> = g
                    .marks
                    .iter()
                    .map(|m| Value::from(r.render("glyph.stroke", &stroke_context(m))))
                    .collect();
                let vars = glyph_context(strokes, g.marks.len());
                let text = r.render(slot, &vars);
                PreviewRow {
                    vars,
                    text,
                    svg: Some(g.svg(48)),
                }
            })
            .collect();
    }
    if slot == "glyph.stroke" {
        let mut seen = Vec::new();
        return sample_glyphs(seed)
            .iter()
            .flat_map(|g| g.marks.clone())
            .filter(|m| {
                let new = !seen.contains(m);
                seen.push(*m);
                new
            })
            .take(count)
            .map(|m| {
                let vars = stroke_context(&m);
                let text = r.render(slot, &vars);
                let svg = Glyph { marks: vec![m] }.svg(48);
                PreviewRow {
                    vars,
                    text,
                    svg: Some(svg),
                }
            })
            .collect();
    }
    let Some(def) = r.registry.get(slot) else {
        return Vec::new();
    };
    def.samples(&[seed])
        .into_iter()
        .take(count)
        .map(|(_, vars)| {
            let text = r.render(slot, &vars);
            PreviewRow {
                vars,
                text,
                svg: None,
            }
        })
        .collect()
}

/// Language hooks for templates: `{lang.word gate}` gives the word for a
/// concept, `{lang.text x}` passes generated text through unchanged.
pub struct LangHooks<'a> {
    pub lang: &'a Language,
}

impl Hooks for LangHooks<'_> {
    fn call(&self, name: &str, args: &[Value]) -> Option<String> {
        match (name, args) {
            ("lang.word", [concept]) => {
                let id = concept.text();
                self.lang
                    .lexicon
                    .roots
                    .get(&id)
                    .map(|root| self.lang.romanise(root))
            }
            ("lang.text", [text]) => Some(text.text()),
            // DESIGN-Q: until display modes exist (M05), {lang.glyphs x}
            // lists the word's glyphs by table number.
            ("lang.glyphs", [concept]) => {
                let root = self.lang.lexicon.roots.get(&concept.text())?;
                let ipa = self.lang.phonology.to_ipa(root);
                Some(
                    self.lang
                        .script
                        .spell(&ipa)
                        .iter()
                        .map(|k| format!("#{}", self.lang.script.index(k)))
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }
            _ => None,
        }
    }
}

/// Lowercase serde names of the script's enums.
mod erased {
    pub trait Named {
        fn name(&self) -> String;
    }

    impl<T: serde::Serialize> Named for T {
        fn name(&self) -> String {
            serde_json::to_value(self)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraped_content::{Pack, PackFile, Variant};

    #[test]
    fn samples_cover_the_declared_values() {
        let reg = registry();
        let stroke = reg.get("glyph.stroke").unwrap();
        for (_, c) in stroke.samples(&[1, 2, 3]) {
            let VarType::Enum { values } = &stroke.variable("stroke").unwrap().ty else {
                panic!()
            };
            assert!(values.contains(&c["stroke"].text()));
        }
        assert!(!reg.get("glyph.describe").unwrap().samples(&[1]).is_empty());
    }

    #[test]
    fn glyphs_describe_through_templates() {
        let reg = registry();
        let v = |slot: &str, text: &str| Variant {
            slot: slot.into(),
            text: text.into(),
            when: None,
            weight: 1,
            example: false,
        };
        let pack = Pack {
            files: vec![PackFile {
                path: "glyphs.toml".into(),
                notes: None,
                variants: vec![
                    v("glyph.stroke", "{a stroke} at the {spot}"),
                    v("glyph.describe", "{cap list strokes}"),
                ],
                storylets: Vec::new(),
            }],
        };
        let lang = Language::generate(42);
        let hooks = LangHooks { lang: &lang };
        let mut r = Renderer::new(&reg, &pack, 42, &hooks);
        let g = &lang.script.glyphs[0].1;
        let text = describe_glyph(&mut r, g);
        assert!(text.contains(" at the "), "{text}");
        assert!(!text.contains('⟦'), "{text}");
    }

    #[test]
    fn hooks_give_words() {
        let lang = Language::generate(42);
        let h = LangHooks { lang: &lang };
        assert_eq!(
            h.call("lang.word", &[Value::from("gate")]),
            Some(lang.romanise(&lang.lexicon.root("gate")))
        );
        assert!(h.call("lang.word", &[Value::from("nonsense")]).is_none());
        assert!(h
            .call("lang.glyphs", &[Value::from("gate")])
            .unwrap()
            .starts_with('#'));
    }
}
