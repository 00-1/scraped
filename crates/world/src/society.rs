//! The life of the past beyond kings, plagues and wars (D08): people's
//! lives and families, institutions, trade, worship, building projects,
//! disasters and what was done about them, law and quarrels.
//!
//! Run after the older history ([`crate::history`]) on streams of its own,
//! so the kings, towns and wars of a seed stay as they were. Everything
//! here is a fact the texts of D08 write about: each person, institution
//! and project is recorded, each happening is an [`Event`], and related
//! happenings are gathered into [`Arc`]s, the stories a reader can piece
//! together from several texts in several places.

use serde::Serialize;

use scraped_lang::concepts::Pos;
use scraped_lang::rng::{Rng, Stream};
use scraped_lang::Language;

use crate::history::{Cause, Event, EventKind, Evidence, History, Person, Role};
use crate::terrain::{Terrain, SIZE};
use crate::water::Water;

/// A body of people that outlives any one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstitutionKind {
    /// A temple's priesthood.
    Temple,
    /// Craftsmen of one craft.
    Guild,
    /// The elders who govern a town.
    Council,
    /// Where disputes are judged.
    Court,
    /// Where scribes are taught.
    School,
}

/// One institution of one town.
#[derive(Debug, Clone, Serialize)]
pub struct Institution {
    pub id: usize,
    pub kind: InstitutionKind,
    pub settlement: usize,
    pub era: u32,
    pub founded: i32,
    /// Those who led it, in order.
    pub heads: Vec<usize>,
    /// For a guild: the craft (a lexicon concept: "smith", "potter"…).
    pub craft: Option<&'static str>,
}

/// A god, known by what it is god of.
#[derive(Debug, Clone, Serialize)]
pub struct Deity {
    pub id: usize,
    /// What it is god of (a lexicon concept: "river", "sun"…).
    pub of: &'static str,
    /// Its cult centre.
    pub settlement: usize,
    /// The season of its festival (a lexicon concept).
    pub festival: &'static str,
}

/// Something built, or meant to be.
#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: usize,
    /// What was built (a lexicon concept: "wall", "bridge"…).
    pub thing: &'static str,
    pub settlement: usize,
    /// Who paid for it, and who built it.
    pub patron: usize,
    pub builder: usize,
    pub era: u32,
    pub begun: i32,
    /// When it was finished or given up.
    pub ended: i32,
    pub finished: bool,
}

/// A law laid down by a ruler: a deed allowed or forbidden.
#[derive(Debug, Clone, Serialize)]
pub struct Law {
    pub verb: &'static str,
    pub object: &'static str,
    pub forbidden: bool,
}

/// What kind of story an arc tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArcKind {
    /// A notable person from birth to death.
    Life,
    /// Two families at odds, a judgement, and its end.
    Feud,
    /// A theft and its trial.
    Trial,
    /// A building begun, and finished or given up.
    Project,
    /// A flood, fire or earthquake, and what was done.
    Disaster,
    /// A merchant's journey and what came of it.
    Venture,
    /// A sign in the sky and what the god said.
    Omen,
    /// Two people far apart, writing to each other over years.
    Letters,
}

/// Related happenings that make one story, told in several texts.
#[derive(Debug, Clone, Serialize)]
pub struct Arc {
    pub id: usize,
    pub kind: ArcKind,
    /// The events, in order.
    pub events: Vec<usize>,
    /// The people at its heart.
    pub people: Vec<usize>,
    /// The towns it touches.
    pub settlements: Vec<usize>,
}

/// Goods that are traded, stolen, lent and lacked (core lexicon concepts).
pub const GOODS: &[&str] = &[
    "grain", "oil", "wine", "cloth", "sheep", "goat", "ox", "fish", "salt", "jar", "bowl", "knife",
    "lamp",
];
/// What gods are gods of.
const GODS_OF: &[&str] = &[
    "sun", "moon", "river", "sea", "storm", "rain", "mountain", "fire", "grain", "earth", "star",
];
const SEASONS: &[&str] = &["springtide", "summer", "autumn", "winter"];
/// What is built.
const WORKS: &[&str] = &[
    "wall", "bridge", "road", "well", "tower", "temple", "gate", "hall", "palace", "granary",
];
/// Laws: (verb, object) a ruler allows or forbids.
const LAWS: &[(&str, &str)] = &[
    ("sell", "grain"),
    ("carry", "knife"),
    ("cut", "tree"),
    ("take", "water"),
    ("enter", "temple"),
    ("sell", "wine"),
    ("hunt", "animal"),
    ("build", "house"),
    ("bury", "person"),
    ("kindle", "fire"),
];

struct Society<'a> {
    rng: Rng,
    names: Rng,
    t: &'a Terrain,
    w: &'a Water,
    h: &'a mut History,
}

impl Society<'_> {
    fn event(
        &mut self,
        era: u32,
        year: i32,
        settlement: usize,
        kind: EventKind,
        actors: Vec<usize>,
        cause: Option<usize>,
    ) -> usize {
        let id = self.h.events.len();
        let cell = self.h.settlements[settlement].cell;
        self.h.events.push(Event {
            id,
            era,
            year,
            cell,
            kind,
            actors,
            cause,
            evidence: Vec::<Evidence>::new(),
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn person(
        &mut self,
        lang: &Language,
        maker: &mut scraped_lang::lexicon::WordMaker,
        era: u32,
        born: i32,
        role: Role,
        settlement: usize,
        relation: Option<(String, usize)>,
    ) -> usize {
        let id = self.h.people.len();
        let n = lang.person_name(&mut self.names, maker);
        maker.reserve(&n.form, Pos::Noun);
        let life = 40 + self.rng.below(35) as i32;
        let faction = self.h.settlements[settlement].faction;
        self.h.people.push(Person {
            id,
            name: n.form,
            name_meaning: n.meaning,
            era,
            role,
            faction,
            settlement,
            relation,
            born,
            died: born + life,
        });
        id
    }

    /// People of a town who are grown and alive in a year.
    fn adults(&self, settlement: usize, year: i32) -> Vec<usize> {
        self.h
            .people
            .iter()
            .filter(|p| p.settlement == settlement && p.born + 16 <= year && year < p.died)
            .map(|p| p.id)
            .collect()
    }

    fn adults_with(&self, settlement: usize, year: i32, role: Role) -> Vec<usize> {
        self.adults(settlement, year)
            .into_iter()
            .filter(|&p| self.h.people[p].role == role)
            .collect()
    }

    fn ruler_at(&self, year: i32, settlement: usize) -> Option<usize> {
        let faction = self.h.settlements[settlement].faction;
        self.h
            .people
            .iter()
            .filter(|p| p.role == Role::Ruler && p.faction == faction)
            .find(|p| p.born + 16 <= year && year < p.died)
            .map(|p| p.id)
    }

    fn by_river(&self, settlement: usize) -> bool {
        let c = self.h.settlements[settlement].cell;
        (-3i32..=3).any(|dy| {
            (-3i32..=3).any(|dx| {
                let (x, y) = (c.ux() as i32 + dx, c.uy() as i32 + dy);
                x >= 0
                    && y >= 0
                    && (x as usize) < SIZE
                    && (y as usize) < SIZE
                    && self.w.is_river(self.t, x as usize, y as usize)
            })
        })
    }

    fn arc(&mut self, kind: ArcKind, events: Vec<usize>, people: Vec<usize>) {
        let mut settlements: Vec<usize> = Vec::new();
        for &e in &events {
            let c = self.h.events[e].cell;
            if let Some(s) = self.h.settlements.iter().find(|s| s.cell == c) {
                if !settlements.contains(&s.id) {
                    settlements.push(s.id);
                }
            }
        }
        let id = self.h.arcs.len();
        self.h.arcs.push(Arc {
            id,
            kind,
            events,
            people,
            settlements,
        });
    }
}

/// Deepens a history generated by [`History::generate`]'s older passes.
pub fn deepen(seed: u64, h: &mut History, t: &Terrain, w: &Water, langs: &[Language]) {
    let mut s = Society {
        rng: Rng::new(seed, Stream::World(21)),
        names: Rng::new(seed, Stream::World(22)),
        t,
        w,
        h,
    };
    for (e, lang) in langs.iter().enumerate() {
        let mut maker = lang.word_maker();
        for p in &s.h.people {
            maker.reserve(&p.name, Pos::Noun);
            if let Some(again) = lang.say_name(&p.name_meaning) {
                maker.reserve(&again, Pos::Noun);
            }
        }
        for t in &s.h.settlements {
            maker.reserve(&t.name, Pos::Noun);
            if let Some(again) = lang.say_name(&t.name_meaning) {
                maker.reserve(&again, Pos::Noun);
            }
        }
        era(&mut s, e as u32, lang, &mut maker);
    }
}

fn era(s: &mut Society, e: u32, lang: &Language, maker: &mut scraped_lang::lexicon::WordMaker) {
    let era = s.h.eras[e as usize].clone();
    let len = (era.end - era.start).max(60);
    let mid = era.start + len / 2;
    // Towns standing through the era's stories: founded before they begin.
    let towns: Vec<usize> =
        s.h.standing(mid)
            .iter()
            .filter(|t| t.founded <= era.start + 20)
            .map(|t| t.id)
            .collect();
    if towns.is_empty() {
        return;
    }

    // Gods, from the first era on: a few more as the world fills.
    let gods = if e == 0 { s.rng.range(3, 6) } else { 1 };
    for _ in 0..gods {
        let of = *s.rng.pick(GODS_OF);
        if s.h.deities.iter().any(|d| d.of == of) {
            continue;
        }
        let id = s.h.deities.len();
        let settlement = *s.rng.pick(&towns);
        let festival = *s.rng.pick(SEASONS);
        s.h.deities.push(Deity {
            id,
            of,
            settlement,
            festival,
        });
    }

    // People of every town: craftsmen, merchants, builders, healers, and
    // the families they marry into.
    for &town in &towns {
        let size = s.h.settlements[town].size as u32;
        for _ in 0..2 + size {
            let role = *s.rng.pick(&[
                Role::Merchant,
                Role::Builder,
                Role::Healer,
                Role::Smith,
                Role::Commoner,
                Role::Merchant,
            ]);
            let born = era.start + s.rng.below((len - 40).max(1) as u32) as i32;
            let p = s.person(lang, maker, e, born, role, town, None);
            s.event(e, born, town, EventKind::Birth { person: p }, vec![p], None);
            let died = s.h.people[p].died;
            if died < era.end {
                s.event(
                    e,
                    died,
                    town,
                    EventKind::Death {
                        person: p,
                        cause: Cause::Age,
                    },
                    vec![p],
                    None,
                );
            }
            // A marriage, now and then into a family of the town.
            if s.rng.chance(45) {
                let year = born + 18 + s.rng.below(8) as i32;
                let others: Vec<usize> = s
                    .adults(town, year)
                    .into_iter()
                    .filter(|&o| o != p && s.h.people[o].relation.is_none())
                    .collect();
                if !others.is_empty() && year < era.end {
                    let o = *s.rng.pick(&others);
                    s.event(
                        e,
                        year,
                        town,
                        EventKind::Marriage { a: p, b: o },
                        vec![p, o],
                        None,
                    );
                }
            }
        }
    }

    // Institutions: every town a temple; larger towns a council, a guild,
    // a court; the greatest a school.
    for &town in &towns {
        let t = &s.h.settlements[town];
        let (size, capital) = (t.size, t.capital);
        let founded_at = t.founded.max(era.start) + s.rng.below(30) as i32;
        let mut kinds = vec![InstitutionKind::Temple];
        if size >= 2 || capital {
            kinds.push(InstitutionKind::Council);
            kinds.push(InstitutionKind::Guild);
        }
        if size >= 3 || capital {
            kinds.push(InstitutionKind::Court);
        }
        if capital {
            kinds.push(InstitutionKind::School);
        }
        for kind in kinds {
            // An institution lasts: founded once, led anew each era.
            let existing =
                s.h.institutions
                    .iter()
                    .find(|i| i.settlement == town && i.kind == kind)
                    .map(|i| i.id);
            let inst = match existing {
                Some(i) => i,
                None => {
                    let id = s.h.institutions.len();
                    let craft = (kind == InstitutionKind::Guild)
                        .then(|| *s.rng.pick(&["smith", "potter", "weaver", "mason"]));
                    s.h.institutions.push(Institution {
                        id,
                        kind,
                        settlement: town,
                        era: e,
                        founded: founded_at,
                        heads: Vec::new(),
                        craft,
                    });
                    s.event(
                        e,
                        founded_at,
                        town,
                        EventKind::Founded { institution: id },
                        vec![],
                        None,
                    );
                    id
                }
            };
            let role = match kind {
                InstitutionKind::Temple => Role::Priest,
                InstitutionKind::Guild => Role::Smith,
                InstitutionKind::Council => Role::Elder,
                InstitutionKind::Court => Role::Judge,
                InstitutionKind::School => Role::Scribe,
            };
            let year = founded_at + 5 + s.rng.below(20) as i32;
            let head = match s.adults_with(town, year, role).first() {
                Some(&p) => p,
                None => {
                    let born = year - 30 - s.rng.below(10) as i32;
                    s.person(lang, maker, e, born, role, town, None)
                }
            };
            s.h.institutions[inst].heads.push(head);
            s.event(
                e,
                year,
                town,
                EventKind::Appointment {
                    person: head,
                    institution: inst,
                },
                vec![head],
                None,
            );
        }
    }

    // The stories of the era.
    let year_in = |rng: &mut Rng| era.start + 20 + rng.below((len - 40).max(1) as u32) as i32;
    // DESIGN-Q: how many stories of each kind an era holds.
    for _ in 0..s.rng.range(3, 6) {
        let year = year_in(&mut s.rng);
        trial(s, e, year, &towns);
    }
    for _ in 0..s.rng.range(2, 4) {
        let year = year_in(&mut s.rng);
        feud(s, e, year, &towns);
    }
    for _ in 0..s.rng.range(2, 4) {
        let year = year_in(&mut s.rng);
        project(s, e, year, &towns, lang, maker, None);
    }
    for _ in 0..s.rng.range(1, 3) {
        let year = year_in(&mut s.rng);
        disaster(s, e, year, &towns, lang, maker);
    }
    for _ in 0..s.rng.range(2, 4) {
        let year = year_in(&mut s.rng);
        venture(s, e, year, &towns);
    }
    for _ in 0..s.rng.range(1, 3) {
        let year = year_in(&mut s.rng);
        omen(s, e, year, &towns);
    }
    for _ in 0..s.rng.range(2, 4) {
        let year = year_in(&mut s.rng);
        letters(s, e, year, &towns);
    }
    lives(s, e);
    // Festivals of the gods, and laws.
    for d in 0..s.h.deities.len() {
        let town = s.h.deities[d].settlement;
        if s.h.settlements[town].founded <= era.end
            && s.h.settlements[town]
                .abandoned
                .is_none_or(|a| a > era.start + 20)
        {
            let year = year_in(&mut s.rng);
            s.event(
                e,
                year,
                town,
                EventKind::Festival {
                    deity: d,
                    settlement: town,
                },
                vec![],
                None,
            );
        }
    }
    for _ in 0..s.rng.range(1, 3) {
        let year = year_in(&mut s.rng);
        let town = *s.rng.pick(&towns);
        if let Some(ruler) = s.ruler_at(year, town) {
            let (verb, object) = *s.rng.pick(LAWS);
            let forbidden = s.rng.chance(75);
            s.event(
                e,
                year,
                town,
                EventKind::Decree {
                    ruler,
                    settlement: town,
                    law: Law {
                        verb,
                        object,
                        forbidden,
                    },
                },
                vec![ruler],
                None,
            );
        }
    }
}

/// A theft, the dispute before the court, and the judgement.
fn trial(s: &mut Society, e: u32, year: i32, towns: &[usize]) {
    let Some(court) = court_near(s, towns) else {
        return;
    };
    let town = s.h.institutions[court].settlement;
    let people = s.adults(town, year);
    if people.len() < 3 {
        return;
    }
    let thief = *s.rng.pick(&people);
    let victim = *s.rng.pick(&people);
    if thief == victim {
        return;
    }
    let good = *s.rng.pick(GOODS);
    let theft = s.event(
        e,
        year,
        town,
        EventKind::Theft {
            thief,
            victim,
            good,
        },
        vec![thief, victim],
        None,
    );
    let dispute = s.event(
        e,
        year + 1,
        town,
        EventKind::Dispute {
            plaintiff: victim,
            defendant: thief,
            court,
        },
        vec![victim, thief],
        Some(theft),
    );
    let judge = s.h.institutions[court]
        .heads
        .last()
        .copied()
        .filter(|&j| s.h.people[j].died > year + 1 && j != thief && j != victim)
        .or_else(|| people.iter().copied().find(|&p| p != thief && p != victim));
    // No one may judge their own case.
    let Some(judge) = judge else {
        return;
    };
    // Most thieves are found out; some walk free.
    let (winner, loser) = if s.rng.chance(75) {
        (victim, thief)
    } else {
        (thief, victim)
    };
    let judgement = s.event(
        e,
        year + 1,
        town,
        EventKind::Judgement {
            judge,
            winner,
            loser,
        },
        vec![judge, winner, loser],
        Some(dispute),
    );
    s.arc(
        ArcKind::Trial,
        vec![theft, dispute, judgement],
        vec![thief, victim, judge],
    );
}

fn court_near(s: &mut Society, towns: &[usize]) -> Option<usize> {
    // Where there is no court, the elders judge; where there are no
    // elders, the priests.
    let town = *s.rng.pick(towns);
    [
        InstitutionKind::Court,
        InstitutionKind::Council,
        InstitutionKind::Temple,
    ]
    .iter()
    .find_map(|k| {
        s.h.institutions
            .iter()
            .find(|i| i.kind == *k && i.settlement == town)
            .map(|i| i.id)
    })
}

/// A loan not repaid, a quarrel between two houses, a judgement, and
/// years later a reconciliation (or none).
fn feud(s: &mut Society, e: u32, year: i32, towns: &[usize]) {
    let town = *s.rng.pick(towns);
    let people = s.adults(town, year);
    if people.len() < 3 {
        return;
    }
    let a = *s.rng.pick(&people);
    let b = *s.rng.pick(&people);
    if a == b {
        return;
    }
    let good = *s.rng.pick(GOODS);
    let amount = s.rng.range(5, 60) as u16;
    let loan = s.event(
        e,
        year,
        town,
        EventKind::Loan {
            lender: a,
            borrower: b,
            good,
            amount,
        },
        vec![a, b],
        None,
    );
    let feud = s.event(
        e,
        year + 3,
        town,
        EventKind::Feud { a, b },
        vec![a, b],
        Some(loan),
    );
    let mut events = vec![loan, feud];
    let mut people_in = vec![a, b];
    let judges = [
        InstitutionKind::Court,
        InstitutionKind::Council,
        InstitutionKind::Temple,
    ];
    if let Some(court) = judges.iter().find_map(|k| {
        s.h.institutions
            .iter()
            .find(|i| i.kind == *k && i.settlement == town)
            .map(|i| i.id)
    }) {
        let dispute = s.event(
            e,
            year + 4,
            town,
            EventKind::Dispute {
                plaintiff: a,
                defendant: b,
                court,
            },
            vec![a, b],
            Some(feud),
        );
        let judge = s.h.institutions[court]
            .heads
            .last()
            .copied()
            .filter(|&j| j != a && j != b)
            .or_else(|| people.iter().copied().find(|&p| p != a && p != b))
            .unwrap_or(a);
        let judgement = s.event(
            e,
            year + 4,
            town,
            EventKind::Judgement {
                judge,
                winner: a,
                loser: b,
            },
            vec![judge, a, b],
            Some(dispute),
        );
        events.extend([dispute, judgement]);
        people_in.push(judge);
    }
    let end = year + 10 + s.rng.below(15) as i32;
    if s.rng.chance(60) && end < s.h.people[a].died.min(s.h.people[b].died) {
        events.push(s.event(
            e,
            end,
            town,
            EventKind::Reconciliation { a, b },
            vec![a, b],
            Some(feud),
        ));
    }
    s.arc(ArcKind::Feud, events, people_in);
}

/// A work begun by a patron and a builder; finished, or given up when
/// the patron dies or the money runs out.
#[allow(clippy::too_many_arguments)]
fn project(
    s: &mut Society,
    e: u32,
    year: i32,
    towns: &[usize],
    lang: &Language,
    maker: &mut scraped_lang::lexicon::WordMaker,
    after: Option<(usize, usize)>,
) -> Option<Vec<usize>> {
    let town = after.map_or_else(|| *s.rng.pick(towns), |(t, _)| t);
    let patron = s
        .ruler_at(year, town)
        .or_else(|| s.adults_with(town, year, Role::Merchant).first().copied())?;
    let builder = match s.adults_with(town, year, Role::Builder).first() {
        Some(&b) => b,
        None => {
            let born = year - 25 - s.rng.below(15) as i32;
            s.person(lang, maker, e, born, Role::Builder, town, None)
        }
    };
    let thing = *s.rng.pick(WORKS);
    let id = s.h.projects.len();
    let years = 4 + s.rng.below(20) as i32;
    let finished = s.rng.chance(70) && year + years < s.h.people[builder].died;
    let ended = year + years;
    s.h.projects.push(Project {
        id,
        thing,
        settlement: town,
        patron,
        builder,
        era: e,
        begun: year,
        ended,
        finished,
    });
    let begun = s.event(
        e,
        year,
        town,
        EventKind::ProjectBegun { project: id },
        vec![patron, builder],
        after.map(|(_, ev)| ev),
    );
    let mut events = vec![begun];
    if !finished && s.rng.chance(50) {
        let good = *s.rng.pick(GOODS);
        events.push(s.event(
            e,
            year + years / 2,
            town,
            EventKind::Shortage {
                settlement: town,
                good,
            },
            vec![],
            Some(begun),
        ));
    }
    let end = if finished {
        EventKind::ProjectFinished { project: id }
    } else {
        EventKind::ProjectAbandoned { project: id }
    };
    events.push(s.event(e, ended, town, end, vec![patron, builder], Some(begun)));
    if after.is_none() {
        s.arc(ArcKind::Project, events.clone(), vec![patron, builder]);
    }
    Some(events)
}

/// A flood (by a river), fire or earthquake; the shortage after it; a
/// ruler's decree; and the rebuilding.
fn disaster(
    s: &mut Society,
    e: u32,
    year: i32,
    towns: &[usize],
    lang: &Language,
    maker: &mut scraped_lang::lexicon::WordMaker,
) {
    let town = *s.rng.pick(towns);
    let kind = if s.by_river(town) && s.rng.chance(60) {
        EventKind::Flood { settlement: town }
    } else if s.rng.chance(60) {
        EventKind::Fire { settlement: town }
    } else {
        EventKind::Earthquake { settlement: town }
    };
    let hit = s.event(e, year, town, kind, vec![], None);
    let mut events = vec![hit];
    let mut people = Vec::new();
    // Some die in it.
    let victims: Vec<usize> = s
        .adults(town, year)
        .into_iter()
        .filter(|&p| s.h.people[p].role != Role::Ruler)
        .take(2)
        .collect();
    for v in victims {
        if s.rng.chance(50) {
            s.h.people[v].died = year;
            events.push(s.event(
                e,
                year,
                town,
                EventKind::Death {
                    person: v,
                    cause: Cause::Disaster,
                },
                vec![v],
                Some(hit),
            ));
            people.push(v);
        }
    }
    let good = *s.rng.pick(&["grain", "oil", "wine", "fish", "cloth"]);
    events.push(s.event(
        e,
        year + 1,
        town,
        EventKind::Shortage {
            settlement: town,
            good,
        },
        vec![],
        Some(hit),
    ));
    if let Some(ruler) = s.ruler_at(year + 1, town) {
        events.push(s.event(
            e,
            year + 1,
            town,
            EventKind::Decree {
                ruler,
                settlement: town,
                law: Law {
                    verb: "sell",
                    object: good,
                    forbidden: true,
                },
            },
            vec![ruler],
            Some(hit),
        ));
        people.push(ruler);
    }
    if let Some(rebuilt) = project(s, e, year + 2, towns, lang, maker, Some((town, hit))) {
        events.extend(rebuilt);
    }
    s.arc(ArcKind::Disaster, events, people);
}

/// A merchant carries goods to another town: a glut there, a shortage at
/// home, or a loan to pay for the next journey.
fn venture(s: &mut Society, e: u32, year: i32, towns: &[usize]) {
    if towns.len() < 2 {
        return;
    }
    let from = *s.rng.pick(towns);
    let to = *s.rng.pick(towns);
    if from == to {
        return;
    }
    let merchants = s.adults_with(from, year, Role::Merchant);
    let Some(&merchant) = merchants.first() else {
        return;
    };
    let good = *s.rng.pick(GOODS);
    let go = s.event(
        e,
        year,
        from,
        EventKind::Venture {
            merchant,
            from,
            to,
            good,
        },
        vec![merchant],
        None,
    );
    let mut events = vec![go];
    let after = if s.rng.chance(50) {
        EventKind::Glut {
            settlement: to,
            good,
        }
    } else {
        EventKind::Shortage {
            settlement: from,
            good,
        }
    };
    let at = match after {
        EventKind::Glut { .. } => to,
        _ => from,
    };
    events.push(s.event(e, year + 1, at, after, vec![], Some(go)));
    let lenders: Vec<usize> = s
        .adults(to, year + 2)
        .into_iter()
        .filter(|&p| p != merchant)
        .collect();
    if let Some(&lender) = lenders.first() {
        let amount = s.rng.range(10, 90) as u16;
        events.push(s.event(
            e,
            year + 2,
            to,
            EventKind::Loan {
                lender,
                borrower: merchant,
                good,
                amount,
            },
            vec![lender, merchant],
            Some(go),
        ));
    }
    s.arc(ArcKind::Venture, events, vec![merchant]);
}

/// A sign in the sky, a question put to a god, a vow.
fn omen(s: &mut Society, e: u32, year: i32, towns: &[usize]) {
    if s.h.deities.is_empty() {
        return;
    }
    let town = *s.rng.pick(towns);
    let sign = *s.rng.pick(&["eclipse", "comet", "star"]);
    let seen = s.event(
        e,
        year,
        town,
        EventKind::Omen {
            settlement: town,
            sign,
        },
        vec![],
        None,
    );
    let asker = s
        .ruler_at(year, town)
        .or_else(|| s.adults(town, year).first().copied());
    let Some(asker) = asker else {
        return;
    };
    let deity = s.rng.below(s.h.deities.len() as u32) as usize;
    let oracle = s.event(
        e,
        year,
        town,
        EventKind::Oracle {
            deity,
            settlement: town,
            asker,
        },
        vec![asker],
        Some(seen),
    );
    let vow = s.event(
        e,
        year + 1,
        town,
        EventKind::Vow {
            person: asker,
            deity,
        },
        vec![asker],
        Some(oracle),
    );
    s.arc(ArcKind::Omen, vec![seen, oracle, vow], vec![asker]);
}

/// Two people in different towns who write to each other over the years:
/// one married away from home, or kin apart.
fn letters(s: &mut Society, e: u32, year: i32, towns: &[usize]) {
    if towns.len() < 2 {
        return;
    }
    let (t1, t2) = (*s.rng.pick(towns), *s.rng.pick(towns));
    if t1 == t2 {
        return;
    }
    let (Some(&a), Some(&b)) = (s.adults(t1, year).first(), s.adults(t2, year).last()) else {
        return;
    };
    let wed = s.event(e, year, t2, EventKind::Marriage { a, b }, vec![a, b], None);
    let mut events = vec![wed];
    // Life goes on at home: what the letters tell of.
    let good = *s.rng.pick(GOODS);
    let later = year + 3 + s.rng.below(5) as i32;
    events.push(s.event(
        e,
        later,
        t1,
        EventKind::Shortage {
            settlement: t1,
            good,
        },
        vec![],
        None,
    ));
    s.arc(ArcKind::Letters, events, vec![a, b]);
}

/// Lives worth telling: the heads of institutions and builders of works,
/// birth to death, with what they did in between.
fn lives(s: &mut Society, e: u32) {
    let notable: Vec<usize> =
        s.h.projects
            .iter()
            .filter(|p| p.era == e)
            .map(|p| p.builder)
            .chain(
                s.h.institutions
                    .iter()
                    .filter_map(|i| i.heads.last().copied())
                    .filter(|&p| s.h.people[p].era == e),
            )
            .collect();
    let mut told = Vec::new();
    for p in notable {
        if told.contains(&p) || told.len() >= 3 {
            continue;
        }
        let mut events: Vec<usize> =
            s.h.events
                .iter()
                .filter(|ev| ev.actors.contains(&p))
                .map(|ev| ev.id)
                .collect();
        if events.len() < 3 {
            continue;
        }
        events.sort_by_key(|&i| s.h.events[i].year);
        told.push(p);
        s.arc(ArcKind::Life, events, vec![p]);
    }
}
