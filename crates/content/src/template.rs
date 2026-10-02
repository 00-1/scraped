//! The template language Jb writes in.
//!
//! Small on purpose. Plain text, plus:
//!
//! - `{name}` inserts a variable.
//! - `{a|an|the}` picks one alternative, deterministically.
//! - `{helper args…}` calls a helper, e.g. `{a surface.material}`,
//!   `{count n jar}`, `{list marks}`, `{lang.word gate}`.
//! - `{>slot.id}` inserts another slot's text, rendered with the same
//!   variables.
//! - `[if cond]…[else]…[end]` keeps part of a variant conditional.
//! - `[damaged]…[end]` marks text that can be damaged (reserved for M08).
//! - `{{`, `}}` and `[[` write a literal `{`, `}` or `[`.
//!
//! Conditions use `==`, `!=`, `<`, `>`, `<=`, `>=`, `and`, `or`, `not`,
//! parentheses, variable names, numbers and quoted text.

use serde::Serialize;

/// A parsed template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Node {
    Text(String),
    Var(String),
    Helper {
        name: String,
        args: Vec<Arg>,
    },
    Choice(Vec<String>),
    Call(String),
    If {
        cond: Expr,
        then: Vec<Node>,
        otherwise: Vec<Node>,
    },
    Damaged(Vec<Node>),
}

/// An argument to a helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Arg {
    Var(String),
    Text(String),
    Number(i64),
}

/// A parse error with the character position where it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParseError {
    pub at: usize,
    pub message: String,
}

fn err<T>(at: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError {
        at,
        message: message.into(),
    })
}

/// Parses a template.
pub fn parse(src: &str) -> Result<Vec<Node>, ParseError> {
    let chars: Vec<char> = src.chars().collect();
    let mut pos = 0;
    let (nodes, end) = parse_block(&chars, &mut pos)?;
    match end {
        BlockEnd::Eof => Ok(nodes),
        BlockEnd::Else => err(pos, "[else] without [if]"),
        BlockEnd::End => err(pos, "[end] without [if] or [damaged]"),
    }
}

#[derive(PartialEq)]
enum BlockEnd {
    Eof,
    Else,
    End,
}

fn starts(chars: &[char], pos: usize, s: &str) -> bool {
    s.chars()
        .enumerate()
        .all(|(i, c)| chars.get(pos + i) == Some(&c))
}

fn parse_block(chars: &[char], pos: &mut usize) -> Result<(Vec<Node>, BlockEnd), ParseError> {
    let mut nodes = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, nodes: &mut Vec<Node>| {
        if !text.is_empty() {
            nodes.push(Node::Text(std::mem::take(text)));
        }
    };
    while *pos < chars.len() {
        let c = chars[*pos];
        if starts(chars, *pos, "{{") || starts(chars, *pos, "}}") || starts(chars, *pos, "[[") {
            text.push(c);
            *pos += 2;
        } else if c == '{' {
            flush(&mut text, &mut nodes);
            let start = *pos;
            let close = (start..chars.len()).find(|&i| chars[i] == '}');
            let Some(close) = close else {
                return err(start, "'{' is never closed");
            };
            let inner: String = chars[start + 1..close].iter().collect();
            nodes.push(parse_brace(inner.trim(), start)?);
            *pos = close + 1;
        } else if c == '}' {
            return err(*pos, "'}' without '{' (write '}}' for a literal brace)");
        } else if starts(chars, *pos, "[if ") {
            flush(&mut text, &mut nodes);
            let start = *pos;
            let Some(close) = (start..chars.len()).find(|&i| chars[i] == ']') else {
                return err(start, "[if …] is never closed");
            };
            let cond_src: String = chars[start + 4..close].iter().collect();
            let cond = parse_expr(&cond_src).map_err(|e| ParseError {
                at: start + 4 + e.at,
                message: e.message,
            })?;
            *pos = close + 1;
            let (then, end) = parse_block(chars, pos)?;
            let otherwise = match end {
                BlockEnd::Else => {
                    let (o, end2) = parse_block(chars, pos)?;
                    if end2 != BlockEnd::End {
                        return err(start, "[if …] needs an [end]");
                    }
                    o
                }
                BlockEnd::End => Vec::new(),
                BlockEnd::Eof => return err(start, "[if …] needs an [end]"),
            };
            nodes.push(Node::If {
                cond,
                then,
                otherwise,
            });
        } else if starts(chars, *pos, "[damaged]") {
            flush(&mut text, &mut nodes);
            let start = *pos;
            *pos += "[damaged]".len();
            let (inner, end) = parse_block(chars, pos)?;
            if end != BlockEnd::End {
                return err(start, "[damaged] needs an [end]");
            }
            nodes.push(Node::Damaged(inner));
        } else if starts(chars, *pos, "[else]") {
            flush(&mut text, &mut nodes);
            *pos += "[else]".len();
            return Ok((nodes, BlockEnd::Else));
        } else if starts(chars, *pos, "[end]") {
            flush(&mut text, &mut nodes);
            *pos += "[end]".len();
            return Ok((nodes, BlockEnd::End));
        } else {
            text.push(c);
            *pos += 1;
        }
    }
    flush(&mut text, &mut nodes);
    Ok((nodes, BlockEnd::Eof))
}

fn parse_brace(inner: &str, at: usize) -> Result<Node, ParseError> {
    if inner.is_empty() {
        return err(at, "empty {}");
    }
    if let Some(slot) = inner.strip_prefix('>') {
        let slot = slot.trim();
        if !is_name(slot) {
            return err(at, format!("'{slot}' is not a slot id"));
        }
        return Ok(Node::Call(slot.to_string()));
    }
    if inner.contains('|') {
        return Ok(Node::Choice(
            inner.split('|').map(|s| s.to_string()).collect(),
        ));
    }
    let tokens = split_args(inner).map_err(|m| ParseError { at, message: m })?;
    if tokens.len() == 1 {
        return match &tokens[0] {
            Arg::Var(v) => Ok(Node::Var(v.clone())),
            _ => err(at, "a lone literal in {} does nothing"),
        };
    }
    let Arg::Var(name) = &tokens[0] else {
        return err(at, "a helper name must come first");
    };
    Ok(Node::Helper {
        name: name.clone(),
        args: tokens[1..].to_vec(),
    })
}

/// Identifier: letters, digits, `_` and `.`.
fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && !s.starts_with(|c: char| c.is_ascii_digit())
}

fn split_args(s: &str) -> Result<Vec<Arg>, String> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' || c == '\'' {
            let close = (i + 1..chars.len())
                .find(|&j| chars[j] == c)
                .ok_or("unclosed quote")?;
            out.push(Arg::Text(chars[i + 1..close].iter().collect()));
            i = close + 1;
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if let Ok(n) = word.parse::<i64>() {
                out.push(Arg::Number(n));
            } else if is_name(&word) {
                out.push(Arg::Var(word));
            } else {
                return Err(format!("'{word}' is not a name, number or quoted text"));
            }
        }
    }
    Ok(out)
}

/// A condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Expr {
    Or(Vec<Expr>),
    And(Vec<Expr>),
    Not(Box<Expr>),
    Cmp {
        left: Operand,
        op: Op,
        right: Operand,
    },
    Truthy(Operand),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Operand {
    Var(String),
    Text(String),
    Number(i64),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Name(String),
    Text(String),
    Number(i64),
    Op(Op),
    And,
    Or,
    Not,
    Open,
    Close,
}

fn tokenize(s: &str) -> Result<Vec<(usize, Tok)>, ParseError> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let at = i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        let tok = match (c, two.as_str()) {
            (_, "==") => Tok::Op(Op::Eq),
            (_, "!=") => Tok::Op(Op::Ne),
            (_, "<=") => Tok::Op(Op::Le),
            (_, ">=") => Tok::Op(Op::Ge),
            ('<', _) => Tok::Op(Op::Lt),
            ('>', _) => Tok::Op(Op::Gt),
            ('(', _) => Tok::Open,
            (')', _) => Tok::Close,
            ('"' | '\'', _) => {
                let close = (i + 1..chars.len())
                    .find(|&j| chars[j] == c)
                    .ok_or(ParseError {
                        at,
                        message: "unclosed quote".into(),
                    })?;
                let t = Tok::Text(chars[i + 1..close].iter().collect());
                i = close + 1;
                out.push((at, t));
                continue;
            }
            _ => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '.' | '-'))
                {
                    i += 1;
                }
                if start == i {
                    return err(at, format!("unexpected '{c}'"));
                }
                let word: String = chars[start..i].iter().collect();
                let t = match word.as_str() {
                    "and" => Tok::And,
                    "or" => Tok::Or,
                    "not" => Tok::Not,
                    _ => match word.parse::<i64>() {
                        Ok(n) => Tok::Number(n),
                        Err(_) => Tok::Name(word),
                    },
                };
                out.push((start, t));
                continue;
            }
        };
        i += match tok {
            Tok::Op(Op::Lt | Op::Gt) | Tok::Open | Tok::Close => 1,
            _ => 2,
        };
        out.push((at, tok));
    }
    Ok(out)
}

/// Parses a condition.
pub fn parse_expr(src: &str) -> Result<Expr, ParseError> {
    let toks = tokenize(src)?;
    let mut p = Parser {
        toks: &toks,
        i: 0,
        len: src.chars().count(),
    };
    let e = p.or()?;
    if p.i < toks.len() {
        return err(toks[p.i].0, "unexpected text after the condition");
    }
    Ok(e)
}

struct Parser<'a> {
    toks: &'a [(usize, Tok)],
    i: usize,
    len: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|(_, t)| t)
    }

    fn at(&self) -> usize {
        self.toks.get(self.i).map(|(a, _)| *a).unwrap_or(self.len)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut parts = vec![self.and()?];
        while self.peek() == Some(&Tok::Or) {
            self.i += 1;
            parts.push(self.and()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::Or(parts)
        })
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut parts = vec![self.not()?];
        while self.peek() == Some(&Tok::And) {
            self.i += 1;
            parts.push(self.not()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::And(parts)
        })
    }

    fn not(&mut self) -> Result<Expr, ParseError> {
        if self.peek() == Some(&Tok::Not) {
            self.i += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Expr, ParseError> {
        if self.peek() == Some(&Tok::Open) {
            self.i += 1;
            let e = self.or()?;
            if self.peek() != Some(&Tok::Close) {
                return err(self.at(), "missing ')'");
            }
            self.i += 1;
            return Ok(e);
        }
        let left = self.operand()?;
        if let Some(Tok::Op(op)) = self.peek().cloned() {
            self.i += 1;
            let right = self.operand()?;
            return Ok(Expr::Cmp { left, op, right });
        }
        Ok(Expr::Truthy(left))
    }

    fn operand(&mut self) -> Result<Operand, ParseError> {
        let at = self.at();
        let t = self.peek().cloned();
        self.i += 1;
        match t {
            Some(Tok::Name(n)) if n == "true" => Ok(Operand::Bool(true)),
            Some(Tok::Name(n)) if n == "false" => Ok(Operand::Bool(false)),
            Some(Tok::Name(n)) => Ok(Operand::Var(n)),
            Some(Tok::Text(s)) => Ok(Operand::Text(s)),
            Some(Tok::Number(n)) => Ok(Operand::Number(n)),
            _ => err(at, "expected a name, number or quoted text"),
        }
    }
}

/// Every variable a template or condition mentions.
pub fn variables(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            Node::Var(v) => out.push(v.clone()),
            Node::Helper { args, .. } => out.extend(args.iter().filter_map(|a| match a {
                Arg::Var(v) => Some(v.clone()),
                _ => None,
            })),
            Node::If {
                cond,
                then,
                otherwise,
            } => {
                out.extend(expr_variables(cond));
                out.extend(variables(then));
                out.extend(variables(otherwise));
            }
            Node::Damaged(inner) => out.extend(variables(inner)),
            Node::Text(_) | Node::Choice(_) | Node::Call(_) => {}
        }
    }
    out
}

pub fn expr_variables(e: &Expr) -> Vec<String> {
    match e {
        Expr::Or(v) | Expr::And(v) => v.iter().flat_map(expr_variables).collect(),
        Expr::Not(e) => expr_variables(e),
        Expr::Cmp { left, right, .. } => [left, right]
            .into_iter()
            .filter_map(|o| match o {
                Operand::Var(v) => Some(v.clone()),
                _ => None,
            })
            .collect(),
        Expr::Truthy(Operand::Var(v)) => vec![v.clone()],
        Expr::Truthy(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_construct() {
        let t = parse(
            "{a stroke} turned {turn}, {one|two} {>glyph.x} [if n > 2]many[else]few[end] {{x}}",
        )
        .unwrap();
        assert!(matches!(&t[0], Node::Helper { name, .. } if name == "a"));
        assert_eq!(t[2], Node::Var("turn".into()));
        assert_eq!(t[4], Node::Choice(vec!["one".into(), "two".into()]));
        assert_eq!(t[6], Node::Call("glyph.x".into()));
        assert!(matches!(&t[8], Node::If { .. }));
        assert_eq!(t.last(), Some(&Node::Text(" {x}".into())));
    }

    #[test]
    fn nested_ifs_and_damaged() {
        let t = parse("[if a][if b]x[end][else]y[end][damaged]z[end]").unwrap();
        assert_eq!(t.len(), 2);
        assert!(matches!(&t[1], Node::Damaged(_)));
    }

    #[test]
    fn errors_point_at_the_problem() {
        assert_eq!(parse("abc {oops").unwrap_err().at, 4);
        assert!(parse("[if x]never closed").is_err());
        assert!(parse("[end]").is_err());
        assert!(parse("a } b").is_err());
        assert!(parse("{a 'unclosed}").is_err());
    }

    #[test]
    fn conditions() {
        let e = parse_expr("not (a == 'x' or n >= 3) and flag").unwrap();
        assert!(matches!(e, Expr::And(_)));
        assert_eq!(expr_variables(&e), ["a", "n", "flag"]);
        assert!(parse_expr("a ==").is_err());
        assert!(parse_expr("a b").is_err());
        assert!(parse_expr("(a").is_err());
    }
}
