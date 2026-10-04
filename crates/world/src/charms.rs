//! Writing as the old civilisation's technology (D09): the everyday spells
//! its people wrote into their buildings, and what became of them.
//!
//! A larder kept cold, lamps that lit at dusk, a mill that turned with no
//! water, a door that opened only for its household, a well that never ran
//! dry, a field kept green, a bridge held up beyond its strength. Each is a
//! potent text written when its building was new, in that era's language,
//! on a fitting surface. Later hands wrote over many of them: some renewed
//! the spell, some weakened or hedged it, and some (rivals, enemies,
//! priests of another god) undid it. Whatever was scraped last on a surface
//! is what acts now, so a world is full of live, broken and half-working
//! spells, and of surfaces with long stacks of them.
//!
//! Only meaning is made here; what each spell does is the simulation's
//! business (`scraped_sim::writing`).

use scraped_lang::concepts;
use scraped_lang::meaning::{Clause, Link, NounPhrase as N, Sentence};
use scraped_lang::rng::{Rng, Stream};
use scraped_lang::Language;

use crate::history::History;
use crate::structures::{Structure, StructureKind as K};
use crate::texts::{put, Genre, Spot, Text, Written};

/// A spell as a recipe: its clause, and the surfaces it is written on.
struct Recipe {
    clause: Clause,
    features: &'static [&'static str],
}

fn np(id: &str) -> N {
    N::concept(id)
}

fn this(id: &str) -> N {
    N::concept(id).det("this")
}

fn when(subject: &str, verb: &str) -> Clause {
    Clause::plain(verb, np(subject))
}

fn carries(thing: &str) -> Clause {
    Clause::plain("carry", np("man")).with_object(np(thing))
}

const WALLS: &[&str] = &["wall", "lintel", "door-slab", "niche", "stele", "pillar"];
const HEARTH: &[&str] = &["hearth", "wall", "niche"];
const STORE: &[&str] = &["jar", "shelf", "bin", "wall", "chest"];
const SACRED: &[&str] = &["altar", "niche", "wall", "stele"];
const DOORS: &[&str] = &["door-slab", "lintel", "gate", "wall"];

/// What a people wrote into a building of this kind (each a spell its
/// language may or may not have the words for). `animal`: a beast of the
/// land the language names, for spells that keep beasts off or calm them.
fn recipes(k: K, life: &Life, key: u64) -> Vec<Recipe> {
    let r = |clause: Clause, features: &'static [&'static str]| Recipe { clause, features };
    let lamp = |id: &str| {
        Clause::potent("bring", this(id))
            .with_object(np("light"))
            .with_clause(Link::When, when("night", "come"))
    };
    let household = |id: &str, thing: &str| {
        Clause::potent("open", this(id))
            .denied()
            .with_clause(Link::Unless, carries(thing))
    };
    // DESIGN-Q: a door that opens only for its household (to whoever
    // carries the house's seal or signet) in one house in five, and on
    // halls, gatehouses and prisons: enough to meet, rare enough not to
    // shut a stranger out of every house.
    let mut out = match k {
        K::House if key.is_multiple_of(5) => vec![
            r(
                household(
                    "door",
                    if key.is_multiple_of(2) {
                        "seal"
                    } else {
                        "signet"
                    },
                ),
                DOORS,
            ),
            r(
                Clause::potent("bring", this("house"))
                    .with_object(np("fire"))
                    .with_adverb("slightly"),
                HEARTH,
            ),
        ],
        K::House => vec![
            r(
                Clause::potent("bring", this("house"))
                    .with_object(np("fire"))
                    .with_adverb("slightly"),
                HEARTH,
            ),
            r(lamp("house"), WALLS),
            r(Clause::potent("keep", this("jar")), STORE),
        ],
        K::Storehouse | K::Granary | K::Warehouse => vec![
            r(
                Clause::potent("bring", this("granary")).with_object(np("ice")),
                STORE,
            ),
            r(
                Clause::potent("keep", this("granary")).with_adverb("greatly"),
                STORE,
            ),
            r(
                Clause::potent("take", this("granary")).with_object(np("water")),
                WALLS,
            ),
        ],
        K::Mill => vec![
            r(
                Clause::potent("flow", np("water"))
                    .with_adverb("greatly")
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", this("house")).with_object(np("wind")),
                WALLS,
            ),
        ],
        K::Cistern | K::FountainHouse | K::Aqueduct => vec![
            r(
                Clause::potent("bring", np("well"))
                    .with_object(np("water"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("flow", np("water")).with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("water"))
                    .with_object(np("ice"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("summer", "come")),
                WALLS,
            ),
        ],
        K::Bathhouse => vec![
            r(
                Clause::potent("bring", np("water"))
                    .with_object(np("fire"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", this("house"))
                    .with_object(np("fire"))
                    .with_clause(Link::When, when("winter", "come")),
                WALLS,
            ),
        ],
        K::Bridge => vec![
            r(
                Clause::potent("bring", this("bridge"))
                    .with_object(np("stone"))
                    .with_adverb("greatly"),
                WALLS,
            ),
            r(lamp("bridge"), WALLS),
        ],
        K::Temple | K::WaysideShrine | K::Hermitage => vec![
            r(
                Clause::potent("bring", this("temple")).with_object(np("peace")),
                SACRED,
            ),
            r(
                Clause::potent("shine", np("fire"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("night", "come")),
                SACRED,
            ),
            r(
                Clause::potent("find", this("temple")).with_object(np("man").det("no")),
                SACRED,
            ),
        ],
        K::Tomb | K::Mausoleum | K::Ossuary | K::Catacombs | K::Cemetery => vec![
            r(Clause::potent("open", this("tomb")).denied(), SACRED),
            r(
                Clause::potent("bring", this("tomb"))
                    .with_object(np("darkness"))
                    .with_clause(Link::When, Clause::plain("enter", np("man"))),
                SACRED,
            ),
            r(
                Clause::potent("bring", this("tomb"))
                    .with_object(np("song"))
                    .with_clause(Link::When, Clause::plain("enter", np("man"))),
                SACRED,
            ),
            r(
                Clause::potent("keep", this("tomb")).with_adverb("greatly"),
                SACRED,
            ),
            r(
                Clause::potent("bring", this("tomb")).with_object(np("darkness")),
                SACRED,
            ),
        ],
        K::Gatehouse | K::Wall | K::Prison | K::Barracks | K::Armoury => vec![
            r(household("gate", "signet"), DOORS),
            r(
                Clause::potent("hold", this("door")).with_adverb("greatly"),
                DOORS,
            ),
            r(
                Clause::potent("bring", this("wall")).with_object(np("stone")),
                WALLS,
            ),
        ],
        K::Lighthouse | K::SignalStation | K::Tower | K::Observatory => vec![
            r(
                Clause::potent("shine", np("fire"))
                    .with_adverb("widely")
                    .with_clause(Link::When, when("night", "come")),
                WALLS,
            ),
            r(
                Clause::potent("take", np("sky"))
                    .with_object(np("cloud"))
                    .with_adverb("here"),
                WALLS,
            ),
        ],
        K::Garden | K::Orchard => vec![
            r(
                Clause::potent("grow", np("field"))
                    .with_adverb("greatly")
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("field"))
                    .with_object(np("rain"))
                    .with_adverb("here"),
                WALLS,
            ),
        ],
        K::Smithy | K::Kiln | K::Bakehouse | K::Brewery => vec![
            r(
                Clause::potent("burn", this("kiln")).with_adverb("greatly"),
                HEARTH,
            ),
            r(
                Clause::potent("take", this("house")).with_object(np("smoke")),
                WALLS,
            ),
        ],
        K::Archive | K::Library | K::Scriptorium | K::School => vec![
            r(
                Clause::potent("take", this("house")).with_object(np("water")),
                WALLS,
            ),
            r(
                Clause::potent("keep", this("scroll")).with_adverb("greatly"),
                STORE,
            ),
            r(lamp("house"), WALLS),
        ],
        K::MarketHall | K::Waystation | K::Harbour => vec![
            r(
                Clause::potent("bring", np("road"))
                    .with_object(np("light"))
                    .with_adverb("here")
                    .with_clause(Link::Until, when("sun", "rise")),
                WALLS,
            ),
            r(
                Clause::potent("sleep", np("sea")).with_adverb("here"),
                WALLS,
            ),
        ],
        K::Palace | K::CouncilHall | K::Courthouse | K::Amphitheatre => vec![
            r(
                Clause::potent("bring", this("hall")).with_object(np("song")),
                WALLS,
            ),
            r(household("door", "signet"), DOORS),
            r(
                Clause::potent("bring", this("hall")).with_object(np("peace")),
                WALLS,
            ),
        ],
        K::Tannery | K::DyeWorks | K::WeavingHouse => vec![
            r(Clause::potent("keep", this("cloth")), STORE),
            r(
                Clause::potent("bring", this("house")).with_object(np("wind")),
                WALLS,
            ),
        ],
        K::Mine | K::Cave => vec![
            r(
                Clause::potent("bring", this("cave"))
                    .with_object(np("stone"))
                    .with_adverb("greatly"),
                WALLS,
            ),
            r(
                Clause::potent("bring", this("cave")).with_object(np("light")),
                WALLS,
            ),
        ],
        K::Labyrinth | K::ProcessionalWay => vec![
            r(Clause::potent("lose", this("road")), WALLS),
            r(
                Clause::potent("bring", np("road"))
                    .with_object(np("darkness"))
                    .with_adverb("here"),
                WALLS,
            ),
        ],
        _ => Vec::new(),
    };
    if let Some(a) = life.predator {
        let keep_off = Clause::potent("come", np(a).det("no")).with_adverb("here");
        if matches!(k, K::House | K::Waystation | K::Storehouse | K::Granary) {
            out.push(r(keep_off, WALLS));
        }
    }
    if let Some(a) = life.grazer {
        let calm = Clause::potent("sleep", np(a)).with_adverb("here");
        if matches!(k, K::Garden | K::Orchard | K::Temple) {
            out.push(r(calm, WALLS));
        }
    }
    out
}

/// Lesser spells any building of a family might carry, beyond its own
/// kind's: what people wanted of rooms, doors and things, and of the land,
/// water, plants, beasts and air around them.
fn pool(k: K, life: &Life) -> Vec<Recipe> {
    let r = |clause: Clause, features: &'static [&'static str]| Recipe { clause, features };
    let mut out = vec![
        // Rooms.
        r(
            Clause::potent("take", this("house")).with_object(np("song")),
            WALLS,
        ),
        r(
            Clause::potent("bring", this("house"))
                .with_object(np("song"))
                .with_clause(Link::When, when("festival", "come")),
            WALLS,
        ),
        r(
            Clause::potent("find", this("house")).with_object(np("man").det("no")),
            WALLS,
        ),
        r(
            Clause::potent("bring", this("house")).with_object(np("stone")),
            WALLS,
        ),
        r(
            Clause::potent("take", this("house"))
                .with_object(np("darkness"))
                .with_adverb("slightly"),
            WALLS,
        ),
        // Doors and passages.
        r(Clause::potent("hold", this("door")), DOORS),
        r(Clause::potent("lose", this("door")), DOORS),
        r(Clause::potent("sing", this("door")).denied(), DOORS),
        r(
            Clause::potent("bring", this("door")).with_object(np("iron")),
            DOORS,
        ),
        // Things.
        r(
            Clause::potent("fill", this("jar")).with_clause(Link::When, when("rain", "fall")),
            STORE,
        ),
        r(
            Clause::potent("rise", this("jar")).with_adverb("slightly"),
            STORE,
        ),
        r(
            Clause::potent("bring", this("house"))
                .with_object(np("water"))
                .with_clause(Link::When, when("summer", "come")),
            WALLS,
        ),
        r(
            Clause::potent("bring", this("house")).with_object(np("wolf").det("no")),
            WALLS,
        ),
        r(
            Clause::potent("take", this("box")).with_object(np("fire")),
            STORE,
        ),
        r(
            Clause::potent("break", this("wall"))
                .with_clause(Link::When, Clause::plain("enter", np("man"))),
            WALLS,
        ),
        r(Clause::potent("weigh", this("stone")).denied(), WALLS),
        r(Clause::potent("lose", this("box")), STORE),
        r(
            Clause::potent("bring", this("jar")).with_object(np("ice")),
            STORE,
        ),
        r(
            Clause::potent("hold", this("box")).with_adverb("greatly"),
            STORE,
        ),
        r(
            Clause::potent("shine", this("stone")).with_clause(Link::When, when("night", "come")),
            WALLS,
        ),
        r(
            Clause::potent("weigh", this("box")).with_adverb("greatly"),
            STORE,
        ),
    ];
    if k.outdoors_work() {
        out.extend([
            // Land.
            r(
                Clause::potent("bring", np("field"))
                    .with_object(np("water"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("summer", "come")),
                WALLS,
            ),
            r(
                Clause::potent("take", np("field"))
                    .with_object(np("frost"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("road"))
                    .with_object(np("stone"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("take", np("road"))
                    .with_object(np("song"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("lose", np("road"))
                    .with_adverb("here")
                    .with_clause(Link::Unless, carries("map")),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("hill"))
                    .with_object(np("wind"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("die", np("field"))
                    .with_adverb("here")
                    .denied(),
                WALLS,
            ),
            r(
                Clause::potent("take", np("field"))
                    .with_object(np("rain"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("road"))
                    .with_object(np("night"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("road"))
                    .with_object(np("peace"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("break", np("road"))
                    .with_adverb("here")
                    .with_clause(Link::Unless, carries("signet")),
                WALLS,
            ),
            // Water.
            r(
                Clause::potent("rise", np("water")).with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("freeze", np("river"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("winter", "come")),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("river"))
                    .with_object(np("light"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("sleep", np("river"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("storm", "come")),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("water"))
                    .with_object(np("song"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("flow", np("river"))
                    .with_adverb("here")
                    .denied()
                    .with_clause(Link::When, when("night", "come")),
                WALLS,
            ),
            r(
                Clause::potent("take", np("river"))
                    .with_object(np("song"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("water"))
                    .with_object(np("fire"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("winter", "come")),
                WALLS,
            ),
            // Air.
            r(
                Clause::potent("bring", np("sky"))
                    .with_object(np("rain"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("summer", "come")),
                WALLS,
            ),
            r(
                Clause::potent("take", np("sky"))
                    .with_object(np("fog"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("sky"))
                    .with_object(np("fire"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("winter", "come")),
                WALLS,
            ),
            r(
                Clause::potent("take", np("sky"))
                    .with_object(np("storm"))
                    .with_adverb("here"),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("sky"))
                    .with_object(np("thunder"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("night", "come")),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("sky"))
                    .with_object(np("ice"))
                    .with_adverb("here")
                    .with_clause(Link::When, when("summer", "come")),
                WALLS,
            ),
            r(
                Clause::potent("bring", np("sky"))
                    .with_object(np("light"))
                    .with_adverb("here")
                    .with_clause(Link::Until, when("sun", "rise")),
                WALLS,
            ),
        ]);
        // Plants and beasts of the land.
        if let Some(p) = life.tree {
            out.push(r(
                Clause::potent("grow", np(p))
                    .with_adverb("greatly")
                    .with_adverb("here"),
                WALLS,
            ));
            out.push(r(
                Clause::potent("keep", np(p))
                    .with_adverb("here")
                    .with_clause(Link::When, when("winter", "come")),
                WALLS,
            ));
            out.push(r(
                Clause::potent("grow", np(p)).with_adverb("here").denied(),
                WALLS,
            ));
            out.push(r(
                Clause::potent("bring", np(p))
                    .with_object(np("light"))
                    .with_adverb("here"),
                WALLS,
            ));
        }
        if let Some(p) = life.flower {
            out.push(r(
                Clause::potent("bring", np(p))
                    .with_object(np("rain"))
                    .with_adverb("here"),
                WALLS,
            ));
        }
        if let Some(b) = life.bird {
            out.push(r(Clause::potent("come", np(b)).with_adverb("here"), WALLS));
            out.push(r(
                Clause::potent("sing", np(b)).with_adverb("here").denied(),
                WALLS,
            ));
            out.push(r(
                Clause::potent("rise", np(b)).with_adverb("here").denied(),
                WALLS,
            ));
        }
        if let Some(i) = life.insect {
            out.push(r(Clause::potent("call", np(i)).with_adverb("here"), WALLS));
            out.push(r(
                Clause::potent("find", np(i)).with_adverb("here").denied(),
                WALLS,
            ));
        }
        if let Some(a) = life.predator {
            out.push(r(Clause::potent("hold", np(a)).with_adverb("here"), WALLS));
            out.push(r(
                Clause::potent("wake", np(a)).with_adverb("here").denied(),
                WALLS,
            ));
        }
        if let Some(a) = life.grazer {
            out.push(r(
                Clause::potent("weigh", np(a)).with_adverb("here").denied(),
                WALLS,
            ));
            out.push(r(Clause::potent("lose", np(a)).with_adverb("here"), WALLS));
            out.push(r(
                Clause::potent("wake", np(a))
                    .with_adverb("here")
                    .with_clause(Link::When, when("night", "come")),
                WALLS,
            ));
            out.push(r(
                Clause::potent("hold", np(a)).with_adverb("here").denied(),
                WALLS,
            ));
        }
    }
    out
}

/// Living things of the land that a language names, for spells.
#[derive(Default)]
struct Life {
    tree: Option<&'static str>,
    flower: Option<&'static str>,
    bird: Option<&'static str>,
    insect: Option<&'static str>,
    predator: Option<&'static str>,
    grazer: Option<&'static str>,
}

impl Life {
    fn of(rng: &mut Rng, lang: &Language) -> Life {
        let mut pick = |tag: &str| {
            let named: Vec<&'static str> = concepts::all()
                .iter()
                .filter(|c| c.has_tag(tag) && lang.lexicon.has(&c.id))
                .map(|c| c.id.as_str())
                .collect();
            (!named.is_empty()).then(|| named[rng.index(named.len())])
        };
        Life {
            tree: pick("tree"),
            flower: pick("flowering"),
            bird: pick("bird"),
            insect: pick("insect"),
            predator: pick("predator"),
            grazer: pick("grazer"),
        }
    }
}

impl K {
    /// Buildings whose work lies out of doors, or that stand in the open:
    /// their spells reach the land, water, plants, beasts and air around.
    fn outdoors_work(self) -> bool {
        matches!(
            self,
            K::Mill
                | K::Bridge
                | K::Garden
                | K::Orchard
                | K::WaysideShrine
                | K::Waystation
                | K::Harbour
                | K::Lighthouse
                | K::SignalStation
                | K::Tower
                | K::Observatory
                | K::Aqueduct
                | K::Cistern
                | K::FountainHouse
                | K::Wall
                | K::Gatehouse
                | K::Hermitage
                | K::Temple
                | K::Granary
                | K::Storehouse
        )
    }
}

/// What a later hand did to a spell.
#[derive(Clone, Copy)]
enum After {
    /// Left as it was.
    Kept,
    /// Written again, the same.
    Renewed,
    /// Written again, but only slightly, or only until a time.
    Weakened,
    /// Undone: the spell denied by someone who wanted it gone.
    Undone,
}

// DESIGN-Q: of the everyday spells, about half are left as written, a
// sixth renewed by a later hand, a sixth weakened (rewritten "slightly",
// or to hold only until winter), and a sixth undone (rewritten denied).
fn after(rng: &mut Rng) -> After {
    match rng.below(12) {
        0..=5 => After::Kept,
        6 | 7 => After::Renewed,
        8 | 9 => After::Weakened,
        _ => After::Undone,
    }
}

/// Whether every concept the clause names is a word of the language.
fn sayable(lang: &Language, c: &Clause) -> bool {
    let s = Sentence::Clause(c.clone());
    s.clauses().iter().all(|c| {
        lang.lexicon.has(&c.predicate)
            && c.adverbs.iter().all(|a| lang.lexicon.has(a))
            && c.subordinate
                .iter()
                .all(|sub| lang.lexicon.has(sub.link.concept()))
    }) && s.noun_phrases().iter().all(|n| match &n.head {
        scraped_lang::meaning::Head::Concept(id) => {
            lang.lexicon.has(id) && n.determiner.as_ref().is_none_or(|d| lang.lexicon.has(d))
        }
        _ => true,
    })
}

/// A surface of the building to write on: preferring the given features,
/// and often one already written on, so spells stack on older writing.
fn surface(rng: &mut Rng, st: &Structure, features: &[&str]) -> Spot {
    let mut best: Vec<(usize, usize, usize)> = Vec::new();
    for (ri, room) in st.interior.rooms.iter().enumerate() {
        for (fi, f) in room.features.iter().enumerate() {
            if let Some(rank) = features.iter().position(|x| *x == f.kind) {
                best.push((rank, ri, fi));
            }
        }
    }
    if best.is_empty() {
        return (st.id, None);
    }
    best.sort_unstable();
    let rank = best[0].0;
    let same: Vec<(usize, usize)> = best
        .iter()
        .filter(|b| b.0 == rank)
        .map(|b| (b.1, b.2))
        .collect();
    let (r, f) = same[rng.index(same.len())];
    (st.id, Some((r, f)))
}

/// Writes the everyday spells of every building that stood long enough.
pub fn write(
    seed: u64,
    h: &History,
    langs: &[Language],
    structures: &mut [Structure],
    texts: &mut Vec<Text>,
) {
    let mut rng = Rng::new(seed, Stream::World(24));
    let mut uses: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let key_of = |c: &Clause| format!("{c:?}");
    let last = h.eras.last().map_or(0, |e| e.end);
    for sid in 0..structures.len() {
        let st = &structures[sid];
        let era = st.era.min(langs.len() as u32 - 1);
        let lang = &langs[era as usize];
        // The living things of the land, for spells about them.
        let life = Life::of(&mut rng, lang);
        let own: Vec<Recipe> = recipes(st.kind, &life, sid as u64)
            .into_iter()
            .filter(|r| sayable(lang, &r.clause))
            .collect();
        let lesser: Vec<Recipe> = pool(st.kind, &life)
            .into_iter()
            .filter(|r| sayable(lang, &r.clause))
            .collect();
        // The building's own spells, and one or two lesser ones.
        let mut options = own;
        let mut lesser = lesser;
        let more = if st.kind.outdoors_work() { 2 } else { 1 };
        for _ in 0..more + rng.below(2) {
            if lesser.is_empty() {
                break;
            }
            // People wrote all sorts: the least written lesser spells first.
            let counts: Vec<u32> = lesser
                .iter()
                .map(|r| uses.get(&key_of(&r.clause)).copied().unwrap_or(0))
                .collect();
            let fewest = counts.iter().copied().min().unwrap_or(0);
            let least: Vec<usize> = (0..lesser.len()).filter(|&i| counts[i] == fewest).collect();
            let i = least[rng.index(least.len())];
            *uses.entry(key_of(&lesser[i].clause)).or_default() += 1;
            options.insert(0, lesser.swap_remove(i));
        }
        if options.is_empty() {
            continue;
        }
        // DESIGN-Q: one to three everyday spells a building, by size of
        // the place: a house one or two, a hall or temple up to three.
        let n =
            (more as usize + 1 + rng.below(if st.settlement.is_some() { 2 } else { 1 }) as usize)
                .min(options.len());
        let start = rng.index(options.len());
        for k in 0..n {
            let recipe = &options[(start + k) % options.len()];
            let at = surface(&mut rng, &structures[sid], recipe.features);
            let year = structures[sid].built + rng.below(20) as i32;
            put(
                texts,
                structures,
                at,
                Written {
                    era,
                    year,
                    genre: Genre::Potent,
                    meaning: Sentence::Clause(recipe.clause.clone()),
                    author: None,
                    event: None,
                    arc: None,
                },
            );
            // A later hand, in a later era's words.
            let later_year = year + 40 + rng.below(400) as i32;
            if later_year >= last {
                continue;
            }
            let later_era = h
                .eras
                .iter()
                .find(|e| e.start <= later_year && later_year < e.end)
                .map_or(era, |e| e.index)
                .min(langs.len() as u32 - 1);
            let later_lang = &langs[later_era as usize];
            let clause = match after(&mut rng) {
                After::Kept => continue,
                After::Renewed => recipe.clause.clone(),
                After::Weakened => {
                    if recipe.clause.subordinate.is_empty() && rng.chance(50) {
                        recipe
                            .clause
                            .clone()
                            .with_clause(Link::Until, when("winter", "come"))
                    } else {
                        let mut c = recipe.clause.clone();
                        c.adverbs.retain(|a| a != "greatly");
                        c.with_adverb("slightly")
                    }
                }
                After::Undone => {
                    let mut c = recipe.clause.clone();
                    c.polarity = match c.polarity {
                        scraped_lang::meaning::Polarity::Positive => {
                            scraped_lang::meaning::Polarity::Negative
                        }
                        scraped_lang::meaning::Polarity::Negative => {
                            scraped_lang::meaning::Polarity::Positive
                        }
                    };
                    c.subordinate.clear();
                    c
                }
            };
            if !sayable(later_lang, &clause) {
                continue;
            }
            put(
                texts,
                structures,
                at,
                Written {
                    era: later_era,
                    year: later_year,
                    genre: Genre::Potent,
                    meaning: Sentence::Clause(clause),
                    author: None,
                    event: None,
                    arc: None,
                },
            );
        }
    }
}
