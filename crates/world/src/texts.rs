//! Writing placed by history: every text records which event it is
//! evidence of, who wrote it, and the surface it is on.
//!
//! Texts are meanings, rendered through the language of the era they were
//! written in.

use serde::Serialize;

use scraped_lang::concepts;
use scraped_lang::corpus::Kind;
use scraped_lang::meaning::{
    Argument, Clause, Head, Mood, NounPhrase, Polarity, Role as ArgRole, Sentence, Tense,
};
use scraped_lang::rng::{Rng, Stream};

use crate::history::{EventKind, Evidence, History, Role};
use crate::structures::{Material, Structure, StructureKind};

/// What kind of writing a text is (D08): finer than [`Kind`], which
/// says only how the language frames it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Genre {
    Epitaph,
    Ledger,
    Warning,
    Dedication,
    Label,
    Letter,
    Potent,
    /// A year-by-year record of a reign.
    Annal,
    /// The rulers, one after another.
    KingList,
    Decree,
    /// A dispute and its judgement.
    Court,
    /// A loan or sale, with what is owed.
    Contract,
    /// What was paid, and by whom.
    Receipt,
    /// What a store holds.
    Inventory,
    Prayer,
    Hymn,
    Myth,
    Oracle,
    /// How to work what it is written beside.
    Instructions,
    /// A school exercise.
    Lesson,
    /// Words of one kind, listed for learning.
    WordList,
    Boundary,
    Milestone,
    /// Who built a building, and when.
    Building,
    Graffito,
    Curse,
    Blessing,
    /// Festivals, and the moon's days.
    Calendar,
    /// An everyday spell on a house, a store or a thing (D09).
    Charm,
    /// A spell on a door, a tomb or who may pass (D09).
    Ward,
    /// A spell on the land, water, sky, plants or beasts (D09).
    Invocation,
}

impl Genre {
    /// The genre of the writing the older history leaves.
    pub fn of(kind: Kind) -> Self {
        match kind {
            Kind::Tomb => Genre::Epitaph,
            Kind::Ledger => Genre::Ledger,
            Kind::Warning => Genre::Warning,
            Kind::Dedication => Genre::Dedication,
            Kind::Label => Genre::Label,
            Kind::Letter => Genre::Letter,
            Kind::Potent => Genre::Potent,
            Kind::Account => Genre::Annal,
        }
    }

    /// How the language frames it.
    pub fn kind(self) -> Kind {
        match self {
            Genre::Epitaph => Kind::Tomb,
            Genre::Ledger | Genre::Inventory | Genre::Receipt | Genre::Contract => Kind::Ledger,
            Genre::Warning | Genre::Decree | Genre::Boundary | Genre::Instructions => Kind::Warning,
            Genre::Dedication | Genre::Building | Genre::Prayer | Genre::Hymn | Genre::Blessing => {
                Kind::Dedication
            }
            Genre::Label | Genre::Milestone | Genre::Graffito | Genre::WordList => Kind::Label,
            Genre::Letter => Kind::Letter,
            Genre::Potent | Genre::Charm | Genre::Ward | Genre::Invocation => Kind::Potent,
            Genre::Annal
            | Genre::KingList
            | Genre::Court
            | Genre::Myth
            | Genre::Oracle
            | Genre::Lesson
            | Genre::Curse
            | Genre::Calendar => Kind::Account,
        }
    }
}

/// A piece of writing somewhere in the world.
#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub id: usize,
    pub era: u32,
    pub year: i32,
    pub kind: Kind,
    pub genre: Genre,
    /// The story it is part of (D08).
    pub arc: Option<usize>,
    pub meaning: Sentence,
    pub author: Option<usize>,
    /// The event this text is evidence of.
    pub event: Option<usize>,
    pub structure: usize,
    /// Room and feature it is written on; `None` means outside.
    pub room: Option<usize>,
    pub feature: Option<usize>,
    pub material: Material,
}

fn statement(predicate: &str, tense: Tense, args: Vec<(ArgRole, NounPhrase)>) -> Clause {
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

/// "[Name], [kin] of [Name], [title], lies here."
fn epitaph(h: &History, person: usize) -> Sentence {
    let p = &h.people[person];
    let mut subject = NounPhrase::name(person);
    if let Some((kin, of)) = &p.relation {
        subject
            .apposition
            .push(NounPhrase::concept(kin).with_possessor(NounPhrase::name(*of)));
    }
    if let Some(title) = p.role.title() {
        subject.apposition.push(NounPhrase::concept(title));
    }
    let mut c = statement("lie", Tense::NonPast, vec![(ArgRole::Subject, subject)]);
    c.adverbs.push("here".to_string());
    Sentence::Clause(c)
}

/// "[Person] made this [thing] for [the god / the temple]."
fn dedication(rng: &mut Rng, person: usize, h: &History) -> Sentence {
    let mut maker = NounPhrase::name(person);
    if let Some(title) = h.people[person].role.title() {
        maker.apposition.push(NounPhrase::concept(title));
    }
    let mut thing = NounPhrase::concept(rng.pick(&["altar", "statue", "seal", "bowl", "lamp"]));
    thing.determiner = Some("this".to_string());
    let recipient = NounPhrase::concept(rng.pick(&["god", "temple", "shrine", "city"]));
    Sentence::Clause(statement(
        "make",
        Tense::Past,
        vec![
            (ArgRole::Subject, maker),
            (ArgRole::Object, thing),
            (ArgRole::Recipient, recipient),
        ],
    ))
}

/// A list of goods with a total; in a famine, few and small.
fn ledger(rng: &mut Rng, scarce: bool) -> Sentence {
    let goods = concepts::nouns_tagged("good");
    let mut items: Vec<NounPhrase> = Vec::new();
    for _ in 0..rng.range(1, 3) {
        let c = rng.pick(&goods);
        if items.iter().any(|i| i.head == Head::Concept(c.id.clone())) {
            continue;
        }
        let n = if scarce {
            rng.range(1, 3)
        } else {
            rng.range(4, 90)
        } as u16;
        items.push(NounPhrase::concept(&c.id).counted(n));
    }
    if items.len() >= 2 {
        let sum = items.iter().filter_map(|i| i.quantity).sum();
        let mut total = NounPhrase::concept("total");
        total.quantity = Some(sum);
        items.push(total);
    }
    Sentence::List(items)
}

/// "Do not [verb] the [thing]."
fn warning(verb: &str, thing: &str) -> Sentence {
    Sentence::Clause(Clause {
        predicate: verb.to_string(),
        mood: Mood::Imperative,
        tense: Tense::NonPast,
        polarity: Polarity::Negative,
        args: vec![Argument {
            role: ArgRole::Object,
            np: NounPhrase::concept(thing),
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

/// "[A] said to [B]: [body]."
fn letter(from: usize, to: usize, body: Sentence) -> Sentence {
    Sentence::Text(vec![
        Sentence::Clause(statement(
            "say",
            Tense::Past,
            vec![
                (ArgRole::Subject, NounPhrase::name(from)),
                (ArgRole::Recipient, NounPhrase::name(to)),
            ],
        )),
        body,
    ])
}

/// The potent claim of a writing event.
fn potent(verb: &str, subject: &str, negative: bool) -> Sentence {
    Sentence::Clause(Clause {
        predicate: verb.to_string(),
        mood: Mood::Potent,
        tense: Tense::NonPast,
        polarity: if negative {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args: vec![Argument {
            role: ArgRole::Subject,
            np: NounPhrase::concept(subject),
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

/// A structure, and the room and feature within it (`None`: outside).
pub(crate) type Spot = (usize, Option<(usize, usize)>);

// Finds the least-written suitable surface in a settlement: structures
// of the given kinds (in order of preference), features of the given
// kinds, the emptiest first.
pub(crate) fn spot(
    structures: &[Structure],
    settlement: usize,
    kinds: &[StructureKind],
    features: &[&str],
) -> Option<(usize, Option<(usize, usize)>)> {
    for kind in kinds {
        let mut best: Option<(usize, Spot)> = None;
        for s in structures
            .iter()
            .filter(|s| s.kind == *kind && s.settlement == Some(settlement))
        {
            for (ri, room) in s.interior.rooms.iter().enumerate() {
                for (fi, f) in room.features.iter().enumerate() {
                    if features.contains(&f.kind) && best.is_none_or(|(n, _)| f.texts.len() < n) {
                        best = Some((f.texts.len(), (s.id, Some((ri, fi)))));
                    }
                }
            }
            if best.is_none() {
                best = Some((s.outside.len(), (s.id, None)));
            }
        }
        if let Some((_, at)) = best {
            return Some(at);
        }
    }
    None
}

/// A text about to be written.
pub(crate) struct Written {
    pub era: u32,
    pub year: i32,
    pub genre: Genre,
    pub meaning: Sentence,
    pub author: Option<usize>,
    pub event: Option<usize>,
    pub arc: Option<usize>,
}

/// Writes a text on a surface.
pub(crate) fn put(texts: &mut Vec<Text>, structures: &mut [Structure], at: Spot, w: Written) {
    let id = texts.len();
    let (sid, place) = at;
    let material = match place {
        Some((r, f)) => {
            structures[sid].interior.rooms[r].features[f].texts.push(id);
            structures[sid].interior.rooms[r].features[f].material
        }
        None => {
            structures[sid].outside.push(id);
            Material::Stone
        }
    };
    texts.push(Text {
        id,
        era: w.era,
        year: w.year,
        kind: w.genre.kind(),
        genre: w.genre,
        arc: w.arc,
        meaning: w.meaning,
        author: w.author,
        event: w.event,
        structure: sid,
        room: place.map(|p| p.0),
        feature: place.map(|p| p.1),
        material,
    });
}

/// Places every text history implies on the structures that carry them.
pub fn place(seed: u64, h: &History, structures: &mut [Structure]) -> Vec<Text> {
    let mut rng = Rng::new(seed, Stream::World(4));
    let mut texts: Vec<Text> = Vec::new();

    let put = |texts: &mut Vec<Text>,
               structures: &mut [Structure],
               at: Spot,
               era,
               year,
               kind,
               meaning,
               author,
               event| {
        put(
            texts,
            structures,
            at,
            Written {
                era,
                year,
                genre: Genre::of(kind),
                meaning,
                author,
                event,
                arc: None,
            },
        )
    };

    let era_people = |e: u32, s: usize| -> Vec<usize> {
        h.people
            .iter()
            .filter(|p| p.era == e && p.settlement == s)
            .map(|p| p.id)
            .collect()
    };

    for ev in &h.events {
        let settlement = match &ev.kind {
            EventKind::Founding { settlement }
            | EventKind::Abandonment { settlement, .. }
            | EventKind::Plague { settlement }
            | EventKind::Famine { settlement }
            | EventKind::War { settlement, .. } => Some(*settlement),
            EventKind::Migration { to, .. } => Some(*to),
            EventKind::Succession { ruler, .. } => Some(h.people[*ruler].settlement),
            EventKind::Death { person, .. } => Some(h.people[*person].settlement),
            EventKind::Writing { author, .. } => Some(h.people[*author].settlement),
            // Later schisms may reassign towns, so find it by place.
            EventKind::Schism { .. } => h
                .settlements
                .iter()
                .find(|s| s.cell == ev.cell)
                .map(|s| s.id),
            // D08's events leave no evidence of their own here: the genres
            // write about them.
            _ => None,
        };
        let Some(s) = settlement else { continue };
        // People of the place, or of the era when the place is too new to
        // have any (a letter about a migration comes from the old town).
        // Only those born by then (D08: a text names no one unborn).
        let mut people = era_people(ev.era, s);
        people.retain(|&p| h.people[p].born <= ev.year);
        if people.len() < 2 {
            people.extend(
                h.people
                    .iter()
                    .filter(|p| p.era == ev.era && p.settlement != s && p.born <= ev.year)
                    .map(|p| p.id)
                    .take(2),
            );
        }
        for evidence in &ev.evidence {
            match evidence {
                Evidence::Dedication => {
                    let author = match &ev.kind {
                        EventKind::Succession { ruler, .. } => Some(*ruler),
                        _ => people
                            .iter()
                            .copied()
                            .find(|&p| matches!(h.people[p].role, Role::Priest | Role::Ruler))
                            .or(people.first().copied()),
                    };
                    let Some(author) = author else { continue };
                    if let Some(at) = spot(
                        structures,
                        s,
                        &[StructureKind::Temple],
                        &["stele", "altar", "statue", "wall", "niche"],
                    ) {
                        let m = dedication(&mut rng, author, h);
                        put(
                            &mut texts,
                            structures,
                            at,
                            ev.era,
                            ev.year,
                            Kind::Dedication,
                            m,
                            Some(author),
                            Some(ev.id),
                        );
                    }
                }
                Evidence::Tomb => {
                    let person = match &ev.kind {
                        EventKind::Death { person, .. } => Some(*person),
                        _ => people.first().copied(),
                    };
                    let Some(person) = person else { continue };
                    // A ruler's own tomb, else the town cemetery.
                    let at = structures
                        .iter()
                        .find(|st| st.kind == StructureKind::Tomb && st.person == Some(person))
                        .map(|st| (st.id, Some((0, 0))))
                        .or_else(|| {
                            spot(structures, s, &[StructureKind::Cemetery], &["gravestone"])
                        });
                    if let Some(at) = at {
                        put(
                            &mut texts,
                            structures,
                            at,
                            ev.era,
                            ev.year,
                            Kind::Tomb,
                            epitaph(h, person),
                            None,
                            Some(ev.id),
                        );
                    }
                }
                Evidence::Ledger => {
                    if let Some(at) = spot(structures, s, &[StructureKind::Storehouse], &["tablet"])
                    {
                        let scarce = matches!(ev.kind, EventKind::Famine { .. });
                        let m = ledger(&mut rng, scarce);
                        let author = people
                            .iter()
                            .copied()
                            .find(|&p| h.people[p].role == Role::Scribe);
                        put(
                            &mut texts,
                            structures,
                            at,
                            ev.era,
                            ev.year,
                            Kind::Ledger,
                            m,
                            author,
                            Some(ev.id),
                        );
                    }
                }
                Evidence::Letter => {
                    if people.len() >= 2 {
                        let (a, b) = (people[0], people[1]);
                        let body = match &ev.kind {
                            // What the famine left, said in words (a list
                            // after "said to" would not read as speech).
                            EventKind::Famine { .. } => {
                                let _ = ledger(&mut rng, true);
                                let mut c = statement(
                                    "find",
                                    Tense::Past,
                                    vec![(ArgRole::Object, NounPhrase::concept("grain"))],
                                );
                                c.polarity = Polarity::Negative;
                                c.args.insert(
                                    0,
                                    Argument {
                                        role: ArgRole::Subject,
                                        np: NounPhrase::concept("people"),
                                    },
                                );
                                Sentence::Clause(c)
                            }
                            _ => {
                                let (verb, thing) = *rng.pick(&[
                                    ("enter", "city"),
                                    ("cross", "river"),
                                    ("take", "grain"),
                                    ("enter", "temple"),
                                ]);
                                warning(verb, thing)
                            }
                        };
                        if let Some(at) = spot(structures, s, &[StructureKind::House], &["wall"]) {
                            put(
                                &mut texts,
                                structures,
                                at,
                                ev.era,
                                ev.year,
                                Kind::Letter,
                                letter(a, b, body),
                                Some(a),
                                Some(ev.id),
                            );
                        }
                    }
                }
                Evidence::Sign => {
                    let (verb, thing, kinds): (&str, &str, &[StructureKind]) = match &ev.kind {
                        EventKind::War { .. } => (
                            "cross",
                            "wall",
                            &[StructureKind::Wall, StructureKind::Temple],
                        ),
                        _ => ("enter", "house", &[StructureKind::Temple]),
                    };
                    if let Some(at) = spot(structures, s, kinds, &["gate", "wall", "stele"]) {
                        put(
                            &mut texts,
                            structures,
                            at,
                            ev.era,
                            ev.year,
                            Kind::Warning,
                            warning(verb, thing),
                            None,
                            Some(ev.id),
                        );
                    }
                }
                Evidence::Inscription => {
                    let EventKind::Writing {
                        claim,
                        author,
                        root,
                        ..
                    } = &ev.kind
                    else {
                        continue;
                    };
                    let (kinds, features): (&[StructureKind], &[&str]) = if *root {
                        (
                            &[StructureKind::Archive, StructureKind::Temple],
                            &["chest", "altar"],
                        )
                    } else {
                        (
                            &[StructureKind::Temple, StructureKind::House],
                            &["wall", "niche", "stele"],
                        )
                    };
                    if let Some(at) = spot(structures, s, kinds, features) {
                        let m = potent(claim.verb, claim.subject, claim.negative);
                        put(
                            &mut texts,
                            structures,
                            at,
                            ev.era,
                            ev.year,
                            Kind::Potent,
                            m,
                            Some(*author),
                            Some(ev.id),
                        );
                    }
                }
                Evidence::Ruin | Evidence::Wall => {}
            }
        }
    }

    // Everyday writing that no single event explains: jar labels, routine
    // ledgers, milestones.
    let ids: Vec<usize> = structures.iter().map(|s| s.id).collect();
    for sid in ids {
        let st = &structures[sid];
        let era = st.era;
        let year = st.built + 5;
        match st.kind {
            StructureKind::Storehouse => {
                let s = st.settlement.unwrap_or(0);
                if let Some(owner) = era_people(era, s)
                    .into_iter()
                    .find(|&p| h.people[p].born <= year)
                {
                    let mut jar = NounPhrase::concept("jar");
                    jar = jar.with_possessor(NounPhrase::name(owner));
                    let at = spot(structures, s, &[StructureKind::Storehouse], &["jar"])
                        .unwrap_or((sid, None));
                    put(
                        &mut texts,
                        structures,
                        at,
                        era,
                        year,
                        Kind::Label,
                        Sentence::List(vec![jar]),
                        None,
                        None,
                    );
                }
                let at = spot(structures, s, &[StructureKind::Storehouse], &["tablet"])
                    .unwrap_or((sid, None));
                let m = ledger(&mut rng, false);
                put(
                    &mut texts,
                    structures,
                    at,
                    era,
                    year + 20,
                    Kind::Ledger,
                    m,
                    None,
                    None,
                );
            }
            StructureKind::Waystation | StructureKind::Bridge => {
                let (verb, thing) = if st.kind == StructureKind::Bridge {
                    ("break", "wall")
                } else {
                    *rng.pick(&[("cross", "river"), ("enter", "tower"), ("take", "stone")])
                };
                let m = warning(verb, thing);
                put(
                    &mut texts,
                    structures,
                    (sid, Some((0, 0))),
                    era,
                    year,
                    Kind::Warning,
                    m,
                    None,
                    None,
                );
            }
            _ => {}
        }
    }
    texts
}
