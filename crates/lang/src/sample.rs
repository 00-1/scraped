//! Random meanings that use every construction of the grammar (D07): for
//! the round-trip tests and the bench. Not used for the game's texts.
//!
//! The meanings respect the grammar's rules (commands only where commands
//! stand, embedded clauses only in main clauses, reported speech only after
//! "say") but not sense: a gate may sing.

use crate::concepts::{self, Concept, Domain, Pos};
use crate::meaning::{
    Argument, Aspect, Clause, Compare, Complement, Conj, Degree, Head, Link, Mood, NounPhrase,
    Number, Polarity, Relative, Role, Sentence, Subordinate, Tense,
};
use crate::rng::Rng;
use crate::Language;

/// The words a sampler may use.
struct Words<'a> {
    nouns: Vec<&'static Concept>,
    verbs: Vec<&'static Concept>,
    adjectives: Vec<&'static Concept>,
    determiners: Vec<&'static Concept>,
    adverbs: Vec<&'static Concept>,
    lang: &'a Language,
    names: usize,
}

impl<'a> Words<'a> {
    fn new(lang: &'a Language, names: usize) -> Self {
        let has = |c: &&'static Concept| lang.lexicon.has(&c.id) && c.domain != Domain::Grammar;
        Words {
            nouns: concepts::any_with_pos(Pos::Noun).filter(has).collect(),
            verbs: concepts::any_with_pos(Pos::Verb).filter(has).collect(),
            adjectives: concepts::any_with_pos(Pos::Adj).filter(has).collect(),
            determiners: concepts::any_with_pos(Pos::Det).filter(has).collect(),
            adverbs: concepts::any_with_pos(Pos::Adv).filter(has).collect(),
            lang,
            names,
        }
    }
}

/// A random sentence, using up to `names` named people.
pub fn sentence(lang: &Language, rng: &mut Rng, names: usize) -> Sentence {
    let w = Words::new(lang, names);
    match rng.below(10) {
        0..=5 => Sentence::Clause(clause(&w, rng, 0, true, None)),
        6 | 7 => {
            let conj = *rng.pick(&Conj::ALL);
            let n = 2 + rng.below(2) as usize;
            Sentence::Joined(
                conj,
                (0..n).map(|_| clause(&w, rng, 0, true, None)).collect(),
            )
        }
        8 => Sentence::Text(vec![
            Sentence::Clause(clause(&w, rng, 0, true, None)),
            Sentence::Clause(clause(&w, rng, 0, true, None)),
        ]),
        _ => Sentence::List((0..2).map(|_| noun_phrase(&w, rng, 0, 1)).collect()),
    }
}

/// A clause at clause depth `cd`. `top`: commands may stand here. `gap`:
/// the role a relative clause leaves out.
fn clause(w: &Words, rng: &mut Rng, cd: usize, top: bool, gap: Option<Role>) -> Clause {
    // Reported speech now and then, in main clauses.
    let speech = cd == 0 && gap.is_none() && rng.chance(15) && w.lang.lexicon.has("say");
    let verb = if speech {
        concepts::get("say")
    } else {
        loop {
            let v = *rng.pick(&w.verbs);
            if gap != Some(Role::Object) || !v.objects.is_empty() {
                break v;
            }
        }
    };
    let mood = if gap.is_some() || !top {
        if rng.chance(25) {
            *rng.pick(&Mood::MARKED)
        } else {
            Mood::Declarative
        }
    } else {
        rng.weighted(&[
            (Mood::Declarative, 50),
            (Mood::Imperative, 12),
            (Mood::Optative, 10),
            (Mood::Conditional, 10),
            (Mood::Interrogative, 10),
            (Mood::Potent, 8),
        ])
    };
    let aspect = if w.lang.morphology.aspect != Aspect::Simple && rng.chance(30) {
        w.lang.morphology.aspect
    } else {
        Aspect::Simple
    };
    let mut args = Vec::new();
    if mood != Mood::Imperative && gap != Some(Role::Subject) {
        args.push(Argument {
            role: Role::Subject,
            np: noun_phrase(w, rng, 0, cd),
        });
    }
    if !verb.objects.is_empty() && gap != Some(Role::Object) && rng.chance(80) {
        args.push(Argument {
            role: Role::Object,
            np: noun_phrase(w, rng, 0, cd),
        });
    }
    if gap != Some(Role::Recipient) && rng.chance(15) {
        args.push(Argument {
            role: Role::Recipient,
            np: noun_phrase(w, rng, 0, cd),
        });
    }
    let adverbs = if rng.chance(15) && !w.adverbs.is_empty() {
        vec![rng.pick(&w.adverbs).id.clone()]
    } else {
        Vec::new()
    };
    let complement = speech.then(|| {
        let direct = rng.chance(50);
        let content = if direct && rng.chance(30) {
            Sentence::Joined(
                Conj::And,
                vec![
                    clause(w, rng, cd + 1, true, None),
                    clause(w, rng, cd + 1, true, None),
                ],
            )
        } else {
            Sentence::Clause(clause(w, rng, cd + 1, direct, None))
        };
        Box::new(Complement { direct, content })
    });
    let subordinate = if cd == 0 && gap.is_none() && rng.chance(25) {
        (0..1 + rng.below(2))
            .map(|_| Subordinate {
                link: *rng.pick(&Link::ALL),
                clause: clause(w, rng, cd + 1, false, None),
            })
            .collect()
    } else {
        Vec::new()
    };
    Clause {
        predicate: verb.id.clone(),
        mood,
        tense: if rng.chance(50) {
            Tense::Past
        } else {
            Tense::NonPast
        },
        polarity: if rng.chance(20) {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args,
        adverbs,
        aspect,
        subordinate,
        complement,
    }
}

fn noun_phrase(w: &Words, rng: &mut Rng, depth: usize, cd: usize) -> NounPhrase {
    let head = if w.names > 0 && rng.chance(15) {
        Head::Name(rng.index(w.names))
    } else {
        Head::Concept(rng.pick(&w.nouns).id.clone())
    };
    let mut n = NounPhrase {
        head,
        number: if rng.chance(30) {
            Number::Plural
        } else {
            Number::Singular
        },
        quantity: None,
        determiner: None,
        adjectives: Vec::new(),
        possessor: None,
        apposition: Vec::new(),
        degree: None,
        relative: None,
    };
    if rng.chance(15) {
        n = n.counted(2 + rng.below(18) as u16);
    }
    if rng.chance(25) && !w.determiners.is_empty() {
        n.determiner = Some(rng.pick(&w.determiners).id.clone());
    }
    if rng.chance(30) && !w.adjectives.is_empty() {
        n.adjectives.push(rng.pick(&w.adjectives).id.clone());
    }
    // One level of possessor: a possessor's own possessor could read as
    // its apposition (a pre-D07 ambiguity texts avoid).
    if depth == 0 && rng.chance(15) {
        n.possessor = Some(Box::new(noun_phrase(w, rng, depth + 1, cd)));
    }
    if rng.chance(15) && !w.adjectives.is_empty() {
        let adjective = rng.pick(&w.adjectives).id.clone();
        if !n.adjectives.contains(&adjective) {
            let compare = *rng.pick(&[Compare::More, Compare::Most, Compare::As]);
            let standard = (compare != Compare::Most && depth < 2 && rng.chance(60))
                .then(|| noun_phrase(w, rng, depth + 1, 1));
            n.degree = Some(Box::new(Degree {
                compare,
                adjective,
                standard,
            }));
        }
    }
    if cd == 0 && depth == 0 && rng.chance(15) {
        let gap = *rng.pick(&[Role::Subject, Role::Subject, Role::Object, Role::Recipient]);
        n.relative = Some(Box::new(Relative {
            gap,
            clause: clause(w, rng, cd + 1, false, Some(gap)),
        }));
    }
    n
}
