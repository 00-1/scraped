//! Objects with histories in play (D05): examined, they show what they
//! are, what they're made of, how they've fared and whose emblem they
//! carry; looked at closer, a maker's mark, wear and mending, and the
//! border style of their era.

use scraped_content::{Context, Value};
use scraped_sim::outdoors::{bearing, distance_band, Pos, BEARINGS};
use scraped_world::objects::{emblem, Emblem, Opens, Owner};

use crate::attention::Fact;
use crate::site::{ctx, Place};
use crate::{Game, Output};

impl Game {
    /// An emblem in words, through `emblem.describe`.
    pub(crate) fn emblem_words(&mut self, e: Emblem) -> String {
        let c = ctx(&[
            ("motif", Value::from(e.motif)),
            ("device", Value::from(e.device)),
            ("border", Value::from(e.border)),
        ]);
        let key = 30_000_000
            + scraped_world::objects::MOTIFS
                .iter()
                .position(|m| *m == e.motif)
                .unwrap_or(0) as u64
                * 64
            + scraped_world::objects::DEVICES
                .iter()
                .position(|d| *d == e.device)
                .unwrap_or(0) as u64
                * 8
            + scraped_world::objects::BORDERS
                .iter()
                .position(|b| *b == e.border)
                .unwrap_or(0) as u64;
        self.stable("emblem.describe", c, key)
    }

    /// `examine` an object: the first look, or (`closer`) a closer one.
    pub(crate) fn examine_object(&mut self, t: usize, closer: bool) -> Output {
        let o = self.site.world.objects[self.thing(t).object.expect("an object")].clone();
        let seed = self.site.world.seed;
        self.pass(if closer { 3 } else { 1 });
        let owner = match o.owner {
            Owner::Faction(_) => "people",
            Owner::Family(_) => "family",
            Owner::Temple(_) => "temple",
            Owner::Era(_) => "era",
        };
        let em = if o.marked {
            let e = emblem(seed, o.owner);
            self.emblem_words(e)
        } else {
            String::new()
        };
        let mut c: Context = ctx(&[
            ("kind", Value::from(o.kind)),
            ("family", Value::from(family_id(o.family))),
            ("material", Value::from(o.stuff)),
            ("condition", Value::from(o.condition)),
            ("emblem", Value::from(em)),
            ("owner", Value::from(if o.marked { owner } else { "" })),
            ("left", Value::Bool(o.event.is_some())),
        ]);
        if !closer {
            let tx = self.say("object.examine", c);
            return self.output(vec![tx], None);
        }
        let maker = match o.maker {
            Some(m) => {
                let e = emblem(seed, Owner::Family(m));
                self.emblem_words(e)
            }
            None => String::new(),
        };
        c.insert("maker".into(), Value::from(maker));
        c.insert(
            "border".into(),
            Value::from(emblem(seed, Owner::Era(o.era)).border),
        );
        let tx = self.say("object.closer", c);
        self.output(vec![tx], None)
    }
}

/// A family's id, as content sees it.
pub(crate) fn family_id(f: scraped_world::objects::Family) -> String {
    serde_json::to_value(f)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

impl Game {
    /// Whether a thing can be seen where it lies: not inside a container
    /// not yet opened, not hidden and not yet found (D05).
    pub(crate) fn in_sight(&self, t: usize) -> bool {
        let Some(o) = self.thing(t).object else {
            return true;
        };
        let o = &self.site.world.objects[o];
        o.inside.is_none_or(|c| self.state.opened.contains(&c))
            && (o.cache.is_empty() || self.state.uncovered.contains(&o.id))
    }

    /// The thing that is a given object.
    pub(crate) fn object_thing(&self, o: usize) -> Option<usize> {
        self.site.things.iter().position(|t| t.object == Some(o))
    }

    /// Whether the player carries the key that opens this.
    fn carrying_key(&self, opens: Opens) -> bool {
        self.state.carried.iter().any(|&t| {
            self.thing(t)
                .object
                .is_some_and(|o| self.site.world.objects[o].opens == Some(opens))
        })
    }

    /// `open the casket`: a container shows what it holds; a locked one
    /// needs its key (the lock carries the owner's emblem, and so does the
    /// key).
    pub(crate) fn open_object(&mut self, t: usize) -> Output {
        let name = self.thing_name(t);
        let Some(id) = self.thing(t).object else {
            let tx = self.say("object.not_open", ctx(&[("thing", Value::from(name))]));
            return self.output(vec![tx], None);
        };
        let o = self.site.world.objects[id].clone();
        let holds = self.site.world.objects.iter().any(|x| x.inside == Some(id))
            || o.family == scraped_world::objects::Family::Box;
        if !holds {
            let tx = self.say("object.not_open", ctx(&[("thing", Value::from(name))]));
            return self.output(vec![tx], None);
        }
        let mut parts = Vec::new();
        if o.key.is_some() && !self.state.opened.contains(&id) {
            if !self.carrying_key(Opens::Object { id }) {
                let em = self.emblem_words(emblem(self.site.world.seed, o.owner));
                let tx = self.say(
                    "object.locked",
                    ctx(&[("thing", Value::from(name)), ("emblem", Value::from(em))]),
                );
                return self.output(vec![tx], None);
            }
            parts.push(self.say(
                "object.unlocked",
                ctx(&[("thing", Value::from(name.clone()))]),
            ));
        }
        self.pass(1);
        self.state.opened.insert(id);
        let inside: Vec<usize> = self
            .site
            .world
            .objects
            .iter()
            .filter(|x| x.inside == Some(id))
            .map(|x| x.id)
            .collect();
        let things: Vec<usize> = inside
            .iter()
            .filter_map(|&x| self.object_thing(x))
            .filter(|&th| self.where_is(th) == self.where_is(t) || self.state.carried.contains(&t))
            .collect();
        let names: Vec<Value> = things
            .into_iter()
            .map(|th| Value::from(self.thing_name(th)))
            .collect();
        let tx = self.say(
            "object.opened",
            ctx(&[
                ("thing", Value::from(name)),
                ("count", Value::Number(names.len() as i64)),
                ("contents", Value::List(names)),
            ]),
        );
        parts.push(tx);
        self.output(parts, None)
    }

    /// A locked door here: unlocked with its key, else it won't open.
    /// Returns whether it is (still) locked.
    pub(crate) fn locked_door(&mut self, structure: usize, link: usize) -> Option<bool> {
        let lock = *self
            .site
            .world
            .locks
            .iter()
            .find(|l| l.structure == structure && l.link == link)?;
        if self.state.unlocked.contains(&(structure, link)) {
            return Some(false);
        }
        if self.carrying_key(Opens::Door { structure, link }) {
            self.state.unlocked.insert((structure, link));
            let _ = lock;
            return Some(false);
        }
        Some(true)
    }

    /// Caches in this room: on looking closer they are found; until then,
    /// a sign of them (the owner's emblem cut in the floor or wall) can be
    /// noticed (D05).
    pub(crate) fn cache_facts(&mut self, closer: bool, out: &mut Vec<Fact>) {
        let Place::Room { structure, room } = self.state.place else {
            return;
        };
        let here: Vec<scraped_world::objects::Object> = self
            .site
            .world
            .objects
            .iter()
            .filter(|o| {
                o.structure == structure
                    && o.room == room
                    && o.feature.is_none()
                    && !o.cache.is_empty()
                    && !self.state.uncovered.contains(&o.id)
            })
            .cloned()
            .collect();
        for o in here {
            let em = self.emblem_words(emblem(self.site.world.seed, o.owner));
            let c = ctx(&[("place", Value::from(o.cache)), ("emblem", Value::from(em))]);
            if closer {
                self.state.uncovered.insert(o.id);
                let mut c = c;
                let name = self
                    .object_thing(o.id)
                    .map(|t| self.thing_name(t))
                    .unwrap_or_default();
                c.insert("thing".into(), Value::from(name));
                out.push(
                    Fact::new("cache.found", format!("cache:{}", o.id), 90.0, c).interrupting(),
                );
            } else {
                out.push(Fact::new(
                    "cache.sign",
                    format!("cachesign:{}", o.id),
                    22.0,
                    c,
                ));
            }
        }
    }

    /// `dig`: out by a feature, whatever was buried there comes up (D05).
    // DESIGN-Q: digging needs no tool and takes half an hour.
    pub(crate) fn dig(&mut self) -> Output {
        if self.state.place != Place::Outside {
            let tx = self.say("dig.nothing", ctx(&[("indoors", Value::Bool(true))]));
            return self.output(vec![tx], None);
        }
        self.pass(30);
        let here = self.state.pos;
        let found: Vec<usize> = self
            .site
            .world
            .objects
            .iter()
            .filter(|o| o.cache == "buried" && !self.state.uncovered.contains(&o.id))
            .filter(|o| {
                o.feature.is_some_and(|f| {
                    let c = self.site.world.features[f].cell;
                    scraped_sim::outdoors::Pos::of_cell(c.ux(), c.uy()).dist(here)
                        <= f64::from(scraped_sim::outdoors::LOCAL)
                })
            })
            .map(|o| o.id)
            .collect();
        if found.is_empty() {
            let tx = self.say("dig.nothing", ctx(&[("indoors", Value::Bool(false))]));
            return self.output(vec![tx], None);
        }
        let mut names = Vec::new();
        for o in found {
            self.state.uncovered.insert(o);
            if let Some(t) = self.object_thing(o) {
                // It comes up where the player stands.
                self.state.dropped.insert(t, here);
                names.push(Value::from(self.thing_name(t)));
            }
        }
        let tx = self.say("dig.found", ctx(&[("things", Value::List(names))]));
        self.output(vec![tx], None)
    }

    /// `read the map`: an old map, as the land was in its era around the
    /// town it was drawn for: the towns of then (some gone now, none of
    /// those founded later), and a cross (D05).
    pub(crate) fn read_map(&mut self, t: usize) -> Output {
        let o = self.site.world.objects[self.thing(t).object.expect("an object")].clone();
        let m = o.map.expect("a map");
        self.pass(5);
        let h = &self.site.world.history;
        let era = &h.eras[m.era as usize];
        let centre = h.settlements[m.centre].cell;
        let at = |c: scraped_world::history::Cell| Pos::of_cell(c.ux(), c.uy());
        let from = at(centre);
        // Towns standing in the map's era, nearest first.
        let mut towns: Vec<(f64, usize)> = h
            .settlements
            .iter()
            .filter(|s| s.id != m.centre && s.founded <= era.end)
            .filter(|s| s.abandoned.is_none_or(|a| a >= era.start))
            .map(|s| (at(s.cell).dist(from), s.id))
            .filter(|(d, _)| *d <= 15_000.0)
            .collect();
        towns.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let seen: Vec<(f64, &'static str, bool)> = towns
            .iter()
            .take(6)
            .map(|&(d, s)| {
                let b = bearing(from, at(h.settlements[s].cell)).map_or("here", |b| BEARINGS[b]);
                (d, b, h.settlements[s].abandoned.is_some())
            })
            .collect();
        let mut places = Vec::new();
        for (d, b, gone) in seen {
            let c = ctx(&[
                ("what", Value::from("town")),
                ("bearing", Value::from(b)),
                ("distance", Value::from(distance_band(d))),
                ("gone", Value::Bool(gone)),
            ]);
            places.push(Value::from(self.say("map.place", c)));
        }
        let cross = m
            .cross
            .and_then(|c| self.site.world.objects[c].feature)
            .map(|f| {
                let p = at(self.site.world.features[f].cell);
                (
                    bearing(from, p).map_or("here", |b| BEARINGS[b]),
                    distance_band(p.dist(from)),
                    self.site.world.features[f].kind,
                )
            });
        let (cb, cd, ck) = cross.unwrap_or(("here", "near", ""));
        let c = ctx(&[
            ("places", Value::List(places)),
            ("cross", Value::Bool(cross.is_some())),
            ("cross_bearing", Value::from(cb)),
            ("cross_distance", Value::from(cd)),
            ("cross_by", Value::from(ck)),
            (
                "border",
                Value::from(emblem(self.site.world.seed, Owner::Era(m.era)).border),
            ),
        ]);
        let tx = self.say("map.read", c);
        self.output(vec![tx], None)
    }
}
