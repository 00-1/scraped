//! Formulaic inscriptions: tombs, ledgers, warnings and dedications.
//!
//! Real decipherments start from repeated formulae, numerals and names, so a
//! corpus is built from a few fixed text types filled from a small recurring
//! cast. The same people appear on tombs and dedications, always with the
//! same relatives and titles, giving readers cross-references to work with.

use serde::Serialize;
use serde_json::{json, Value};

use crate::concepts::{self, Concept, Pos};
use crate::english;
use crate::meaning::{Argument, Clause, Mood, NounPhrase, Polarity, Role, Sentence, Tense};
use crate::phonology::Phonemes;
use crate::render::{Rendered, Renderer};
use crate::rng::{Rng, Stream};
use crate::Language;

/// How many named people a corpus draws on.
// DESIGN-Q: cast size trades variety against repetition. 24 means most
// names recur once or twice in a 40-line corpus; the same tomb can still
// occasionally appear twice.
pub const CAST_SIZE: usize = 24;

/// The first few people are ancestors with no recorded relation.
const ANCESTORS: usize = 4;

/// A named person who can appear in inscriptions.
#[derive(Debug, Clone, Serialize)]
pub struct Person {
    pub name: Phonemes,
    /// Kinship to an earlier person, e.g. ("daughter", 2).
    pub relation: Option<(String, usize)>,
    pub title: Option<String>,
}

/// The people of one corpus.
#[derive(Debug, Clone, Serialize)]
pub struct Cast {
    pub people: Vec<Person>,
}

impl Cast {
    pub fn generate(lang: &Language) -> Self {
        let mut rng = Rng::new(lang.seed, Stream::Cast);
        let mut maker = lang.word_maker();
        let kin = concepts::nouns_tagged("kin");
        let titles = concepts::nouns_tagged("title");
        let people = (0..CAST_SIZE)
            .map(|i| {
                let _syllables = rng.weighted(&[(2, 60), (3, 40)]);
                let name = lang.person_name(&mut rng, &mut maker).form;
                let relation = (i >= ANCESTORS).then(|| (rng.pick(&kin).id.clone(), rng.index(i)));
                let title = rng.chance(40).then(|| rng.pick(&titles).id.clone());
                Person {
                    name,
                    relation,
                    title,
                }
            })
            .collect();
        Cast { people }
    }

    pub fn names(&self) -> Vec<Phonemes> {
        self.people.iter().map(|p| p.name.clone()).collect()
    }
}

/// The formulaic text types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// "[Name], child of [Name], lies here."
    Tomb,
    /// Quantities of goods.
    Ledger,
    /// "Do not [verb] the [noun]", or a positive command.
    Warning,
    /// "[Person] made this [object] for [person/place]."
    Dedication,
    /// An owner's or contents label: "[Name]'s jar".
    Label,
    /// "[Name] said to [Name]:" followed by a message.
    Letter,
    /// A claim in the potent register. Rare.
    Potent,
    /// An account of what happened, by those who came after: the deepest
    /// text and the end-of-run chronicle (M11). Never generated as corpus.
    Account,
}

/// Everyday writing does nothing; potent writing can act once scraped (M08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Register {
    Everyday,
    Potent,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Tomb => "tomb",
            Kind::Ledger => "ledger",
            Kind::Warning => "warning",
            Kind::Dedication => "dedication",
            Kind::Label => "label",
            Kind::Letter => "letter",
            Kind::Potent => "potent",
            Kind::Account => "account",
        }
    }

    pub fn register(self) -> Register {
        if self == Kind::Potent {
            Register::Potent
        } else {
            Register::Everyday
        }
    }
}

/// One generated inscription: its meaning and its rendering.
#[derive(Debug, Clone, Serialize)]
pub struct Inscription {
    pub kind: Kind,
    pub meaning: Sentence,
    pub rendered: Rendered,
}

/// A set of inscriptions in one language.
pub struct Corpus<'a> {
    pub lang: &'a Language,
    pub cast: Cast,
    pub names: Vec<Phonemes>,
    pub inscriptions: Vec<Inscription>,
}

impl<'a> Corpus<'a> {
    /// Generates `count` inscriptions. Each uses its own random stream, so a
    /// shorter corpus is always a prefix of a longer one.
    pub fn generate(lang: &'a Language, count: u32) -> Self {
        let cast = Cast::generate(lang);
        let names = cast.names();
        let inscriptions = (0..count)
            .map(|i| {
                let mut rng = Rng::new(lang.seed, Stream::Inscription(i));
                let (kind, meaning) = compose(&mut rng, &cast);
                let rendered = Renderer {
                    lang,
                    names: &names,
                }
                .render(&meaning);
                Inscription {
                    kind,
                    meaning,
                    rendered,
                }
            })
            .collect();
        Corpus {
            lang,
            cast,
            names,
            inscriptions,
        }
    }

    pub fn renderer(&self) -> Renderer<'_> {
        Renderer {
            lang: self.lang,
            names: &self.names,
        }
    }

    /// Romanised text of one inscription.
    pub fn text(&self, i: &Inscription) -> String {
        self.renderer().surface(&i.rendered)
    }

    /// Placeholder English translation (spoiler).
    pub fn translation(&self, i: &Inscription) -> String {
        let r = self.renderer();
        english::translate(&i.meaning, &|p| r.name(p))
    }

    /// Plain-text corpus, one inscription per line. With `spoil`, each line
    /// is followed by its type, translation and an aligned interlinear gloss.
    pub fn to_text(&self, spoil: bool) -> String {
        let r = self.renderer();
        let mut out = String::new();
        for (n, i) in self.inscriptions.iter().enumerate() {
            out.push_str(&self.text(i));
            out.push('\n');
            if spoil {
                out.push_str(&format!(
                    "    #{} {} · {}\n",
                    n + 1,
                    i.kind.label(),
                    self.translation(i)
                ));
                let segs: Vec<String> = i.rendered.words.iter().map(|w| r.segmented(w)).collect();
                let glosses: Vec<String> = i.rendered.words.iter().map(|w| w.gloss()).collect();
                let (a, b) = align(&segs, &glosses);
                out.push_str(&format!("    {a}\n    {b}\n\n"));
            }
        }
        out
    }

    /// Width of a glyph line on a surface, in glyphs.
    // DESIGN-Q: a fixed line width until surfaces have real sizes (M04/M08).
    pub const GLYPH_LINE: usize = 16;

    /// Every inscription as numbered glyphs, laid out in lines in physical
    /// order (the script's direction applied), separated by blank lines.
    /// Numbers refer to the `script` table.
    pub fn to_glyph_text(&self) -> String {
        let r = self.renderer();
        self.inscriptions
            .iter()
            .map(|i| r.glyph_lines(&i.rendered, Self::GLYPH_LINE).join("\n") + "\n")
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// JSON array of inscriptions. Ground truth only with `spoil`.
    pub fn to_json(&self, spoil: bool) -> Value {
        let r = self.renderer();
        let items: Vec<Value> = self
            .inscriptions
            .iter()
            .enumerate()
            .map(|(n, i)| {
                let mut v = json!({
                    "index": n + 1,
                    "text": self.text(i),
                    "glyph_lines": r.glyph_lines(&i.rendered, Self::GLYPH_LINE),
                });
                if spoil {
                    v["kind"] = json!(i.kind);
                    v["register"] = json!(i.kind.register());
                    v["translation"] = json!(self.translation(i));
                    v["words"] = i
                        .rendered
                        .words
                        .iter()
                        .map(|w| {
                            json!({
                                "text": self.lang.romanise(&w.phonemes()),
                                "segmented": r.segmented(w),
                                "gloss": w.gloss(),
                            })
                        })
                        .collect();
                    v["meaning"] = json!(i.meaning);
                }
                v
            })
            .collect();
        let mut out =
            json!({ "seed": self.lang.seed, "era": self.lang.era, "inscriptions": items });
        if spoil {
            let people: Vec<Value> = self
                .cast
                .people
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    json!({
                        "index": i,
                        "name": r.name(i),
                        "relation": p.relation.as_ref().map(|(k, o)| json!({"kin": k, "of": o})),
                        "title": p.title,
                    })
                })
                .collect();
            out["cast"] = json!(people);
        }
        out
    }
}

/// Pads two rows of cells so they line up in columns.
fn align(top: &[String], bottom: &[String]) -> (String, String) {
    let mut a = String::new();
    let mut b = String::new();
    for (x, y) in top.iter().zip(bottom) {
        let w = x.chars().count().max(y.chars().count()) + 2;
        a.push_str(&format!("{x:<w$}"));
        b.push_str(&format!("{y:<w$}"));
    }
    (a.trim_end().to_string(), b.trim_end().to_string())
}

fn compose(rng: &mut Rng, cast: &Cast) -> (Kind, Sentence) {
    let kind = rng.weighted(&[
        (Kind::Tomb, 22),
        (Kind::Ledger, 20),
        (Kind::Warning, 18),
        (Kind::Dedication, 15),
        (Kind::Label, 10),
        (Kind::Letter, 9),
        (Kind::Potent, 6),
    ]);
    let s = match kind {
        Kind::Tomb => tomb(rng, cast),
        Kind::Ledger => ledger(rng, cast),
        Kind::Warning => warning(rng, cast),
        Kind::Dedication => dedication(rng, cast),
        Kind::Label => label(rng, cast),
        Kind::Letter => letter(rng, cast),
        Kind::Potent | Kind::Account => potent(rng),
    };
    (kind, s)
}

fn statement(predicate: &str, tense: Tense, args: Vec<(Role, NounPhrase)>) -> Clause {
    Clause {
        predicate: predicate.to_string(),
        mood: Mood::Declarative,
        tense,
        polarity: Polarity::Positive,
        args: args
            .into_iter()
            .map(|(role, np)| Argument { role, np })
            .collect(),
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    }
}

/// A person's name with their title in apposition, if they have one.
fn titled(rng: &mut Rng, cast: &Cast, p: usize, chance: u32) -> NounPhrase {
    let mut np = NounPhrase::name(p);
    if let Some(t) = &cast.people[p].title {
        if rng.chance(chance) {
            let mut title = NounPhrase::concept(t);
            maybe_adjective(rng, &mut title, 25);
            np.apposition.push(title);
        }
    }
    np
}

fn maybe_adjective(rng: &mut Rng, np: &mut NounPhrase, chance: u32) {
    let crate::meaning::Head::Concept(id) = &np.head else {
        return;
    };
    let noun = concepts::get(id);
    let fitting: Vec<&Concept> = concepts::with_pos(Pos::Adj)
        .filter(|a| a.applies_to(noun))
        .collect();
    if !fitting.is_empty() && rng.chance(chance) {
        np.adjectives.push(rng.pick(&fitting).id.clone());
    }
}

fn tomb(rng: &mut Rng, cast: &Cast) -> Sentence {
    let related: Vec<usize> = (0..cast.people.len())
        .filter(|&i| cast.people[i].relation.is_some())
        .collect();
    let p = *rng.pick(&related);
    let (kin, of) = cast.people[p].relation.clone().expect("related");
    let mut subject = NounPhrase::name(p);
    subject
        .apposition
        .push(NounPhrase::concept(&kin).with_possessor(NounPhrase::name(of)));
    if let Some(t) = &cast.people[p].title {
        if rng.chance(50) {
            subject.apposition.push(NounPhrase::concept(t));
        }
    }
    let mut c = statement("lie", Tense::NonPast, vec![(Role::Subject, subject)]);
    c.adverbs.push("here".to_string());
    Sentence::Clause(c)
}

/// A count of goods: mostly small, sometimes in the tens or hundreds.
fn quantity(rng: &mut Rng) -> u16 {
    let n = match rng.weighted(&[(0, 55), (1, 33), (2, 12)]) {
        0 => rng.range(1, 10),
        1 => rng.range(11, 99),
        // At most three items of at most 300 keeps every total under 1000.
        _ => rng.range(100, 300),
    };
    n as u16
}

fn goods(rng: &mut Rng, avoid: &[String]) -> NounPhrase {
    let options: Vec<&Concept> = concepts::nouns_tagged("good")
        .into_iter()
        .filter(|c| !avoid.contains(&c.id))
        .collect();
    let n = quantity(rng);
    NounPhrase::concept(&rng.pick(&options).id).counted(n)
}

fn ledger(rng: &mut Rng, cast: &Cast) -> Sentence {
    if rng.chance(50) {
        let count = rng.range(1, 3);
        let mut items: Vec<NounPhrase> = Vec::new();
        for _ in 0..count {
            let used: Vec<String> = items
                .iter()
                .map(|np| match &np.head {
                    crate::meaning::Head::Concept(id) => id.clone(),
                    crate::meaning::Head::Name(_) => String::new(),
                })
                .collect();
            items.push(goods(rng, &used));
        }
        // A total line under two or more items lets readers check numbers
        // by arithmetic. It is always correct.
        if items.len() >= 2 {
            let sum: u16 = items.iter().filter_map(|np| np.quantity).sum();
            let mut total = NounPhrase::concept("total");
            debug_assert!(sum <= crate::numerals::MAX);
            total.quantity = Some(sum);
            items.push(total);
        }
        return Sentence::List(items);
    }
    let verb = if rng.chance(50) { "bring" } else { "give" };
    let giver = rng.index(cast.people.len());
    let mut args = vec![
        (Role::Subject, titled(rng, cast, giver, 30)),
        (Role::Object, goods(rng, &[])),
    ];
    if rng.chance(50) {
        args.push((Role::Recipient, place_or_god(rng)));
    }
    Sentence::Clause(statement(verb, Tense::Past, args))
}

fn label(rng: &mut Rng, cast: &Cast) -> Sentence {
    let made = concepts::nouns_tagged("made");
    let mut thing = NounPhrase::concept(&rng.pick(&made).id);
    let owner = if rng.chance(60) {
        NounPhrase::name(rng.index(cast.people.len()))
    } else {
        NounPhrase::concept(rng.pick(&["king", "queen", "god", "priest", "scribe"]))
    };
    thing = thing.with_possessor(owner);
    Sentence::List(vec![thing])
}

fn letter(rng: &mut Rng, cast: &Cast) -> Sentence {
    let from = rng.index(cast.people.len());
    let others: Vec<usize> = (0..cast.people.len()).filter(|&i| i != from).collect();
    let to = *rng.pick(&others);
    let opening = statement(
        "say",
        Tense::Past,
        vec![
            (Role::Subject, titled(rng, cast, from, 40)),
            (Role::Recipient, titled(rng, cast, to, 40)),
        ],
    );
    let body = if rng.chance(50) {
        warning(rng, cast)
    } else {
        ledger_clause(rng, cast)
    };
    Sentence::Text(vec![Sentence::Clause(opening), body])
}

fn ledger_clause(rng: &mut Rng, cast: &Cast) -> Sentence {
    loop {
        if let s @ Sentence::Clause(_) = ledger(rng, cast) {
            return s;
        }
    }
}

/// A potent claim: "let the gate not open". Only the form exists in M02.
// DESIGN-Q: claims use a few verbs that read naturally without an object
// (open, burn, break), with the thing as subject. M08 decides what claims
// can say and what they do.
fn potent(rng: &mut Rng) -> Sentence {
    let verb = concepts::get(rng.pick(&["open", "burn", "break"]));
    let subjects: Vec<&Concept> = concepts::with_pos(Pos::Noun)
        .filter(|n| verb.accepts_object(n))
        .collect();
    let mut subject = NounPhrase::concept(&rng.pick(&subjects).id);
    maybe_adjective(rng, &mut subject, 25);
    let polarity = if rng.chance(50) {
        Polarity::Negative
    } else {
        Polarity::Positive
    };
    Sentence::Clause(Clause {
        predicate: verb.id.clone(),
        mood: Mood::Potent,
        tense: Tense::NonPast,
        polarity,
        args: vec![Argument {
            role: Role::Subject,
            np: subject,
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

fn place_or_god(rng: &mut Rng) -> NounPhrase {
    let mut options = concepts::nouns_tagged("place");
    options.extend(concepts::nouns_tagged("deity"));
    let mut np = NounPhrase::concept(&rng.pick(&options).id);
    maybe_adjective(rng, &mut np, 30);
    np
}

/// Verbs used on signs. Making, bringing and giving belong to other genres.
// DESIGN-Q: commands are a bare non-past verb with no subject; there is no
// imperative affix. The milestone's morphology list has none, so a command
// is recognisable only by the missing subject.
fn sign_verbs() -> Vec<&'static Concept> {
    concepts::with_pos(Pos::Verb)
        .filter(|v| !v.objects.is_empty())
        .filter(|v| !matches!(v.id.as_str(), "make" | "bring" | "give"))
        .collect()
}

fn warning(rng: &mut Rng, cast: &Cast) -> Sentence {
    let verbs = sign_verbs();
    let verb = *rng.pick(&verbs);
    let objects: Vec<&Concept> = concepts::with_pos(Pos::Noun)
        .filter(|n| verb.accepts_object(n))
        .collect();
    let noun = *rng.pick(&objects);
    let mut object = NounPhrase::concept(&noun.id);
    if rng.chance(15) {
        object.number = crate::meaning::Number::Plural;
    }
    maybe_adjective(rng, &mut object, 30);
    if !noun.has_tag("deity") && rng.chance(20) {
        let owner = if rng.chance(50) {
            let owners = ["king", "queen", "god", "priest"];
            NounPhrase::concept(rng.pick(&owners))
        } else {
            NounPhrase::name(rng.index(cast.people.len()))
        };
        object = object.with_possessor(owner);
    }
    let polarity = if rng.chance(65) {
        Polarity::Negative
    } else {
        Polarity::Positive
    };
    Sentence::Clause(Clause {
        predicate: verb.id.clone(),
        mood: Mood::Imperative,
        tense: Tense::NonPast,
        polarity,
        args: vec![Argument {
            role: Role::Object,
            np: object,
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

fn dedication(rng: &mut Rng, cast: &Cast) -> Sentence {
    let maker = rng.index(cast.people.len());
    let made = concepts::nouns_tagged("made");
    let mut object = NounPhrase::concept(&rng.pick(&made).id);
    object.determiner = Some("this".to_string());
    maybe_adjective(rng, &mut object, 30);
    let roll = rng.below(100);
    let recipient = if roll < 45 {
        let others: Vec<usize> = (0..cast.people.len()).filter(|&i| i != maker).collect();
        let who = *rng.pick(&others);
        titled(rng, cast, who, 40)
    } else {
        let mut np = place_or_god(rng);
        if np.head != crate::meaning::Head::Concept("god".into()) && rng.chance(30) {
            np = np.with_possessor(NounPhrase::concept("god"));
        }
        np
    };
    Sentence::Clause(statement(
        "make",
        Tense::Past,
        vec![
            (Role::Subject, titled(rng, cast, maker, 50)),
            (Role::Object, object),
            (Role::Recipient, recipient),
        ],
    ))
}
