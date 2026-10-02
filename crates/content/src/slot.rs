//! Slots: the places where the engine needs hand-written text.
//!
//! Engine code declares every slot with a description written for Jb, the
//! variables a template may use, and a sampler that builds example contexts
//! from real seeds, so the authoring tool can preview text against real data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A value a template can use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Number(i64),
    Text(String),
    List(Vec<Value>),
}

impl Value {
    /// The value as text, for insertion into a template.
    pub fn text(&self) -> String {
        match self {
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Text(s) => s.clone(),
            Value::List(items) => items.iter().map(Value::text).collect::<Vec<_>>().join(", "),
        }
    }

    /// Truth in a condition: false, zero, empty text and empty lists are
    /// false.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0,
            Value::Text(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
        }
    }

    pub fn number(&self) -> Option<i64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

/// Variable name → value. Names may contain dots (`surface.material`).
pub type Context = BTreeMap<String, Value>;

/// What kind of value a variable holds; drives lint checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VarType {
    Text,
    Number,
    Bool,
    /// One of a fixed set of words.
    Enum {
        values: Vec<String>,
    },
    /// A list of texts, for `{list x}`.
    List,
}

/// A variable a slot offers to its templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: VarType,
    /// What the variable means, written for Jb.
    pub description: String,
}

/// Builds example contexts for a slot from a seed.
pub type Sampler = fn(u64) -> Vec<Context>;

/// A place where the engine needs text.
#[derive(Debug, Clone, Serialize)]
pub struct SlotDef {
    pub id: String,
    /// When the text appears, what it must convey and what it must not
    /// reveal. Written for Jb, not for programmers.
    pub description: String,
    pub variables: Vec<VarDef>,
    /// Real (non-example) variants needed before release.
    pub min_variants: usize,
    /// Whether the release check requires this slot to be filled.
    pub required: bool,
    /// Longest acceptable rendering, in characters.
    pub max_len: usize,
    #[serde(skip)]
    pub sampler: Option<Sampler>,
}

impl SlotDef {
    /// Starts a slot declaration with sensible defaults: required, two real
    /// variants, 200 characters.
    pub fn new(id: &str, description: &str) -> Self {
        SlotDef {
            id: id.to_string(),
            description: description.to_string(),
            variables: Vec::new(),
            min_variants: 2,
            required: true,
            max_len: 200,
            sampler: None,
        }
    }

    pub fn var(mut self, name: &str, ty: VarType, description: &str) -> Self {
        self.variables.push(VarDef {
            name: name.to_string(),
            ty,
            description: description.to_string(),
        });
        self
    }

    pub fn min_variants(mut self, n: usize) -> Self {
        self.min_variants = n;
        self
    }

    pub fn max_len(mut self, n: usize) -> Self {
        self.max_len = n;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn sampler(mut self, f: Sampler) -> Self {
        self.sampler = Some(f);
        self
    }

    /// The slot family: the part of the id before the first dot. One pack
    /// file per family.
    pub fn family(&self) -> &str {
        self.id.split('.').next().unwrap_or(&self.id)
    }

    pub fn variable(&self, name: &str) -> Option<&VarDef> {
        self.variables.iter().find(|v| v.name == name)
    }

    /// Example contexts from the given seeds.
    pub fn samples(&self, seeds: &[u64]) -> Vec<(u64, Context)> {
        let Some(f) = self.sampler else {
            return Vec::new();
        };
        seeds
            .iter()
            .flat_map(|&s| f(s).into_iter().map(move |c| (s, c)))
            .collect()
    }
}

/// Every slot the engine declares.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Registry {
    pub slots: Vec<SlotDef>,
}

impl Registry {
    pub fn new(mut slots: Vec<SlotDef>) -> Self {
        slots.sort_by(|a, b| a.id.cmp(&b.id));
        Registry { slots }
    }

    pub fn get(&self, id: &str) -> Option<&SlotDef> {
        self.slots.iter().find(|s| s.id == id)
    }
}

/// Seeds used for samples by lint, coverage and the authoring tool.
pub const SAMPLE_SEEDS: [u64; 6] = [1, 2, 3, 42, 777, 9001];
