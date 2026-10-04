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
use crate::meaning::{
    Argument, Clause, Compare, Complement, Conj, Degree, Head, Link, Mood, NounPhrase, Relative,
    Role, Sentence, Subordinate,
};
use crate::morphology::{Case, Number, VerbForm};
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
/// How deep clauses may nest inside clauses (D07): relative clauses,
/// adverbial clauses and reported speech belong to main clauses only, so
/// the surface never leaves it open which clause one belongs to.
// DESIGN-Q: one level of embedding. Deeper embedding makes attachment
// ambiguous in many word orders (a second "when…" could belong to the
// first), and long texts can chain sentences instead.
const CLAUSE_DEPTH: usize = 1;

pub struct Parser<'a> {
    r: &'a Renderer<'a>,
    mode: Mode,
    /// Plain words (adjectives, determiners, adverbs, particles) by symbols.
    plain: BTreeMap<Vec<String>, String>,
    /// Numeral word groups by value, as each word's symbols.
    numerals: Vec<(u16, Vec<Vec<String>>)>,
    /// Head forms with particles, per case, and by first symbol.
    heads: Vec<(Case, Vec<Form<HeadKey>>, Index)>,
    /// Verb groups: (verb, form, potent) → words' symbols, and by first
    /// symbol.
    verbs: Vec<Form<VerbKey>>,
    verb_index: Index,
    /// The relative word in each role.
    relatives: Vec<(Role, Vec<Vec<String>>)>,
    /// Noun phrases already parsed at a position.
    memo: RefCell<BTreeMap<MemoKey, Alt<NounPhrase>>>,
}

/// Something, and the symbols of each of its words.
type Form<T> = (T, Vec<Vec<String>>);
type VerbKey = (String, VerbForm, bool);
type HeadKey = (Head, Number);
/// Forms by the first symbol of their first word.
type Index = BTreeMap<String, Vec<usize>>;
/// (position, case, depth, appositions allowed, clause depth).
type MemoKey = (usize, Case, usize, bool, usize);

/// A partial parse: something, and where the input continues.
type Alt<T> = Vec<(T, usize)>;

fn index<T>(forms: &[Form<T>]) -> Index {
    let mut out: Index = BTreeMap::new();
    for (i, (_, words)) in forms.iter().enumerate() {
        if let Some(first) = words.first().and_then(|w| w.first()) {
            out.entry(first.clone()).or_default().push(i);
        }
    }
    out
}

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
            verb_index: BTreeMap::new(),
            relatives: Vec::new(),
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
            .filter(|c| c.domain != concepts::Domain::Grammar && r.lang.lexicon.has(&c.id))
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
            let idx = index(&forms);
            p.heads.push((case, forms, idx));
        }
        let verb_forms = r.lang.morphology.verb_forms();
        for v in concepts::with_pos(Pos::Verb) {
            if !r.lang.lexicon.has(&v.id) {
                continue;
            }
            for &form in &verb_forms {
                for potent in [false, true] {
                    let words: Vec<Vec<String>> = r
                        .verb_group_form(&v.id, form, potent)
                        .iter()
                        .map(|w| p.word_syms(w))
                        .collect();
                    p.verbs.push(((v.id.clone(), form, potent), words));
                }
            }
        }
        p.verb_index = index(&p.verbs);
        if r.lang.lexicon.has("rel") {
            for role in [Role::Subject, Role::Object, Role::Recipient] {
                let words = r
                    .relative_word(role)
                    .iter()
                    .map(|w| p.word_syms(w))
                    .collect();
                p.relatives.push((role, words));
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

    /// The first symbol at or after `at`, skipping gaps.
    fn next_sym<'b>(&self, input: &'b [Tok], at: usize) -> Option<&'b String> {
        input[at.min(input.len())..].iter().find_map(|t| match t {
            Tok::Sym(s) => Some(s),
            Tok::Gap => None,
        })
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

    /// A particular function word.
    fn fixed(&self, input: &[Tok], at: usize, concept: &str) -> Option<usize> {
        if !self.r.lang.lexicon.has(concept) {
            return None;
        }
        self.word(input, at, &self.word_syms(&self.r.plain(concept)))
    }

    fn plain_word(&self, input: &[Tok], at: usize, pos: &[Pos]) -> Alt<String> {
        let mut out = Vec::new();
        for (syms, id) in &self.plain {
            let c = concepts::get(id);
            if !pos.contains(&c.pos) || c.domain == concepts::Domain::Grammar {
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
        let (_, forms, idx) = self
            .heads
            .iter()
            .find(|(c, _, _)| *c == case)
            .expect("every case");
        let Some(first) = self.next_sym(input, at) else {
            return out;
        };
        for &i in idx.get(first).map(Vec::as_slice).unwrap_or(&[]) {
            let (h, words) = &forms[i];
            if let Some(end) = self.sym_words(input, at, words) {
                out.push((h.clone(), end));
            }
        }
        out
    }

    /// A compared adjective: the degree word and the adjective, in the
    /// language's order (D07).
    fn degree(&self, input: &[Tok], at: usize) -> Alt<(Compare, String)> {
        let mut out = Vec::new();
        for compare in [Compare::More, Compare::Most, Compare::As] {
            let marker = match compare {
                Compare::More => "cmp.more",
                Compare::Most => "cmp.most",
                Compare::As => "cmp.as",
            };
            match self.r.lang.syntax.degree {
                Side::Before => {
                    if let Some(a) = self.fixed(input, at, marker) {
                        for (adj, b) in self.plain_word(input, a, &[Pos::Adj]) {
                            out.push(((compare, adj), b));
                        }
                    }
                }
                Side::After => {
                    for (adj, a) in self.plain_word(input, at, &[Pos::Adj]) {
                        if let Some(b) = self.fixed(input, a, marker) {
                            out.push(((compare, adj), b));
                        }
                    }
                }
            }
        }
        out
    }

    /// What a compared adjective is measured against, with its linking word.
    fn standard(
        &self,
        input: &[Tok],
        at: usize,
        compare: Compare,
        depth: usize,
        cd: usize,
    ) -> Alt<NounPhrase> {
        let marker = if compare == Compare::As {
            "cmp.like"
        } else {
            "cmp.than"
        };
        let mut out = Vec::new();
        if depth >= DEPTH {
            return out;
        }
        match self.r.lang.syntax.modifiers {
            Side::After => {
                if let Some(a) = self.fixed(input, at, marker) {
                    out.extend(self.noun_phrase(input, a, Case::Subject, depth + 1, false, cd));
                }
            }
            Side::Before => {
                for (n, a) in self.noun_phrase(input, at, Case::Subject, depth + 1, false, cd) {
                    if let Some(b) = self.fixed(input, a, marker) {
                        out.push((n, b));
                    }
                }
            }
        }
        out
    }

    /// Adjectives, compared adjective, numeral, determiner and head, in the
    /// language's order, with what a compared adjective is measured against.
    fn core(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        depth: usize,
        cd: usize,
    ) -> Alt<NounPhrase> {
        let mut out = Vec::new();
        let degrees =
            |at: usize| -> Alt<Option<(Compare, String)>> { self.opt(self.degree(input, at), at) };
        let with_degree =
            |mut n: NounPhrase, d: Option<(Compare, String)>, st: Option<NounPhrase>| {
                n.degree = d.map(|(compare, adjective)| {
                    Box::new(Degree {
                        compare,
                        adjective,
                        standard: st,
                    })
                });
                n
            };
        match self.r.lang.syntax.modifiers {
            Side::Before => {
                // [det] [numeral] [degree] [adj_n … adj_1] head
                for (det, a) in self.opt(self.plain_word(input, at, &[Pos::Det]), at) {
                    for (qty, b) in self.opt(self.numeral(input, a), a) {
                        for (deg, b2) in degrees(b) {
                            for (adjs, c) in self.many(input, b2, &[Pos::Adj]) {
                                for ((head, number), d) in self.head(input, c, case) {
                                    let mut adjectives = adjs.clone();
                                    adjectives.reverse();
                                    let n = np(head, number, qty, det.clone(), adjectives);
                                    out.push((with_degree(n, deg.clone(), None), d));
                                }
                            }
                        }
                    }
                }
            }
            Side::After => {
                // head [adj_1 … adj_n] [degree] [numeral] [det]
                for ((head, number), a) in self.head(input, at, case) {
                    for (adjs, b) in self.many(input, a, &[Pos::Adj]) {
                        for (deg, b2) in degrees(b) {
                            for (qty, c) in self.opt(self.numeral(input, b2), b2) {
                                for (det, d) in self.opt(self.plain_word(input, c, &[Pos::Det]), c)
                                {
                                    let n = np(head.clone(), number, qty, det, adjs.clone());
                                    out.push((with_degree(n, deg.clone(), None), d));
                                }
                            }
                        }
                    }
                }
            }
        }
        let _ = (depth, cd);
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

    /// A noun phrase in a case: possessor, core, relative clause and
    /// appositions. `cd` is the clause depth it stands at.
    fn noun_phrase(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        depth: usize,
        appositions: bool,
        cd: usize,
    ) -> Alt<NounPhrase> {
        let key = (at, case, depth, appositions, cd);
        if let Some(found) = self.memo.borrow().get(&key) {
            return found.clone();
        }
        let out = self.noun_phrase_uncached(input, at, case, depth, appositions, cd);
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
        cd: usize,
    ) -> Alt<NounPhrase> {
        let mut out = Vec::new();
        let possessed =
            |out: &mut Alt<NounPhrase>, core: NounPhrase, poss: Option<NounPhrase>, end: usize| {
                let mut n = core;
                n.possessor = poss.map(Box::new);
                out.push((n, end));
            };
        // Possessor and core, from a start.
        let cores_at = |start: usize| -> Alt<NounPhrase> {
            let mut cores: Alt<NounPhrase> = Vec::new();
            match self.r.lang.syntax.genitive {
                Side::Before => {
                    let mut poss: Alt<Option<NounPhrase>> = vec![(None, start)];
                    if depth < DEPTH {
                        for (p, e) in
                            self.noun_phrase(input, start, Case::Genitive, depth + 1, true, cd)
                        {
                            poss.push((Some(p), e));
                        }
                    }
                    for (p, e) in poss {
                        for (c, e2) in self.core(input, e, case, depth, cd) {
                            possessed(&mut cores, c, p.clone(), e2);
                        }
                    }
                }
                Side::After => {
                    for (c, e) in self.core(input, start, case, depth, cd) {
                        possessed(&mut cores, c.clone(), None, e);
                        if depth < DEPTH {
                            for (p, e2) in
                                self.noun_phrase(input, e, Case::Genitive, depth + 1, true, cd)
                            {
                                possessed(&mut cores, c.clone(), Some(p), e2);
                            }
                        }
                    }
                }
            }
            cores
        };
        // What a compared adjective is measured against, outside the
        // possessor on the adjectives' side.
        let cores_at = |start: usize| -> Alt<NounPhrase> {
            let set = |mut n: NounPhrase, st: NounPhrase| -> Option<NounPhrase> {
                let d = n.degree.as_mut()?;
                if d.compare == Compare::Most {
                    return None;
                }
                d.standard = Some(st);
                Some(n)
            };
            let mut v = Vec::new();
            match self.r.lang.syntax.modifiers {
                Side::Before => {
                    v.extend(cores_at(start));
                    for compare in [Compare::More, Compare::As] {
                        for (st, e) in self.standard(input, start, compare, depth, cd) {
                            for (n, e2) in cores_at(e) {
                                if n.degree.as_ref().is_some_and(|d| d.compare == compare) {
                                    v.extend(set(n, st.clone()).map(|n| (n, e2)));
                                }
                            }
                        }
                    }
                }
                Side::After => {
                    for (n, e) in cores_at(start) {
                        v.push((n.clone(), e));
                        if let Some(compare) = n.degree.as_ref().map(|d| d.compare) {
                            if compare != Compare::Most {
                                for (st, e2) in self.standard(input, e, compare, depth, cd) {
                                    v.extend(set(n.clone(), st).map(|n| (n, e2)));
                                }
                            }
                        }
                    }
                }
            }
            v
        };
        // A relative clause (D07), its relative word next to the noun.
        let mut phrases: Alt<NounPhrase> = Vec::new();
        let relatives_ok = cd < CLAUSE_DEPTH && depth == 0 && !self.relatives.is_empty();
        match self.r.lang.syntax.relative {
            Side::After => {
                for (n, e) in cores_at(at) {
                    phrases.push((n.clone(), e));
                    if !relatives_ok {
                        continue;
                    }
                    for (role, words) in &self.relatives {
                        for (c, e2) in self.clause_core(input, e, false, Some(*role), cd + 1) {
                            if let Some(e3) = self.sym_words(input, e2, words) {
                                let mut n2 = n.clone();
                                n2.relative = Some(Box::new(Relative {
                                    gap: *role,
                                    clause: c,
                                }));
                                phrases.push((n2, e3));
                            }
                        }
                    }
                }
            }
            Side::Before => {
                phrases.extend(cores_at(at));
                if relatives_ok {
                    for (role, words) in &self.relatives {
                        if let Some(e) = self.sym_words(input, at, words) {
                            for (c, e2) in self.clause_core(input, e, false, Some(*role), cd + 1) {
                                for (n, e3) in cores_at(e2) {
                                    let mut n2 = n;
                                    n2.relative = Some(Box::new(Relative {
                                        gap: *role,
                                        clause: c.clone(),
                                    }));
                                    phrases.push((n2, e3));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (n, e) in phrases {
            if appositions {
                for (apps, e2) in self.appositions(input, e, case, depth, cd) {
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
        cd: usize,
    ) -> Alt<Vec<NounPhrase>> {
        let mut out = Vec::new();
        if depth < DEPTH + 2 {
            for (a, e) in self.noun_phrase(input, at, case, depth + 1, false, cd) {
                for (mut rest, e2) in self.appositions(input, e, case, depth + 1, cd) {
                    rest.insert(0, a.clone());
                    out.push((rest, e2));
                }
            }
        }
        out.push((Vec::new(), at));
        out
    }

    fn verb_group(&self, input: &[Tok], at: usize, potent: bool) -> Alt<(String, VerbForm)> {
        let mut out = Vec::new();
        let Some(first) = self.next_sym(input, at) else {
            return out;
        };
        for &i in self.verb_index.get(first).map(Vec::as_slice).unwrap_or(&[]) {
            let ((v, form, p), words) = &self.verbs[i];
            if *p != potent {
                continue;
            }
            if let Some(end) = self.sym_words(input, at, words) {
                out.push(((v.clone(), *form), end));
            }
        }
        out
    }

    fn np_opt(
        &self,
        input: &[Tok],
        at: usize,
        case: Case,
        skip: bool,
        cd: usize,
    ) -> Alt<Option<NounPhrase>> {
        if skip {
            return vec![(None, at)];
        }
        self.opt(self.noun_phrase(input, at, case, 0, true, cd), at)
    }

    /// What a speech verb reports, where the object goes (D07).
    fn complement(&self, input: &[Tok], at: usize, cd: usize) -> Alt<Complement> {
        let mut out = Vec::new();
        if cd >= CLAUSE_DEPTH {
            return out;
        }
        // A quotation: opening word, any sentence, closing word.
        if let Some(a) = self.fixed(input, at, "quote.open") {
            for (content, b) in self.units(input, a, cd + 1, true) {
                if let Some(c) = self.fixed(input, b, "quote.close") {
                    out.push((
                        Complement {
                            direct: true,
                            content,
                        },
                        c,
                    ));
                }
            }
        }
        // Reported speech: "that" and one clause.
        let reported = |start: usize| -> Alt<Sentence> {
            self.clause(input, start, cd + 1, false)
                .into_iter()
                .map(|(c, e)| (Sentence::Clause(c), e))
                .collect()
        };
        match self.r.lang.syntax.linker {
            Side::Before => {
                if let Some(a) = self.fixed(input, at, "comp.that") {
                    for (content, b) in reported(a) {
                        out.push((
                            Complement {
                                direct: false,
                                content,
                            },
                            b,
                        ));
                    }
                }
            }
            Side::After => {
                for (content, a) in reported(at) {
                    if let Some(b) = self.fixed(input, a, "comp.that") {
                        out.push((
                            Complement {
                                direct: false,
                                content,
                            },
                            b,
                        ));
                    }
                }
            }
        }
        out
    }

    /// A clause in the language's word order, with its adverbial clauses
    /// and the potent frame.
    fn clause(&self, input: &[Tok], at: usize, cd: usize, commands: bool) -> Alt<Clause> {
        let mut out = Vec::new();
        // The potent frame: pot.open … pot.close, with "pot" before the verb.
        let starts: Vec<(bool, usize)> = match self.fixed(input, at, "pot.open") {
            Some(e) => vec![(true, e), (false, at)],
            None => vec![(false, at)],
        };
        for (potent, start) in starts {
            for (c, end) in self.with_adverbials(input, start, potent, cd, commands) {
                if potent {
                    if let Some(e) = self.fixed(input, end, "pot.close") {
                        out.push((c, e));
                    }
                } else {
                    out.push((c, end));
                }
            }
        }
        out
    }

    /// One adverbial clause with its linking word.
    fn adverbial(&self, input: &[Tok], at: usize, cd: usize) -> Alt<Subordinate> {
        let mut out = Vec::new();
        if cd >= CLAUSE_DEPTH {
            return out;
        }
        for link in Link::ALL {
            match self.r.lang.syntax.adverbial {
                Side::After => {
                    if let Some(a) = self.fixed(input, at, link.concept()) {
                        for (c, b) in self.clause(input, a, cd + 1, false) {
                            out.push((Subordinate { link, clause: c }, b));
                        }
                    }
                }
                Side::Before => {
                    for (c, a) in self.clause(input, at, cd + 1, false) {
                        if let Some(b) = self.fixed(input, a, link.concept()) {
                            out.push((Subordinate { link, clause: c }, b));
                        }
                    }
                }
            }
        }
        out
    }

    /// Zero or more adverbial clauses in a row.
    fn adverbials(
        &self,
        input: &[Tok],
        at: usize,
        cd: usize,
        left: usize,
    ) -> Alt<Vec<Subordinate>> {
        let mut out = Vec::new();
        if left > 0 {
            for (s, e) in self.adverbial(input, at, cd) {
                if e == at {
                    continue;
                }
                for (mut rest, e2) in self.adverbials(input, e, cd, left - 1) {
                    rest.insert(0, s.clone());
                    out.push((rest, e2));
                }
            }
        }
        out.push((Vec::new(), at));
        out
    }

    fn with_adverbials(
        &self,
        input: &[Tok],
        at: usize,
        potent: bool,
        cd: usize,
        commands: bool,
    ) -> Alt<Clause> {
        let mut out = Vec::new();
        match self.r.lang.syntax.adverbial {
            Side::Before => {
                for (subs, a) in self.adverbials(input, at, cd, 2) {
                    for (mut c, b) in self.clause_core_mood(input, a, potent, None, cd, commands) {
                        c.subordinate = subs.clone();
                        out.push((c, b));
                    }
                }
            }
            Side::After => {
                for (c, a) in self.clause_core_mood(input, at, potent, None, cd, commands) {
                    for (subs, b) in self.adverbials(input, a, cd, 2) {
                        let mut c2 = c.clone();
                        c2.subordinate = subs;
                        out.push((c2, b));
                    }
                }
            }
        }
        out
    }

    /// A clause core with a missing argument, for relative clauses.
    fn clause_core(
        &self,
        input: &[Tok],
        at: usize,
        potent: bool,
        gap: Option<Role>,
        cd: usize,
    ) -> Alt<Clause> {
        self.clause_core_mood(input, at, potent, gap, cd, false)
    }

    /// The arguments and verb in the language's order. `commands`: a clause
    /// with no subject may be a command (only where commands can stand).
    fn clause_core_mood(
        &self,
        input: &[Tok],
        at: usize,
        potent: bool,
        gap: Option<Role>,
        cd: usize,
        commands: bool,
    ) -> Alt<Clause> {
        let mut out = Vec::new();
        let finish = |subject: Option<NounPhrase>,
                      object: Option<NounPhrase>,
                      complement: Option<Complement>,
                      recipient: Option<NounPhrase>,
                      adverbs: Vec<String>,
                      verb: (String, VerbForm)|
         -> Option<Clause> {
            let (predicate, form) = verb;
            if complement.is_some() && predicate != "say" {
                return None;
            }
            let mood = if potent {
                if form.mood.is_some() {
                    return None;
                }
                Mood::Potent
            } else if let Some(m) = form.mood {
                if subject.is_none() && gap != Some(Role::Subject) {
                    return None;
                }
                m
            } else if subject.is_none() && gap != Some(Role::Subject) {
                if !commands {
                    return None;
                }
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
            Some(Clause {
                predicate,
                mood,
                tense: form.tense,
                polarity: form.polarity,
                args,
                adverbs,
                aspect: form.aspect,
                subordinate: Vec::new(),
                complement: complement.map(Box::new),
            })
        };
        let gs = gap == Some(Role::Subject);
        let go = gap == Some(Role::Object);
        let gr = gap == Some(Role::Recipient);
        // Reported speech, if any (only after "say").
        let reported = |start: usize| -> Alt<Option<Complement>> {
            let mut v: Alt<Option<Complement>> = vec![(None, start)];
            v.extend(
                self.complement(input, start, cd)
                    .into_iter()
                    .map(|(c, e)| (Some(c), e)),
            );
            v
        };
        match self.r.lang.syntax.word_order {
            WordOrder::Sov => {
                for (s, a) in self.np_opt(input, at, Case::Subject, gs, cd) {
                    for (rc, b) in self.np_opt(input, a, Case::Dative, gr, cd) {
                        for (o, c) in self.np_opt(input, b, Case::Object, go, cd) {
                            for (comp, c2) in reported(c) {
                                for (adv, d) in self.many(input, c2, &[Pos::Adv]) {
                                    for (v, e) in self.verb_group(input, d, potent) {
                                        if let Some(cl) = finish(
                                            s.clone(),
                                            o.clone(),
                                            comp.clone(),
                                            rc.clone(),
                                            adv.clone(),
                                            v,
                                        ) {
                                            out.push((cl, e));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            WordOrder::Svo => {
                for (s, a) in self.np_opt(input, at, Case::Subject, gs, cd) {
                    for (v, b) in self.verb_group(input, a, potent) {
                        for (o, c) in self.np_opt(input, b, Case::Object, go, cd) {
                            for (rc, d) in self.np_opt(input, c, Case::Dative, gr, cd) {
                                for (adv, e) in self.many(input, d, &[Pos::Adv]) {
                                    for (comp, e2) in reported(e) {
                                        if let Some(cl) = finish(
                                            s.clone(),
                                            o.clone(),
                                            comp,
                                            rc.clone(),
                                            adv.clone(),
                                            v.clone(),
                                        ) {
                                            out.push((cl, e2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            WordOrder::Vso => {
                for (v, a) in self.verb_group(input, at, potent) {
                    for (s, b) in self.np_opt(input, a, Case::Subject, gs, cd) {
                        for (o, c) in self.np_opt(input, b, Case::Object, go, cd) {
                            for (rc, d) in self.np_opt(input, c, Case::Dative, gr, cd) {
                                for (adv, e) in self.many(input, d, &[Pos::Adv]) {
                                    for (comp, e2) in reported(e) {
                                        if let Some(cl) = finish(
                                            s.clone(),
                                            o.clone(),
                                            comp,
                                            rc.clone(),
                                            adv.clone(),
                                            v.clone(),
                                        ) {
                                            out.push((cl, e2));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// One clause, or clauses joined by a conjunction (D07).
    fn unit(&self, input: &[Tok], at: usize, cd: usize, commands: bool) -> Alt<Sentence> {
        let mut out = Vec::new();
        for (c, e) in self.clause(input, at, cd, commands) {
            out.push((Sentence::Clause(c.clone()), e));
            for conj in Conj::ALL {
                let mut parts = vec![c.clone()];
                let mut end = e;
                loop {
                    let Some(a) = self.fixed(input, end, conj.concept()) else {
                        break;
                    };
                    // The longest next clause that leaves a parse.
                    let next = self
                        .clause(input, a, cd, commands)
                        .into_iter()
                        .max_by_key(|(_, e)| *e);
                    let Some((c2, e2)) = next else { break };
                    parts.push(c2);
                    end = e2;
                    out.push((Sentence::Joined(conj, parts.clone()), end));
                }
            }
        }
        out
    }

    /// Several units in a row (a letter, a quotation), or one.
    fn units(&self, input: &[Tok], at: usize, cd: usize, commands: bool) -> Alt<Sentence> {
        let mut out = self.unit(input, at, cd, commands);
        let mut seqs: Alt<Vec<Sentence>> = out.iter().map(|(s, e)| (vec![s.clone()], *e)).collect();
        for _ in 0..6 {
            let mut next = Vec::new();
            for (seq, e) in &seqs {
                for (s, e2) in self.unit(input, *e, cd, commands) {
                    if e2 > *e {
                        let mut v = seq.clone();
                        v.push(s);
                        next.push((v, e2));
                    }
                }
            }
            if next.is_empty() {
                break;
            }
            out.extend(next.iter().map(|(v, e)| (Sentence::Text(v.clone()), *e)));
            seqs = next;
        }
        out
    }

    /// Parses a whole inscription. Prefers one clause, then joined clauses,
    /// then several in a row (a letter), then a verbless list.
    pub fn sentence(&self, input: &[Tok]) -> Option<Sentence> {
        self.memo.borrow_mut().clear();
        let end = trim_end(input);
        let input = &input[..end];
        let start = input
            .iter()
            .position(|t| *t != Tok::Gap)
            .unwrap_or(input.len());
        let units = self.unit(input, start, 0, true);
        for (s, e) in &units {
            if *e == input.len() && matches!(s, Sentence::Clause(_)) {
                return Some(s.clone());
            }
        }
        for (s, e) in &units {
            if *e == input.len() {
                return Some(s.clone());
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
        let mut units = self.unit(input, at, 0, true);
        // Longest first, so a joined sentence is not split.
        units.sort_by_key(|(_, e)| std::cmp::Reverse(*e));
        for (u, e) in units {
            if e == at {
                continue;
            }
            if let Some(mut rest) = self.clauses(input, e, depth + 1) {
                rest.insert(0, u);
                return Some(rest);
            }
        }
        None
    }

    fn list(&self, input: &[Tok], at: usize) -> Option<Vec<NounPhrase>> {
        if at >= input.len() {
            return Some(Vec::new());
        }
        for (n, e) in self.noun_phrase(input, at, Case::Subject, 0, false, CLAUSE_DEPTH) {
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
        degree: None,
        relative: None,
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
    use crate::rng::{Rng, Stream};
    use crate::Language;

    /// Random meanings using every construction (D07), in every era of
    /// several seeds, parse back to themselves.
    #[test]
    fn every_construction_round_trips() {
        for seed in [1u64, 2, 3, 42, 9001] {
            for lang in Language::generate(seed).eras() {
                let corpus = Corpus::generate(&lang, 4);
                let r = corpus.renderer();
                let p = Parser::new(&r, Mode::Phonemes);
                let g = Parser::new(&r, Mode::Glyphs);
                let mut rng = Rng::new(seed, Stream::Inscription(777));
                for i in 0..120 {
                    let m = crate::sample::sentence(&lang, &mut rng, r.names.len());
                    let words = r.render(&m).words;
                    let back = p.sentence(&p.tokens(&words));
                    assert_eq!(
                        back.as_ref(),
                        Some(&m),
                        "seed {seed} era {} #{i}: {}",
                        lang.era,
                        crate::english::translate(&m, &|n| r.name(n))
                    );
                    if i % 4 == 0 {
                        let gb = g.sentence(&g.tokens(&words)).expect("glyph parse");
                        assert_eq!(g.tokens(&r.render(&gb).words), g.tokens(&words));
                    }
                }
            }
        }
    }

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
