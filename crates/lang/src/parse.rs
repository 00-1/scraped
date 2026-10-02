//! Surface → meaning: the renderer run backwards.
//!
//! The parser mirrors `render.rs` construction by construction. At each
//! point it generates the forms the grammar allows there (every noun in
//! every number for the case the slot needs, every verb in every tense and
//! polarity, every numeral) and keeps those that match the input. Because
//! the renderer is deterministic and every surface word has one analysis
//! (M01), a rendered sentence parses back to the meaning it came from.
//!
//! Input is a stream of symbols with gaps between words: phonemes, or
//! glyphs as a player writes them. Words run together where the script
//! does not separate them; the parser copes with that too.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::concepts::{self, Pos};
use crate::meaning::{Argument, Clause, Head, Mood, NounPhrase, Role, Sentence};
use crate::morphology::{Case, Number, Polarity, Tense};
use crate::render::{Renderer, Word};
use crate::script::GlyphKey;
use crate::syntax::{Side, WordOrder};

/// One symbol of input, or a gap between words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tok {
    Sym(String),
    Gap,
}

/// What symbols a word is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Phonemes (exact; for tests and debugging).
    Phonemes,
    /// Glyphs, as written: what a player can produce.
    Glyphs,
}

/// How deep possessors may nest ("the son of the king of the city").
const DEPTH: usize = 3;

pub struct Parser<'a> {
    r: &'a Renderer<'a>,
    mode: Mode,
    /// Plain words (adjectives, determiners, adverbs, particles) by symbols.
    plain: BTreeMap<Vec<String>, String>,
    /// Numeral word groups by value, as each word's symbols.
    numerals: Vec<(u16, Vec<Vec<String>>)>,
    /// Head forms with particles, per case.
    heads: Vec<(Case, Vec<Form<HeadKey>>)>,
    /// Verb groups: (verb, tense, polarity, potent) → words' symbols.
    verbs: Vec<Form<VerbKey>>,
    /// Noun phrases already parsed at a position.
    memo: RefCell<BTreeMap<MemoKey, Alt<NounPhrase>>>,
}

/// Something, and the symbols of each of its words.
type Form<T> = (T, Vec<Vec<String>>);
type VerbKey = (String, Tense, Polarity, bool);
type HeadKey = (Head, Number);
/// (position, case, depth, appositions allowed).
type MemoKey = (usize, Case, usize, bool);

/// A partial parse: something, and where the input continues.
type Alt<T> = Vec<(T, usize)>;

impl<'a> Parser<'a> {
    pub fn new(r: &'a Renderer<'a>, mode: Mode) -> Self {
        Self::build(r, mode, true)
    }

    /// A parser that knows no personal names: for text a player writes,
    /// where the renderer's name table may belong to other eras.
    pub fn without_names(r: &'a Renderer<'a>, mode: Mode) -> Self {
        Self::build(r, mode, false)
    }

    fn build(r: &'a Renderer<'a>, mode: Mode, names: bool) -> Self {
        let mut p = Parser {
            r,
            mode,
            plain: BTreeMap::new(),
            numerals: Vec::new(),
            heads: Vec::new(),
            verbs: Vec::new(),
            memo: RefCell::new(BTreeMap::new()),
        };
        for c in concepts::all() {
            if matches!(c.pos, Pos::Adj | Pos::Det | Pos::Adv | Pos::Particle)
                && r.lang.lexicon.has(&c.id)
            {
                let syms = p.syms(&r.plain(&c.id));
                p.plain.entry(syms).or_insert_with(|| c.id.clone());
            }
        }
        for n in 0..=999u16 {
            let words: Vec<Vec<String>> = r
                .lang
                .numerals
                .words(n)
                .iter()
                .map(|w| p.word_syms(&r.plain(w)))
                .collect();
            p.numerals.push((n, words));
        }
        let mut heads: Vec<Head> = concepts::with_pos(Pos::Noun)
            .filter(|c| r.lang.lexicon.has(&c.id))
            .map(|c| Head::Concept(c.id.clone()))
            .collect();
        if names {
            heads.extend((0..r.names.len()).map(Head::Name));
        }
        for case in Case::ALL {
            let mut forms = Vec::new();
            for h in &heads {
                for number in [Number::Singular, Number::Plural] {
                    let words: Vec<Vec<String>> = r
                        .head(h, number, case)
                        .iter()
                        .map(|w| p.word_syms(w))
                        .collect();
                    forms.push(((h.clone(), number), words));
                }
            }
            p.heads.push((case, forms));
        }
        for v in concepts::with_pos(Pos::Verb) {
            if !r.lang.lexicon.has(&v.id) {
                continue;
            }
            for tense in [Tense::NonPast, Tense::Past] {
                for polarity in [Polarity::Positive, Polarity::Negative] {
                    for potent in [false, true] {
                        let words: Vec<Vec<String>> = r
                            .verb_group(&v.id, tense, polarity, potent)
                            .iter()
                            .map(|w| p.word_syms(w))
                            .collect();
                        p.verbs
                            .push(((v.id.clone(), tense, polarity, potent), words));
                    }
                }
            }
        }
        p
    }

    /// The symbols of one word.
    fn syms(&self, w: &Word) -> Vec<String> {
        self.word_syms(w)
    }

    fn word_syms(&self, w: &Word) -> Vec<String> {
        let lang = self.r.lang;
        match self.mode {
            Mode::Phonemes => {
                let mut v: Vec<String> = w.phonemes().iter().map(|p| p.to_string()).collect();
                if w.name {
                    v.insert(0, "°".to_string());
                }
                v
            }
            Mode::Glyphs => {
                let mut v = Vec::new();
                if w.name && lang.difficulty.names == crate::difficulty::NameMarking::Determinative
                {
                    v.push(glyph_sym(&GlyphKey::Determinative));
                }
                let ipa = lang.phonology.to_ipa(&w.phonemes());
                v.extend(lang.script.spell(&ipa).iter().map(glyph_sym));
                v
            }
        }
    }

    /// Input tokens for rendered words, as the parser expects them.
    pub fn tokens(&self, words: &[Word]) -> Vec<Tok> {
        let mut out = Vec::new();
        for (i, w) in words.iter().enumerate() {
            if i > 0 {
                out.push(Tok::Gap);
            }
            out.extend(self.word_syms(w).into_iter().map(Tok::Sym));
        }
        out
    }

    /// Matches one word's symbols at `at`, skipping gaps before it. The word
    /// must end at a gap or the end of input (unless words run together).
    fn word(&self, input: &[Tok], at: usize, syms: &[String]) -> Option<usize> {
        let mut i = at;
        while i < input.len() && input[i] == Tok::Gap {
            i += 1;
        }
        if syms.is_empty() || i + syms.len() > input.len() {
            return None;
        }
        for (k, s) in syms.iter().enumerate() {
            match &input[i + k] {
                Tok::Sym(x) if x == s => {}
                _ => return None,
            }
        }
        let end = i + syms.len();
        let runs_together =
            self.r.lang.difficulty.separation == crate::difficulty::Separation::None;
        if end < input.len() && input[end] != Tok::Gap && !runs_together {
            return None;
        }
        Some(end)
    }

    fn sym_words(&self, input: &[Tok], at: usize, words: &[Vec<String>]) -> Option<usize> {
        let mut i = at;
        for w in words {
            i = self.word(input, i, w)?;
        }
        Some(i)
    }

    fn plain_word(&self, input: &[Tok], at: usize, pos: &[Pos]) -> Alt<String> {
        let mut out = Vec::new();
        for (syms, id) in &self.plain {
            if !pos.contains(&concepts::get(id).pos) {
                continue;
            }
            if let Some(end) = self.word(input, at, syms) {
                out.push((id.clone(), end));
            }
        }
        out
    }

    fn numeral(&self, input: &[Tok], at: usize) -> Alt<u16> {
        let mut out = Vec::new();
        for (n, words) in &self.numerals {
            if let Some(end) = self.sym_words(input, at, words) {
                out.push((*n, end));
            }
        }
        out
    }

    /// Heads with their particles: every noun and name, in both numbers.
    fn head(&self, input: &[Tok], at: usize, case: Case) -> Alt<(Head, Number)> {
        let mut out = Vec::new();
        let forms = &self
            .heads
            .iter()
            .find(|(c, _)| *c == case)
            .expect("every case")
            .1;
        for (h, words) in forms {
            if let Some(end) = self.sym_words(input, at, words) {
                out.push((h.clone(), end));
            }
        }
        out
    }

    /// Adjectives, numeral, determiner and head, in the language's order.
    fn core(&self, input: &[Tok], at: usize, case: Case) -> Alt<NounPhrase> {
        let mut out = Vec::new();
        match self.r.lang.syntax.modifiers {
            Side::Before => {
                // [det] [numeral] [adj_n … adj_1] head
                for (det, a) in self.opt(self.plain_word(input, at, &[Pos::Det]), at) {
                    for (qty, b) in self.opt(self.numeral(input, a), a) {
                        for (adjs, c) in self.many(input, b, &[Pos::Adj]) {
                            for ((head, number), d) in self.head(input, c, case) {
                                let mut adjectives = adjs.clone();
                                adjectives.reverse();
                                out.push((np(head, number, qty, det.clone(), adjectives), d));
                            }
                        }
                    }
                }
            }
            Side::After => {
                // head [adj_1 … adj_n] [numeral] [det]
                for ((head, number), a) in self.head(input, at, case) {
                    for (adjs, b) in self.many(input, a, &[Pos::Adj]) {
                        for (qty, c) in self.opt(self.numeral(input, b), b) {
                            for (det, d) in self.opt(self.plain_word(input, c, &[Pos::Det]), c) {
                                out.push((np(head.clone(), number, qty, det, adjs.clone()), d));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn opt<T: Clone>(&self, found: Alt<T>, at: usize) -> Alt<Option<T>> {
        let mut out: Alt<Option<T>> = found.into_iter().map(|(t, e)| (Some(t), e)).collect();
        out.push((None, at));
        out
    }

    /// Zero or more plain words of the given kinds, longest first.
    fn many(&self, input: &[Tok], at: usize, pos: &[Pos]) -> Alt<Vec<String>> {
        let mut out = Vec::new();
        for (w, e) in self.plain_word(input, at, pos) {
            for (mut rest, e2) in self.many(input, e, pos) {
                rest.insert(0, w.clone());
                out.push((rest, e2));
            }
        }
        out.push((Vec::new(), at));
        out
    }

    /// A noun phrase in a case: possessor, core, and appositions.
    fn noun_phrase(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        depth: usize,
        appositions: bool,
    ) -> Alt<NounPhrase> {
        let key = (at, case, depth, appositions);
        if let Some(found) = self.memo.borrow().get(&key) {
            return found.clone();
        }
        let out = self.noun_phrase_uncached(input, at, case, depth, appositions);
        self.memo.borrow_mut().insert(key, out.clone());
        out
    }

    fn noun_phrase_uncached(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        depth: usize,
        appositions: bool,
    ) -> Alt<NounPhrase> {
        let mut out = Vec::new();
        let possessed =
            |out: &mut Alt<NounPhrase>, core: NounPhrase, poss: Option<NounPhrase>, end: usize| {
                let mut n = core;
                n.possessor = poss.map(Box::new);
                out.push((n, end));
            };
        let mut cores: Alt<NounPhrase> = Vec::new();
        match self.r.lang.syntax.genitive {
            Side::Before => {
                let mut poss: Alt<Option<NounPhrase>> = vec![(None, at)];
                if depth < DEPTH {
                    for (p, e) in self.noun_phrase(input, at, Case::Genitive, depth + 1, true) {
                        poss.push((Some(p), e));
                    }
                }
                for (p, e) in poss {
                    for (c, e2) in self.core(input, e, case) {
                        possessed(&mut cores, c, p.clone(), e2);
                    }
                }
            }
            Side::After => {
                for (c, e) in self.core(input, at, case) {
                    possessed(&mut cores, c.clone(), None, e);
                    if depth < DEPTH {
                        for (p, e2) in self.noun_phrase(input, e, Case::Genitive, depth + 1, true) {
                            possessed(&mut cores, c.clone(), Some(p), e2);
                        }
                    }
                }
            }
        }
        for (n, e) in cores {
            if appositions {
                for (apps, e2) in self.appositions(input, e, case, depth) {
                    let mut n2 = n.clone();
                    n2.apposition = apps;
                    out.push((n2, e2));
                }
            } else {
                out.push((n, e));
            }
        }
        out
    }

    fn appositions(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        depth: usize,
    ) -> Alt<Vec<NounPhrase>> {
        let mut out = Vec::new();
        if depth < DEPTH + 2 {
            for (a, e) in self.noun_phrase(input, at, case, depth + 1, false) {
                for (mut rest, e2) in self.appositions(input, e, case, depth + 1) {
                    rest.insert(0, a.clone());
                    out.push((rest, e2));
                }
            }
        }
        out.push((Vec::new(), at));
        out
    }

    fn verb_group(&self, input: &[Tok], at: usize, potent: bool) -> Alt<(String, Tense, Polarity)> {
        let mut out = Vec::new();
        for ((v, tense, polarity, p), words) in &self.verbs {
            if *p != potent {
                continue;
            }
            if let Some(end) = self.sym_words(input, at, words) {
                out.push(((v.clone(), *tense, *polarity), end));
            }
        }
        out
    }

    fn np_opt(&self, input: &[Tok], at: usize, case: Case) -> Alt<Option<NounPhrase>> {
        self.opt(self.noun_phrase(input, at, case, 0, true), at)
    }

    /// A clause in the language's word order.
    fn clause(&self, input: &[Tok], at: usize) -> Alt<Clause> {
        let mut out = Vec::new();
        // The potent frame: pot.open … pot.close, with "pot" before the verb.
        let open = self.word(input, at, &self.word_syms(&self.r.plain("pot.open")));
        let starts: Vec<(bool, usize)> = match open {
            Some(e) => vec![(true, e), (false, at)],
            None => vec![(false, at)],
        };
        for (potent, start) in starts {
            for (c, end) in self.bare_clause(input, start, potent) {
                if potent {
                    if let Some(e) =
                        self.word(input, end, &self.word_syms(&self.r.plain("pot.close")))
                    {
                        out.push((c, e));
                    }
                } else {
                    out.push((c, end));
                }
            }
        }
        out
    }

    fn bare_clause(&self, input: &[Tok], at: usize, potent: bool) -> Alt<Clause> {
        let mut out = Vec::new();
        let finish = |subject: Option<NounPhrase>,
                      object: Option<NounPhrase>,
                      recipient: Option<NounPhrase>,
                      adverbs: Vec<String>,
                      verb: (String, Tense, Polarity)|
         -> Clause {
            let mood = if potent {
                Mood::Potent
            } else if subject.is_none() {
                Mood::Imperative
            } else {
                Mood::Declarative
            };
            let mut args = Vec::new();
            if let Some(s) = subject {
                args.push(Argument {
                    role: Role::Subject,
                    np: s,
                });
            }
            if let Some(o) = object {
                args.push(Argument {
                    role: Role::Object,
                    np: o,
                });
            }
            if let Some(r) = recipient {
                args.push(Argument {
                    role: Role::Recipient,
                    np: r,
                });
            }
            Clause {
                predicate: verb.0,
                mood,
                tense: verb.1,
                polarity: verb.2,
                args,
                adverbs,
            }
        };
        match self.r.lang.syntax.word_order {
            WordOrder::Sov => {
                for (s, a) in self.np_opt(input, at, Case::Subject) {
                    for (rc, b) in self.np_opt(input, a, Case::Dative) {
                        for (o, c) in self.np_opt(input, b, Case::Object) {
                            for (adv, d) in self.many(input, c, &[Pos::Adv]) {
                                for (v, e) in self.verb_group(input, d, potent) {
                                    out.push((
                                        finish(s.clone(), o.clone(), rc.clone(), adv.clone(), v),
                                        e,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            WordOrder::Svo => {
                for (s, a) in self.np_opt(input, at, Case::Subject) {
                    for (v, b) in self.verb_group(input, a, potent) {
                        for (o, c) in self.np_opt(input, b, Case::Object) {
                            for (rc, d) in self.np_opt(input, c, Case::Dative) {
                                for (adv, e) in self.many(input, d, &[Pos::Adv]) {
                                    out.push((
                                        finish(s.clone(), o.clone(), rc.clone(), adv, v.clone()),
                                        e,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            WordOrder::Vso => {
                for (v, a) in self.verb_group(input, at, potent) {
                    for (s, b) in self.np_opt(input, a, Case::Subject) {
                        for (o, c) in self.np_opt(input, b, Case::Object) {
                            for (rc, d) in self.np_opt(input, c, Case::Dative) {
                                for (adv, e) in self.many(input, d, &[Pos::Adv]) {
                                    out.push((
                                        finish(s.clone(), o.clone(), rc.clone(), adv, v.clone()),
                                        e,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Parses a whole inscription. Prefers one clause, then several in a row
    /// (a letter), then a verbless list.
    pub fn sentence(&self, input: &[Tok]) -> Option<Sentence> {
        self.memo.borrow_mut().clear();
        let end = trim_end(input);
        let input = &input[..end];
        let start = input
            .iter()
            .position(|t| *t != Tok::Gap)
            .unwrap_or(input.len());
        for (c, e) in self.clause(input, start) {
            if e == input.len() {
                return Some(Sentence::Clause(c));
            }
        }
        if let Some(parts) = self.clauses(input, start, 0) {
            if parts.len() >= 2 {
                return Some(Sentence::Text(parts));
            }
        }
        self.list(input, start).map(Sentence::List)
    }

    fn clauses(&self, input: &[Tok], at: usize, depth: usize) -> Option<Vec<Sentence>> {
        if at >= input.len() {
            return Some(Vec::new());
        }
        if depth > 6 {
            return None;
        }
        for (c, e) in self.clause(input, at) {
            if e == at {
                continue;
            }
            if let Some(mut rest) = self.clauses(input, e, depth + 1) {
                rest.insert(0, Sentence::Clause(c));
                return Some(rest);
            }
        }
        None
    }

    fn list(&self, input: &[Tok], at: usize) -> Option<Vec<NounPhrase>> {
        if at >= input.len() {
            return Some(Vec::new());
        }
        for (n, e) in self.noun_phrase(input, at, Case::Subject, 0, false) {
            if e == at {
                continue;
            }
            if let Some(mut rest) = self.list(input, e) {
                rest.insert(0, n);
                return Some(rest);
            }
        }
        None
    }
}

fn trim_end(input: &[Tok]) -> usize {
    let mut end = input.len();
    while end > 0 && input[end - 1] == Tok::Gap {
        end -= 1;
    }
    end
}

fn np(
    head: Head,
    number: Number,
    quantity: Option<u16>,
    determiner: Option<String>,
    adjectives: Vec<String>,
) -> NounPhrase {
    NounPhrase {
        head,
        number,
        quantity,
        determiner,
        adjectives,
        possessor: None,
        apposition: Vec::new(),
    }
}

/// A stable symbol for a glyph.
pub fn glyph_sym(k: &GlyphKey) -> String {
    format!("{k:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::Corpus;
    use crate::Language;

    /// Every sentence of the corpus, in every era of three seeds, parses
    /// back to its meaning.
    #[test]
    fn parse_inverts_render() {
        for seed in [1u64, 42, 9001] {
            for lang in Language::generate(seed).eras() {
                let corpus = Corpus::generate(&lang, 40);
                let r = corpus.renderer();
                for mode in [Mode::Phonemes, Mode::Glyphs] {
                    let p = Parser::new(&r, mode);
                    for ins in &corpus.inscriptions {
                        let words = r.render(&ins.meaning).words;
                        let back = p.sentence(&p.tokens(&words));
                        if mode == Mode::Glyphs && back.as_ref() != Some(&ins.meaning) {
                            // Glyphs can be ambiguous where the script drops
                            // sounds (abjads); the parse must still render
                            // to the same glyphs.
                            let b = back.unwrap_or_else(|| {
                                panic!("seed {seed} era {}: no parse", lang.era)
                            });
                            assert_eq!(p.tokens(&r.render(&b).words), p.tokens(&words));
                            continue;
                        }
                        assert_eq!(
                            back.as_ref(),
                            Some(&ins.meaning),
                            "seed {seed} era {} mode {mode:?}",
                            lang.era
                        );
                    }
                }
            }
        }
    }
}
