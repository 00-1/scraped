//! Content slots the language engine needs, and the language hooks that
//! let Jb's templates include generated language.
//!
//! Glyph descriptions are the first slot family: the player never sees a
//! glyph, only its description, so the wording is Jb's to write.

use scraped_content::{Context, Hooks, Registry, Renderer, SlotDef, Value, VarType};

use crate::impression::{
    impressions, Part, Told, BUSY, CURVES, FACINGS, LEANS, PARTS, PLACES, PROPORTIONS,
    RESEMBLANCES, RUNS, SITS, SYMMETRY, WEIGHT,
};
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
            "glyph.part",
            "One part of a sign's shape as the eye sees it, used inside glyph.impression (\"a \
             dot above\", \"a loop\", \"a saw edge beside\"). A phrase. A shape part, never a stroke \
             to draw: no directions, no exact positions.",
        )
        .var("part", enum_of(PARTS), "What the part looks like.")
        .var(
            "place",
            enum_of(PLACES),
            "Where it sits, coarsely ('none': it is the body of the sign).",
        )
        .var(
            "facing",
            enum_of(FACINGS),
            "Which way it faces, only when needed to tell signs apart ('' otherwise): forward \
             is the way the writing runs, back against it; high or low; a line standing or \
             lying.",
        )
        .max_len(40)
        .sampler(part_samples),
        SlotDef::new(
            "glyph.impression",
            "A sign as a person sees and remembers it at a glance, in a close reading (S01, \
             S02): its whole shape, never how it is built (\"a tall, curved sign like a lamp, \
             with a dot above\", \"a squat, spiky sign in two pieces\"). Not exact enough to \
             draw (tracing gives that), and no stroke names or directions. The same sign always \
             gives the same impression, so the player can recognise it again, and no two signs \
             of a script may read the same: the engine tells only as many features as are \
             needed to tell them apart and leaves the rest empty, so use every variable it \
             fills. A sign that is another sign plus a part or two comes with 'like' (that \
             sign's impression) and 'added': say it that way (\"like the lamp sign, with a dot \
             below\"), so related signs read as related. A phrase, not a sentence. Never mention \
             sounds or meanings.",
        )
        .var("proportion", enum_of(PROPORTIONS), "Its proportions.")
        .var("curve", enum_of(CURVES), "Curved, angular or both.")
        .var(
            "resembles",
            enum_of(RESEMBLANCES),
            "An everyday thing it looks like, where one fits ('' if none does).",
        )
        .var(
            "parts",
            VarType::List,
            "Parts that stand out, the most striking first, each from glyph.part. Often one or \
             none.",
        )
        .var(
            "pieces",
            enum_of(&COUNTS),
            "How many separate pieces it falls into ('' when not needed).",
        )
        .var(
            "holes",
            enum_of(&COUNTS),
            "How many enclosed spaces it has ('' when not needed).",
        )
        .var(
            "symmetry",
            enum_of(SYMMETRY),
            "Which way it is symmetric ('' when not needed).",
        )
        .var(
            "busy",
            enum_of(BUSY),
            "Spare or busy: how much is cut ('' when not needed).",
        )
        .var(
            "leans",
            enum_of(LEANS),
            "Which way its weight leans along the line: forward is the way the writing runs \
             ('' when not needed).",
        )
        .var(
            "weight",
            enum_of(WEIGHT),
            "Whether its weight sits high or low ('' when not needed).",
        )
        .var(
            "sits",
            enum_of(SITS),
            "Where it sits on the line of writing: high, low or in the middle ('' when not \
             needed).",
        )
        .var(
            "runs",
            enum_of(RUNS),
            "Which way its lines mostly run: across, upright or every way ('' when not needed).",
        )
        .var(
            "like",
            VarType::Text,
            "For a sign that is another plus a part or two: that sign's impression ('' \
             otherwise).",
        )
        .var(
            "like_sound",
            VarType::Text,
            "S03: the sound of the 'like' sign, romanised, once the player has heard it \
             ('' otherwise): say the base by its sound, the way the player knows it.",
        )
        .var(
            "like_resembles",
            VarType::Text,
            "S03: what the 'like' sign resembles, when no other sign of the script \
             resembles it ('' otherwise): the base said so it can be found.",
        )
        .var(
            "added",
            VarType::List,
            "What this sign adds to the 'like' sign, each from glyph.part.",
        )
        .max_len(200)
        .sampler(impression_samples),
    ]
}

/// Counts as words, for pieces and enclosed spaces ('' when not told).
const COUNTS: [&str; 7] = ["", "none", "one", "two", "three", "four", "several"];

fn count_word(n: Option<usize>) -> &'static str {
    match n {
        None => "",
        Some(n) => COUNTS[(n + 1).min(COUNTS.len() - 1)],
    }
}

fn part_context(p: &Part) -> Context {
    [
        ("part".to_string(), Value::from(p.part)),
        ("place".to_string(), Value::from(p.place)),
        ("facing".to_string(), Value::from(p.facing)),
    ]
    .into_iter()
    .collect()
}

fn part_samples(_: u64) -> Vec<Context> {
    PARTS
        .iter()
        .zip(PLACES.iter().cycle())
        .zip(FACINGS.iter().cycle())
        .map(|((part, place), facing)| {
            part_context(&Part {
                part,
                place,
                facing,
            })
        })
        .collect()
}

fn impression_samples(seed: u64) -> Vec<Context> {
    let lang = Language::generate(seed);
    let imps = impressions(&lang.script, false);
    let plain = |p: &Part| {
        let words: Vec<&str> = [p.part, p.place, p.facing]
            .into_iter()
            .filter(|w| !w.is_empty() && *w != "none")
            .collect();
        Value::from(words.join(" "))
    };
    imps.iter()
        .take(30)
        .map(|imp| {
            let t = &imp.told;
            let like = Base {
                look: t
                    .like
                    .map(|j| format!("a {} sign", imps[j].told.proportion))
                    .unwrap_or_default(),
                sound: String::new(),
                resembles: t
                    .like
                    .map(|j| imps[j].told.resembles.to_string())
                    .unwrap_or_default(),
            };
            impression_context(
                t,
                t.parts.iter().map(plain).collect(),
                like,
                t.added.iter().map(plain).collect(),
            )
        })
        .collect()
}

/// What a related sign's base is called (S03): its sound if heard, else
/// its resemblance if no other sign shares it, else its whole look.
#[derive(Default)]
pub struct Base {
    pub look: String,
    pub sound: String,
    pub resembles: String,
}

fn impression_context(t: &Told, parts: Vec<Value>, like: Base, added: Vec<Value>) -> Context {
    [
        ("proportion", Value::from(t.proportion)),
        ("curve", Value::from(t.curve)),
        ("resembles", Value::from(t.resembles)),
        ("parts", Value::List(parts)),
        ("pieces", Value::from(count_word(t.pieces))),
        ("holes", Value::from(count_word(t.holes))),
        ("symmetry", Value::from(t.symmetry)),
        ("busy", Value::from(t.busy)),
        ("leans", Value::from(t.leans)),
        ("weight", Value::from(t.weight)),
        ("sits", Value::from(t.sits)),
        ("runs", Value::from(t.runs)),
        ("like", Value::from(like.look)),
        ("like_sound", Value::from(like.sound)),
        ("like_resembles", Value::from(like.resembles)),
        ("added", Value::List(added)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// A part of a sign's shape through `glyph.part`.
pub fn part_phrase(r: &mut Renderer, p: &Part) -> String {
    r.render("glyph.part", &part_context(p))
}

/// Every sign of a script as it is seen at a glance, through Jb's
/// templates (`glyph.impression`, with parts from `glyph.part`), in
/// table order. `confusable` leaves a few alike (archaeologist).
pub fn impression_texts(
    r: &mut Renderer,
    script: &Script,
    confusable: bool,
    heard: &dyn Fn(usize) -> Option<String>,
) -> Vec<String> {
    let imps = impressions(script, confusable);
    let shared = |res: &str| imps.iter().filter(|i| i.told.resembles == res).count() > 1;
    let mut texts: Vec<Option<String>> = vec![None; imps.len()];
    // Signs told by their own look first: related signs use them.
    for pass in 0..2 {
        for (i, imp) in imps.iter().enumerate() {
            if (pass == 0) != imp.told.like.is_none() {
                continue;
            }
            let t = &imp.told;
            let phrases = |r: &mut Renderer, ps: &[crate::impression::Part]| -> Vec<String> {
                let mut out: Vec<String> = Vec::new();
                for p in ps {
                    let ph = part_phrase(r, p);
                    // Never the same part twice (S04).
                    if !out.contains(&ph) {
                        out.push(ph);
                    }
                }
                out
            };
            let parts = phrases(r, &t.parts);
            let added: Vec<String> = phrases(r, &t.added)
                .into_iter()
                .filter(|a| !parts.contains(a))
                .collect();
            let like = Base {
                look: t.like.and_then(|j| texts[j].clone()).unwrap_or_default(),
                sound: t.like.and_then(heard).unwrap_or_default(),
                resembles: t
                    .like
                    .map(|j| imps[j].told.resembles)
                    .filter(|r| !r.is_empty() && !shared(r))
                    .unwrap_or_default()
                    .to_string(),
            };
            // The base must be one the player can find (S04): by its sound,
            // a resemblance no other sign shares, or a look no other sign
            // has. Otherwise the sign is told on its own: the base's look
            // and every part, with no "like".
            let look_unique = !like.look.is_empty()
                && texts.iter().flatten().filter(|x| **x == like.look).count() == 1;
            let findable = !like.sound.is_empty() || !like.resembles.is_empty() || look_unique;
            let alone = if t.like.is_some() && (!findable || added.is_empty()) {
                let mut all = parts.clone();
                all.extend(added.iter().cloned());
                let own = Told {
                    like: None,
                    added: Vec::new(),
                    ..t.clone()
                };
                let c = impression_context(
                    &own,
                    all.into_iter().map(Value::from).collect(),
                    Base::default(),
                    Vec::new(),
                );
                Some(r.render("glyph.impression", &c))
                    // Told on its own it must still read like no other sign.
                    .filter(|x| !texts.iter().flatten().any(|y| y == x))
            } else {
                None
            };
            texts[i] = Some(alone.unwrap_or_else(|| {
                let c = impression_context(
                    t,
                    parts.into_iter().map(Value::from).collect(),
                    like,
                    added.into_iter().map(Value::from).collect(),
                );
                r.render("glyph.impression", &c)
            }));
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
