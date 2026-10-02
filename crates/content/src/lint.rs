//! Lint and coverage: what is wrong with the pack, and what is missing.
//!
//! Lint runs as Jb types in the authoring tool, so messages say what is
//! wrong and how to fix it, in plain words.

use serde::Serialize;

use crate::pack::{Pack, PackError, Variant};
use crate::render::{eval, is_helper, word_position, Hooks, Renderer, HELPERS};
use crate::slot::{Context, Registry, SlotDef, VarType, SAMPLE_SEEDS};
use crate::template::{self, Expr, Node, Operand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Breaks rendering or the release check.
    Error,
    /// Probably a mistake, or not yet good enough for release.
    Warning,
}

/// One problem found by lint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub slot: String,
    /// File and position of the variant, when the issue belongs to one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<usize>,
    /// Short machine-readable kind, e.g. `unknown-variable`.
    pub kind: &'static str,
    pub message: String,
}

/// Checks one variant against its slot. Used by full lint and by the
/// authoring tool as Jb types.
pub fn lint_variant(registry: &Registry, slot: &SlotDef, v: &Variant) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut issue = |severity, kind, message: String| {
        out.push(Issue {
            severity,
            slot: slot.id.clone(),
            file: None,
            variant: None,
            kind,
            message,
        })
    };
    let nodes = match template::parse(&v.text) {
        Ok(n) => n,
        Err(e) => {
            issue(
                Severity::Error,
                "syntax",
                format!("text, character {}: {}", e.at + 1, e.message),
            );
            return out;
        }
    };
    let cond = match v.when.as_deref().map(template::parse_expr) {
        Some(Err(e)) => {
            issue(
                Severity::Error,
                "syntax",
                format!("condition, character {}: {}", e.at + 1, e.message),
            );
            None
        }
        Some(Ok(c)) => Some(c),
        None => None,
    };

    let mut vars = template::variables(&nodes);
    if let Some(c) = &cond {
        vars.extend(template::expr_variables(c));
    }
    vars.sort();
    vars.dedup();
    let words = word_args(&nodes);
    for var in vars {
        if slot.variable(&var).is_none() && !words.contains(&var) {
            issue(
                Severity::Error,
                "unknown-variable",
                format!(
                    "'{var}' is not a variable of this slot (it has: {})",
                    var_names(slot)
                ),
            );
        }
    }
    check_calls_and_helpers(&nodes, registry, &mut issue);
    let mut conds: Vec<&Expr> = cond.iter().collect();
    collect_ifs(&nodes, &mut conds);
    for c in conds {
        check_types(c, slot, &mut issue);
    }
    out
}

fn var_names(slot: &SlotDef) -> String {
    if slot.variables.is_empty() {
        "none".to_string()
    } else {
        slot.variables
            .iter()
            .map(|v| v.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Bare words in helper word positions (`{count n jar}`) and language
/// hooks are literals, not variables.
fn word_args(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            Node::Helper { name, args } => {
                out.extend(args.iter().enumerate().filter_map(|(i, a)| match a {
                    template::Arg::Var(v) if word_position(name, i) || (i == 0 && is_helper(v)) => {
                        Some(v.clone())
                    }
                    _ => None,
                }))
            }
            Node::If {
                then, otherwise, ..
            } => {
                out.extend(word_args(then));
                out.extend(word_args(otherwise));
            }
            Node::Damaged(inner) => out.extend(word_args(inner)),
            _ => {}
        }
    }
    out
}

fn collect_ifs<'a>(nodes: &'a [Node], out: &mut Vec<&'a Expr>) {
    for n in nodes {
        match n {
            Node::If {
                cond,
                then,
                otherwise,
            } => {
                out.push(cond);
                collect_ifs(then, out);
                collect_ifs(otherwise, out);
            }
            Node::Damaged(inner) => collect_ifs(inner, out),
            _ => {}
        }
    }
}

fn check_calls_and_helpers(
    nodes: &[Node],
    registry: &Registry,
    issue: &mut impl FnMut(Severity, &'static str, String),
) {
    for n in nodes {
        match n {
            Node::Call(slot) if registry.get(slot).is_none() => issue(
                Severity::Error,
                "unknown-slot",
                format!("{{>{slot}}} calls a slot that does not exist"),
            ),
            Node::Helper { name, args } => check_helper(name, args, issue),
            Node::If {
                then, otherwise, ..
            } => {
                check_calls_and_helpers(then, registry, issue);
                check_calls_and_helpers(otherwise, registry, issue);
            }
            Node::Damaged(inner) => check_calls_and_helpers(inner, registry, issue),
            _ => {}
        }
    }
}

/// Checks a (possibly chained) helper call.
fn check_helper(
    name: &str,
    args: &[template::Arg],
    issue: &mut impl FnMut(Severity, &'static str, String),
) {
    if name.starts_with("lang.") {
        return;
    }
    let Some(h) = HELPERS.iter().find(|h| h.name == name) else {
        issue(
            Severity::Error,
            "unknown-helper",
            format!(
                "'{name}' is not a helper (helpers: {})",
                HELPERS
                    .iter()
                    .map(|h| h.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return;
    };
    if let Some(template::Arg::Var(inner)) = args.first() {
        if is_helper(inner) && args.len() > h.arity {
            check_helper(inner, &args[1..], issue);
            return;
        }
    }
    if args.len() != h.arity {
        issue(
            Severity::Error,
            "helper-arguments",
            format!("wrong number of arguments; use {}", h.usage),
        );
    }
}

/// Comparisons must make sense for the variable's type.
fn check_types(e: &Expr, slot: &SlotDef, issue: &mut impl FnMut(Severity, &'static str, String)) {
    match e {
        Expr::Or(v) | Expr::And(v) => v.iter().for_each(|x| check_types(x, slot, issue)),
        Expr::Not(x) => check_types(x, slot, issue),
        Expr::Truthy(_) => {}
        Expr::Cmp { left, right, .. } => {
            for (var, lit) in [(left, right), (right, left)] {
                let Operand::Var(name) = var else { continue };
                let Some(def) = slot.variable(name) else {
                    continue;
                };
                match (&def.ty, lit) {
                    (VarType::Enum { values }, Operand::Text(t)) if !values.contains(t) => issue(
                        Severity::Error,
                        "impossible-value",
                        format!("'{name}' is never '{t}'; it can be: {}", values.join(", ")),
                    ),
                    (VarType::Number, Operand::Text(t)) => issue(
                        Severity::Error,
                        "type",
                        format!("'{name}' is a number, so comparing it with the text '{t}' never matches"),
                    ),
                    (VarType::Text | VarType::Enum { .. }, Operand::Number(n)) => issue(
                        Severity::Error,
                        "type",
                        format!("'{name}' is text, so comparing it with the number {n} never matches"),
                    ),
                    _ => {}
                }
            }
        }
    }
}

/// Edit distance over characters, for near-duplicate detection.
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(x != y))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn normalise(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lints the whole pack against the registry.
pub fn lint(
    registry: &Registry,
    pack: &Pack,
    errors: &[PackError],
    hooks: &dyn Hooks,
) -> Vec<Issue> {
    let mut out: Vec<Issue> = errors
        .iter()
        .map(|e| Issue {
            severity: Severity::Error,
            slot: String::new(),
            file: Some(e.path.clone()),
            variant: None,
            kind: "file",
            message: format!("this file could not be read: {}", e.message),
        })
        .collect();
    for (file, i, v) in pack.all_variants() {
        let place = |mut x: Issue| {
            x.file = Some(file.path.clone());
            x.variant = Some(i);
            x
        };
        let Some(slot) = registry.get(&v.slot) else {
            out.push(place(Issue {
                severity: Severity::Error,
                slot: v.slot.clone(),
                file: None,
                variant: None,
                kind: "unknown-slot",
                message: format!(
                    "there is no slot called '{}' (was it renamed or removed?)",
                    v.slot
                ),
            }));
            continue;
        };
        out.extend(lint_variant(registry, slot, v).into_iter().map(place));
    }
    for slot in &registry.slots {
        out.extend(slot_issues(registry, pack, slot, hooks));
    }
    out.sort_by(|a, b| {
        (a.severity, &a.slot, &a.file, a.variant).cmp(&(b.severity, &b.slot, &b.file, b.variant))
    });
    out
}

/// Slot-level issues: too few variants, unreachable conditions, over-long
/// text and near duplicates, judged against sample contexts.
fn slot_issues(registry: &Registry, pack: &Pack, slot: &SlotDef, hooks: &dyn Hooks) -> Vec<Issue> {
    let mut out = Vec::new();
    let variants = pack.variants(&slot.id);
    let real = variants.iter().filter(|v| !v.example).count();
    let mk = |severity, kind, message: String| Issue {
        severity,
        slot: slot.id.clone(),
        file: None,
        variant: None,
        kind,
        message,
    };
    if !variants.is_empty() && real < slot.min_variants {
        out.push(mk(
            Severity::Warning,
            "too-few-variants",
            format!(
                "{real} written variant{} of the {} needed for release",
                if real == 1 { "" } else { "s" },
                slot.min_variants
            ),
        ));
    }
    let samples = slot.samples(&SAMPLE_SEEDS);
    for v in &variants {
        if let Some(cond) = v.when.as_deref().and_then(|w| template::parse_expr(w).ok()) {
            if !samples.is_empty() && !samples.iter().any(|(_, c)| eval(&cond, c).unwrap_or(false))
            {
                out.push(mk(
                    Severity::Warning,
                    "unreachable",
                    format!(
                        "the condition '{}' was never true in {} sample situations",
                        v.when.as_deref().unwrap_or(""),
                        samples.len()
                    ),
                ));
            }
        }
    }
    let mut longest = 0;
    for (seed, ctx) in samples.iter().take(60) {
        let mut r = Renderer::new(registry, pack, *seed, hooks);
        longest = longest.max(r.render(&slot.id, ctx).chars().count());
    }
    if longest > slot.max_len {
        out.push(mk(
            Severity::Warning,
            "too-long",
            format!(
                "renders up to {longest} characters; the limit is {}",
                slot.max_len
            ),
        ));
    }
    for (i, a) in variants.iter().enumerate() {
        for b in &variants[i + 1..] {
            let (x, y) = (normalise(&a.text), normalise(&b.text));
            let d = distance(&x, &y);
            if d * 8 < x.chars().count().max(y.chars().count()) {
                out.push(mk(
                    Severity::Warning,
                    "near-duplicate",
                    format!(
                        "two variants are almost the same: \"{}\" and \"{}\"",
                        a.text, b.text
                    ),
                ));
            }
        }
    }
    out
}

/// How complete a slot is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// No variants at all.
    Empty,
    /// Only agent-written examples.
    ExampleOnly,
    /// Some written variants, fewer than needed.
    UnderFilled,
    /// Some sample situations have no variant that applies.
    Partial,
    Ok,
}

/// One slot's coverage.
#[derive(Debug, Clone, Serialize)]
pub struct Coverage {
    pub slot: String,
    pub status: Status,
    pub variants: usize,
    pub written: usize,
    pub needed: usize,
    pub required: bool,
    /// Sample situations with no applicable variant.
    pub uncovered: usize,
    pub samples: usize,
    pub errors: usize,
}

/// Coverage for every slot.
pub fn coverage(registry: &Registry, pack: &Pack, issues: &[Issue]) -> Vec<Coverage> {
    registry
        .slots
        .iter()
        .map(|slot| {
            let variants = pack.variants(&slot.id);
            let written = variants.iter().filter(|v| !v.example).count();
            let samples = slot.samples(&SAMPLE_SEEDS);
            let applies = |c: &Context, v: &Variant| match &v.when {
                None => true,
                Some(w) => template::parse_expr(w)
                    .ok()
                    .is_some_and(|e| eval(&e, c).unwrap_or(false)),
            };
            let uncovered = samples
                .iter()
                .filter(|(_, c)| !variants.iter().any(|v| applies(c, v)))
                .count();
            let errors = issues
                .iter()
                .filter(|i| i.slot == slot.id && i.severity == Severity::Error)
                .count();
            let status = if variants.is_empty() {
                Status::Empty
            } else if written == 0 {
                Status::ExampleOnly
            } else if written < slot.min_variants {
                Status::UnderFilled
            } else if uncovered > 0 {
                Status::Partial
            } else {
                Status::Ok
            };
            Coverage {
                slot: slot.id.clone(),
                status,
                variants: variants.len(),
                written,
                needed: slot.min_variants,
                required: slot.required,
                uncovered,
                samples: samples.len(),
                errors,
            }
        })
        .collect()
}

/// Release check: every required slot fully written and no lint errors.
/// Returns the reasons it fails; empty means the pack can ship.
pub fn release_check(registry: &Registry, pack: &Pack, issues: &[Issue]) -> Vec<String> {
    let mut out: Vec<String> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| {
            format!(
                "{}: {}",
                if i.slot.is_empty() {
                    i.file.as_deref().unwrap_or("")
                } else {
                    &i.slot
                },
                i.message
            )
        })
        .collect();
    for c in coverage(registry, pack, issues) {
        if c.required && c.status != Status::Ok {
            out.push(format!(
                "{}: {:?} ({} of {} written)",
                c.slot, c.status, c.written, c.needed
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackFile;
    use crate::render::NoHooks;
    use crate::slot::{SlotDef, Value};

    fn sampler(_: u64) -> Vec<Context> {
        (0..4)
            .map(|n| {
                [
                    ("n".to_string(), Value::Number(n)),
                    (
                        "kind".to_string(),
                        Value::Text(["hook", "dot"][n as usize % 2].into()),
                    ),
                ]
                .into_iter()
                .collect()
            })
            .collect()
    }

    fn registry() -> Registry {
        Registry::new(vec![SlotDef::new("t.one", "test")
            .var("n", VarType::Number, "a number")
            .var(
                "kind",
                VarType::Enum {
                    values: vec!["hook".into(), "dot".into()],
                },
                "which",
            )
            .max_len(30)
            .sampler(sampler)])
    }

    fn pack(variants: &[(&str, Option<&str>, bool)]) -> Pack {
        Pack {
            files: vec![PackFile {
                path: "t.toml".into(),
                notes: None,
                variants: variants
                    .iter()
                    .map(|(t, w, ex)| Variant {
                        slot: "t.one".into(),
                        text: t.to_string(),
                        when: w.map(str::to_string),
                        weight: 1,
                        example: *ex,
                    })
                    .collect(),
                storylets: Vec::new(),
            }],
        }
    }

    fn kinds(issues: &[Issue]) -> Vec<&'static str> {
        issues.iter().map(|i| i.kind).collect()
    }

    #[test]
    fn each_lint_has_a_fixture() {
        let reg = registry();
        type Case<'a> = (&'a [(&'a str, Option<&'a str>, bool)], &'a str);
        let cases: &[Case] = &[
            (&[("{nope}", None, false)], "unknown-variable"),
            (&[("{>t.missing}", None, false)], "unknown-slot"),
            (&[("{shout n}", None, false)], "unknown-helper"),
            (&[("{count n}", None, false)], "helper-arguments"),
            (&[("{oops", None, false)], "syntax"),
            (&[("x", Some("kind == 'ring'"), false)], "impossible-value"),
            (&[("x", Some("n == 'three'"), false)], "type"),
            (&[("x", Some("n > 10"), false)], "unreachable"),
            (&[("only one", None, false)], "too-few-variants"),
            (
                &[(
                    "{count n jar}, and then a very long tail of words",
                    None,
                    false,
                )],
                "too-long",
            ),
            (
                &[
                    ("the hook is bent", None, false),
                    ("the hook is bent.", None, false),
                ],
                "near-duplicate",
            ),
        ];
        for (variants, kind) in cases {
            let issues = lint(&reg, &pack(variants), &[], &NoHooks);
            assert!(kinds(&issues).contains(kind), "{kind}: {issues:?}");
        }
        let mut bad = pack(&[]);
        bad.files[0].variants.push(Variant {
            slot: "t.gone".into(),
            text: "x".into(),
            when: None,
            weight: 1,
            example: false,
        });
        assert!(kinds(&lint(&reg, &bad, &[], &NoHooks)).contains(&"unknown-slot"));
    }

    #[test]
    fn clean_pack_has_no_issues_and_is_releasable() {
        let reg = registry();
        let p = pack(&[
            ("a {kind}", None, false),
            ("{count n jar} here", None, false),
        ]);
        let issues = lint(&reg, &p, &[], &NoHooks);
        assert!(issues.is_empty(), "{issues:?}");
        assert!(release_check(&reg, &p, &issues).is_empty());
    }

    #[test]
    fn coverage_statuses() {
        let reg = registry();
        let status = |p: &Pack| coverage(&reg, p, &[])[0].status;
        assert_eq!(status(&pack(&[])), Status::Empty);
        assert_eq!(status(&pack(&[("x", None, true)])), Status::ExampleOnly);
        assert_eq!(status(&pack(&[("x", None, false)])), Status::UnderFilled);
        assert_eq!(
            status(&pack(&[
                ("x", Some("n < 2"), false),
                ("y", Some("n < 1"), false)
            ])),
            Status::Partial
        );
        assert_eq!(
            status(&pack(&[("x", None, false), ("y", None, false)])),
            Status::Ok
        );
        assert!(!release_check(&reg, &pack(&[("x", None, true)]), &[]).is_empty());
    }
}
