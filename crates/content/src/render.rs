//! Rendering: slot + variables → text, using Jb's templates.
//!
//! Deterministic: the same seed, context, pack and render history always
//! give the same text. Where several variants apply, the renderer avoids
//! repeating the one it used last for that slot.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::english;
use crate::pack::{Pack, Variant};
use crate::slot::{Context, Registry, Value};
use crate::template::{self, Arg, Expr, Node, Op, Operand};

/// Generated-language hooks (`{lang.word gate}` and friends). The language
/// engine implements this; the content crate knows nothing about languages.
pub trait Hooks {
    /// Returns the text for a hook call, or `None` if the hook is unknown.
    fn call(&self, name: &str, args: &[Value]) -> Option<String>;
}

/// For contexts with no language attached.
pub struct NoHooks;

impl Hooks for NoHooks {
    fn call(&self, _: &str, _: &[Value]) -> Option<String> {
        None
    }
}

/// Shown when a slot has no usable variant. Loud on purpose: never mistaken
/// for real prose.
pub fn placeholder(slot: &str) -> String {
    format!("⟦slot: {slot}⟧")
}

/// A helper: name, argument count, which arguments may be bare words
/// (`{count n jar}`: "jar" is a word, not a variable), and usage.
pub struct Helper {
    pub name: &'static str,
    pub arity: usize,
    /// Argument positions where a bare name that is not a variable is taken
    /// as a literal word.
    pub words: &'static [usize],
    pub usage: &'static str,
}

/// Helpers the template language offers.
pub const HELPERS: &[Helper] = &[
    Helper {
        name: "a",
        arity: 1,
        words: &[0],
        usage: "{a x}: x with 'a' or 'an' in front",
    },
    Helper {
        name: "cap",
        arity: 1,
        words: &[],
        usage: "{cap x}: x with a capital first letter",
    },
    Helper {
        name: "plural",
        arity: 2,
        words: &[0],
        usage: "{plural word n}: word, made plural unless n is 1",
    },
    Helper {
        name: "count",
        arity: 2,
        words: &[1],
        usage: "{count n word}: 'three jars', 'one jar'",
    },
    Helper {
        name: "number",
        arity: 1,
        words: &[],
        usage: "{number n}: n in words",
    },
    Helper {
        name: "list",
        arity: 1,
        words: &[],
        usage: "{list x}: 'a, b and c'",
    },
    Helper {
        name: "bearing",
        arity: 1,
        words: &[],
        usage: "{bearing degrees}: 'north-east'",
    },
    Helper {
        name: "distance",
        arity: 1,
        words: &[],
        usage: "{distance metres}: 'about 300 metres'",
    },
    Helper {
        name: "duration",
        arity: 1,
        words: &[],
        usage: "{duration minutes}: 'about an hour'",
    },
];

/// Whether a name is a helper or language hook.
pub fn is_helper(name: &str) -> bool {
    name.starts_with("lang.") || HELPERS.iter().any(|h| h.name == name)
}

/// Whether argument `i` of helper `name` may be a bare word. Language hooks
/// take bare words everywhere: `{lang.word gate}`.
pub fn word_position(name: &str, i: usize) -> bool {
    name.starts_with("lang.")
        || HELPERS
            .iter()
            .any(|h| h.name == name && h.words.contains(&i))
}

/// One slot rendered: which variant made it, which `[if]` branches were
/// taken, and the text. The authoring tool uses these to say why a line
/// was shown and to measure coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Trace {
    pub slot: String,
    /// The variant used, as (file, index in file); `None`: no variant
    /// applied and a placeholder was shown.
    pub file: Option<String>,
    pub variant: Option<usize>,
    /// Whether the variant is an agent-written example.
    pub example: bool,
    /// Whether the variant has a `when` condition.
    pub conditional: bool,
    /// `[if]` outcomes in the order met (true: the first branch).
    pub branches: Vec<bool>,
    pub text: String,
    /// 0 for a slot the game asked for; deeper for `{>slot}` calls.
    pub depth: usize,
}

/// Renders slots from a pack.
pub struct Renderer<'a> {
    pub registry: &'a Registry,
    pub pack: &'a Pack,
    pub seed: u64,
    pub hooks: &'a dyn Hooks,
    last: BTreeMap<String, String>,
    depth: usize,
    /// Every slot rendered so far, outermost last.
    pub trace: Vec<Trace>,
    open: Vec<Trace>,
}

/// A problem met while rendering. Rendering still produces text: the error
/// appears inline so a dev build shows exactly where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError(pub String);

impl<'a> Renderer<'a> {
    pub fn new(registry: &'a Registry, pack: &'a Pack, seed: u64, hooks: &'a dyn Hooks) -> Self {
        Renderer {
            registry,
            pack,
            seed,
            hooks,
            last: BTreeMap::new(),
            depth: 0,
            trace: Vec::new(),
            open: Vec::new(),
        }
    }

    /// Which variant was used last for each slot. Games keep this between
    /// turns so a replay renders exactly as the original did.
    pub fn memory(&self) -> BTreeMap<String, String> {
        self.last.clone()
    }

    /// Restores memory saved with `memory`.
    pub fn set_memory(&mut self, memory: BTreeMap<String, String>) {
        self.last = memory;
    }

    /// Renders a slot. Missing content shows as a placeholder; errors show
    /// inline as `⟦error: …⟧`.
    pub fn render(&mut self, slot: &str, ctx: &Context) -> String {
        match self.try_render(slot, ctx) {
            Ok(s) => s,
            Err(RenderError(m)) => format!("⟦error: {m}⟧"),
        }
    }

    /// Like `render`, but reports the first error instead of inlining it.
    pub fn try_render(&mut self, slot: &str, ctx: &Context) -> Result<String, RenderError> {
        self.open.push(Trace {
            slot: slot.to_string(),
            file: None,
            variant: None,
            example: false,
            conditional: false,
            branches: Vec::new(),
            text: String::new(),
            depth: self.depth,
        });
        let out = self.render_inner(slot, ctx);
        let mut t = self.open.pop().expect("opened above");
        t.text = match &out {
            Ok(s) => s.clone(),
            Err(RenderError(m)) => format!("⟦error: {m}⟧"),
        };
        self.trace.push(t);
        out
    }

    fn render_inner(&mut self, slot: &str, ctx: &Context) -> Result<String, RenderError> {
        let variants = self.pack.variants(slot);
        let eligible: Vec<&Variant> = variants
            .into_iter()
            .filter(|v| match &v.when {
                None => true,
                Some(w) => template::parse_expr(w)
                    .ok()
                    .and_then(|e| eval(&e, ctx).ok())
                    .unwrap_or(false),
            })
            .collect();
        if eligible.is_empty() {
            return Ok(placeholder(slot));
        }
        let chosen = self.choose(slot, ctx, &eligible);
        if let Some((f, i, _)) = self
            .pack
            .all_variants()
            .find(|(_, _, v)| std::ptr::eq(*v, chosen))
        {
            let t = self.open.last_mut().expect("open");
            t.file = Some(f.path.clone());
            t.variant = Some(i);
            t.example = chosen.example;
            t.conditional = chosen.when.is_some();
        }
        let nodes = template::parse(&chosen.text)
            .map_err(|e| RenderError(format!("{slot}: {}", e.message)))?;
        self.last.insert(slot.to_string(), chosen.text.clone());
        self.nodes(&nodes, ctx, slot)
    }

    /// Weighted, deterministic choice; avoids the last variant used for this
    /// slot when there is an alternative.
    fn choose<'v>(&self, slot: &str, ctx: &Context, eligible: &[&'v Variant]) -> &'v Variant {
        let key = hash(&[
            &self.seed.to_le_bytes(),
            slot.as_bytes(),
            canonical(ctx).as_bytes(),
        ]);
        let pick = |key: u64, from: &[&'v Variant]| -> &'v Variant {
            let total: u64 = from.iter().map(|v| u64::from(v.weight.max(1))).sum();
            let mut roll = key % total;
            for v in from {
                let w = u64::from(v.weight.max(1));
                if roll < w {
                    return v;
                }
                roll -= w;
            }
            from[0]
        };
        let first = pick(key, eligible);
        if eligible.len() > 1 && self.last.get(slot) == Some(&first.text) {
            let others: Vec<&Variant> = eligible
                .iter()
                .copied()
                .filter(|v| v.text != first.text)
                .collect();
            if !others.is_empty() {
                return pick(key.rotate_left(17) ^ 0x9e37_79b9_7f4a_7c15, &others);
            }
        }
        first
    }

    fn nodes(&mut self, nodes: &[Node], ctx: &Context, slot: &str) -> Result<String, RenderError> {
        let mut out = String::new();
        for (i, n) in nodes.iter().enumerate() {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Var(v) => out.push_str(&lookup(ctx, v)?.text()),
                Node::Choice(options) => {
                    let key = hash(&[
                        &self.seed.to_le_bytes(),
                        slot.as_bytes(),
                        canonical(ctx).as_bytes(),
                        &(i as u64).to_le_bytes(),
                    ]);
                    out.push_str(&options[(key % options.len() as u64) as usize]);
                }
                Node::Call(other) => {
                    if self.depth >= 8 {
                        return Err(RenderError(format!(
                            "slots call each other too deeply at {other}"
                        )));
                    }
                    self.depth += 1;
                    let r = self.try_render(other, ctx);
                    self.depth -= 1;
                    out.push_str(&r?);
                }
                Node::If {
                    cond,
                    then,
                    otherwise,
                } => {
                    let taken = eval(cond, ctx)?;
                    if let Some(t) = self.open.last_mut() {
                        t.branches.push(taken);
                    }
                    let branch = if taken { then } else { otherwise };
                    out.push_str(&self.nodes(branch, ctx, slot)?);
                }
                // Damage filtering arrives in M08; until then text is whole.
                Node::Damaged(inner) => out.push_str(&self.nodes(inner, ctx, slot)?),
                Node::Helper { name, args } => out.push_str(&self.apply(name, args, ctx)?),
            }
        }
        Ok(out)
    }

    /// Applies a helper. Helpers chain: `{cap list strokes}` applies `list`
    /// to `strokes`, then `cap` to the result.
    fn apply(&self, name: &str, args: &[Arg], ctx: &Context) -> Result<String, RenderError> {
        if let Some(Arg::Var(inner)) = args.first() {
            if is_helper(inner) && !ctx.contains_key(inner) {
                let value = self.apply(inner, &args[1..], ctx)?;
                return self.helper(name, &[Value::Text(value)]);
            }
        }
        let values: Vec<Value> = args
            .iter()
            .enumerate()
            .map(|(i, a)| match a {
                Arg::Var(v) => match ctx.get(v) {
                    Some(x) => Ok(x.clone()),
                    None if word_position(name, i) => Ok(Value::Text(v.clone())),
                    None => Err(RenderError(format!("no variable '{v}' here"))),
                },
                Arg::Text(t) => Ok(Value::Text(t.clone())),
                Arg::Number(n) => Ok(Value::Number(*n)),
            })
            .collect::<Result<_, _>>()?;
        self.helper(name, &values)
    }

    fn helper(&self, name: &str, args: &[Value]) -> Result<String, RenderError> {
        if name.starts_with("lang.") {
            return self
                .hooks
                .call(name, args)
                .ok_or_else(|| RenderError(format!("no language hook '{name}' here")));
        }
        let Some(&Helper { arity, usage, .. }) = HELPERS.iter().find(|h| h.name == name) else {
            return Err(RenderError(format!("unknown helper '{name}'")));
        };
        if args.len() != arity {
            return Err(RenderError(format!(
                "wrong number of arguments; use {usage}"
            )));
        }
        let num = |v: &Value| {
            v.number()
                .ok_or_else(|| RenderError(format!("{name} needs a number; use {usage}")))
        };
        Ok(match name {
            "a" => english::article(&args[0].text()),
            "cap" => english::capitalise(&args[0].text()),
            "plural" => english::plural_if(&args[0].text(), num(&args[1])?),
            "count" => {
                let n = num(&args[0])?;
                format!(
                    "{} {}",
                    english::number(n),
                    english::plural_if(&args[1].text(), n)
                )
            }
            "number" => english::number(num(&args[0])?),
            "list" => match &args[0] {
                Value::List(items) => {
                    english::list(&items.iter().map(Value::text).collect::<Vec<_>>())
                }
                other => other.text(),
            },
            "bearing" => english::bearing(num(&args[0])?),
            "distance" => english::distance(num(&args[0])?),
            "duration" => english::duration(num(&args[0])?),
            _ => unreachable!("helper table and match agree"),
        })
    }
}

fn lookup<'c>(ctx: &'c Context, name: &str) -> Result<&'c Value, RenderError> {
    ctx.get(name)
        .ok_or_else(|| RenderError(format!("no variable '{name}' here")))
}

/// Evaluates a condition against a context.
pub fn eval(e: &Expr, ctx: &Context) -> Result<bool, RenderError> {
    let val = |o: &Operand| -> Result<Value, RenderError> {
        Ok(match o {
            Operand::Var(v) => lookup(ctx, v)?.clone(),
            Operand::Text(s) => Value::Text(s.clone()),
            Operand::Number(n) => Value::Number(*n),
            Operand::Bool(b) => Value::Bool(*b),
        })
    };
    Ok(match e {
        Expr::Or(parts) => {
            for p in parts {
                if eval(p, ctx)? {
                    return Ok(true);
                }
            }
            false
        }
        Expr::And(parts) => {
            for p in parts {
                if !eval(p, ctx)? {
                    return Ok(false);
                }
            }
            true
        }
        Expr::Not(inner) => !eval(inner, ctx)?,
        Expr::Truthy(o) => val(o)?.truthy(),
        Expr::Cmp { left, op, right } => {
            let (l, r) = (val(left)?, val(right)?);
            match (op, l.number(), r.number()) {
                (Op::Has, _, _) => match &l {
                    Value::List(items) => items.iter().any(|i| i == &r || i.text() == r.text()),
                    _ => return Err(RenderError("'has' needs a list on its left".into())),
                },
                (Op::Eq, _, _) => l == r || l.text() == r.text(),
                (Op::Ne, _, _) => !(l == r || l.text() == r.text()),
                (op, Some(a), Some(b)) => match op {
                    Op::Lt => a < b,
                    Op::Gt => a > b,
                    Op::Le => a <= b,
                    Op::Ge => a >= b,
                    _ => unreachable!(),
                },
                _ => return Err(RenderError("<, >, <= and >= compare numbers only".into())),
            }
        }
    })
}

/// A stable text form of a context, for hashing.
fn canonical(ctx: &Context) -> String {
    serde_json::to_string(ctx).expect("context serialises")
}

/// FNV-1a over byte slices: stable on every platform, unlike std's hasher.
fn hash(parts: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in parts {
        for &b in p.iter().chain([0xffu8].iter()) {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackFile;
    use crate::slot::{SlotDef, VarType};

    fn setup(variants: &[(&str, &str, Option<&str>)]) -> (Registry, Pack) {
        let reg = Registry::new(vec![
            SlotDef::new("t.one", "test").var("n", VarType::Number, "a number"),
            SlotDef::new("t.two", "test"),
        ]);
        let pack = Pack {
            files: vec![PackFile {
                path: "t.toml".into(),
                notes: None,
                variants: variants
                    .iter()
                    .map(|(slot, text, when)| Variant {
                        slot: slot.to_string(),
                        text: text.to_string(),
                        when: when.map(str::to_string),
                        weight: 1,
                        example: false,
                    })
                    .collect(),
                storylets: Vec::new(),
            }],
        };
        (reg, pack)
    }

    fn ctx(n: i64) -> Context {
        [("n".to_string(), Value::Number(n))].into_iter().collect()
    }

    #[test]
    fn conditions_choose_variants() {
        let (reg, pack) = setup(&[
            ("t.one", "few", Some("n < 3")),
            ("t.one", "many", Some("n >= 3")),
        ]);
        let mut r = Renderer::new(&reg, &pack, 1, &NoHooks);
        assert_eq!(r.render("t.one", &ctx(1)), "few");
        assert_eq!(r.render("t.one", &ctx(5)), "many");
    }

    #[test]
    fn empty_slots_show_placeholders() {
        let (reg, pack) = setup(&[]);
        let mut r = Renderer::new(&reg, &pack, 1, &NoHooks);
        assert_eq!(r.render("t.two", &Context::new()), "⟦slot: t.two⟧");
    }

    #[test]
    fn helpers_calls_and_inline_ifs() {
        let (reg, pack) = setup(&[
            ("t.one", "{count n jar}[if n > 1], {>t.two}[end]", None),
            ("t.two", "all {number n} of them", None),
        ]);
        let mut r = Renderer::new(&reg, &pack, 1, &NoHooks);
        assert_eq!(r.render("t.one", &ctx(3)), "three jars, all three of them");
        assert_eq!(r.render("t.one", &ctx(1)), "one jar");
    }

    #[test]
    fn helpers_chain() {
        let reg = Registry::new(vec![SlotDef::new("t.l", "test").var(
            "xs",
            VarType::List,
            "items",
        )]);
        let pack = Pack {
            files: vec![PackFile {
                path: "t.toml".into(),
                notes: None,
                variants: vec![Variant {
                    slot: "t.l".into(),
                    text: "{cap list xs}.".into(),
                    when: None,
                    weight: 1,
                    example: false,
                }],
                storylets: Vec::new(),
            }],
        };
        let c: Context = [(
            "xs".to_string(),
            Value::List(vec!["a hook".into(), "a dot".into()]),
        )]
        .into_iter()
        .collect();
        assert_eq!(
            Renderer::new(&reg, &pack, 1, &NoHooks).render("t.l", &c),
            "A hook and a dot."
        );
    }

    #[test]
    fn same_inputs_same_choice_and_no_immediate_repeats() {
        let (reg, pack) = setup(&[
            ("t.two", "a", None),
            ("t.two", "b", None),
            ("t.two", "c", None),
        ]);
        let mut r1 = Renderer::new(&reg, &pack, 7, &NoHooks);
        let mut r2 = Renderer::new(&reg, &pack, 7, &NoHooks);
        let seq1: Vec<String> = (0..10)
            .map(|_| r1.render("t.two", &Context::new()))
            .collect();
        let seq2: Vec<String> = (0..10)
            .map(|_| r2.render("t.two", &Context::new()))
            .collect();
        assert_eq!(seq1, seq2);
        assert!(seq1.windows(2).all(|w| w[0] != w[1]), "{seq1:?}");
    }

    #[test]
    fn errors_render_inline() {
        let (reg, pack) = setup(&[("t.two", "{missing}", None), ("t.one", "{>t.one}", None)]);
        let mut r = Renderer::new(&reg, &pack, 1, &NoHooks);
        assert!(r.render("t.two", &Context::new()).starts_with("⟦error:"));
        assert!(r.render("t.one", &ctx(1)).contains("too deeply"));
    }

    #[test]
    fn inline_choices_are_deterministic() {
        let (reg, pack) = setup(&[("t.two", "{red|green|blue}", None)]);
        let a = Renderer::new(&reg, &pack, 3, &NoHooks).render("t.two", &Context::new());
        let b = Renderer::new(&reg, &pack, 3, &NoHooks).render("t.two", &Context::new());
        assert_eq!(a, b);
    }
}
