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
    Argument, Clause, Head, Mood, NounPhrase, Polarity, Role, Sentence, Tense,
};
use scraped_world::history::{EventKind, Role as PersonRole};
use scraped_world::structures::{Material, Passage, PassageState};
use scraped_world::texts::Text;
use scraped_world::World;

use crate::fixtures::{Fixtures, Spot};
use crate::outdoors::{hash, Land, Pos, CELL};

/// What kind of thing a claim's subject names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Passage,
    Room,
    Land,
    Structure,
}

/// A property claims push.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Property {
    Openness,
    Heat,
    Stability,
}

#[derive(Debug, Clone, Deserialize)]
struct SubjectRow {
    id: String,
    class: Class,
}

#[derive(Debug, Clone, Deserialize)]
struct EffectRow {
    verb: String,
    class: Class,
    property: Property,
    amount: i32,
}

#[derive(Deserialize)]
struct ClaimFile {
    subject: Vec<SubjectRow>,
    effect: Vec<EffectRow>,
}

/// The concept-to-property table (`data/claims.toml`).
pub struct Table {
    subjects: Vec<SubjectRow>,
    effects: Vec<EffectRow>,
}

impl Table {
    pub fn get() -> &'static Table {
        static T: OnceLock<Table> = OnceLock::new();
        T.get_or_init(|| {
            let f: ClaimFile = toml::from_str(include_str!("../data/claims.toml"))
                .expect("data/claims.toml is valid");
            Table {
                subjects: f.subject,
                effects: f.effect,
            }
        })
    }

    pub fn class(&self, subject: &str) -> Option<Class> {
        self.subjects
            .iter()
            .find(|s| s.id == subject)
            .map(|s| s.class)
    }

    /// What a claim does: property and signed amount, or None for a vague
    /// claim the world cannot make true.
    pub fn effect(
        &self,
        verb: &str,
        subject: &str,
        negative: bool,
    ) -> Option<(Class, Property, i32)> {
        let class = self.class(subject)?;
        let row = self
            .effects
            .iter()
            .find(|e| e.verb == verb && e.class == class)?;
        Some((
            class,
            row.property,
            if negative { -row.amount } else { row.amount },
        ))
    }
}

/// A claim acting on the world.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Claim {
    /// The text it comes from.
    pub text: usize,
    pub verb: String,
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
    /// The first unscraped potent inscription a player is likely to meet,
    /// near the scraping tool.
    pub pivot: Option<usize>,
    /// The root inscription: the world's first great writing.
    pub root: Option<usize>,
    /// The deepest stack's accounts beneath the root, oldest first: how
    /// the world came to this, and the words for leaving it (M11). Only the
    /// strongest lens reads them.
    pub deep: Vec<usize>,
    /// A previous run's final inscription, carried into this world as a
    /// faint, very old layer (legacy, M11).
    pub legacy: Option<usize>,
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
    /// Surfaces from history, plus a few latent potent inscriptions: the
    /// pivot beside the scraping tool and some further afield.
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
        // History's potent writing was cast: it lies scraped, and acts.
        for t in &w.texts {
            if t.kind == Kind::Potent {
                out.scraped.insert(t.id);
            }
        }
        // Latent inscriptions.
        // DESIGN-Q: one pivot inscription with the scraping tool, plus four
        // latent ones further from the start than it, in the newest era's
        // language, written after everything else on their surface.
        let era = (w.languages.len() as u32).saturating_sub(1);
        let year = w.history.eras.last().map_or(0, |e| e.end);
        let start_pos = Pos::of_cell(
            w.history.settlements[start].cell.ux(),
            w.history.settlements[start].cell.uy(),
        );
        let scraper = fixtures
            .items
            .iter()
            .find(|p| p.kind == "scraper")
            .map(|p| p.at);
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
        if let Some(Spot::Room { structure, room }) = scraper {
            let reach_rooms = fixtures.reachable_rooms(w, structure);
            let st = &w.structures[structure];
            // A surface in the tool's room if possible, else the nearest
            // reachable room with one.
            let mut spot = None;
            for r in std::iter::once(room).chain(reach_rooms.iter().copied()) {
                if let Some(f) = st.interior.rooms[r]
                    .features
                    .iter()
                    .position(|f| INSCRIBABLE.contains(&f.kind))
                {
                    spot = Some((r, f));
                    break;
                }
            }
            if let Some((r, f)) = spot {
                // Visible, safe and unambiguous: a shut door nearby swings
                // open, or else the rooms grow warm.
                let door = st
                    .interior
                    .links
                    .iter()
                    .any(|l| l.passage == Passage::Door && l.state == PassageState::Closed);
                let (verb, subject) = if door {
                    ("open", "door")
                } else {
                    ("burn", "house")
                };
                out.pivot = Some(add(&mut out, structure, r, f, verb, subject, false));
            }
        }
        let pivot_dist = scraper.and_then(|s| match s {
            Spot::Room { structure, .. } => Some(land.structure_pos[structure].dist(start_pos)),
            _ => None,
        });
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
            if st.settlement == Some(start) || pivot_dist.is_some_and(|d| far <= d) {
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
        // Stack every text on its surface, oldest first.
        let all = w.texts.iter().chain(out.extra.iter());
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
        if id < w.texts.len() {
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
    let (verb, subject, negative) = claim_parts(t)?;
    let (class, property, amount) = Table::get().effect(&verb, &subject, negative)?;
    // History's casts reach as far as their event says (the root's only to
    // its own surroundings until M10); new releases by surface.
    // DESIGN-Q: the root inscription acts within 900 m until M10's great
    // inscriptions.
    let range = match t.event.map(|e| &w.history.events[e].kind) {
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
    Some(Claim {
        text: id,
        verb,
        subject,
        negative,
        class,
        property,
        amount,
        pos: land.structure_pos[t.structure],
        range,
        year: t.year,
        structure: t.structure,
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

/// Spoiler view: every surface's stack, the claims acting now, and why each
/// settlement is strange.
pub fn debug(w: &World, land: &Land, writing: &Writing, scraped: &BTreeSet<usize>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let claims = writing.live_claims(w, land, scraped);
    let _ = writeln!(out, "LIVE CLAIMS ({})", claims.len()); // DEBUG-TEXT
    for c in &claims {
        let st = &w.structures[c.structure];
        let _ = writeln!(
            out,
            "  text {:>4}  {}{} {:<7} → {:?} {:?} {:+}  range {:.0} m  at {:?} {} (year {})", // DEBUG-TEXT
            c.text,
            if c.negative { "not " } else { "" },
            c.verb,
            c.subject,
            c.class,
            c.property,
            c.amount,
            c.range,
            st.kind,
            st.id,
            c.year
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
                let pivot = if writing.pivot == Some(t) {
                    " PIVOT"
                } else {
                    ""
                }; // DEBUG-TEXT
                format!("{t}:era{} {:?} {state}{pivot}", x.era, x.kind) // DEBUG-TEXT
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
