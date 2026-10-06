//! Writing that acts: surfaces with stacks of layers, potent claims, and
//! what they do to the world once scraped.
//!
//! The three laws: writing holds power (a potent text is a latent claim);
//! scraping releases it (the scraped text's claim acts); nothing is lost
//! (scraped layers stay, as traces). Only the most recent scraped layer on
//! a surface is live; older layers beneath it are ghosts.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use scraped_lang::corpus::Kind;
use scraped_lang::meaning::{
    Argument, Clause, Head, Link, Mood, NounPhrase, Polarity, Role, Sentence, Tense,
};
use scraped_lang::powers::{Power, Powers};
use scraped_world::history::{EventKind, Role as PersonRole};
use scraped_world::structures::Material;
use scraped_world::texts::Text;
use scraped_world::World;

use crate::fixtures::Fixtures;
use crate::outdoors::{hash, Land, Pos, CELL};

/// What kind of thing a claim's target is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Passage,
    Room,
    Land,
    Structure,
    Water,
    Plant,
    Animal,
    Thing,
    Air,
    Person,
}

impl Class {
    pub const ALL: [Class; 10] = [
        Class::Passage,
        Class::Room,
        Class::Land,
        Class::Structure,
        Class::Water,
        Class::Plant,
        Class::Animal,
        Class::Thing,
        Class::Air,
        Class::Person,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::Passage => "passage",
            Class::Room => "room",
            Class::Land => "land",
            Class::Structure => "structure",
            Class::Water => "water",
            Class::Plant => "plant",
            Class::Animal => "animal",
            Class::Thing => "thing",
            Class::Air => "air",
            Class::Person => "person",
        }
    }

    /// Whether claims on this class act inside buildings (else outdoors).
    pub fn indoors(self) -> bool {
        matches!(
            self,
            Class::Passage | Class::Room | Class::Structure | Class::Thing | Class::Person
        )
    }
}

/// A quality claims push (D09: the fifteen of `scraped_lang::powers`).
pub use scraped_lang::powers::Quality as Property;

#[derive(Debug, Clone, Deserialize)]
struct SubjectRow {
    id: String,
    class: Class,
}

#[derive(Debug, Clone, Deserialize)]
struct TagRow {
    tags: Vec<String>,
    class: Class,
}

#[derive(Debug, Clone, Deserialize)]
struct ClassRow {
    class: Class,
    qualities: Vec<Property>,
}

#[derive(Deserialize)]
struct ClaimFile {
    subject: Vec<SubjectRow>,
    tag: Vec<TagRow>,
    class: Vec<ClassRow>,
}

/// What spells can act on (`data/claims.toml`): each concept's class, and
/// the qualities each class takes.
pub struct Table {
    subjects: Vec<SubjectRow>,
    tags: Vec<TagRow>,
    classes: Vec<ClassRow>,
}

impl Table {
    pub fn get() -> &'static Table {
        static T: OnceLock<Table> = OnceLock::new();
        T.get_or_init(|| {
            let f: ClaimFile = toml::from_str(include_str!("../data/claims.toml"))
                .expect("data/claims.toml is valid");
            Table {
                subjects: f.subject,
                tags: f.tag,
                classes: f.class,
            }
        })
    }

    /// The class of thing a concept names, if a spell can act on it.
    pub fn class(&self, subject: &str) -> Option<Class> {
        if let Some(s) = self.subjects.iter().find(|s| s.id == subject) {
            return Some(s.class);
        }
        let c = scraped_lang::concepts::find(subject)?;
        self.tags
            .iter()
            .find(|t| t.tags.iter().any(|x| c.has_tag(x)))
            .map(|t| t.class)
    }

    /// Whether a class can take a quality.
    pub fn takes(&self, class: Class, q: Property) -> bool {
        self.classes
            .iter()
            .any(|c| c.class == class && c.qualities.contains(&q))
    }

    /// What a plain claim does (no modifiers): class, property and signed
    /// amount, or None for a vague claim the world cannot make true.
    pub fn effect(
        &self,
        verb: &str,
        subject: &str,
        negative: bool,
    ) -> Option<(Class, Property, i32)> {
        self.effect_in(&Powers::new(0), verb, subject, None, negative)
    }

    /// What a claim does in a world with these powers.
    pub fn effect_in(
        &self,
        powers: &Powers,
        verb: &str,
        target: &str,
        given: Option<&str>,
        negative: bool,
    ) -> Option<(Class, Property, i32)> {
        let class = self.class(target)?;
        let mut power = match (scraped_lang::powers::carrier(verb), given) {
            (Some(sign), Some(o)) => {
                let p = powers.of(o)?;
                if sign < 0 {
                    p.reversed()
                } else {
                    p
                }
            }
            (Some(_), None) => return None,
            (None, _) => powers.of(verb)?,
        };
        if negative {
            power = power.reversed();
        }
        // A door broken stands open.
        if class == Class::Passage
            && power.quality == Property::Stability
            && power.sign < 0
            && !self.takes(class, Property::Stability)
        {
            power = Power {
                quality: Property::Openness,
                sign: 1,
            };
        }
        if !self.takes(class, power.quality) {
            return None;
        }
        Some((class, power.quality, amount(power, 2)))
    }
}

/// A power's amount at a degree (1 slightly, 2 plainly, 3 greatly): heat in
/// degrees, every other quality a signed strength.
pub fn amount(p: Power, degree: u8) -> i32 {
    let d = i32::from(degree.clamp(1, 3));
    match p.quality {
        Property::Heat => i32::from(p.sign) * 6 * d,
        _ => i32::from(p.sign) * d,
    }
}

/// What a conditional spell waits on (D09).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "of")]
pub enum Trigger {
    Night,
    Day,
    Rain,
    Winter,
    Summer,
    /// Someone is inside the building the spell is on.
    Entered,
    /// Someone there carries a thing of this kind.
    Carries(String),
    /// A condition the world can't judge: it never holds.
    Unknown,
}

/// A spell's condition: while its trigger holds (when, if, after), or
/// while it doesn't (until, unless, before).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Condition {
    pub trigger: Trigger,
    /// Acts while the trigger holds (true) or while it doesn't.
    pub while_holds: bool,
}

/// What a condition can see now.
#[derive(Debug, Clone, Default)]
pub struct Now {
    pub night: bool,
    pub raining: bool,
    pub winter: bool,
    pub summer: bool,
    /// The building the player is in.
    pub inside: Option<usize>,
    /// The kinds of thing the player carries.
    pub carrying: Vec<String>,
}

impl Condition {
    /// Whether a spell with this condition, on `structure`, acts now.
    pub fn acts(&self, now: &Now, structure: usize) -> bool {
        let holds = match &self.trigger {
            Trigger::Night => now.night,
            Trigger::Day => !now.night,
            Trigger::Rain => now.raining,
            Trigger::Winter => now.winter,
            Trigger::Summer => now.summer,
            Trigger::Entered => now.inside == Some(structure),
            Trigger::Carries(k) => {
                now.inside == Some(structure) && now.carrying.iter().any(|c| c == k)
            }
            Trigger::Unknown => false,
        };
        holds == self.while_holds
    }
}

/// How far a spell reaches, by its words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Extent {
    /// "this", "here": its own building, or just around it outdoors.
    Here,
    /// As far as its surface carries.
    Plain,
    /// "widely": three times as far.
    Wide,
}

/// A claim acting on the world.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Claim {
    /// The text it comes from.
    pub text: usize,
    pub verb: String,
    /// The target: what the spell changes.
    pub subject: String,
    pub negative: bool,
    pub class: Class,
    pub property: Property,
    pub amount: i32,
    /// Where the surface is, and how far the claim reaches (metres).
    pub pos: Pos,
    pub range: f64,
    pub year: i32,
    pub structure: usize,
    /// What the spell gives or takes away, for "bring" and "take".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<String>,
    pub extent: Extent,
    /// 1 slightly, 2 plainly, 3 greatly.
    pub degree: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
}

impl Claim {
    /// Whether it acts now (a conditional spell lies dormant otherwise).
    pub fn acts(&self, now: &Now) -> bool {
        self.condition
            .as_ref()
            .is_none_or(|c| c.acts(now, self.structure))
    }

    /// The kind of claim, for counting variety: class, quality, direction.
    pub fn kind(&self) -> (Class, Property, bool) {
        (self.class, self.property, self.amount > 0)
    }
}

/// A spell as its words put it, before the world makes it act.
#[derive(Debug, Clone, PartialEq)]
pub struct Spell {
    pub verb: String,
    pub target: Head,
    pub given: Option<String>,
    pub negative: bool,
    /// "this" (or a named place): its own building or place only.
    pub this: bool,
    pub degree: u8,
    pub extent: Extent,
    pub condition: Option<Condition>,
}

/// The spell a potent text makes, if it is one: its target (the object of
/// a verb that acts on something, else the subject), what it gives, how
/// much, how far, and on what condition.
pub fn spell_of(t: &Text) -> Option<Spell> {
    if t.kind != Kind::Potent {
        return None;
    }
    let Sentence::Clause(c) = &t.meaning else {
        return None;
    };
    if c.mood != Mood::Potent {
        return None;
    }
    spell_of_clause(c)
}

/// The spell a potent clause makes (also for player writing).
pub fn spell_of_clause(c: &Clause) -> Option<Spell> {
    let arg = |r: Role| c.args.iter().find(|a| a.role == r).map(|a| &a.np);
    let subject = arg(Role::Subject)?;
    let object = arg(Role::Object);
    let carrier = scraped_lang::powers::carrier(&c.predicate).is_some();
    let (target, given) = match (carrier, object) {
        (true, Some(o)) => (subject, concept_of(o)),
        (false, Some(o)) => (o, None),
        (_, None) => (subject, None),
    };
    let mut negative = c.polarity == Polarity::Negative;
    if target.determiner.as_deref() == Some("no") {
        negative = !negative;
    }
    let mut degree = 2;
    let mut extent = Extent::Plain;
    for a in &c.adverbs {
        match a.as_str() {
            "greatly" => degree = 3,
            "slightly" => degree = 1,
            "here" => extent = Extent::Here,
            "widely" => extent = Extent::Wide,
            "never" => negative = !negative,
            _ => {}
        }
    }
    let this = target.determiner.as_deref() == Some("this") || matches!(target.head, Head::Name(_));
    if this {
        extent = Extent::Here;
    }
    let condition = c.subordinate.iter().find_map(|s| {
        let while_holds = match s.link {
            Link::When | Link::If | Link::After => true,
            Link::Until | Link::Unless | Link::Before => false,
            _ => return None,
        };
        let trigger = trigger_of(&s.clause);
        // "until the night does not come": a denied trigger turns about.
        let neg = s.clause.polarity == Polarity::Negative;
        Some(Condition {
            trigger,
            while_holds: while_holds != neg,
        })
    });
    Some(Spell {
        verb: c.predicate.clone(),
        target: target.head.clone(),
        given,
        negative,
        this,
        degree,
        extent,
        condition,
    })
}

fn concept_of(np: &NounPhrase) -> Option<String> {
    match &np.head {
        Head::Concept(id) => Some(id.clone()),
        _ => None,
    }
}

/// What a condition clause waits on.
// DESIGN-Q: conditions the world can judge: night and day (night, dusk,
// evening, darkness, moon, stars; day, dawn, morning, sun, light; their
// going reverses them), rain (rain, storm, cloud, flood), winter (winter,
// snow, frost, ice), summer, someone entering (enter or come, said of a
// person), and someone carrying a kind of thing (carry, hold or bring it).
pub fn trigger_of(c: &Clause) -> Trigger {
    let arg = |r: Role| c.args.iter().find(|a| a.role == r).map(|a| &a.np);
    let s = arg(Role::Subject).and_then(concept_of);
    let o = arg(Role::Object).and_then(concept_of);
    let going = matches!(
        c.predicate.as_str(),
        "go" | "leave" | "die" | "fall" | "flee" | "depart"
    );
    let person = |id: &str| {
        Table::get().class(id) == Some(Class::Person)
            || arg(Role::Subject).is_some_and(|np| matches!(np.head, Head::Name(_)))
    };
    if matches!(c.predicate.as_str(), "carry" | "hold" | "bring") {
        if let Some(o) = o {
            return Trigger::Carries(o);
        }
    }
    let Some(s) = s else {
        return if matches!(c.predicate.as_str(), "enter" | "come") {
            Trigger::Entered
        } else {
            Trigger::Unknown
        };
    };
    match s.as_str() {
        "night" | "dusk" | "evening" | "darkness" | "moon" | "star" | "shadow" => {
            if going {
                Trigger::Day
            } else {
                Trigger::Night
            }
        }
        "day" | "dawn" | "morning" | "sun" | "light" => {
            if going {
                Trigger::Night
            } else {
                Trigger::Day
            }
        }
        "rain" | "storm" | "cloud" | "flood" => Trigger::Rain,
        "winter" | "snow" | "frost" | "ice" => Trigger::Winter,
        "summer" => Trigger::Summer,
        _ if matches!(c.predicate.as_str(), "enter" | "come") && person(&s) => Trigger::Entered,
        _ => Trigger::Unknown,
    }
}

/// One written-on surface and its layers of writing, oldest first.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Surface {
    pub structure: usize,
    /// `None`: on the outside of the building.
    pub room: Option<usize>,
    pub feature: Option<usize>,
    pub layers: Vec<usize>,
}

/// All writing in a world, as play sees it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Writing {
    /// Latent potent inscriptions added for play; their ids follow the
    /// world's texts.
    pub extra: Vec<Text>,
    pub surfaces: Vec<Surface>,
    /// Texts already scraped when play begins (historic potent writing).
    pub scraped: BTreeSet<usize>,
    /// The root inscription: the world's first great writing.
    pub root: Option<usize>,
    /// The deepest stack's accounts beneath the root, oldest first: how
    /// the world came to this, and the words for leaving it (M11). Only the
    /// strongest lens reads them.
    pub deep: Vec<usize>,
    /// A previous run's final inscription, carried into this world as a
    /// faint, very old layer (legacy, M11).
    pub legacy: Option<usize>,
    /// Places old writing keeps shut (D11), in the order a reader can open
    /// them: what each needs is taught beside the one before.
    pub sealed: Vec<Sealed>,
    /// The texts of the sealed places: their wards, what lies beneath, and
    /// what is written inside. Their ids start at `SEALED_BASE`, so a
    /// player's own texts keep their ids from older builds.
    pub sealed_texts: Vec<Text>,
}

/// Where the sealed places' texts are numbered from (D11): above any
/// world's texts and any player's, so adding them shifts no saved id.
pub const SEALED_BASE: usize = 1 << 24;

/// A place old writing keeps shut (D11): a building whose way in is held by
/// a ward on the stone at its door. Nothing written anywhere else moves it:
/// only what is live on that stone counts. The ward's own words, read off
/// the stone, show the shape a counter-spell must take; the words for the
/// next place lie beneath the ward and inside.
#[derive(Debug, Clone, Serialize)]
pub struct Sealed {
    pub structure: usize,
    /// The ward: "let this <noun> not open", greatly.
    pub ward: usize,
    /// What its ward holds shut, by concept id.
    pub noun: &'static str,
    /// How strongly a counter-spell must open it (1 slightly, 2 plainly,
    /// 3 greatly): the deeper into the chain, the stronger the ward.
    pub degree: u8,
    /// The account just beneath the ward, telling the next place's words.
    pub clue: Option<usize>,
    /// The account inside, telling the next place's words again (in the
    /// last place: where the first great writing was done).
    pub inside: Option<usize>,
    /// Whether it holds a great inscription: the chain's end (D11).
    pub great: bool,
    /// Sealed from within: the way in is open, its ward on a wall of the
    /// entrance room holds every way on from there.
    pub inner: bool,
}

/// The claim a potent text makes, if any: (verb, subject, negative).
pub fn claim_parts(t: &Text) -> Option<(String, String, bool)> {
    if t.kind != Kind::Potent {
        return None;
    }
    let Sentence::Clause(c) = &t.meaning else {
        return None;
    };
    if c.mood != Mood::Potent {
        return None;
    }
    let subject = c.args.iter().find(|a| a.role == Role::Subject)?;
    let Head::Concept(id) = &subject.np.head else {
        return None;
    };
    Some((
        c.predicate.clone(),
        id.clone(),
        c.polarity == Polarity::Negative,
    ))
}

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
            role: Role::Subject,
            np: NounPhrase::concept(subject),
        }],
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

/// A plain statement: "<subject> <verb>ed (<object>)".
fn statement(
    verb: &str,
    tense: Tense,
    negative: bool,
    subject: &str,
    object: Option<&str>,
) -> Sentence {
    let mut args = vec![Argument {
        role: Role::Subject,
        np: NounPhrase::concept(subject),
    }];
    if let Some(o) = object {
        args.push(Argument {
            role: Role::Object,
            np: NounPhrase::concept(o),
        });
    }
    Sentence::Clause(Clause {
        predicate: verb.to_string(),
        mood: Mood::Declarative,
        tense,
        polarity: if negative {
            Polarity::Negative
        } else {
            Polarity::Positive
        },
        args,
        adverbs: Vec::new(),
        aspect: Default::default(),
        subordinate: Vec::new(),
        complement: None,
    })
}

/// How far a released claim reaches, by what it was written on.
// DESIGN-Q: reach by surface material (stone 900 m, metal 700, clay 500,
// wood and plaster 400, vellum 300); M10 scales it with the tool.
pub fn reach(m: Material) -> f64 {
    match m {
        Material::Stone => 900.0,
        Material::Metal => 700.0,
        Material::Clay => 500.0,
        Material::Wood | Material::Plaster => 400.0,
        Material::Vellum => 300.0,
    }
}

/// Features that take an inscription well.
const INSCRIBABLE: &[&str] = &[
    "wall",
    "stele",
    "altar",
    "niche",
    "lintel",
    "door-slab",
    "gate",
    "parapet",
    "gravestone",
];

impl Writing {
    /// Surfaces from history, plus a few latent potent inscriptions away
    /// from the start.
    pub fn new(
        w: &World,
        land: &Land,
        fixtures: &Fixtures,
        start: usize,
        regions: Option<&crate::region::Regions>,
        legacy: Option<&Sentence>,
    ) -> Self {
        let mut out = Writing::default();
        let n = w.texts.len();
        // History's potent writing was cast: it lies scraped, and acts. Some
        // everyday spells were written and never cast (D10): their writers
        // died, fled or thought better of it, and they wait for whoever
        // scrapes or cleans them off.
        // DESIGN-Q: one everyday spell in three lies latent.
        for t in &w.texts {
            let everyday = matches!(
                t.genre,
                scraped_world::texts::Genre::Charm
                    | scraped_world::texts::Genre::Ward
                    | scraped_world::texts::Genre::Invocation
            );
            if t.kind == Kind::Potent
                && !(everyday && hash(&[w.seed, 0x1a7f, t.id as u64]).is_multiple_of(3))
            {
                out.scraped.insert(t.id);
            }
        }
        // Latent inscriptions.
        // DESIGN-Q: four latent ones away from the start (1.5 km or more),
        // in the newest era's language, written after everything else on
        // their surface. Nothing is placed beside a tool (D10).
        let era = (w.languages.len() as u32).saturating_sub(1);
        let year = w.history.eras.last().map_or(0, |e| e.end);
        let start_pos = Pos::of_cell(
            w.history.settlements[start].cell.ux(),
            w.history.settlements[start].cell.uy(),
        );
        let add = |out: &mut Writing,
                   structure: usize,
                   room: usize,
                   feature: usize,
                   verb: &str,
                   subject: &str,
                   negative: bool| {
            let st = &w.structures[structure];
            let material = st.interior.rooms[room].features[feature].material;
            let id = n + out.extra.len();
            out.extra.push(Text {
                id,
                era,
                year,
                kind: Kind::Potent,
                genre: scraped_world::texts::Genre::of(Kind::Potent),
                arc: None,
                meaning: potent(verb, subject, negative),
                author: None,
                event: None,
                structure,
                room: Some(room),
                feature: Some(feature),
                material,
            });
            id
        };
        let claims: [(&str, &[&str]); 3] = [
            ("open", &["gate", "door", "tomb"]),
            ("burn", &["field", "tree", "house"]),
            ("break", &["wall", "stone", "statue"]),
        ];
        let mut placed = 0;
        let mut tries = 0u64;
        while placed < 4 && tries < 200 {
            tries += 1;
            let h = hash(&[w.seed, 0x1a7e, tries]);
            let structure = (h % w.structures.len() as u64) as usize;
            let st = &w.structures[structure];
            let far = land.structure_pos[structure].dist(start_pos);
            if st.settlement == Some(start) || far < 1500.0 {
                continue;
            }
            let rooms = fixtures.reachable_rooms(w, structure);
            let Some((r, f)) = rooms.iter().find_map(|&r| {
                st.interior.rooms[r]
                    .features
                    .iter()
                    .position(|f| INSCRIBABLE.contains(&f.kind))
                    .map(|f| (r, f))
            }) else {
                continue;
            };
            if out.extra.iter().any(|t| t.structure == structure) {
                continue;
            }
            let (verb, subjects) = claims[((h >> 20) % 3) as usize];
            let subject = subjects[((h >> 24) % subjects.len() as u64) as usize];
            add(&mut out, structure, r, f, verb, subject, (h >> 30) & 1 == 1);
            placed += 1;
        }
        // Evidence of the big picture: shortage ledgers in the storehouses
        // of the regions with the least life left.
        // DESIGN-Q: up to four scarce ledgers, in storehouses of regions
        // whose life has fallen most below their land's natural level.
        if let Some(regions) = regions {
            let mut worst: Vec<(i32, usize)> = regions
                .regions
                .iter()
                .map(|r| {
                    (
                        regions.initial.vars[r.id][crate::region::LIFE] * 1000
                            / r.natural[0].max(1),
                        r.id,
                    )
                })
                .filter(|(ratio, _)| *ratio < 900)
                .collect();
            worst.sort_unstable();
            let goods = scraped_lang::concepts::nouns_tagged("good");
            let mut placed = 0;
            for (_, r) in worst {
                if placed >= 4 {
                    break;
                }
                let Some(st) = w.structures.iter().find(|st| {
                    st.kind == scraped_world::structures::StructureKind::Storehouse
                        && regions.at(land.structure_pos[st.id]) == Some(r)
                        && !out.extra.iter().any(|t| t.structure == st.id)
                }) else {
                    continue;
                };
                let rooms = fixtures.reachable_rooms(w, st.id);
                let Some((room, feature)) = rooms.iter().find_map(|&ri| {
                    st.interior.rooms[ri]
                        .features
                        .iter()
                        .position(|f| matches!(f.kind, "tablet" | "shelf" | "wall" | "jar"))
                        .map(|f| (ri, f))
                }) else {
                    continue;
                };
                let h = hash(&[w.seed, 0x5ca2, st.id as u64]);
                let mut items: Vec<NounPhrase> = Vec::new();
                for k in 0..2u64 {
                    let c = &goods[((h >> (8 * k)) % goods.len() as u64) as usize];
                    if items.iter().any(|i| i.head == Head::Concept(c.id.clone())) {
                        continue;
                    }
                    items
                        .push(NounPhrase::concept(&c.id).counted(1 + ((h >> (16 + k)) % 2) as u16));
                }
                if items.len() >= 2 {
                    let mut total = NounPhrase::concept("total");
                    total.quantity = Some(items.iter().filter_map(|i| i.quantity).sum());
                    items.push(total);
                }
                let material = st.interior.rooms[room].features[feature].material;
                out.extra.push(Text {
                    id: n + out.extra.len(),
                    era,
                    year: year - 1,
                    kind: Kind::Ledger,
                    genre: scraped_world::texts::Genre::of(Kind::Ledger),
                    arc: None,
                    meaning: Sentence::List(items),
                    author: None,
                    event: None,
                    structure: st.id,
                    room: Some(room),
                    feature: Some(feature),
                    material,
                });
                placed += 1;
            }
        }
        out.add_deep(w, fixtures);
        if let Some(m) = legacy {
            out.add_legacy(w, fixtures, start, m);
        }
        out.add_sealed(w, land, fixtures, start);
        // Stack every text on its surface, oldest first.
        let all = w
            .texts
            .iter()
            .chain(out.extra.iter())
            .chain(out.sealed_texts.iter());
        let mut surfaces: Vec<Surface> = Vec::new();
        let mut order: Vec<&Text> = all.collect();
        order.sort_by_key(|t| (t.structure, t.room, t.feature, t.year, t.id));
        for t in order {
            match surfaces.last_mut() {
                Some(s)
                    if s.structure == t.structure && s.room == t.room && s.feature == t.feature =>
                {
                    s.layers.push(t.id)
                }
                _ => surfaces.push(Surface {
                    structure: t.structure,
                    room: t.room,
                    feature: t.feature,
                    layers: vec![t.id],
                }),
            }
        }
        out.surfaces = surfaces;
        out
    }

    /// The sealed places (D11): buildings with a stone at their door, held
    /// shut by a ward, away from the start and holding nothing the game
    /// needs without writing. Each ward holds a different kind of way (a
    /// gate, a door, a tomb). The words for the first lie beneath its own
    /// ward and in one ordinary building; for each next one, beneath the
    /// ward before and inside the place before. Opening them in order is
    /// always possible, so the chain has no loop.
    // DESIGN-Q: three sealed places at most, nearest the start first; the
    // ward is "let this <gate|door|tomb> not open", greatly; the accounts
    // say "a man opened the <noun>" ("greatly", for every place after the
    // first). Only what is live on the ward stone counts for the way in,
    // and after the first place only a counter as strong as the ward
    // ("greatly") opens it.
    fn add_sealed(&mut self, w: &World, land: &Land, fixtures: &Fixtures, start: usize) {
        use scraped_world::structures::Condition;
        const NOUNS: [&str; 3] = ["gate", "door", "tomb"];
        let era = (w.languages.len() as u32).saturating_sub(1);
        let year = w.history.eras.last().map_or(0, |e| e.end);
        let start_pos = Pos::of_cell(
            w.history.settlements[start].cell.ux(),
            w.history.settlements[start].cell.uy(),
        );
        let root = w.texts.iter().find(|t| t.event == Some(w.history.root));
        let greats: BTreeSet<usize> = crate::region::great_events(w)
            .into_iter()
            .filter_map(|e| w.texts.iter().find(|t| t.event == Some(e)))
            .map(|t| t.structure)
            .filter(|&s| root.is_none_or(|r| r.structure != s))
            .collect();
        let holding: BTreeSet<usize> = fixtures
            .items
            .iter()
            .filter_map(|p| match p.at {
                crate::fixtures::Spot::Room { structure, .. } => Some(structure),
                crate::fixtures::Spot::Out { .. } => None,
            })
            .collect();
        let busy: BTreeSet<usize> = self
            .extra
            .iter()
            .map(|t| t.structure)
            .chain(greats.iter().copied())
            .chain(root.map(|r| r.structure))
            .chain(holding.iter().copied())
            .collect();
        // An inscribable feature in a room a walker can reach.
        let wall = |s: usize| -> Option<(usize, usize)> {
            let st = &w.structures[s];
            fixtures.reachable_rooms(w, s).into_iter().find_map(|r| {
                st.interior.rooms[r]
                    .features
                    .iter()
                    .position(|f| INSCRIBABLE.contains(&f.kind))
                    .map(|f| (r, f))
            })
        };
        let mut picks: Vec<(i64, usize)> = w
            .structures
            .iter()
            .filter(|st| {
                !st.outside.is_empty()
                    && st.condition != Condition::Buried
                    && st.settlement != Some(start)
                    && !busy.contains(&st.id)
                    && land.structure_pos[st.id].dist(start_pos) >= 1500.0
                    && wall(st.id).is_some()
            })
            .map(|st| (land.structure_pos[st.id].dist2(start_pos), st.id))
            .collect();
        picks.sort_unstable();
        // Spread them: none within 1 km of another.
        let mut chosen: Vec<usize> = Vec::new();
        for (_, s) in picks {
            if chosen.len() == NOUNS.len() {
                break;
            }
            if chosen
                .iter()
                .all(|&c| land.structure_pos[c].dist(land.structure_pos[s]) >= 1000.0)
                && land.route(w, start_pos, land.structure_pos[s]).is_some()
            {
                chosen.push(s);
            }
        }
        // A great inscription's building has no stone at its door and
        // always holds something: it is sealed from within, its ward on a
        // wall of the entrance room holding every way on, so long as no
        // tool lies inside and the great writing lies beyond that room.
        let tools = |s: usize| {
            fixtures.items.iter().any(|p| {
                matches!(p.at, crate::fixtures::Spot::Room { structure, .. } if structure == s)
                    && (crate::items::scrape_power(p.kind) > 0
                        || matches!(p.kind, "stylus" | "lens" | "loupe"))
            })
        };
        let inner = |g: usize| -> Option<usize> {
            let st = &w.structures[g];
            let beyond = w
                .texts
                .iter()
                .any(|t| t.structure == g && t.event.is_some() && t.room.is_some_and(|r| r > 0));
            if st.condition == Condition::Buried
                || st.settlement == Some(start)
                || tools(g)
                || !beyond
                || land.structure_pos[g].dist(start_pos) < 1500.0
                || !fixtures.reachable_rooms(w, g).contains(&0)
                || land.route(w, start_pos, land.structure_pos[g]).is_none()
            {
                return None;
            }
            st.interior
                .rooms
                .first()?
                .features
                .iter()
                .position(|f| INSCRIBABLE.contains(&f.kind))
        };
        let mut great: Option<usize> = None;
        if let Some((g, f0)) = greats.iter().find_map(|&g| inner(g).map(|f| (g, f))) {
            chosen.truncate(NOUNS.len() - 1);
            chosen.retain(|&c| land.structure_pos[c].dist(land.structure_pos[g]) >= 1000.0);
            chosen.push(g);
            great = Some(f0);
        }
        if chosen.is_empty() {
            return;
        }
        let account = |k: usize| {
            let mut c = Clause::plain("open", NounPhrase::concept("man"))
                .with_object(NounPhrase::concept(NOUNS[k]));
            if k > 0 {
                c = c.with_adverb("greatly");
            }
            c.tense = Tense::Past;
            Sentence::Clause(c)
        };
        let push = |out: &mut Writing, t: Text| -> usize {
            let id = SEALED_BASE + out.sealed_texts.len();
            out.sealed_texts.push(Text { id, ..t });
            id
        };
        let base = |structure: usize, room: Option<usize>, feature: Option<usize>, material| Text {
            id: 0,
            era,
            year,
            kind: Kind::Account,
            genre: scraped_world::texts::Genre::of(Kind::Account),
            arc: None,
            meaning: Sentence::List(Vec::new()),
            author: None,
            event: None,
            structure,
            room,
            feature,
            material,
        };
        // The first place's words, once in an ordinary building nearest it.
        let first = chosen[0];
        if let Some((s, (r, f))) = w
            .structures
            .iter()
            .filter(|st| !chosen.contains(&st.id) && !busy.contains(&st.id))
            .filter_map(|st| wall(st.id).map(|rf| (st.id, rf)))
            .min_by_key(|(s, _)| land.structure_pos[*s].dist2(land.structure_pos[first]))
        {
            let material = w.structures[s].interior.rooms[r].features[f].material;
            let t = Text {
                meaning: account(0),
                ..base(s, Some(r), Some(f), material)
            };
            push(self, t);
        }
        for (k, &s) in chosen.iter().enumerate() {
            let last = k + 1 == chosen.len();
            // Where the ward lies: the stone at the door, or (a great
            // inscription's building) a wall of the entrance room.
            let inner = great.filter(|_| last);
            let (room, feature, material) = match inner {
                Some(f) => (
                    Some(0),
                    Some(f),
                    w.structures[s].interior.rooms[0].features[f].material,
                ),
                None => (None, None, Material::Stone),
            };
            // Beneath the ward: the words for this place (the first) or the
            // next one; then the ward itself, on top, cast.
            let next = if k == 0 { Some(0) } else { None }
                .into_iter()
                .chain((k + 1 < chosen.len()).then_some(k + 1))
                .collect::<Vec<_>>();
            let mut clue = None;
            for n in next {
                let id = push(
                    self,
                    Text {
                        meaning: account(n),
                        ..base(s, room, feature, material)
                    },
                );
                self.scraped.insert(id);
                if n == k + 1 {
                    clue = Some(id);
                }
            }
            let ward = push(
                self,
                Text {
                    kind: Kind::Potent,
                    genre: scraped_world::texts::Genre::Ward,
                    meaning: Sentence::Clause(
                        Clause::potent("open", NounPhrase::concept(NOUNS[k]).det("this"))
                            .denied()
                            .with_adverb("greatly"),
                    ),
                    ..base(s, room, feature, material)
                },
            );
            self.scraped.insert(ward);
            // Inside: the next place's words again; inside the last, where
            // the first great writing was done: the thread to the deepest
            // text. (Beyond the entrance room, where a ward holds within.)
            let thread = root
                .and_then(|r| w.structures[r.structure].settlement)
                .map(|town| {
                    let mut c = Clause::plain("scrape", NounPhrase::concept("king"))
                        .with_object(NounPhrase::concept("tablet"));
                    c.args.push(Argument {
                        role: Role::Recipient,
                        np: NounPhrase::name(w.history.people.len() + town),
                    });
                    c.tense = Tense::Past;
                    Sentence::Clause(c)
                });
            let meaning = if last { thread } else { Some(account(k + 1)) };
            let within = |s: usize| -> Option<(usize, usize)> {
                let st = &w.structures[s];
                fixtures
                    .reachable_rooms(w, s)
                    .into_iter()
                    .filter(|&r| inner.is_none() || r > 0)
                    .find_map(|r| {
                        st.interior.rooms[r]
                            .features
                            .iter()
                            .position(|f| INSCRIBABLE.contains(&f.kind))
                            .map(|f| (r, f))
                    })
            };
            let inside = meaning.zip(within(s)).map(|(meaning, (r, f))| {
                let material = w.structures[s].interior.rooms[r].features[f].material;
                push(
                    self,
                    Text {
                        meaning,
                        ..base(s, Some(r), Some(f), material)
                    },
                )
            });
            self.sealed.push(Sealed {
                structure: s,
                ward,
                noun: NOUNS[k],
                degree: if k == 0 { 2 } else { 3 },
                great: great.is_some() && last,
                inner: inner.is_some(),
                clue,
                inside,
            });
        }
    }

    /// Accounts beneath the root inscription, in the first era's language:
    /// the oldest tells of the self departing and holds the departure claim;
    /// above it, how the root was cast and what it did, recopied until the
    /// stack is the deepest in the world.
    // DESIGN-Q: the departure claim is "let the self depart"; the cause is
    // told as "the <king|priest|scribe> scraped the tablet. The <subject>
    // did (not) <verb>. The self did not depart." Padding layers repeat
    // the middle sentence only.
    fn add_deep(&mut self, w: &World, fixtures: &Fixtures) {
        let Some(root) = w.texts.iter().find(|t| t.event == Some(w.history.root)) else {
            return;
        };
        self.root = Some(root.id);
        let Some((verb, subject, negative)) = claim_parts(root) else {
            return;
        };
        // Where the accounts lie: beneath the root if a player can reach
        // it; otherwise on an inscribable surface in a room of the same
        // building they can reach, under a scraped recopy of the root.
        // DESIGN-Q: the recopy is an account (it never acts).
        let rooms = fixtures.reachable_rooms(w, root.structure);
        let at_root = match root.room {
            None => {
                w.structures[root.structure].condition
                    != scraped_world::structures::Condition::Buried
            }
            Some(r) => rooms.contains(&r),
        };
        let host = if at_root {
            Some((root.structure, root.room, root.feature, root.material))
        } else {
            let st = &w.structures[root.structure];
            rooms.iter().rev().find_map(|&r| {
                st.interior.rooms[r]
                    .features
                    .iter()
                    .position(|f| INSCRIBABLE.contains(&f.kind))
                    .map(|f| {
                        (
                            root.structure,
                            Some(r),
                            Some(f),
                            st.interior.rooms[r].features[f].material,
                        )
                    })
            })
        };
        let Some((hs, hr, hf, material)) = host else {
            return;
        };
        let on_host = |t: &&Text| (t.structure, t.room, t.feature) == (hs, hr, hf);
        let oldest = w
            .texts
            .iter()
            .chain(self.extra.iter())
            .filter(on_host)
            .map(|t| t.year)
            .min()
            .unwrap_or(root.year)
            .min(root.year);
        if !at_root {
            let id = w.texts.len() + self.extra.len();
            self.extra.push(Text {
                id,
                era: 0,
                year: root.year,
                kind: Kind::Account,
                genre: scraped_world::texts::Genre::of(Kind::Account),
                arc: None,
                meaning: root.meaning.clone(),
                author: None,
                event: None,
                structure: hs,
                room: hr,
                feature: hf,
                material,
            });
            self.scraped.insert(id);
        }
        let deepest = {
            let mut depth: BTreeMap<(usize, Option<usize>, Option<usize>), usize> = BTreeMap::new();
            for t in w.texts.iter().chain(self.extra.iter()) {
                *depth.entry((t.structure, t.room, t.feature)).or_default() += 1;
            }
            let own = depth.get(&(hs, hr, hf)).copied().unwrap_or(0);
            let other = depth
                .iter()
                .filter(|(k, _)| **k != (hs, hr, hf))
                .map(|(_, &d)| d)
                .max()
                .unwrap_or(0);
            (own, other)
        };
        let author = match w.history.events[w.history.root].kind {
            EventKind::Writing { author, .. } => match w.history.people[author].role {
                PersonRole::Priest => "priest",
                PersonRole::Scribe => "scribe",
                _ => "king",
            },
            _ => "king",
        };
        let departure = Sentence::Text(vec![
            statement("depart", Tense::Past, false, "self", None),
            potent("depart", "self", false),
        ]);
        let cause = Sentence::Text(vec![
            statement("scrape", Tense::Past, false, author, Some("tablet")),
            statement(&verb, Tense::Past, negative, &subject, None),
            statement("depart", Tense::Past, true, "self", None),
        ]);
        // Later layers only repeat what came of it.
        let echo = statement(&verb, Tense::Past, negative, &subject, None);
        let n = w.texts.len();
        let (own, other) = deepest;
        // At least the departure and one account; then echoes of what came
        // of it until no stack is as deep.
        let echoes = other.saturating_sub(own + 1);
        let copies = 1 + echoes;
        for (k, meaning) in [departure, cause]
            .into_iter()
            .chain(std::iter::repeat_n(echo, echoes))
            .enumerate()
        {
            let id = n + self.extra.len();
            self.extra.push(Text {
                id,
                era: 0,
                year: oldest - 1 - (copies + 1 - k) as i32,
                kind: Kind::Account,
                genre: scraped_world::texts::Genre::of(Kind::Account),
                arc: None,
                meaning,
                author: None,
                event: None,
                structure: hs,
                room: hr,
                feature: hf,
                material,
            });
            self.deep.push(id);
        }
    }

    /// Places a previous run's final inscription beneath a cast of history
    /// that can be reached: older than anything else on that surface, so it
    /// is a ghost only the lenses show.
    // DESIGN-Q: the legacy inscription goes beneath a reachable history
    // cast outside the starting town (not the root), in the first era's
    // language, as an account that never acts.
    fn add_legacy(&mut self, w: &World, fixtures: &Fixtures, start: usize, meaning: &Sentence) {
        let root = self.root;
        let mut options: Vec<&Text> = w
            .texts
            .iter()
            .filter(|t| t.kind == Kind::Potent && Some(t.id) != root)
            .filter(|t| w.structures[t.structure].settlement != Some(start))
            .filter(|t| {
                t.room
                    .is_none_or(|r| fixtures.reachable_rooms(w, t.structure).contains(&r))
            })
            .collect();
        if options.is_empty() {
            options = w.texts.iter().filter(|t| Some(t.id) == root).collect();
        }
        if options.is_empty() {
            return;
        }
        let host = options[(hash(&[w.seed, 0x1e9a]) % options.len() as u64) as usize];
        let first = w.history.eras.first().map_or(0, |e| e.start);
        let id = w.texts.len() + self.extra.len();
        self.extra.push(Text {
            id,
            era: 0,
            year: first - 100,
            kind: Kind::Account,
            genre: scraped_world::texts::Genre::of(Kind::Account),
            arc: None,
            meaning: meaning.clone(),
            author: None,
            event: None,
            structure: host.structure,
            room: host.room,
            feature: host.feature,
            material: host.material,
        });
        self.legacy = Some(id);
    }

    pub fn text<'a>(&'a self, w: &'a World, id: usize) -> &'a Text {
        if id >= SEALED_BASE {
            &self.sealed_texts[id - SEALED_BASE]
        } else if id < w.texts.len() {
            &w.texts[id]
        } else {
            &self.extra[id - w.texts.len()]
        }
    }

    pub fn count(&self, w: &World) -> usize {
        w.texts.len() + self.extra.len()
    }

    /// The surface a text is on.
    pub fn surface_of(&self, text: usize) -> Option<usize> {
        self.surfaces.iter().position(|s| s.layers.contains(&text))
    }

    /// The live layer of a surface: its most recent scraped layer.
    pub fn live(&self, surface: usize, scraped: &BTreeSet<usize>) -> Option<usize> {
        live_of(&self.surfaces[surface].layers, scraped)
    }

    /// What a reader sees on a surface (see `visible_of`).
    pub fn visible(&self, surface: usize, scraped: &BTreeSet<usize>) -> Vec<(usize, bool)> {
        visible_of(&self.surfaces[surface].layers, scraped)
    }

    /// How many ghost layers lie beneath what can be read.
    pub fn ghosts(&self, surface: usize, scraped: &BTreeSet<usize>) -> usize {
        ghosts_of(&self.surfaces[surface].layers, scraped)
    }

    /// The top unscraped layer of a surface: what a scrape removes.
    pub fn top_unscraped(&self, surface: usize, scraped: &BTreeSet<usize>) -> Option<usize> {
        top_unscraped_of(&self.surfaces[surface].layers, scraped)
    }

    /// The claim a text would make if live.
    pub fn claim(&self, w: &World, land: &Land, text: usize) -> Option<Claim> {
        claim_of(w, land, self.text(w, text), text)
    }

    /// Every claim acting now.
    pub fn live_claims(&self, w: &World, land: &Land, scraped: &BTreeSet<usize>) -> Vec<Claim> {
        (0..self.surfaces.len())
            .filter_map(|s| self.live(s, scraped))
            .filter_map(|t| self.claim(w, land, t))
            .collect()
    }
}

/// The live layer of a stack: its most recent scraped layer.
pub fn live_of(layers: &[usize], scraped: &BTreeSet<usize>) -> Option<usize> {
    layers.iter().rev().copied().find(|t| scraped.contains(t))
}

/// What a reader sees of a stack: unscraped layers above the live one in
/// full, and the live one in part. (Text, partial.) Older layers are ghosts.
pub fn visible_of(layers: &[usize], scraped: &BTreeSet<usize>) -> Vec<(usize, bool)> {
    match layers.iter().rposition(|t| scraped.contains(t)) {
        None => layers.iter().map(|&t| (t, false)).collect(),
        Some(i) => layers[i..]
            .iter()
            .enumerate()
            .map(|(k, &t)| (t, k == 0))
            .collect(),
    }
}

/// How many ghost layers lie beneath what can be read.
pub fn ghosts_of(layers: &[usize], scraped: &BTreeSet<usize>) -> usize {
    layers
        .iter()
        .rposition(|t| scraped.contains(t))
        .unwrap_or(0)
}

/// The ghost layer just beneath the live one: what the deep-reading tool
/// shows, and what new writing must agree with.
pub fn beneath_of(layers: &[usize], scraped: &BTreeSet<usize>) -> Option<usize> {
    let live = layers.iter().rposition(|t| scraped.contains(t))?;
    live.checked_sub(1).map(|i| layers[i])
}

/// The top unscraped layer of a stack: what a scrape removes.
pub fn top_unscraped_of(layers: &[usize], scraped: &BTreeSet<usize>) -> Option<usize> {
    let live = layers.iter().rposition(|t| scraped.contains(t));
    layers
        .iter()
        .enumerate()
        .rev()
        .find(|(i, t)| !scraped.contains(t) && live.is_none_or(|l| *i > l))
        .map(|(_, &t)| t)
}

/// The claim a text makes when live, if it is potent and not vague.
pub fn claim_of(w: &World, land: &Land, t: &Text, id: usize) -> Option<Claim> {
    let spell = spell_of(t)?;
    let powers = Powers::new(w.seed);
    let table = Table::get();
    // A named place: the spell acts on that town's land.
    let (target, named) = match &spell.target {
        Head::Concept(c) => (c.clone(), None),
        Head::Name(n) => {
            let town = n.checked_sub(w.history.people.len())?;
            (
                "city".to_string(),
                Some(w.history.settlements.get(town)?.cell),
            )
        }
    };
    let (class, property, plain) = table.effect_in(
        &powers,
        &spell.verb,
        &target,
        spell.given.as_deref(),
        spell.negative,
    )?;
    let power = Power {
        quality: property,
        sign: if plain > 0 { 1 } else { -1 },
    };
    // History's casts reach as far as their event says (the root's only to
    // its own surroundings until M10); other writing by its surface and
    // its words.
    // DESIGN-Q: the root inscription acts within 900 m until M10's great
    // inscriptions. "Here" and "this" hold a spell to its own building
    // (indoors) or 150 m around it (outdoors); "widely" triples its reach;
    // a named town is its land, 600 m about its centre.
    let base = match t.event.map(|e| &w.history.events[e].kind) {
        Some(EventKind::Writing { effect, root, .. }) => {
            let r = f64::from(effect.radius) * f64::from(CELL);
            if *root {
                r.min(900.0)
            } else {
                r
            }
        }
        _ => reach(t.material),
    };
    let (pos, range) = match named {
        Some(cell) => (Pos::of_cell(cell.ux(), cell.uy()), 600.0),
        None => (
            land.structure_pos[t.structure],
            match spell.extent {
                Extent::Here if class.indoors() => 0.0,
                Extent::Here => 150.0,
                Extent::Plain => base,
                Extent::Wide => base * 3.0,
            },
        ),
    };
    Some(Claim {
        text: id,
        verb: spell.verb,
        subject: target,
        negative: spell.negative,
        class,
        property,
        amount: amount(power, spell.degree),
        pos,
        range,
        year: t.year,
        structure: t.structure,
        given: spell.given,
        extent: spell.extent,
        degree: spell.degree,
        condition: spell.condition,
    })
}

/// The amount a property takes at a point from the claims reaching it:
/// nearer beats farther, at equal distance newer beats older, and directly
/// contradictory claims at the same place and time cancel.
pub fn resolve(claims: &[&Claim], at: Pos) -> Option<i32> {
    let mut near: Vec<(i64, i32, i32)> = claims
        .iter()
        .filter(|c| c.pos.dist(at) <= c.range)
        .map(|c| (c.pos.dist2(at), -c.year, c.amount))
        .collect();
    near.sort();
    let first = *near.first()?;
    if let Some(second) = near.get(1) {
        if second.0 == first.0 && second.1 == first.1 && second.2 == -first.2 {
            return None;
        }
    }
    Some(first.2)
}

/// The D09 measures of a world's magic.
#[derive(Debug, Clone, Default)]
pub struct MagicCounts {
    /// Distinct kinds of live claim (class, quality, direction).
    pub kinds: usize,
    pub conditional: usize,
    /// Settlements with no live spell within reach of their centre.
    pub settlements_without: usize,
    /// Concepts the world's languages name that have a power.
    pub concepts_with_powers: usize,
    /// Qualities live spells push.
    pub qualities: usize,
}

/// How many regions hold a large spell (D09): one reaching a kilometre
/// and a half or more, written within the region. Of how many regions.
pub fn regions_with_large(regions: &crate::region::Regions, claims: &[Claim]) -> (usize, usize) {
    let with: BTreeSet<usize> = claims
        .iter()
        .filter(|c| c.range >= 1500.0)
        .filter_map(|c| regions.at(c.pos))
        .collect();
    (with.len(), regions.regions.len())
}

pub fn magic_counts(w: &World, land: &Land, claims: &[Claim]) -> MagicCounts {
    let kinds: BTreeSet<(Class, Property, bool)> = claims.iter().map(Claim::kind).collect();
    let qualities: BTreeSet<Property> = claims.iter().map(|c| c.property).collect();
    let settlements_without = w
        .history
        .settlements
        .iter()
        .filter(|s| {
            !w.structures.iter().any(|st| {
                st.settlement == Some(s.id) && {
                    let p = land.structure_pos[st.id];
                    claims.iter().any(|c| c.pos.dist(p) <= c.range)
                }
            })
        })
        .count();
    let powers = Powers::new(w.seed);
    let concepts_with_powers = scraped_lang::concepts::all()
        .iter()
        .filter(|c| w.languages.iter().any(|l| l.lexicon.has(&c.id)) && powers.of(&c.id).is_some())
        .count();
    MagicCounts {
        kinds: kinds.len(),
        conditional: claims.iter().filter(|c| c.condition.is_some()).count(),
        settlements_without,
        concepts_with_powers,
        qualities: qualities.len(),
    }
}

/// Spoiler view: every surface's stack, the claims acting now, and why each
/// settlement is strange.
pub fn debug(w: &World, land: &Land, writing: &Writing, scraped: &BTreeSet<usize>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let claims = writing.live_claims(w, land, scraped);
    let m = magic_counts(w, land, &claims);
    let _ = writeln!(
        out,
        "MAGIC  {} live, {} kinds, {} conditional, {} settlements without, {} concepts with powers, {} qualities", // DEBUG-TEXT
        claims.len(),
        m.kinds,
        m.conditional,
        m.settlements_without,
        m.concepts_with_powers,
        m.qualities
    );
    let mut genres: BTreeMap<String, usize> = BTreeMap::new();
    for t in &w.texts {
        *genres.entry(format!("{:?}", t.genre)).or_default() += 1;
    }
    let top = genres.iter().max_by_key(|(_, &n)| n);
    let _ = writeln!(
        out,
        "TEXTS  {}, largest genre {:?} at {:.0}%", // DEBUG-TEXT
        w.texts.len(),
        top.map(|t| t.0),
        top.map_or(0.0, |t| *t.1 as f64 * 100.0 / w.texts.len().max(1) as f64)
    );
    // Sealed places (D11), in the order the chain opens them.
    let _ = writeln!(out, "SEALED ({})", writing.sealed.len()); // DEBUG-TEXT
    for (k, x) in writing.sealed.iter().enumerate() {
        let st = &w.structures[x.structure];
        let _ = writeln!(
            out,
            "  {}. {:?} #{} ward {} on {} (\"{}\", degree {}){}{}", // DEBUG-TEXT
            k + 1,
            st.kind,
            x.structure,
            x.ward,
            if x.inner {
                "a wall of the entrance room"
            } else {
                "the stone at its door"
            }, // DEBUG-TEXT
            x.noun,
            x.degree,
            if x.great {
                ", a great inscription inside"
            } else {
                ""
            }, // DEBUG-TEXT
            if x.inner { ", sealed from within" } else { "" } // DEBUG-TEXT
        );
    }
    let _ = writeln!(out, "LIVE CLAIMS ({})", claims.len()); // DEBUG-TEXT
    for c in &claims {
        let st = &w.structures[c.structure];
        let _ =
            writeln!(
            out,
            "  text {:>4}  {}{} {:<7}{} → {:?} {:?} {:+}  range {:.0} m  at {:?} {} (year {}){}", // DEBUG-TEXT
            c.text,
            if c.negative { "not " } else { "" },
            c.verb,
            c.subject,
            c.given.as_deref().map_or(String::new(), |g| format!(" ({g})")),
            c.class,
            c.property,
            c.amount,
            c.range,
            st.kind,
            st.id,
            c.year,
            c.condition
                .as_ref()
                .map_or(String::new(), |k| format!("  {}{:?}", if k.while_holds { "while " } else { "unless " }, k.trigger)) // DEBUG-TEXT
        );
    }
    let _ = writeln!(out, "\nWHY EACH SETTLEMENT IS STRANGE"); // DEBUG-TEXT
    for s in &w.history.settlements {
        let p = Pos::of_cell(s.cell.ux(), s.cell.uy());
        let near: Vec<String> = claims
            .iter()
            .filter(|c| c.pos.dist(p) <= c.range + 1000.0)
            .map(|c| {
                format!(
                    "{}{} {} ({:?} {:+})",
                    if c.negative { "not " } else { "" },
                    c.verb,
                    c.subject,
                    c.property,
                    c.amount
                )
            }) // DEBUG-TEXT
            .collect();
        if !near.is_empty() {
            let _ = writeln!(out, "  settlement {}: {}", s.id, near.join("; "));
            // DEBUG-TEXT
        }
    }
    let _ = writeln!(out, "\nSURFACES WITH LAYERS"); // DEBUG-TEXT
    for (i, s) in writing.surfaces.iter().enumerate() {
        if s.layers.len() < 2
            && !s
                .layers
                .iter()
                .any(|t| writing.text(w, *t).kind == Kind::Potent)
        {
            continue;
        }
        let layers: Vec<String> = s
            .layers
            .iter()
            .map(|&t| {
                let x = writing.text(w, t);
                let state = if scraped.contains(&t) {
                    "scraped"
                } else {
                    "unscraped"
                };
                format!("{t}:era{} {:?} {state}", x.era, x.kind) // DEBUG-TEXT
            })
            .collect();
        let _ = writeln!(
            out,
            "  surface {i} on structure {} room {:?}: {}",
            s.structure,
            s.room,
            layers.join(" < ")
        ); // DEBUG-TEXT
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(amount: i32, x: i32, year: i32) -> Claim {
        Claim {
            text: 0,
            verb: "burn".into(),
            subject: "field".into(),
            negative: amount < 0,
            class: Class::Land,
            property: Property::Heat,
            amount,
            pos: Pos::new(x, 0),
            range: 1000.0,
            year,
            structure: 0,
            given: None,
            extent: Extent::Plain,
            degree: 2,
            condition: None,
        }
    }

    #[test]
    fn conflicts_resolve_as_specified() {
        let at = Pos::new(0, 0);
        let (warm_near, cold_far) = (claim(12, 100, 10), claim(-12, 500, 50));
        assert_eq!(
            resolve(&[&warm_near, &cold_far], at),
            Some(12),
            "nearer wins"
        );
        let (old, new) = (claim(12, 300, 10), claim(-12, 300, 50));
        assert_eq!(
            resolve(&[&old, &new], at),
            Some(-12),
            "newer wins at equal distance"
        );
        let (a, b) = (claim(12, 300, 10), claim(-12, 300, 10));
        assert_eq!(resolve(&[&a, &b], at), None, "contradictions cancel");
        let out = claim(12, 5000, 10);
        assert_eq!(resolve(&[&out], at), None, "out of range");
    }

    #[test]
    fn every_potent_capable_concept_has_a_power() {
        let t = Table::get();
        for verb in ["open", "burn", "break"] {
            let v = scraped_lang::concepts::get(verb);
            for n in scraped_lang::concepts::with_pos(scraped_lang::concepts::Pos::Noun) {
                if !v.accepts_object(n) {
                    continue;
                }
                let pos = t.effect(verb, &n.id, false);
                let neg = t.effect(verb, &n.id, true);
                let (Some(p), Some(q)) = (pos, neg) else {
                    // A vague claim: the subject's class has no effect for this
                    // verb. Still deterministic, and must say so in the table.
                    assert!(
                        t.class(&n.id).is_some(),
                        "{verb} {}: subject missing from the table",
                        n.id
                    );
                    continue;
                };
                assert_eq!(p.2, -q.2, "{verb} {}", n.id);
                assert_eq!(t.effect(verb, &n.id, false), pos);
            }
        }
    }

    #[test]
    fn spells_take_their_shape_from_their_words() {
        use scraped_lang::meaning::NounPhrase as N;
        let p = Powers::new(1);
        let t = Table::get();
        // "Let this house bring fire when night comes": the house warms,
        // only at night, only itself.
        let c = Clause::potent("bring", N::concept("house").det("this"))
            .with_object(N::concept("fire"))
            .with_clause(Link::When, Clause::plain("come", N::concept("night")));
        let s = spell_of_clause(&c).unwrap();
        assert_eq!(s.given.as_deref(), Some("fire"));
        assert_eq!(s.extent, Extent::Here);
        let cond = s.condition.clone().unwrap();
        assert_eq!(
            (cond.trigger.clone(), cond.while_holds),
            (Trigger::Night, true)
        );
        let (class, q, a) = t
            .effect_in(&p, &s.verb, "house", s.given.as_deref(), s.negative)
            .unwrap();
        assert_eq!(class, Class::Room);
        assert_eq!(p.of("fire").unwrap().quality, q);
        assert!(a > 0);
        // Taking it away pushes the other way.
        let (_, _, b) = t
            .effect_in(&p, "take", "house", Some("fire"), false)
            .unwrap();
        assert_eq!(a, -b);
        // "No wolf": the determiner denies it.
        let w = Clause::potent("come", N::concept("wolf").det("no")).with_adverb("greatly");
        let s = spell_of_clause(&w).unwrap();
        assert!(s.negative);
        assert_eq!(s.degree, 3);
        // An exception: shut unless someone carries the seal.
        let d = Clause::potent("open", N::concept("door").det("this"))
            .denied()
            .with_clause(
                Link::Unless,
                Clause::plain("carry", N::concept("man")).with_object(N::concept("seal")),
            );
        let cond = spell_of_clause(&d).unwrap().condition.unwrap();
        assert_eq!(cond.trigger, Trigger::Carries("seal".into()));
        let mut now = Now {
            inside: Some(7),
            ..Now::default()
        };
        assert!(cond.acts(&now, 7), "a stranger is kept out");
        now.carrying.push("seal".into());
        assert!(!cond.acts(&now, 7), "the household passes");
        // A spell with no meaning in this class is vague.
        assert!(t.effect_in(&p, "flow", "door", None, false).is_none());
    }

    #[test]
    fn conditions_hold_exactly_when_their_trigger_does() {
        let at_night = Condition {
            trigger: Trigger::Night,
            while_holds: true,
        };
        let until_night = Condition {
            trigger: Trigger::Night,
            while_holds: false,
        };
        for night in [false, true] {
            let now = Now {
                night,
                ..Now::default()
            };
            assert_eq!(at_night.acts(&now, 0), night);
            assert_eq!(until_night.acts(&now, 0), !night);
        }
        let entered = Condition {
            trigger: Trigger::Entered,
            while_holds: true,
        };
        let inside = Now {
            inside: Some(3),
            ..Now::default()
        };
        assert!(entered.acts(&inside, 3));
        assert!(!entered.acts(&inside, 4));
        let never = Condition {
            trigger: Trigger::Unknown,
            while_holds: true,
        };
        assert!(!never.acts(&inside, 3));
    }

    #[test]
    fn resolution_does_not_depend_on_order() {
        let at = Pos::new(0, 0);
        let cs = [claim(12, 100, 10), claim(-12, 200, 50), claim(6, 300, 70)];
        let a = resolve(&[&cs[0], &cs[1], &cs[2]], at);
        let b = resolve(&[&cs[2], &cs[0], &cs[1]], at);
        assert_eq!(a, b);
    }

    #[test]
    fn scraping_moves_the_live_layer_and_layers_only_grow() {
        let w = Writing {
            surfaces: vec![Surface {
                structure: 0,
                room: Some(0),
                feature: Some(0),
                layers: vec![1, 2, 3],
            }],
            ..Writing::default()
        };
        let mut scraped: BTreeSet<usize> = [2].into();
        assert_eq!(w.live(0, &scraped), Some(2));
        assert_eq!(w.visible(0, &scraped), vec![(2, true), (3, false)]);
        assert_eq!(w.ghosts(0, &scraped), 1);
        assert_eq!(w.top_unscraped(0, &scraped), Some(3));
        scraped.insert(3);
        assert_eq!(w.live(0, &scraped), Some(3));
        assert_eq!(w.top_unscraped(0, &scraped), None);
        assert_eq!(w.surfaces[0].layers, vec![1, 2, 3]);
    }
}
