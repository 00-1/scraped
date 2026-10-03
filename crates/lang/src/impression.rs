//! How a sign looks at a glance (S01): the impression a person would see
//! and remember (tall and hooked; a ring like an eye), rather than the
//! stroke-by-stroke build that tracing gives.
//!
//! Impressions are worked out from a sign's strokes, so they are stable:
//! the same sign always gives the same impression, and a player can match
//! signs across texts by themselves. Within one script they are given just
//! enough detail to tell every sign apart (on archaeologist, a few pairs
//! are left alike on purpose). A sign that is another sign plus a mark or
//! two (the script logic of M02, a syllabary's vowel marks) is told as
//! that sign with the mark added, so related signs read as related.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::script::{Glyph, Mark, Script, Spot, Stroke, Turn};

/// The impression of one sign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Impression {
    /// Overall outline: round, tall, wide, angular, slight or plain.
    pub outline: &'static str,
    /// The stroke that carries the sign, and which way it turns.
    pub main: Mark,
    /// Further marks worth telling, most distinctive first.
    pub others: Vec<Mark>,
    /// Whether the others' turns are told (only when needed to tell signs
    /// apart).
    pub turns: bool,
    /// Whether the main stroke's place is told.
    pub main_spot: bool,
    /// What it looks like, where something fits ("" if nothing).
    pub resembles: &'static str,
    /// How many strokes it has.
    pub count: usize,
    /// A related sign it is told by (its index in the script's table),
    /// and the marks this one adds to it.
    pub like: Option<usize>,
    pub added: Vec<Mark>,
}

/// What an impression actually tells, for telling signs apart: its
/// outline, main stroke and turn, the marks told (their turns only when
/// told), the main stroke's place when told, and any relation. The stroke
/// count and resemblance are left out: a wording may not use them.
type Told = (
    &'static str,
    Stroke,
    Turn,
    Option<Spot>,
    Vec<(Stroke, Spot, Option<Turn>)>,
    Option<usize>,
    Vec<Mark>,
);

impl Impression {
    pub fn told(&self) -> Told {
        (
            self.outline,
            self.main.stroke,
            self.main.turn,
            self.main_spot.then_some(self.main.spot),
            self.others
                .iter()
                .map(|m| (m.stroke, m.spot, self.turns.then_some(m.turn)))
                .collect(),
            self.like,
            self.added.clone(),
        )
    }
}

/// How heavy a stroke looks: the main stroke is the heaviest one.
fn weight(s: Stroke) -> u8 {
    match s {
        Stroke::Ring => 9,
        Stroke::Cross => 8,
        Stroke::Zigzag => 7,
        Stroke::Wedge => 6,
        Stroke::Hook => 5,
        Stroke::Tail => 4,
        Stroke::Arc => 3,
        Stroke::Bar => 2,
        Stroke::Dot => 1,
    }
}

fn main_mark(g: &Glyph) -> Mark {
    *g.marks
        .iter()
        .max_by_key(|m| {
            (
                m.spot == Spot::Centre,
                weight(m.stroke),
                std::cmp::Reverse(**m),
            )
        })
        .expect("a sign has strokes")
}

fn outline(g: &Glyph) -> &'static str {
    let has = |s: Stroke| g.marks.iter().any(|m| m.stroke == s);
    let at = |p: Spot| g.marks.iter().any(|m| m.spot == p);
    if g.marks.len() == 1 && has(Stroke::Dot) {
        "slight"
    } else if has(Stroke::Ring) {
        "round"
    } else if (at(Spot::Top) && at(Spot::Bottom))
        || g.marks
            .iter()
            .any(|m| m.stroke == Stroke::Bar && m.turn == Turn::Up && m.spot == Spot::Centre)
    {
        "tall"
    } else if (at(Spot::Left) && at(Spot::Right))
        || g.marks
            .iter()
            .any(|m| m.stroke == Stroke::Bar && m.turn == Turn::Right && m.spot == Spot::Centre)
    {
        "wide"
    } else if has(Stroke::Wedge) || has(Stroke::Zigzag) || has(Stroke::Cross) {
        "angular"
    } else {
        "plain"
    }
}

/// Things a sign can look like.
pub const RESEMBLANCES: &[&str] = &["", "eye", "wheel", "crook", "comb", "arrow", "seed"];

fn resembles(g: &Glyph) -> &'static str {
    let has = |s: Stroke| g.marks.iter().any(|m| m.stroke == s);
    if g.marks.len() > 3 {
        ""
    } else if has(Stroke::Ring) && has(Stroke::Dot) {
        "eye"
    } else if has(Stroke::Ring) && has(Stroke::Cross) {
        "wheel"
    } else if has(Stroke::Bar) && has(Stroke::Hook) {
        "crook"
    } else if has(Stroke::Bar) && has(Stroke::Zigzag) {
        "comb"
    } else if has(Stroke::Bar) && has(Stroke::Wedge) {
        "arrow"
    } else if has(Stroke::Tail) && has(Stroke::Dot) {
        "seed"
    } else {
        ""
    }
}

/// Whether `q` is `g` less a mark or two (and not smaller than what is
/// added to it).
fn part_of(q: &Glyph, g: &Glyph) -> Option<Vec<Mark>> {
    if q.marks.len() >= g.marks.len() || !q.marks.iter().all(|m| g.marks.contains(m)) {
        return None;
    }
    let added: Vec<Mark> = g
        .marks
        .iter()
        .filter(|m| !q.marks.contains(m))
        .copied()
        .collect();
    (added.len() <= 2 && q.marks.len() >= added.len()).then_some(added)
}

/// The impression of every sign of a script, in table order.
/// `confusable`: leave a few alike signs untold apart (archaeologist).
pub fn impressions(script: &Script, confusable: bool) -> Vec<Impression> {
    let glyphs: Vec<&Glyph> = script.glyphs.iter().map(|(_, g)| g).collect();
    // A sign with no sign inside it is told by its own look.
    let root: Vec<bool> = glyphs
        .iter()
        .enumerate()
        .map(|(i, g)| {
            !glyphs
                .iter()
                .enumerate()
                .any(|(j, q)| i != j && part_of(q, g).is_some())
        })
        .collect();
    let mut like: Vec<Option<(usize, Vec<Mark>)>> = vec![None; glyphs.len()];
    for (i, g) in glyphs.iter().enumerate() {
        if root[i] {
            continue;
        }
        like[i] = glyphs
            .iter()
            .enumerate()
            .filter(|&(j, _)| root[j])
            .filter_map(|(j, q)| part_of(q, g).map(|a| (j, a)))
            .max_by_key(|(j, _)| (glyphs[*j].marks.len(), std::cmp::Reverse(*j)));
    }
    // How common each stroke is at each place: rarer marks are the ones
    // that stand out.
    let mut common: BTreeMap<(Stroke, Spot), usize> = BTreeMap::new();
    for g in &glyphs {
        for m in &g.marks {
            *common.entry((m.stroke, m.spot)).or_default() += 1;
        }
    }
    let base = |g: &Glyph| {
        let main = main_mark(g);
        let mut others: Vec<Mark> = g.marks.iter().filter(|m| **m != main).copied().collect();
        others.sort_by_key(|m| (common[&(m.stroke, m.spot)], *m));
        Impression {
            outline: outline(g),
            main,
            others,
            turns: true,
            main_spot: true,
            resembles: resembles(g),
            count: g.marks.len(),
            like: None,
            added: Vec::new(),
        }
    };
    // Told at a level of detail: 0 the outline and main stroke; 1..=n that
    // many other marks; then their turns; then the main stroke's place.
    let at_level = |full: &Impression, level: usize| {
        let n = full.others.len();
        let mut imp = full.clone();
        imp.others.truncate(level.min(n));
        // Turns only matter where there are other marks to turn.
        imp.turns = level > n && n > 0;
        imp.main_spot = level > n + 1;
        imp
    };
    let roots: Vec<usize> = (0..glyphs.len())
        .filter(|&i| root[i] || like[i].is_none())
        .collect();
    let full: BTreeMap<usize, Impression> = roots.iter().map(|&i| (i, base(glyphs[i]))).collect();
    let cap = if confusable { 1 } else { usize::MAX };
    // Raise the detail of any sign that clashes with another until none do
    // (or, on archaeologist, until the cap).
    let mut level: BTreeMap<usize, usize> = roots.iter().map(|&i| (i, 0)).collect();
    let told = loop {
        let told: BTreeMap<usize, Impression> = roots
            .iter()
            .map(|&i| (i, at_level(&full[&i], level[&i])))
            .collect();
        let mut raised = false;
        for &i in &roots {
            let most = (full[&i].others.len() + 2).min(cap);
            let clash = roots
                .iter()
                .any(|&j| j != i && told[&j].told() == told[&i].told());
            if clash && level[&i] < most {
                *level.get_mut(&i).expect("a root") += 1;
                raised = true;
            }
        }
        if !raised {
            break told;
        }
    };
    (0..glyphs.len())
        .map(|i| match told.get(&i) {
            Some(imp) => imp.clone(),
            None => {
                let (j, added) = like[i].clone().expect("a related sign");
                Impression {
                    like: Some(j),
                    added,
                    count: glyphs[i].marks.len(),
                    ..told[&j].clone()
                }
            }
        })
        .collect()
}

/// Pairs of signs a script leaves alike (the same impression).
pub fn alike(imps: &[Impression]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..imps.len() {
        for j in i + 1..imps.len() {
            if imps[i].told() == imps[j].told() {
                out.push((i, j));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerals::Numerals;
    use crate::phonology::Phonology;
    use crate::rng::{Rng, Stream};
    use crate::script::ScriptKind;

    fn script(seed: u64, kind: ScriptKind) -> Script {
        let p = Phonology::generate(seed);
        let n = Numerals::generate(seed);
        let mut rng = Rng::new(seed, Stream::Script);
        Script::generate(&mut rng, &p, &n, Some(kind))
    }

    #[test]
    fn every_sign_is_told_apart() {
        for seed in 0..40 {
            for kind in [
                ScriptKind::Alphabet,
                ScriptKind::Abjad,
                ScriptKind::Syllabary,
            ] {
                let s = script(seed, kind);
                let imps = impressions(&s, false);
                assert!(alike(&imps).is_empty(), "seed {seed} {kind:?}");
            }
        }
    }

    #[test]
    fn related_signs_are_told_by_their_relation() {
        let mut related = 0;
        for seed in 0..20 {
            let s = script(seed, ScriptKind::Syllabary);
            for imp in impressions(&s, false) {
                if let Some(j) = imp.like {
                    related += 1;
                    let mut marks = s.glyphs[j].1.marks.clone();
                    marks.extend(imp.added.iter().copied());
                    assert_eq!(marks.len(), imp.count);
                }
            }
        }
        assert!(related > 100, "{related}");
    }

    #[test]
    fn archaeologist_leaves_a_few_alike() {
        let mut alike_total = 0;
        for seed in 0..20 {
            let s = script(seed, ScriptKind::Alphabet);
            alike_total += alike(&impressions(&s, true)).len();
        }
        assert!(alike_total > 0 && alike_total <= 3 * 20, "{alike_total}");
    }

    #[test]
    fn impressions_are_stable() {
        let s = script(7, ScriptKind::Abjad);
        assert_eq!(impressions(&s, false), impressions(&s, false));
    }
}
