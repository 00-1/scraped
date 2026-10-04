//! Places with character, as the player meets them (D03): the part of a
//! town they stand in and the streets on to the rest, the town's layout
//! when they look around, natural features close by, and scenes. All of
//! it goes through the attention model as candidate facts; most of it is
//! found by walking about and looking closer.

use scraped_content::Value;
use scraped_sim::outdoors::{bearing, distance_band, label, Pos, BEARINGS, LOCAL};
use scraped_world::features::kind as feature_kind;
use scraped_world::scenes::{parts, SceneAt};

use crate::attention::{Fact, Response};
use crate::parser::Candidate;
use crate::site::{ctx, Place};
use crate::{Game, Output, Target};

/// How far off a feature is noticed when looking around (metres).
// DESIGN-Q: features within 1.5 km show on `look around`; within 300 m
// they are noticed on arriving.
const AROUND: f64 = 1500.0;

impl Game {
    /// The town underfoot and the district of it the player stands in.
    pub(crate) fn district_here(&self) -> Option<(usize, usize)> {
        let t = self.town_here()?;
        let town = self.site.world.towns.get(t)?;
        let (x, y) = self.state.pos.cell();
        let d = town.district_at(scraped_world::history::Cell::new(x, y));
        Some((t, d))
    }

    /// Where a district is, for walking to it.
    fn district_pos(&self, t: usize, d: usize) -> Pos {
        let c = self.site.world.towns[t].districts[d].cell;
        // The square is the town's own spot (where journeys arrive).
        if d == 0 {
            self.site.town_pos(t)
        } else {
            Pos::of_cell(c.ux(), c.uy())
        }
    }

    /// Districts a street leads to from this one.
    fn streets_from(&self, t: usize, d: usize) -> Vec<usize> {
        let town = &self.site.world.towns[t];
        town.streets
            .iter()
            .filter_map(|s| {
                if s.from == d {
                    Some(s.to)
                } else if s.to == d {
                    Some(s.from)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn district_name(&mut self, t: usize, d: usize) -> String {
        let kind = self.site.world.towns[t].districts[d].kind;
        self.stable(
            "place.district_name",
            ctx(&[("kind", Value::from(kind))]),
            9_000_000 + (t * 64 + d) as u64,
        )
    }

    pub(crate) fn feature_name(&mut self, f: usize) -> String {
        let kind = self.site.world.features[f].kind;
        self.stable(
            "land.feature_name",
            ctx(&[("kind", Value::from(kind))]),
            9_500_000 + f as u64,
        )
    }

    fn feature_pos(&self, f: usize) -> Pos {
        let c = self.site.world.features[f].cell;
        Pos::of_cell(c.ux(), c.uy())
    }

    /// Features within `range` metres, nearest first.
    fn features_near(&self, range: f64) -> Vec<(usize, f64)> {
        let p = self.state.pos;
        let mut v: Vec<(usize, f64)> = self
            .site
            .world
            .features
            .iter()
            .map(|f| (f.id, self.feature_pos(f.id).dist(p)))
            .filter(|(_, d)| *d <= range)
            .collect();
        v.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        v
    }

    /// The facts of places with character outdoors: the district and the
    /// town's layout, features, and scenes in the open.
    pub(crate) fn place_facts(&mut self, response: Response, out: &mut Vec<Fact>) {
        if let Some((t, d)) = self.district_here() {
            let town = &self.site.world.towns[t];
            let kind = town.districts[d].kind;
            let street = town
                .streets
                .iter()
                .find(|s| s.to == d && s.from == 0)
                .map_or("main street", |s| s.kind);
            let ways: Vec<Value> = self
                .streets_from(t, d)
                .into_iter()
                .map(|o| Value::from(self.site.world.towns[t].districts[o].kind))
                .collect();
            let role = town.role.id();
            out.push(Fact::new(
                "place.district",
                format!("district:{t}:{d}"),
                if d == 0 { 20.0 } else { 34.0 },
                ctx(&[
                    ("kind", Value::from(kind)),
                    ("role", Value::from(role)),
                    ("street", Value::from(street)),
                    ("ways", Value::List(ways)),
                ]),
            ));
            if response == Response::Around {
                let town = &self.site.world.towns[t];
                let districts: Vec<Value> = town
                    .districts
                    .iter()
                    .skip(1)
                    .map(|d| Value::from(d.kind))
                    .collect();
                out.push(Fact::new(
                    "place.layout",
                    format!("layout:{t}"),
                    60.0,
                    ctx(&[
                        ("role", Value::from(town.role.id())),
                        ("reason", Value::from(town.reason)),
                        ("plan", Value::from(town.plan)),
                        ("walled", Value::Bool(town.walled)),
                        ("gates", Value::Number(town.gates as i64)),
                        ("districts", Value::List(districts)),
                    ]),
                ));
            }
        }
        // Features: close by on arriving; further off on looking around.
        let range = if response == Response::Around {
            AROUND
        } else {
            f64::from(LOCAL)
        };
        let p = self.state.pos;
        // Beyond arm's reach, features that stand up from the land are
        // already landmarks in view.
        let near: Vec<(usize, f64)> = self
            .features_near(range)
            .into_iter()
            .filter(|&(f, d)| {
                d <= f64::from(LOCAL)
                    || feature_kind(self.site.world.features[f].kind).height <= 0.0
            })
            .take(4)
            .collect();
        for (f, d) in near {
            let feat = &self.site.world.features[f];
            let k = feature_kind(feat.kind);
            let here = d <= f64::from(LOCAL);
            let b = bearing(p, self.feature_pos(f)).filter(|_| d > 150.0);
            let sal = if here {
                30.0 + k.weight
            } else {
                12.0 + k.weight
            };
            out.push(Fact::new(
                "land.feature",
                format!("feature:{f}"),
                sal,
                ctx(&[
                    ("kind", Value::from(feat.kind)),
                    (
                        "plural",
                        Value::Bool(scraped_world::features::plural_name(feat.kind)),
                    ),
                    ("group", Value::from(label(&k.group))),
                    ("bearing", Value::from(b.map_or("here", |b| BEARINGS[b]))),
                    ("distance", Value::from(distance_band(d))),
                    ("rock", Value::from(feat.rock.id())),
                ]),
            ));
        }
        // Scenes in the open, close by: found more on looking closer.
        let near: Vec<usize> = self
            .site
            .world
            .scenes
            .iter()
            .filter(|s| match s.at {
                SceneAt::Outside { x, y } => {
                    Pos::of_cell(usize::from(x), usize::from(y)).dist(p) <= 220.0
                }
                _ => false,
            })
            .map(|s| s.id)
            .collect();
        for s in near {
            let f = self.scene_fact(s, "", response);
            out.push(f);
        }
    }

    /// Scenes in this room.
    pub(crate) fn room_scene_facts(&mut self, response: Response, out: &mut Vec<Fact>) {
        let Place::Room { structure, room } = self.state.place else {
            return;
        };
        let purpose = self.site.structure(structure).interior.rooms[room].purpose;
        let here: Vec<usize> = self
            .site
            .world
            .scenes
            .iter()
            .filter(|s| s.at == SceneAt::Room { structure, room })
            .map(|s| s.id)
            .collect();
        for s in here {
            let f = self.scene_fact(s, purpose, response);
            out.push(f);
        }
    }

    fn scene_fact(&self, s: usize, purpose: &str, response: Response) -> Fact {
        let sc = &self.site.world.scenes[s];
        // DESIGN-Q: scenes weigh little at first glance, so they are mostly
        // found by looking closer.
        let sal = if response == Response::Closer {
            45.0
        } else {
            14.0
        };
        Fact::new(
            "place.scene",
            format!("scene:{s}"),
            sal,
            ctx(&[
                ("kind", Value::from(sc.kind)),
                (
                    "parts",
                    Value::List(parts(sc.kind).iter().map(|p| Value::from(*p)).collect()),
                ),
                ("cause", Value::from(sc.cause)),
                ("purpose", Value::from(purpose)),
            ]),
        )
    }

    /// What the player can refer to here among places: the town's other
    /// districts, and features close by.
    pub(crate) fn place_targets(&mut self) -> Vec<Candidate<Target>> {
        let mut out = Vec::new();
        if let Some((t, _)) = self.district_here() {
            for d in 0..self.site.world.towns[t].districts.len() {
                let name = self.district_name(t, d);
                let kind = self.site.world.towns[t].districts[d].kind;
                out.push(Candidate::new(Target::District(d), &name, &[kind]));
            }
        }
        // Close by, or in view within a look around (S01: a feature seen
        // can be set out for).
        // DESIGN-Q: features within 1.5 km can be gone to by name.
        for (f, _) in self.features_near(AROUND) {
            let name = self.feature_name(f);
            let kind = self.site.world.features[f].kind;
            out.push(Candidate::new(Target::Feature(f), &name, &[kind]));
        }
        out
    }

    /// Walking to another part of town.
    pub(crate) fn go_district(&mut self, d: usize) -> Output {
        let Some((t, here)) = self.district_here() else {
            let tx = self.say("say.not_here", ctx(&[("words", Value::from(""))]));
            return self.output(vec![tx], None);
        };
        if d == here {
            let tx = self.describe(Response::Look);
            return self.output(vec![tx], None);
        }
        let to = self.district_pos(t, d);
        let minutes = (self.state.pos.dist(to) / 60.0).ceil().max(2.0) as u32;
        self.pass(minutes);
        self.state.pos = to;
        let tx = self.describe(Response::Arrival);
        self.output(vec![tx], None)
    }

    /// Looking closer at a feature.
    pub(crate) fn examine_feature(&mut self, f: usize) -> Output {
        self.pass(2);
        let feat = &self.site.world.features[f];
        let k = feature_kind(feat.kind);
        let last = self.site.world.history.eras.len() as u32;
        let c = ctx(&[
            ("kind", Value::from(feat.kind)),
            (
                "plural",
                Value::Bool(scraped_world::features::plural_name(feat.kind)),
            ),
            ("group", Value::from(label(&k.group))),
            ("rock", Value::from(feat.rock.id())),
            ("inside", Value::Bool(feat.inside.is_some())),
            ("old", Value::Bool(feat.era == Some(0) && last > 1)),
        ]);
        let tx = self.say("feature.closer", c);
        self.output(vec![tx], None)
    }

    /// Walking to a feature close by.
    pub(crate) fn go_feature(&mut self, f: usize) -> Output {
        let to = self.feature_pos(f);
        if self
            .site
            .land
            .passable(&self.site.world, to.cell().0, to.cell().1)
        {
            let minutes = (self.state.pos.dist(to) / 60.0).ceil().max(1.0) as u32;
            self.pass(minutes);
            self.state.pos = to;
        }
        self.examine_feature(f)
    }
}
