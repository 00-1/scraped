//! What the writing says (D08): the genres people wrote in, each built as
//! meaning from the history and rendered through the era's grammar.
//!
//! Annals and king lists in archives, decrees in council halls, court
//! records, contracts and receipts, prayers, hymns and myths in temples,
//! instructions beside the work they explain, school lessons, boundary
//! stones and milestones, building inscriptions, graffiti, curses,
//! blessings and calendars. The stories of [`crate::society`] are told in
//! several of these, in several places, so a reader pieces them together
//! by travelling.
//!
//! Only words the era's language has are used: culture words where the
//! culture has them, plain ones otherwise.

use scraped_lang::meaning::{
    Argument, Aspect, Clause, Complement, Conj, Head, Link, Mood, NounPhrase, Number, Polarity,
    Relative, Role as ArgRole, Sentence, Subordinate, Tense,
};
use scraped_lang::rng::{Rng, Stream};
use scraped_lang::Language;

use crate::history::{Event, EventKind, History, Role};
use crate::society::{ArcKind, InstitutionKind};
use crate::structures::{Structure, StructureKind as K};
use crate::texts::{put, spot, Genre, Spot, Text, Written};

// ---------- building meanings ----------

fn np(id: &str) -> NounPhrase {
    NounPhrase::concept(id)
}

fn plural(id: &str) -> NounPhrase {
    let mut n = np(id);
    n.number = Number::Plural;
    n
}

fn det(mut n: NounPhrase, d: &str) -> NounPhrase {
    n.determiner = Some(d.to_string());
    n
}

fn adj(mut n: NounPhrase, a: &str) -> NounPhrase {
    n.adjectives.push(a.to_string());
    n
}

fn of(n: NounPhrase, owner: NounPhrase) -> NounPhrase {
    n.with_possessor(owner)
}

fn clause(predicate: &str, subject: Option<NounPhrase>, object: Option<NounPhrase>) -> Clause {
    let mut args = Vec::new();
    if let Some(s) = subject {
        args.push(Argument {
            role: ArgRole::Subject,
            np: s,
        });
    }
    if let Some(o) = object {
        args.push(Argument {
            role: ArgRole::Object,
            np: o,
        });
    }
    Clause {
        predicate: predicate.to_string(),
        mood: Mood::Declarative,
        tense: Tense::Past,
        polarity: Polarity::Positive,
        args,
        adverbs: Vec::new(),
        aspect: Aspect::Simple,
        subordinate: Vec::new(),
        complement: None,
    }
}

/// Small changes to a clause, chained.
trait Tweak {
    fn now(self) -> Self;
    fn not(self) -> Self;
    fn mood(self, m: Mood) -> Self;
    fn to(self, n: NounPhrase) -> Self;
    fn when(self, n: Option<NounPhrase>) -> Self;
    fn adv(self, a: &str) -> Self;
    fn sub(self, link: Link, c: Clause) -> Self;
    fn says(self, direct: bool, content: Sentence) -> Self;
    fn aspect(self, a: Aspect, lang: &Language) -> Self;
}

impl Tweak for Clause {
    fn now(mut self) -> Self {
        self.tense = Tense::NonPast;
        self
    }
    fn not(mut self) -> Self {
        self.polarity = Polarity::Negative;
        self
    }
    fn mood(mut self, m: Mood) -> Self {
        self.mood = m;
        if m == Mood::Imperative {
            self.args.retain(|a| a.role != ArgRole::Subject);
            self.tense = Tense::NonPast;
        }
        if m == Mood::Optative {
            self.tense = Tense::NonPast;
        }
        self
    }
    fn to(mut self, n: NounPhrase) -> Self {
        self.args.push(Argument {
            role: ArgRole::Recipient,
            np: n,
        });
        self
    }
    fn when(mut self, n: Option<NounPhrase>) -> Self {
        if let Some(n) = n {
            self.args.push(Argument {
                role: ArgRole::Time,
                np: n,
            });
        }
        self
    }
    fn adv(mut self, a: &str) -> Self {
        self.adverbs.push(a.to_string());
        self
    }
    fn sub(mut self, link: Link, c: Clause) -> Self {
        self.subordinate.push(Subordinate { link, clause: c });
        self
    }
    fn says(mut self, direct: bool, content: Sentence) -> Self {
        self.complement = Some(Box::new(Complement { direct, content }));
        self
    }
    /// The aspect, where the language marks it.
    fn aspect(mut self, a: Aspect, lang: &Language) -> Self {
        if lang.morphology.aspect == a {
            self.aspect = a;
        }
        self
    }
}

fn one(c: Clause) -> Sentence {
    Sentence::Clause(c)
}

/// Removes embedded clauses a clause may not carry where it is going
/// (inside a quotation or another clause: one level only).
fn flat(mut c: Clause) -> Clause {
    c.subordinate.clear();
    c.complement = None;
    for a in &mut c.args {
        a.np.relative = None;
    }
    c
}

/// What the writers of one era had to work with.
struct Scribe<'a> {
    h: &'a History,
    lang: &'a Language,
    /// Reigns: (ruler, faction, first year, last year).
    reigns: &'a [(usize, usize, i32, i32)],
}

impl Scribe<'_> {
    fn has(&self, id: &str) -> bool {
        self.lang.lexicon.has(id)
    }

    /// The first word the language has; the last is always a core word.
    fn word<'w>(&self, ids: &[&'w str]) -> &'w str {
        ids.iter()
            .copied()
            .find(|w| self.has(w))
            .unwrap_or(ids[ids.len() - 1])
    }

    fn name(&self, p: usize) -> NounPhrase {
        NounPhrase::name(p)
    }

    /// A person with their calling: "Ana, the smith".
    fn who(&self, p: usize) -> NounPhrase {
        let mut n = NounPhrase::name(p);
        if let Some(t) = self.h.people[p].role.title() {
            if self.has(t) {
                n.apposition.push(np(t));
            }
        }
        n
    }

    /// A person with their parent: "Ana, daughter of Bel".
    fn child_of(&self, p: usize) -> NounPhrase {
        let mut n = self.who(p);
        if let Some((kin, parent)) = &self.h.people[p].relation {
            if self.has(kin) {
                n.apposition
                    .insert(0, np(kin).with_possessor(NounPhrase::name(*parent)));
            }
        }
        n
    }

    fn town(&self, s: usize) -> NounPhrase {
        NounPhrase::name(self.h.people.len() + s)
    }

    fn townsfolk(&self, s: usize) -> NounPhrase {
        of(plural("people"), self.town(s))
    }

    fn god(&self, d: usize) -> NounPhrase {
        of(np("god"), np(self.h.deities[d].of))
    }

    fn good(&self, g: &str) -> NounPhrase {
        let mut n = np(g);
        if !["grain", "oil", "wine", "salt", "cloth"].contains(&g) {
            n.number = Number::Plural;
        }
        n
    }

    /// "In the Nth year of King X", for a year in a town's realm.
    fn year(&self, faction: usize, year: i32) -> Option<NounPhrase> {
        let &(ruler, _, start, _) = self
            .reigns
            .iter()
            .find(|(_, f, a, b)| *f == faction && *a <= year && year < *b)?;
        let mut n = np("year").with_possessor(NounPhrase::name(ruler));
        n.ordinal = Some((year - start + 1).clamp(1, 60) as u16);
        Some(n)
    }

    fn year_at(&self, ev: &Event) -> Option<NounPhrase> {
        let s = self.h.settlements.iter().find(|s| s.cell == ev.cell)?;
        self.year(s.faction, ev.year)
    }

    fn institution(&self, i: usize) -> NounPhrase {
        let inst = &self.h.institutions[i];
        let id = match inst.kind {
            InstitutionKind::Temple => "temple",
            InstitutionKind::Court => self.word(&["court", "judge+place"]),
            InstitutionKind::Council => self.word(&["council", "rule+place"]),
            InstitutionKind::School => self.word(&["teach+place"]),
            InstitutionKind::Guild => self.word(&["workshop", "make+place"]),
        };
        of(np(id), self.town(inst.settlement))
    }

    fn work(&self, p: usize) -> NounPhrase {
        let pr = &self.h.projects[p];
        of(np(self.word(&[pr.thing, "wall"])), self.town(pr.settlement))
    }

    /// One clause telling what happened, as a scribe would put it: each
    /// event put its own way (a title here, a word of colour there), fixed
    /// by the event so every copy agrees.
    fn about(&self, ev: &Event) -> Option<Clause> {
        let c = self.bare(ev)?;
        Some(self.vary(c, ev.id as u64))
    }

    fn vary(&self, c: Clause, key: u64) -> Clause {
        vary(self.h, self.lang, c, key)
    }

    fn bare(&self, ev: &Event) -> Option<Clause> {
        let h = self.h;
        Some(match &ev.kind {
            EventKind::Founding { settlement } => clause(
                "build",
                Some(plural("people")),
                Some(self.town(*settlement)),
            ),
            EventKind::Abandonment { settlement, .. } => clause(
                "leave",
                Some(self.townsfolk(*settlement)),
                Some(self.town(*settlement)),
            ),
            EventKind::Succession { ruler, .. } => clause(
                "rule",
                Some(self.name(*ruler)),
                Some(self.town(h.people[*ruler].settlement)),
            ),
            EventKind::Death { person, .. } => clause("die", Some(self.name(*person)), None),
            EventKind::War { settlement, .. } => clause(
                "break",
                Some(plural("enemy")),
                Some(of(np("wall"), self.town(*settlement))),
            ),
            EventKind::Plague { settlement } => {
                clause("die", Some(det(self.townsfolk(*settlement), "many")), None)
            }
            EventKind::Famine { settlement } => {
                clause("eat", Some(self.townsfolk(*settlement)), Some(np("grain"))).not()
            }
            EventKind::Migration { from, to } => {
                clause("build", Some(self.townsfolk(*from)), Some(self.town(*to)))
            }
            EventKind::Marriage { a, b } => {
                clause("marry", Some(self.name(*a)), Some(self.name(*b)))
            }
            EventKind::Founded { institution } => {
                let s = h.institutions[*institution].settlement;
                clause(
                    "build",
                    Some(self.townsfolk(s)),
                    Some(self.institution(*institution)),
                )
            }
            EventKind::Appointment {
                person,
                institution,
            } => clause(
                "serve",
                Some(self.who(*person)),
                Some(self.institution(*institution)),
            ),
            EventKind::Venture {
                merchant, to, good, ..
            } => clause("carry", Some(self.name(*merchant)), Some(self.good(good)))
                .to(self.townsfolk(*to)),
            EventKind::Shortage { settlement, good } => clause(
                "find",
                Some(self.townsfolk(*settlement)),
                Some(self.good(good)),
            )
            .not(),
            EventKind::Glut { settlement, good } => clause(
                "sell",
                Some(self.townsfolk(*settlement)),
                Some(det(self.good(good), "many")),
            ),
            EventKind::Loan {
                lender,
                borrower,
                good,
                amount,
            } => clause(
                "lend",
                Some(self.name(*lender)),
                Some(np(good).counted(*amount)),
            )
            .to(self.name(*borrower)),
            EventKind::Festival { deity, settlement } => clause(
                "honour",
                Some(self.townsfolk(*settlement)),
                Some(self.god(*deity)),
            ),
            EventKind::Vow { person, deity } => clause(
                "give",
                Some(self.name(*person)),
                Some(np(self.word(&["offering", "gift", "bowl"]))),
            )
            .to(self.god(*deity)),
            EventKind::Omen { settlement, sign } => {
                clause("see", Some(self.townsfolk(*settlement)), Some(np(sign)))
            }
            EventKind::Oracle { deity, asker, .. } => {
                clause("say", Some(self.god(*deity)), None).to(self.name(*asker))
            }
            EventKind::ProjectBegun { project } => {
                let p = &h.projects[*project];
                clause(
                    "dig",
                    Some(self.name(p.builder)),
                    Some(of(np("earth"), np(self.word(&[p.thing, "wall"])))),
                )
                .to(self.name(p.patron))
            }
            EventKind::ProjectFinished { project } => {
                let p = &h.projects[*project];
                clause(
                    "build",
                    Some(self.name(p.builder)),
                    Some(self.work(*project)),
                )
                .to(self.name(p.patron))
            }
            EventKind::ProjectAbandoned { project } => {
                let p = &h.projects[*project];
                clause(
                    "build",
                    Some(self.name(p.builder)),
                    Some(self.work(*project)),
                )
                .not()
            }
            EventKind::Flood { settlement } => clause(
                "take",
                Some(np("river")),
                Some(of(plural("house"), self.town(*settlement))),
            ),
            EventKind::Fire { settlement } => clause(
                "burn",
                Some(np("fire")),
                Some(of(plural("house"), self.town(*settlement))),
            ),
            EventKind::Earthquake { settlement } => clause(
                "break",
                Some(np("earth")),
                Some(of(plural("wall"), self.town(*settlement))),
            ),
            EventKind::Decree { ruler, law, .. } => {
                clause("say", Some(self.who(*ruler)), None).says(true, one(self.law(law)))
            }
            EventKind::Theft {
                thief,
                victim,
                good,
            } => clause(
                "steal",
                Some(self.name(*thief)),
                Some(of(self.good(good), self.name(*victim))),
            ),
            EventKind::Dispute {
                plaintiff,
                defendant,
                ..
            } => clause(
                "call",
                Some(self.name(*plaintiff)),
                Some(self.name(*defendant)),
            ),
            EventKind::Judgement { judge, loser, .. } => {
                clause("judge", Some(self.who(*judge)), Some(self.name(*loser)))
            }
            EventKind::Feud { a, b } => clause("hate", Some(self.name(*a)), Some(self.name(*b))),
            EventKind::Reconciliation { a, b } => {
                clause("love", Some(self.name(*a)), Some(self.name(*b))).adv("again")
            }
            EventKind::Schism { .. } | EventKind::Writing { .. } | EventKind::Birth { .. } => {
                return None
            }
        })
    }

    /// A law as a command: "do not sell grain".
    fn law(&self, law: &crate::society::Law) -> Clause {
        let object = match law.object {
            "person" => np("person"),
            o => self.good(o),
        };
        let c = clause(law.verb, None, Some(object)).mood(Mood::Imperative);
        if law.forbidden {
            c.not()
        } else {
            c
        }
    }
}

/// Puts an event or text its own way: a title, a word of colour, a word
/// of when, an aspect; fixed by `key`.
fn vary(h: &History, lang: &Language, mut c: Clause, key: u64) -> Clause {
    let k = key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 33;
    // The subject named with their calling, now and then.
    if k.is_multiple_of(3) {
        if let Some(a) = c.args.iter_mut().find(|a| a.role == ArgRole::Subject) {
            if let Head::Name(p) = a.np.head {
                if p < h.people.len() && a.np.apposition.is_empty() {
                    a.np = NounPhrase::name(p);
                    if let Some(t) = h.people[p].role.title() {
                        if lang.lexicon.has(t) {
                            a.np.apposition.push(np(t));
                        }
                    }
                }
            }
        }
    }
    // A word of colour on the thing done to.
    const COLOUR: &[&str] = &[
        "great",
        "old",
        "new",
        "small",
        "good",
        "white",
        "red",
        "dark",
        "high",
        "strong",
        "beautiful",
        "holy",
    ];
    if (k >> 4).is_multiple_of(2) {
        if let Some(a) = c.args.iter_mut().find(|a| a.role == ArgRole::Object) {
            if let Head::Concept(id) = &a.np.head {
                let id = id.clone();
                let fits = scraped_lang::concepts::get(&id);
                let pick = COLOUR[((k >> 8) as usize) % COLOUR.len()];
                let adj = scraped_lang::concepts::get(pick);
                let ok = adj.applies.is_empty() || adj.applies.iter().any(|t| fits.has_tag(t));
                if ok
                    && lang.lexicon.has(pick)
                    && a.np.adjectives.is_empty()
                    && a.np.quantity.is_none()
                {
                    a.np.adjectives.push(pick.to_string());
                }
            }
        }
    }
    // A word of when.
    if (k >> 12).is_multiple_of(4) && c.adverbs.is_empty() && c.mood == Mood::Declarative {
        c.adverbs
            .push(["then", "again", "there"][((k >> 16) % 3) as usize].to_string());
    }
    // How it went on, where the language marks it.
    if (k >> 20).is_multiple_of(4) && c.mood == Mood::Declarative {
        let a = lang.morphology.aspect;
        if a != Aspect::Simple {
            c.aspect = a;
        }
    }
    c
}

// ---------- placing ----------

/// Where each genre is kept: kinds of building (most fitting first) and
/// the surfaces in them.
fn home(g: Genre) -> (&'static [K], &'static [&'static str]) {
    match g {
        Genre::Annal | Genre::KingList => (
            &[K::Archive, K::Library, K::Palace, K::Temple],
            &["shelf", "chest", "tablet", "scroll", "wall"],
        ),
        Genre::Decree => (
            &[
                K::CouncilHall,
                K::Palace,
                K::MarketHall,
                K::Gatehouse,
                K::Temple,
            ],
            &["stele", "wall", "pillar", "gate"],
        ),
        Genre::Court => (
            &[K::Courthouse, K::Archive, K::Palace, K::CouncilHall],
            &["tablet", "shelf", "chest", "wall"],
        ),
        Genre::Contract | Genre::Receipt => (
            &[K::MarketHall, K::Warehouse, K::Storehouse, K::House],
            &["tablet", "chest", "table", "shelf"],
        ),
        Genre::Inventory => (
            &[K::Storehouse, K::Warehouse, K::Granary, K::Palace],
            &["tablet", "bin", "jar", "shelf", "crate"],
        ),
        Genre::Prayer | Genre::Hymn | Genre::Oracle => (
            &[K::Temple, K::WaysideShrine, K::Hermitage],
            &["altar", "wall", "statue", "niche", "stele"],
        ),
        Genre::Myth => (
            &[K::Temple, K::Library, K::Scriptorium],
            &["wall", "scroll", "shelf"],
        ),
        Genre::Lesson | Genre::WordList => (
            &[K::School, K::Scriptorium, K::Library, K::Temple],
            &["tablet", "desk", "table", "wall"],
        ),
        Genre::Calendar => (
            &[K::Observatory, K::Temple, K::Library],
            &["dial", "table", "wall", "stele"],
        ),
        Genre::Letter => (
            &[K::House, K::Palace],
            &["chest", "shelf", "niche", "table"],
        ),
        Genre::Curse => (
            &[K::Tomb, K::Mausoleum, K::Cemetery, K::House],
            &["sarcophagus", "door-slab", "gravestone", "chest"],
        ),
        Genre::Blessing => (&[K::House, K::Palace], &["lintel", "door-slab", "wall"]),
        Genre::Graffito => (
            &[
                K::House,
                K::MarketHall,
                K::Bathhouse,
                K::Barracks,
                K::Waystation,
            ],
            &["wall", "bench", "pillar"],
        ),
        Genre::Building => (&[K::Temple], &["lintel", "gate", "stele"]),
        Genre::Epitaph => (&[K::Cemetery], &["gravestone"]),
        _ => (&[K::House], &["wall"]),
    }
}

struct Pen<'a> {
    h: &'a History,
    langs: &'a [Language],
    texts: &'a mut Vec<Text>,
    structures: &'a mut [Structure],
}

/// Every name in a meaning, wherever it stands.
fn names_in(s: &Sentence, out: &mut Vec<usize>) {
    fn np(n: &NounPhrase, out: &mut Vec<usize>) {
        if let Head::Name(i) = n.head {
            out.push(i);
        }
        n.possessor.iter().for_each(|p| np(p, out));
        n.apposition.iter().for_each(|a| np(a, out));
        if let Some(d) = &n.degree {
            d.standard.iter().for_each(|s| np(s, out));
        }
        if let Some(r) = &n.relative {
            clause(&r.clause, out);
        }
    }
    fn clause(c: &Clause, out: &mut Vec<usize>) {
        c.args.iter().for_each(|a| np(&a.np, out));
        c.subordinate.iter().for_each(|s| clause(&s.clause, out));
        if let Some(k) = &c.complement {
            names_in(&k.content, out);
        }
    }
    match s {
        Sentence::Clause(c) => clause(c, out),
        Sentence::List(l) => l.iter().for_each(|n| np(n, out)),
        Sentence::Text(t) => t.iter().for_each(|s| names_in(s, out)),
        Sentence::Joined(_, cs) => cs.iter().for_each(|c| clause(c, out)),
    }
}

impl Pen<'_> {
    /// Where a genre is kept in a town, if the town has such a place.
    fn find(&self, g: Genre, s: usize) -> Option<Spot> {
        let (kinds, features) = home(g);
        spot(self.structures, s, kinds, features).or_else(|| {
            // A town without the proper place keeps it where it can.
            spot(
                self.structures,
                s,
                &[K::Archive, K::Temple, K::Palace, K::House],
                &["chest", "shelf", "niche", "wall"],
            )
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn write(
        &mut self,
        at: Spot,
        era: u32,
        year: i32,
        genre: Genre,
        meaning: Sentence,
        author: Option<usize>,
        event: Option<usize>,
        arc: Option<usize>,
    ) {
        // Written no earlier than the newest person or town it names.
        let mut names = Vec::new();
        names_in(&meaning, &mut names);
        let people = self.h.people.len();
        let newest = names
            .iter()
            .map(|&i| {
                if i < people {
                    self.h.people[i].era
                } else {
                    self.h.settlements[i - people].era
                }
            })
            .max()
            .unwrap_or(era);
        let (era, year) = if newest > era {
            (newest, year.max(self.h.eras[newest as usize].start))
        } else {
            (era, year)
        };
        // A text of one sentence is that sentence.
        let meaning = match meaning {
            Sentence::Text(mut parts) if parts.len() == 1 => parts.remove(0),
            m => m,
        };
        // Each text in its own words: the same event, the same words.
        let key = event.map_or(self.texts.len() as u64 + 1_000_000, |e| e as u64);
        let lang = &self.langs[(era as usize).min(self.langs.len() - 1)];
        let meaning = match meaning {
            Sentence::Clause(c) => Sentence::Clause(vary(self.h, lang, c, key)),
            Sentence::Text(parts) => Sentence::Text(
                parts
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| match p {
                        Sentence::Clause(c) => {
                            Sentence::Clause(vary(self.h, lang, c, key * 31 + i as u64))
                        }
                        other => other,
                    })
                    .collect(),
            ),
            Sentence::Joined(conj, cs) => Sentence::Joined(
                conj,
                cs.into_iter()
                    .enumerate()
                    .map(|(i, c)| vary(self.h, lang, c, key * 37 + i as u64))
                    .collect(),
            ),
            other => other,
        };
        put(
            self.texts,
            self.structures,
            at,
            Written {
                era,
                year,
                genre,
                meaning,
                author,
                event,
                arc,
            },
        );
    }
}

// ---------- the genres ----------

fn era_of(h: &History, year: i32) -> u32 {
    h.eras
        .iter()
        .find(|e| e.start <= year && year < e.end)
        .map_or(h.eras.len() as u32 - 1, |e| e.index)
}

/// Writes every genre's texts into the world.
pub fn write(
    seed: u64,
    h: &History,
    langs: &[Language],
    structures: &mut [Structure],
    texts: &mut Vec<Text>,
) {
    let mut rng = Rng::new(seed, Stream::World(23));
    let reigns: Vec<(usize, usize, i32, i32)> = h
        .events
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Succession { ruler, .. } => {
                let p = &h.people[ruler];
                Some((ruler, p.faction, e.year, p.died))
            }
            _ => None,
        })
        .collect();
    let scribe = |era: u32| Scribe {
        h,
        lang: &langs[(era as usize).min(langs.len() - 1)],
        reigns: &reigns,
    };
    let mut pen = Pen {
        h,
        langs,
        texts,
        structures,
    };

    epitaphs(h, &scribe, &mut pen);
    for arc in &h.arcs {
        tell(&mut rng, h, arc.id, &scribe, &mut pen);
    }
    decrees(h, &scribe, &mut pen);
    annals(h, &scribe, &mut pen);
    king_lists(h, &scribe, &mut pen);
    everyday(&mut rng, h, &scribe, &mut pen);
}

/// Epitaphs of people who did something tell what they did.
fn epitaphs<'a>(h: &History, scribe: &dyn Fn(u32) -> Scribe<'a>, pen: &mut Pen) {
    for t in 0..pen.texts.len() {
        if pen.texts[t].genre != Genre::Epitaph {
            continue;
        }
        let Some(EventKind::Death { person, .. }) = pen.texts[t].event.map(|e| &h.events[e].kind)
        else {
            continue;
        };
        let person = *person;
        let s = scribe(pen.texts[t].era);
        let deeds: Vec<Clause> = h
            .events
            .iter()
            .filter(|e| e.actors.first() == Some(&person))
            .filter(|e| {
                matches!(
                    e.kind,
                    EventKind::ProjectFinished { .. }
                        | EventKind::Appointment { .. }
                        | EventKind::Judgement { .. }
                        | EventKind::Venture { .. }
                        | EventKind::Vow { .. }
                        | EventKind::Decree { .. }
                )
            })
            .filter_map(|e| s.about(e).map(|c| flat(c).when(s.year_at(e))))
            .take(2)
            .collect();
        let mut lies = clause("lie", Some(s.child_of(person)), None)
            .now()
            .adv("here");
        if let Some(d) = s.year(h.people[person].faction, h.people[person].died) {
            lies = lies.sub(
                Link::After,
                clause("die", Some(s.name(person)), None).when(Some(d)),
            );
        }
        // Put its own way, like every other text.
        let lies = vary(h, s.lang, lies, 2_000_000 + t as u64);
        let mut parts = vec![one(lies)];
        parts.extend(deeds.into_iter().map(one));
        let meaning = if parts.len() == 1 {
            parts.remove(0)
        } else {
            Sentence::Text(parts)
        };
        // Only names the epitaph's own era knew.
        let mut names = Vec::new();
        names_in(&meaning, &mut names);
        let people = h.people.len();
        let era = pen.texts[t].era;
        let known = names.iter().all(|&i| {
            if i < people {
                h.people[i].era <= era
            } else {
                h.settlements[i - people].era <= era
            }
        });
        if known {
            pen.texts[t].meaning = meaning;
        }
    }
}

/// The stories, each told in several genres in several places.
fn tell<'a>(
    rng: &mut Rng,
    h: &History,
    a: usize,
    scribe: &dyn Fn(u32) -> Scribe<'a>,
    pen: &mut Pen,
) {
    let arc = &h.arcs[a];
    let Some(&first) = arc.events.first() else {
        return;
    };
    let ev0 = &h.events[first];
    let era = ev0.era;
    let s = scribe(era);
    let town = |e: usize| {
        h.settlements
            .iter()
            .find(|s| s.cell == h.events[e].cell)
            .map(|s| s.id)
    };
    let Some(home_town) = town(first) else {
        return;
    };
    let clauses: Vec<(usize, Clause)> = arc
        .events
        .iter()
        .filter_map(|&e| s.about(&h.events[e]).map(|c| (e, c)))
        .collect();
    let last = *arc.events.last().unwrap_or(&first);
    let year = h.events[last].year;
    let wrote = |pen: &mut Pen, g: Genre, place: usize, m: Sentence, author, event| {
        if let Some(at) = pen.find(g, place) {
            pen.write(at, era, year, g, m, author, Some(event), Some(a));
        }
    };
    // The story as a chronicle would tell it: each step in its year.
    let chronicle: Vec<Sentence> = clauses
        .iter()
        .take(5)
        .map(|(e, c)| one(c.clone().when(s.year_at(&h.events[*e]))))
        .collect();
    if !chronicle.is_empty() {
        wrote(
            pen,
            Genre::Annal,
            home_town,
            Sentence::Text(chronicle),
            None,
            first,
        );
    }
    match arc.kind {
        ArcKind::Trial | ArcKind::Feud => {
            // The court's record: what each side said, and the finding.
            let dispute = arc
                .events
                .iter()
                .copied()
                .find(|&e| matches!(h.events[e].kind, EventKind::Dispute { .. }));
            let judgement = arc
                .events
                .iter()
                .copied()
                .find(|&e| matches!(h.events[e].kind, EventKind::Judgement { .. }));
            if let (Some(d), Some(j)) = (dispute, judgement) {
                let (
                    EventKind::Dispute {
                        plaintiff,
                        defendant,
                        ..
                    },
                    EventKind::Judgement { judge, winner, .. },
                ) = (&h.events[d].kind, &h.events[j].kind)
                else {
                    return;
                };
                let charge = s
                    .about(&h.events[arc.events[0]])
                    .map(flat)
                    .unwrap_or_else(|| clause("hate", Some(s.name(*defendant)), None));
                let mut denial = charge.clone().not();
                denial.tense = Tense::Past;
                let record = Sentence::Text(vec![
                    one(clause("say", Some(s.name(*plaintiff)), None)
                        .to(s.who(*judge))
                        .says(false, one(charge))
                        .when(s.year_at(&h.events[d]))),
                    one(clause("say", Some(s.name(*defendant)), None).says(true, one(denial))),
                    one(
                        clause("judge", Some(s.who(*judge)), Some(s.name(*defendant))).sub(
                            Link::After,
                            clause(
                                "hear",
                                Some(s.name(*judge)),
                                Some(plural(s.word(&["witness", "elder"]))),
                            ),
                        ),
                    ),
                    one(clause(
                        "give",
                        Some(s.name(*judge)),
                        Some(np(s.word(&["fine", "silver", "ox"]))),
                    )
                    .to(s.name(*winner))),
                ]);
                wrote(pen, Genre::Court, home_town, record, None, j);
            }
            // The loan or theft, as the wronged one recorded it.
            if let EventKind::Loan {
                lender,
                borrower,
                good,
                amount,
            } = &h.events[first].kind
            {
                let back = (u32::from(*amount) * 5 / 4) as u16;
                let contract = Sentence::Text(vec![
                    one(s
                        .about(ev0)
                        .map(flat)
                        .unwrap_or_else(|| clause("lend", Some(s.name(*lender)), Some(np(good))))
                        .when(s.year_at(ev0))),
                    one(clause(
                        "give",
                        Some(s.name(*borrower)),
                        Some(np(good).counted(back)),
                    )
                    .now()
                    .to(s.name(*lender))
                    .when(Some(np(s.word(&["harvest", "autumn"]))))),
                    one(clause(
                        "take",
                        Some(s.name(*lender)),
                        Some(of(np("field"), s.name(*borrower))),
                    )
                    .now()
                    .sub(
                        Link::If,
                        clause("give", Some(s.name(*borrower)), Some(np(good)))
                            .now()
                            .not(),
                    )),
                ]);
                wrote(
                    pen,
                    Genre::Contract,
                    home_town,
                    contract,
                    Some(*lender),
                    first,
                );
            }
            if let EventKind::Theft { thief, victim, .. } = &h.events[first].kind {
                // The wronged one's mark: a curse, or a name on a wall.
                let mark = if h.deities.is_empty() || rng.chance(50) {
                    one(clause("steal", Some(s.name(*thief)), None).adv("here"))
                } else {
                    let d = rng.below(h.deities.len() as u32) as usize;
                    one(clause("curse", Some(s.god(d)), Some(s.name(*thief))).mood(Mood::Optative))
                };
                wrote(pen, Genre::Graffito, home_town, mark, Some(*victim), first);
            }
            if let Some(&end) = arc.events.last() {
                if let EventKind::Reconciliation { a: x, b: y } = &h.events[end].kind {
                    let gift = Sentence::Text(vec![one(clause(
                        "give",
                        Some(s.name(*x)),
                        Some(det(np("altar"), "this")),
                    )
                    .to(np("god"))
                    .sub(
                        Link::After,
                        clause("love", Some(s.name(*x)), Some(s.name(*y))),
                    ))]);
                    wrote(pen, Genre::Dedication, home_town, gift, Some(*x), end);
                }
            }
        }
        ArcKind::Project | ArcKind::Disaster => {
            for &e in &arc.events {
                match &h.events[e].kind {
                    EventKind::ProjectFinished { project } => {
                        let p = &h.projects[*project];
                        let kinds: &[K] = match p.thing {
                            "wall" => &[K::Wall, K::Gatehouse],
                            "bridge" => &[K::Bridge],
                            "tower" => &[K::Tower, K::SignalStation],
                            "temple" => &[K::Temple],
                            "granary" => &[K::Granary, K::Storehouse],
                            "palace" => &[K::Palace],
                            "hall" => &[K::CouncilHall, K::MarketHall],
                            "well" => &[K::Cistern, K::FountainHouse],
                            _ => &[K::Gatehouse, K::Temple],
                        };
                        let mut parts = vec![one(clause(
                            "build",
                            Some(s.who(p.builder)),
                            Some(det(np(s.word(&[p.thing, "wall"])), "this")),
                        )
                        .to(s.who(p.patron))
                        .when(s.year_at(&h.events[e])))];
                        if !h.deities.is_empty() {
                            parts.push(one(clause(
                                "bless",
                                Some(s.god(0)),
                                Some(det(np(s.word(&[p.thing, "wall"])), "this")),
                            )
                            .mood(Mood::Optative)));
                        }
                        let m = Sentence::Text(parts);
                        let at = spot(
                            pen.structures,
                            p.settlement,
                            kinds,
                            &["lintel", "gate", "stele", "wall", "pillar"],
                        )
                        .or_else(|| pen.find(Genre::Building, p.settlement));
                        if let Some(at) = at {
                            pen.write(
                                at,
                                era,
                                h.events[e].year,
                                Genre::Building,
                                m,
                                Some(p.patron),
                                Some(e),
                                Some(a),
                            );
                        }
                        // What it took, counted by the storekeeper.
                        let mut items: Vec<NounPhrase> =
                            ["stone", "timber", "brick", "rope", "oil", "loaf"]
                                .iter()
                                .filter(|w| s.has(w))
                                .take(3)
                                .map(|w| np(w).counted(rng.range(20, 120) as u16))
                                .collect();
                        let sum: u16 = items
                            .iter()
                            .filter_map(|i| i.quantity)
                            .fold(0u16, |a, b| a.saturating_add(b));
                        let mut total = np("total");
                        total.quantity = Some(sum);
                        items.push(total);
                        wrote(
                            pen,
                            Genre::Receipt,
                            p.settlement,
                            Sentence::List(items),
                            None,
                            e,
                        );
                    }
                    EventKind::ProjectAbandoned { project } => {
                        let p = &h.projects[*project];
                        let m = Sentence::Text(vec![
                            one(clause("say", Some(s.who(p.builder)), None)
                                .to(s.who(p.patron))
                                .says(
                                    true,
                                    one(clause(
                                        "build",
                                        Some(plural("man")),
                                        Some(s.work(*project)),
                                    )
                                    .not()
                                    .now()),
                                )),
                            one(clause("find", Some(s.name(p.builder)), Some(np("stone"))).not()),
                        ]);
                        wrote(pen, Genre::Letter, p.settlement, m, Some(p.builder), e);
                    }
                    EventKind::Flood { settlement }
                    | EventKind::Fire { settlement }
                    | EventKind::Earthquake { settlement } => {
                        if h.deities.is_empty() {
                            continue;
                        }
                        let d = rng.below(h.deities.len() as u32) as usize;
                        let hit = s.about(&h.events[e]).map(flat);
                        let mut prayer = vec![one(clause(
                            "keep",
                            Some(s.god(d)),
                            Some(s.townsfolk(*settlement)),
                        )
                        .mood(Mood::Optative))];
                        if let Some(hit) = hit {
                            prayer.push(one(clause("weep", Some(plural("people")), None)
                                .sub(Link::Because, hit)));
                        }
                        wrote(
                            pen,
                            Genre::Prayer,
                            *settlement,
                            Sentence::Text(prayer),
                            None,
                            e,
                        );
                    }
                    EventKind::Shortage { settlement, good } => {
                        let n = rng.range(1, 6) as u16;
                        let m = Sentence::List(vec![
                            np(good).counted(n),
                            adj(np("jar"), "empty").counted(rng.range(10, 40) as u16),
                        ]);
                        wrote(pen, Genre::Inventory, *settlement, m, None, e);
                    }
                    _ => {}
                }
            }
        }
        ArcKind::Venture => {
            if let EventKind::Venture {
                merchant,
                from,
                to,
                good,
            } = &ev0.kind
            {
                let cargo = Sentence::List(vec![
                    np(good).counted(rng.range(10, 200) as u16),
                    np("jar").counted(rng.range(2, 30) as u16),
                    of(np("seal"), s.name(*merchant)),
                ]);
                wrote(pen, Genre::Inventory, *to, cargo, Some(*merchant), first);
                let news = arc
                    .events
                    .iter()
                    .skip(1)
                    .filter_map(|&e| s.about(&h.events[e]).map(flat))
                    .next();
                let mut body = vec![one(clause(
                    "carry",
                    Some(s.name(*merchant)),
                    Some(s.good(good)),
                )
                .to(s.townsfolk(*to)))];
                if let Some(n) = news {
                    body.push(one(n));
                }
                body.push(one(clause(
                    "send",
                    None,
                    Some(np(s.word(&["silver", "coin", "sheep"])).counted(rng.range(2, 20) as u16)),
                )
                .mood(Mood::Imperative)
                .to(s.name(*merchant))));
                let kin = h
                    .people
                    .iter()
                    .find(|p| {
                        p.settlement == *from
                            && p.relation.as_ref().is_some_and(|r| r.1 == *merchant)
                            && p.born + 10 <= ev0.year
                    })
                    .map(|p| p.id)
                    .or_else(|| {
                        h.people
                            .iter()
                            .find(|p| {
                                p.settlement == *from
                                    && p.id != *merchant
                                    && p.born + 16 <= ev0.year
                                    && ev0.year < p.died
                            })
                            .map(|p| p.id)
                    });
                if let Some(kin) = kin {
                    let letter = Sentence::Text(
                        vec![one(
                            clause("say", Some(s.name(*merchant)), None).to(s.name(kin))
                        )]
                        .into_iter()
                        .chain(body)
                        .collect(),
                    );
                    wrote(pen, Genre::Letter, *from, letter, Some(*merchant), first);
                }
            }
            if let Some(&l) = arc
                .events
                .iter()
                .find(|&&e| matches!(h.events[e].kind, EventKind::Loan { .. }))
            {
                if let EventKind::Loan {
                    lender,
                    borrower,
                    good,
                    amount,
                } = &h.events[l].kind
                {
                    let m = Sentence::Text(vec![
                        one(s
                            .about(&h.events[l])
                            .map(flat)
                            .unwrap_or_else(|| {
                                clause("lend", Some(s.name(*lender)), Some(np(good)))
                            })
                            .when(s.year_at(&h.events[l]))),
                        one(clause(
                            "give",
                            Some(s.name(*borrower)),
                            Some(np(good).counted(amount.saturating_add(amount / 4))),
                        )
                        .now()
                        .to(s.name(*lender))),
                    ]);
                    if let Some(t) = town(l) {
                        wrote(pen, Genre::Contract, t, m, Some(*lender), l);
                    }
                }
            }
        }
        ArcKind::Omen => {
            for &e in &arc.events {
                match &h.events[e].kind {
                    EventKind::Oracle {
                        deity,
                        settlement,
                        asker,
                    } => {
                        let question = clause("fall", Some(s.town(*settlement)), None)
                            .now()
                            .mood(Mood::Interrogative);
                        let answer = Sentence::Joined(
                            Conj::And,
                            vec![
                                clause(
                                    "give",
                                    None,
                                    Some(np(s.word(&["offering", "gift", "bowl"]))),
                                )
                                .mood(Mood::Imperative)
                                .to(s.god(*deity)),
                                clause("fall", Some(s.town(*settlement)), None).now().not(),
                            ],
                        );
                        let m = Sentence::Text(vec![
                            one(clause("say", Some(s.who(*asker)), None)
                                .to(s.god(*deity))
                                .says(true, one(question))),
                            one(clause("say", Some(s.god(*deity)), None)
                                .to(s.name(*asker))
                                .says(true, answer)),
                        ]);
                        wrote(pen, Genre::Oracle, *settlement, m, None, e);
                    }
                    EventKind::Vow { person, deity } => {
                        let m = one(clause(
                            "give",
                            Some(s.who(*person)),
                            Some(det(np("altar"), "this")),
                        )
                        .to(s.god(*deity))
                        .sub(
                            Link::After,
                            clause(
                                "see",
                                Some(plural("people")),
                                Some(np(match &ev0.kind {
                                    EventKind::Omen { sign, .. } => sign,
                                    _ => "star",
                                })),
                            ),
                        ));
                        if let Some(t) = town(e) {
                            wrote(pen, Genre::Dedication, t, m, Some(*person), e);
                        }
                    }
                    _ => {}
                }
            }
        }
        ArcKind::Letters => {
            let [x, y] = [arc.people[0], arc.people[1]];
            let (tx, ty) = (h.people[x].settlement, h.people[y].settlement);
            for (k, &e) in arc.events.iter().enumerate() {
                let news = s.about(&h.events[e]).map(flat);
                let (from, to, at) = if k % 2 == 0 { (x, y, ty) } else { (y, x, tx) };
                let mut body = vec![one(clause("say", Some(s.name(from)), None).to(s.name(to)))];
                if let Some(n) = news {
                    body.push(one(n));
                }
                if !h.deities.is_empty() {
                    let d = rng.below(h.deities.len() as u32) as usize;
                    body.push(one(
                        clause("keep", Some(s.god(d)), Some(s.name(to))).mood(Mood::Optative)
                    ));
                }
                body.push(one(clause(
                    "send",
                    None,
                    Some(np(s.word(&["wool", "oil", "cloth"]))),
                )
                .mood(Mood::Imperative)
                .to(s.name(from))
                .sub(Link::Before, clause("come", Some(np("winter")), None).now())));
                wrote(pen, Genre::Letter, at, Sentence::Text(body), Some(from), e);
            }
        }
        ArcKind::Life => {
            let p = arc.people[0];
            // A blessing on their house, and their mark on a wall.
            if !h.deities.is_empty() {
                let d = rng.below(h.deities.len() as u32) as usize;
                let m = Sentence::Text(vec![
                    one(
                        clause("bless", Some(s.god(d)), Some(of(np("house"), s.name(p))))
                            .mood(Mood::Optative),
                    ),
                    one(clause("live", Some(of(plural("child"), s.name(p))), None)
                        .mood(Mood::Optative)
                        .adv("always")),
                ]);
                wrote(
                    pen,
                    Genre::Blessing,
                    h.people[p].settlement,
                    m,
                    Some(p),
                    first,
                );
            }
            let m = one(clause("come", Some(s.who(p)), None)
                .adv("here")
                .when(s.year_at(ev0)));
            wrote(
                pen,
                Genre::Graffito,
                h.people[p].settlement,
                m,
                Some(p),
                first,
            );
        }
    }
}

/// A ruler's decree, set up in every town of the realm then standing.
fn decrees<'a>(h: &History, scribe: &dyn Fn(u32) -> Scribe<'a>, pen: &mut Pen) {
    for e in &h.events {
        let EventKind::Decree { ruler, law, .. } = &e.kind else {
            continue;
        };
        let s = scribe(e.era);
        let arc = h
            .arcs
            .iter()
            .find(|a| a.events.contains(&e.id))
            .map(|a| a.id);
        let m = Sentence::Text(vec![
            one(clause("say", Some(s.who(*ruler)), None)
                .says(true, one(s.law(law)))
                .when(s.year_at(e))),
            one(clause(
                "take",
                Some(np(s.word(&["judge+agt"]))),
                Some(det(of(np("ox"), np("man")), "each")),
            )
            .now()
            .sub(
                Link::If,
                clause(law.verb, Some(np("man")), Some(s.good(law.object))).now(),
            )),
        ]);
        let faction = h.people[*ruler].faction;
        let towns: Vec<usize> = h
            .standing(e.year)
            .iter()
            .filter(|t| t.faction == faction)
            .map(|t| t.id)
            .take(4)
            .collect();
        for town in towns {
            if let Some(at) = pen.find(Genre::Decree, town) {
                pen.write(
                    at,
                    e.era,
                    e.year,
                    Genre::Decree,
                    m.clone(),
                    Some(*ruler),
                    Some(e.id),
                    arc,
                );
            }
        }
    }
}

/// Year by year, the doings of each reign, kept in the archives of each
/// realm's towns.
fn annals<'a>(h: &History, scribe: &dyn Fn(u32) -> Scribe<'a>, pen: &mut Pen) {
    for &(ruler, faction, start, end) in scribe(0).reigns {
        let era = h.people[ruler].era;
        let s = scribe(era);
        let doings: Vec<(i32, Sentence)> = h
            .events
            .iter()
            .filter(|e| e.year >= start && e.year < end)
            .filter(|e| {
                h.settlements
                    .iter()
                    .find(|t| t.cell == e.cell)
                    .is_some_and(|t| t.faction == faction)
            })
            .filter(|e| {
                !matches!(
                    e.kind,
                    EventKind::Birth { .. } | EventKind::Appointment { .. }
                )
            })
            .filter_map(|e| {
                s.about(e)
                    .map(|c| (e.year, one(c.when(s.year(faction, e.year)))))
            })
            .take(8)
            .collect();
        if doings.len() < 2 {
            continue;
        }
        // Written up after its last entry.
        let last = doings.iter().map(|d| d.0).max().unwrap_or(start);
        let doings: Vec<Sentence> = doings.into_iter().map(|d| d.1).collect();
        let seat = h.people[ruler].settlement;
        if let Some(at) = pen.find(Genre::Annal, seat) {
            pen.write(
                at,
                era,
                last,
                Genre::Annal,
                Sentence::Text(doings),
                None,
                None,
                None,
            );
        }
    }
}

/// The rulers of each realm and era, one after another.
fn king_lists<'a>(h: &History, scribe: &dyn Fn(u32) -> Scribe<'a>, pen: &mut Pen) {
    for era in &h.eras {
        let s = scribe(era.index);
        for f in 0..h.factions.len() {
            let rulers: Vec<Sentence> = s
                .reigns
                .iter()
                .filter(|r| r.1 == f && h.people[r.0].era == era.index)
                .map(|r| {
                    one(clause(
                        "rule",
                        Some(s.child_of(r.0)),
                        Some(s.town(h.people[r.0].settlement)),
                    ))
                })
                .collect();
            if rulers.len() < 2 {
                continue;
            }
            let seat = h
                .settlements
                .iter()
                .find(|t| t.faction == f && t.founded < era.end)
                .map(|t| t.id);
            for town in seat.into_iter().chain(
                h.settlements
                    .iter()
                    .filter(|t| t.faction == f && t.capital)
                    .map(|t| t.id),
            ) {
                if let Some(at) = pen.find(Genre::KingList, town) {
                    pen.write(
                        at,
                        era.index,
                        era.end - 1,
                        Genre::KingList,
                        Sentence::Text(rulers.clone()),
                        None,
                        None,
                        None,
                    );
                    break;
                }
            }
        }
    }
}

/// What every town wrote day by day: instructions by the work, prayers
/// and hymns, lessons, boundary stones, milestones, blessings, curses,
/// graffiti, store lists and calendars.
fn everyday<'a>(rng: &mut Rng, h: &History, scribe: &dyn Fn(u32) -> Scribe<'a>, pen: &mut Pen) {
    let n = pen.structures.len();
    for sid in 0..n {
        let st = &pen.structures[sid];
        let (kind, era, built, settlement) = (st.kind, st.era, st.built, st.settlement);
        let Some(town) = settlement else { continue };
        let s = scribe(era);
        let year = built + 10 + rng.below(30) as i32;
        let people: Vec<usize> = h
            .people
            .iter()
            .filter(|p| p.settlement == town && p.era == era && p.born <= year)
            .map(|p| p.id)
            .collect();
        let someone = |rng: &mut Rng| (!people.is_empty()).then(|| *rng.pick(&people));
        let god = |rng: &mut Rng| {
            (!h.deities.is_empty()).then(|| rng.below(h.deities.len() as u32) as usize)
        };
        let here = |pen: &Pen, feats: &[&str]| {
            let st = &pen.structures[sid];
            st.interior.rooms.iter().enumerate().find_map(|(r, room)| {
                room.features
                    .iter()
                    .position(|f| feats.contains(&f.kind))
                    .map(|f| (sid, Some((r, f))))
            })
        };
        // Instructions beside the work.
        if let Some((work, feats)) = craft(kind) {
            if let Some(at) = here(pen, feats) {
                let m = instructions(&s, work);
                pen.write(at, era, year, Genre::Instructions, m, None, None, None);
            }
        }
        match kind {
            K::Temple | K::WaysideShrine => {
                if let Some(d) = god(rng) {
                    let deity = &h.deities[d];
                    let hymn = Sentence::Text(vec![
                        one(clause(
                            "give",
                            Some(s.god(d)),
                            Some(np(match deity.of {
                                "sun" | "star" => "light",
                                "rain" | "river" | "sea" => "water",
                                "grain" | "earth" => "grain",
                                "fire" => "fire",
                                _ => "rain",
                            })),
                        )
                        .now()
                        .to(plural("people"))
                        .aspect(Aspect::Habitual, s.lang)),
                        one(clause("rise", Some(np("sun")), None)
                            .now()
                            .sub(Link::When, clause("wake", Some(s.god(d)), None).now())),
                        Sentence::Joined(
                            Conj::And,
                            vec![
                                clause(
                                    "honour",
                                    Some(det(plural("people"), "all")),
                                    Some(s.god(d)),
                                )
                                .now(),
                                clause("sing", Some(det(plural("people"), "all")), None).now(),
                            ],
                        ),
                    ]);
                    if let Some(at) = here(pen, &["wall", "statue", "altar", "niche"]) {
                        pen.write(at, era, year, Genre::Hymn, hymn, None, None, None);
                    }
                    let asker = someone(rng);
                    if let (Some(p), Some(at)) = (asker, pen.find(Genre::Prayer, town)) {
                        let prayer = Sentence::Text(vec![
                            one(clause(
                                "give",
                                Some(s.god(d)),
                                Some(plural(rng.pick(&["child", "sheep", "fish", "grain"]))),
                            )
                            .to(s.name(p))
                            .mood(Mood::Optative)),
                            one(
                                clause("give", Some(s.who(p)), Some(det(np("lamp"), "this")))
                                    .now()
                                    .to(s.god(d))
                                    .sub(
                                        Link::SoThat,
                                        clause("hear", Some(s.god(d)), Some(s.name(p))).now(),
                                    ),
                            ),
                        ]);
                        pen.write(at, era, year, Genre::Prayer, prayer, Some(p), None, None);
                    }
                    if kind == K::Temple && rng.chance(40) && h.deities.len() >= 2 {
                        let other = (d + 1) % h.deities.len();
                        let myth = Sentence::Text(vec![
                            one(clause("fight", Some(s.god(d)), Some(s.god(other)))
                                .sub(Link::When, clause("live", Some(np("people")), None).not())),
                            one(clause("fall", Some(s.god(other)), None)),
                            Sentence::Joined(
                                Conj::Then,
                                vec![
                                    clause("rule", Some(s.god(d)), Some(np("sky"))),
                                    clause("make", Some(s.god(d)), Some(plural("person"))),
                                ],
                            ),
                        ]);
                        if let Some(at) = pen.find(Genre::Myth, town) {
                            pen.write(at, era, year, Genre::Myth, myth, None, None, None);
                        }
                    }
                }
            }
            K::School | K::Scriptorium => {
                // A drill: one sentence, then again with many.
                let doer = *rng.pick(&["scribe", "smith", "priest", "king"]);
                let thing = *rng.pick(&["tablet", "jar", "bowl", "lamp", "seal"]);
                let lesson = Sentence::Text(vec![
                    one(clause("make", Some(np(doer)), Some(np(thing))).now()),
                    one(clause("make", Some(plural(doer)), Some(plural(thing))).now()),
                    one(clause("make", Some(np(doer)), Some(np(thing)))),
                    one(clause("make", Some(plural(doer)), Some(plural(thing)))),
                ]);
                if let Some(at) = pen.find(Genre::Lesson, town) {
                    pen.write(at, era, year, Genre::Lesson, lesson, None, None, None);
                }
                let field = *rng.pick(&[
                    &["head", "hand", "eye", "foot", "heart", "mouth", "ear"][..],
                    &["mother", "father", "son", "daughter", "brother", "sister"][..],
                    &["sun", "moon", "star", "sky", "cloud", "rain", "wind"][..],
                    &["sheep", "ox", "goat", "fish", "bird", "beast"][..],
                ]);
                let words =
                    Sentence::List(field.iter().filter(|w| s.has(w)).map(|w| np(w)).collect());
                if let Some(at) = pen.find(Genre::WordList, town) {
                    pen.write(at, era, year, Genre::WordList, words, None, None, None);
                }
            }
            K::Observatory => {
                let mut days: Vec<Sentence> = vec![
                    one(clause("shine", Some(adj(np("moon"), "full")), None)
                        .now()
                        .when(Some({
                            let mut d = np("day");
                            d.ordinal = Some(15);
                            d
                        }))),
                    one(clause("die", Some(np("moon")), None).now().when(Some({
                        let mut d = np("day");
                        d.ordinal = Some(30);
                        d
                    }))),
                ];
                for d in h.deities.iter().take(3) {
                    days.push(one(clause(
                        "honour",
                        Some(plural("people")),
                        Some(of(np("god"), np(d.of))),
                    )
                    .now()
                    .when(Some(np(d.festival)))));
                }
                if let Some(at) = pen.find(Genre::Calendar, town) {
                    pen.write(
                        at,
                        era,
                        year,
                        Genre::Calendar,
                        Sentence::Text(days),
                        None,
                        None,
                        None,
                    );
                }
            }
            K::House => {
                if rng.chance(45) {
                    if let (Some(p), Some(d)) = (someone(rng), god(rng)) {
                        let m =
                            one(
                                clause("bless", Some(s.god(d)), Some(det(np("house"), "this")))
                                    .mood(Mood::Optative)
                                    .sub(
                                        Link::Because,
                                        clause("honour", Some(s.name(p)), Some(s.god(d))).now(),
                                    ),
                            );
                        if let Some(at) = here(pen, &["lintel", "door-slab", "wall"]) {
                            pen.write(at, era, year, Genre::Blessing, m, Some(p), None, None);
                        }
                    }
                }
                if rng.chance(35) {
                    if let Some(p) = someone(rng) {
                        let m = Sentence::Text(vec![
                            one(
                                clause("hold", Some(s.who(p)), Some(det(np("field"), "this")))
                                    .now(),
                            ),
                            one(clause("take", None, Some(det(np("stone"), "this")))
                                .mood(Mood::Imperative)
                                .not()),
                        ]);
                        pen.write(
                            (sid, None),
                            era,
                            year,
                            Genre::Boundary,
                            m,
                            Some(p),
                            None,
                            None,
                        );
                    }
                }
                if rng.chance(30) {
                    // A jar's owner and contents.
                    if let Some(p) = someone(rng) {
                        let g = *rng.pick(&["oil", "wine", "grain", "salt", "fish"]);
                        let m = Sentence::List(vec![
                            of(np("jar"), s.name(p)),
                            np(g).counted(rng.range(1, 12) as u16),
                        ]);
                        if let Some(at) = here(pen, &["jar", "shelf", "niche", "wall"]) {
                            pen.write(at, era, year, Genre::Label, m, Some(p), None, None);
                        }
                    }
                }
                if rng.chance(25) {
                    // Family news, sent from one house to another.
                    if let (Some(a), Some(b)) = (someone(rng), someone(rng)) {
                        let news = h
                            .events
                            .iter()
                            .filter(|e| e.era == era && e.year <= year && e.actors.contains(&a))
                            .find_map(|e| s.about(e).map(flat));
                        if let (true, Some(news)) = (a != b, news) {
                            let mut body = vec![
                                one(clause("say", Some(s.name(a)), None).to(s.name(b))),
                                one(news),
                            ];
                            if let Some(d) = god(rng) {
                                body.push(one(clause(
                                    "keep",
                                    Some(s.god(d)),
                                    Some(of(np("house"), s.name(b))),
                                )
                                .mood(Mood::Optative)));
                            }
                            body.push(one(clause(
                                "send",
                                None,
                                Some(np(rng.pick(&["oil", "grain", "cloth"]))),
                            )
                            .mood(Mood::Imperative)
                            .to(s.name(a))
                            .sub(Link::Before, clause("come", Some(np("winter")), None).now())));
                            if let Some(at) = here(pen, &["chest", "shelf", "niche", "table"]) {
                                pen.write(
                                    at,
                                    era,
                                    year,
                                    Genre::Letter,
                                    Sentence::Text(body),
                                    Some(a),
                                    None,
                                    None,
                                );
                            }
                        }
                    }
                }
                if rng.chance(45) {
                    if let (Some(a), Some(b)) = (someone(rng), someone(rng)) {
                        let m = if a != b {
                            one(clause("love", Some(s.name(a)), Some(s.name(b))).now())
                        } else {
                            one(clause("sleep", Some(s.name(a)), None).adv("here"))
                        };
                        if let Some(at) = here(pen, &["wall"]) {
                            pen.write(at, era, year, Genre::Graffito, m, Some(a), None, None);
                        }
                    }
                }
            }
            K::Storehouse | K::Warehouse | K::Granary => {
                let goods: Vec<&str> = crate::society::GOODS
                    .iter()
                    .copied()
                    .filter(|g| s.has(g))
                    .collect();
                let mut items: Vec<NounPhrase> = (0..3)
                    .map(|_| {
                        let g = *rng.pick(&goods);
                        let mut n = np(g).counted(rng.range(3, 120) as u16);
                        if rng.chance(30) {
                            n.adjectives.push(
                                rng.pick(&["new", "old", "broken", "whole", "good"])
                                    .to_string(),
                            );
                        }
                        n
                    })
                    .collect();
                items.dedup_by(|a, b| a.head == b.head);
                if let Some(p) = someone(rng) {
                    items.push(of(np("seal"), s.name(p)));
                }
                if let Some(at) = pen.find(Genre::Inventory, town) {
                    pen.write(
                        at,
                        era,
                        year,
                        Genre::Inventory,
                        Sentence::List(items),
                        None,
                        None,
                        None,
                    );
                }
            }
            K::CouncilHall | K::Palace => {
                // Who paid what.
                let mut paid: Vec<Sentence> = Vec::new();
                for _ in 0..3 {
                    if let Some(p) = someone(rng) {
                        let g = *rng.pick(&["grain", "oil", "wine", "cloth"]);
                        paid.push(one(clause(
                            "pay",
                            Some(s.name(p)),
                            Some(np(g).counted(rng.range(2, 50) as u16)),
                        )
                        .to(np(s.word(&["king"])))));
                    }
                }
                if paid.len() >= 2 {
                    if let Some(at) = pen.find(Genre::Receipt, town) {
                        pen.write(
                            at,
                            era,
                            year,
                            Genre::Receipt,
                            Sentence::Text(paid),
                            None,
                            None,
                            None,
                        );
                    }
                }
            }
            K::Waystation | K::Bridge | K::Gatehouse => {
                // How far to the nearest towns.
                let here_cell = h.settlements[town].cell;
                let mut near: Vec<(i64, usize)> = h
                    .settlements
                    .iter()
                    .filter(|t| t.id != town && t.founded <= built)
                    .map(|t| (t.cell.dist2(here_cell), t.id))
                    .collect();
                near.sort();
                let items: Vec<NounPhrase> = near
                    .iter()
                    .take(2)
                    .flat_map(|&(d2, t)| {
                        let days = ((d2 as f64).sqrt() / 10.0).ceil().max(1.0) as u16;
                        [s.town(t), np("day").counted(days)]
                    })
                    .collect();
                if !items.is_empty() {
                    pen.write(
                        (sid, None),
                        era,
                        year,
                        Genre::Milestone,
                        Sentence::List(items),
                        None,
                        None,
                        None,
                    );
                }
            }
            K::Tomb | K::Mausoleum => {
                if let Some(d) = god(rng) {
                    let mut man = np("man");
                    man.relative = Some(Box::new(Relative {
                        gap: ArgRole::Subject,
                        clause: clause("open", None, Some(det(np("tomb"), "this"))).now(),
                    }));
                    let m = one(clause("curse", Some(s.god(d)), Some(man)).mood(Mood::Optative));
                    if let Some(at) = here(pen, &["sarcophagus", "door-slab", "wall"]) {
                        pen.write(at, era, year, Genre::Curse, m, None, None, None);
                    }
                }
            }
            K::MarketHall | K::Bathhouse | K::Barracks => {
                if let Some(p) = someone(rng) {
                    let m = one(clause("come", Some(s.name(p)), None)
                        .adv("here")
                        .when(s.year(h.settlements[town].faction, year)));
                    if let Some(at) = here(pen, &["wall", "bench", "pillar"]) {
                        pen.write(at, era, year, Genre::Graffito, m, Some(p), None, None);
                    }
                }
            }
            _ => {}
        }
        // A sale between two townsfolk, in houses now and then and in
        // every market.
        if (kind == K::House && rng.chance(25)) || matches!(kind, K::MarketHall | K::Warehouse) {
            if let (Some(a), Some(b)) = (someone(rng), someone(rng)) {
                if a != b {
                    let goods: Vec<&str> = crate::society::GOODS
                        .iter()
                        .copied()
                        .filter(|g| s.has(g))
                        .collect();
                    let (sold, paid) = (*rng.pick(&goods), *rng.pick(&goods));
                    let m = Sentence::Text(vec![
                        one(clause(
                            "sell",
                            Some(s.name(a)),
                            Some(np(sold).counted(rng.range(1, 20) as u16)),
                        )
                        .to(s.name(b))
                        .when(s.year(h.settlements[town].faction, year))),
                        one(clause(
                            "give",
                            Some(s.name(b)),
                            Some(np(paid).counted(rng.range(2, 60) as u16)),
                        )
                        .to(s.name(a))),
                    ]);
                    let at = here(pen, &["tablet", "chest", "table", "shelf"])
                        .or_else(|| pen.find(Genre::Contract, town));
                    if let Some(at) = at {
                        pen.write(at, era, year, Genre::Contract, m, Some(a), None, None);
                    }
                }
            }
        }
        // A building's own inscription: who built it, and when.
        if matches!(
            kind,
            K::Granary
                | K::Mill
                | K::Bathhouse
                | K::Cistern
                | K::Aqueduct
                | K::FountainHouse
                | K::MarketHall
                | K::Warehouse
                | K::Lighthouse
                | K::Library
                | K::School
                | K::Courthouse
                | K::CouncilHall
                | K::Palace
                | K::Gatehouse
                | K::Bridge
        ) {
            let patron = h
                .people
                .iter()
                .filter(|p| p.role == Role::Ruler && p.faction == h.settlements[town].faction)
                .find(|p| p.born + 16 <= built && built < p.died)
                .map(|p| p.id);
            if let Some(p) = patron {
                let what = det(np(s.word(&[building_word(kind), "house"])), "this");
                let m = one(clause("build", Some(s.who(p)), Some(what))
                    .when(s.year(h.settlements[town].faction, built)));
                let at = here(pen, &["lintel", "gate", "stele", "pillar"]).unwrap_or((sid, None));
                pen.write(at, era, built, Genre::Building, m, Some(p), None, None);
            }
        }
    }
    let _ = era_of;
}

/// The word for a kind of building, where the language has one.
fn building_word(k: K) -> &'static str {
    match k {
        K::Granary => "granary",
        K::Mill => "grind+place",
        K::Bathhouse => "wash+place",
        K::Cistern | K::FountainHouse => "well",
        K::Aqueduct => "canal",
        K::MarketHall => "market",
        K::Warehouse => "warehouse",
        K::Lighthouse => "tower",
        K::Library => "library",
        K::School => "teach+place",
        K::Courthouse => "judge+place",
        K::CouncilHall => "hall",
        K::Palace => "palace",
        K::Gatehouse => "gate",
        K::Bridge => "bridge",
        _ => "house",
    }
}

/// The work done in a craft building, and the surfaces beside it.
fn craft(k: K) -> Option<(&'static str, &'static [&'static str])> {
    Some(match k {
        K::Kiln => ("kiln", &["kiln", "wall"]),
        K::Bakehouse => ("oven", &["oven", "wall"]),
        K::Smithy => ("anvil", &["anvil", "hearth", "wall"]),
        K::WeavingHouse => ("loom", &["loom", "wall"]),
        K::Mill => ("millstone", &["millstone", "waterwheel", "wall"]),
        K::DyeWorks | K::Brewery | K::Tannery => ("vat", &["vat", "cask", "wall"]),
        K::Lighthouse | K::SignalStation => ("beacon", &["beacon", "wall"]),
        _ => return None,
    })
}

/// How to do the work, step by step: commands joined by "then".
fn instructions(s: &Scribe, work: &str) -> Sentence {
    let steps: Vec<Clause> = match work {
        "kiln" => vec![
            clause("carry", None, Some(np(s.word(&["clay", "earth"])))),
            clause("kindle", None, Some(np("fire"))),
            clause("burn", None, Some(plural("bowl"))).when(Some(np("day").counted(2))),
        ],
        "oven" => vec![
            clause("bring", None, Some(np("grain"))),
            clause("kindle", None, Some(np("fire"))),
            clause("cook", None, Some(plural("loaf"))),
        ],
        "anvil" => vec![
            clause("kindle", None, Some(np("fire"))),
            clause(
                "forge",
                None,
                Some(np(s.word(&["bronze", "iron", "copper", "knife"]))),
            ),
            clause("touch", None, Some(np("fire"))).not(),
        ],
        "loom" => vec![
            clause(
                "bring",
                None,
                Some(np(s.word(&["wool", "linen", "thread", "cloth"]))),
            ),
            clause("weave", None, Some(np("cloth"))),
            clause("cut", None, Some(np("cloth")))
                .sub(Link::Before, clause("count", None, Some(plural("cloth")))),
        ],
        "millstone" => vec![
            clause("pour", None, Some(np("grain"))),
            clause("close", None, Some(np("door"))),
            clause("open", None, Some(np("water"))),
        ],
        "vat" => vec![
            clause("fill", None, Some(np("jar"))),
            clause("pour", None, Some(np("water"))),
            clause("wait", None, None).when(Some(np("day").counted(3))),
        ],
        _ => vec![
            clause("carry", None, Some(np("oil"))),
            clause("kindle", None, Some(np("fire"))),
            clause("sleep", None, None).not(),
        ],
    };
    let steps: Vec<Clause> = steps
        .into_iter()
        .map(|c| {
            let negative = c.polarity == Polarity::Negative;
            let mut c = flat(c).mood(Mood::Imperative);
            if negative {
                c.polarity = Polarity::Negative;
            }
            c
        })
        .collect();
    Sentence::Joined(Conj::Then, steps)
}

#[allow(dead_code)]
fn is_name(n: &NounPhrase) -> bool {
    matches!(n.head, Head::Name(_))
}
