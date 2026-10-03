//! Finding the way inside great interiors and caves (D04): windows onto
//! places not yet reached, hidden ways found by looking closely, marks
//! the player scratches, spaces named and walked back to by the ways
//! known, and long passages followed in one go.

use std::collections::{BTreeMap, VecDeque};

use scraped_content::Value;
use scraped_sim::outdoors::label;
use scraped_world::structures::{Passage, PassageState};

use crate::attention::{Fact, Response};
use crate::parser::Candidate;
use crate::site::{ctx, Place, Way};
use crate::{Game, Output, Target};

/// Spaces a passage run goes through without stopping.
const PASSAGES: [&str; 6] = ["corridor", "tunnel", "passage", "crawl", "gallery", "stair"];

impl Game {
    /// Whether the player can go through a way from this side.
    fn passable(&self, w: &Way) -> bool {
        w.passage != Some(Passage::Window)
            && !w.collapsed
            && w.state == PassageState::Open
            && !w.against
            && !self.flooded(w.to)
            && !self.under_water(w.to)
    }

    /// Facts of a room inside (D04): what is seen through its windows, a
    /// mark the player left, and on looking closer, any hidden way.
    pub(crate) fn interior_facts(&mut self, response: Response, out: &mut Vec<Fact>) {
        let Place::Room { structure, room } = self.state.place else {
            return;
        };
        if self.site.structure(structure).interior.rooms[room].air == "foul" {
            out.push(
                Fact::new(
                    "hazard.air",
                    format!("air:{structure}:{room}"),
                    85.0,
                    Default::default(),
                )
                .interrupting(),
            );
        }
        if self.state.marks.contains(&(structure, room)) {
            out.push(
                Fact::new(
                    "room.mark",
                    format!("mark:{structure}:{room}"),
                    75.0,
                    Default::default(),
                )
                .anchored(),
            );
        }
        for w in self.ways() {
            if w.passage != Some(Passage::Window) {
                continue;
            }
            let Place::Room { room: other, .. } = w.to else {
                continue;
            };
            let r = &self.site.structure(structure).interior.rooms[other];
            let lit = self.site.structure(structure).interior.rooms[other].h == 0;
            out.push(Fact::new(
                "room.window",
                format!("window:{structure}:{}", w.link),
                26.0,
                ctx(&[
                    ("direction", Value::from(label(&w.exit))),
                    ("purpose", Value::from(r.purpose)),
                    ("space", Value::from(r.space)),
                    ("lit", Value::Bool(lit)),
                ]),
            ));
        }
        if response == Response::Closer {
            // Looking closely finds hidden ways.
            let hidden: Vec<Way> = self
                .site
                .ways(self.state.place)
                .into_iter()
                .filter(|w| w.hidden && !self.state.revealed.contains(&(structure, w.link)))
                .collect();
            for w in hidden {
                self.state.revealed.insert((structure, w.link));
                out.push(
                    Fact::new(
                        "room.found",
                        format!("found:{structure}:{}", w.link),
                        90.0,
                        ctx(&[
                            (
                                "passage",
                                Value::from(w.passage.map_or("panel".to_string(), |p| label(&p))),
                            ),
                            ("direction", Value::from(label(&w.exit))),
                        ]),
                    )
                    .interrupting(),
                );
            }
        }
    }

    /// Foul air hurts the longer one stays in it (D04): every 20 minutes.
    // DESIGN-Q: a level of injury per 20 minutes in foul air.
    pub(crate) fn breathe(&mut self) {
        let foul = match self.state.place {
            Place::Room { structure, room } => {
                self.site.structure(structure).interior.rooms[room].air == "foul"
            }
            Place::Outside => false,
        };
        let now = self.state.minutes;
        match (foul, self.state.foul_since) {
            (false, _) => self.state.foul_since = None,
            (true, None) => self.state.foul_since = Some(now),
            (true, Some(t)) if now >= t + 20 => {
                self.state.foul_since = Some(now);
                let t = self.say("hazard.air_hurt", Default::default());
                self.notes.push(t);
                self.hurt(1, "bad air");
            }
            _ => {}
        }
    }

    /// For samples and tests: the player at the entrance of the world's
    /// largest great interior, with a lamp, oil and a firesteel.
    pub fn begin_inside_great(&mut self) {
        let Some(&(i, _)) = self
            .site
            .world
            .greats
            .iter()
            .max_by_key(|(i, _)| self.site.world.structures[*i].interior.rooms.len())
        else {
            return;
        };
        for kind in ["lamp", "oil", "firesteel"] {
            self.make_item(kind, true);
        }
        self.state.place = Place::Room {
            structure: i,
            room: 0,
        };
        self.state.pos = self.site.land.structure_pos[i];
    }

    /// `mark`: a scratch on the wall, to know this space again.
    pub(crate) fn mark_here(&mut self) -> Output {
        let Place::Room { structure, room } = self.state.place else {
            let t = self.say("say.not_here", ctx(&[("words", Value::from("wall"))]));
            return self.output(vec![t], None);
        };
        self.pass(1);
        self.state.marks.insert((structure, room));
        let t = self.say("mark.done", Default::default());
        self.output(vec![t], None)
    }

    /// A known space's name, as the player would call it.
    pub(crate) fn room_name(&mut self, structure: usize, room: usize) -> String {
        let r = self.site.structure(structure).interior.rooms[room].clone();
        let c = ctx(&[
            ("purpose", Value::from(r.purpose)),
            ("space", Value::from(r.space)),
            ("size", Value::from(crate::quiet_slots::size_band(r.area()))),
            ("style", Value::from(r.style)),
            ("level", Value::Number(i64::from(r.level))),
            ("landmark", Value::from(r.landmark)),
            (
                "marked",
                Value::Bool(self.state.marks.contains(&(structure, room))),
            ),
        ]);
        let key = 20_000_000
            + (structure as u64) * 4096
            + room as u64
            + if self.state.marks.contains(&(structure, room)) {
                1 << 40
            } else {
                0
            };
        self.stable("room.name", c, key)
    }

    /// Spaces of this building the player has been in, to go back to.
    pub(crate) fn room_targets(&mut self) -> Vec<Candidate<Target>> {
        let Place::Room { structure, room } = self.state.place else {
            return Vec::new();
        };
        let known: Vec<usize> = self
            .state
            .rooms_seen
            .iter()
            .filter(|&&(s, r)| s == structure && r != room)
            .map(|&(_, r)| r)
            .collect();
        let mut out = Vec::new();
        for r in known {
            let name = self.room_name(structure, r);
            let purpose = self.site.structure(structure).interior.rooms[r].purpose;
            out.push(Candidate::new(Target::Room(r), &name, &[]).loosely(&[purpose]));
        }
        out
    }

    /// The shortest way through known spaces and known, passable ways.
    fn route_inside(&self, structure: usize, from: usize, to: usize) -> Option<Vec<Way>> {
        let mut prev: BTreeMap<usize, Way> = BTreeMap::new();
        let mut q = VecDeque::from([from]);
        let mut seen = std::collections::BTreeSet::from([from]);
        while let Some(r) = q.pop_front() {
            if r == to {
                break;
            }
            for w in self.ways_at(Place::Room { structure, room: r }) {
                let Place::Room { room: n, .. } = w.to else {
                    continue;
                };
                if seen.contains(&n)
                    || !self.passable(&w)
                    || !self.state.rooms_seen.contains(&(structure, n))
                {
                    continue;
                }
                seen.insert(n);
                prev.insert(n, w);
                q.push_back(n);
            }
        }
        if !seen.contains(&to) {
            return None;
        }
        let mut path = Vec::new();
        let mut at = to;
        while at != from {
            let w = prev[&at];
            path.push(w);
            let l = &self.site.structure(structure).interior.links[w.link];
            at = if l.a == at { l.b } else { l.a };
        }
        path.reverse();
        Some(path)
    }

    /// `go to the vaulted hall`: back to a known space by known ways.
    pub(crate) fn go_room(&mut self, to: usize) -> Output {
        let Place::Room { structure, room } = self.state.place else {
            let t = self.say("say.not_here", ctx(&[("words", Value::from(""))]));
            return self.output(vec![t], None);
        };
        let name = self.room_name(structure, to);
        let Some(path) = self.route_inside(structure, room, to) else {
            let t = self.say("move.no_way", ctx(&[("name", Value::from(name))]));
            return self.output(vec![t], None);
        };
        let metres: f64 = path.iter().map(|w| w.metres).sum();
        let pace = if self.is_dark() { 30.0 } else { 70.0 };
        self.pass((metres / pace).ceil().max(1.0) as u32);
        self.state.place = Place::Room {
            structure,
            room: to,
        };
        let t = self.say(
            "move.walk",
            ctx(&[
                ("name", Value::from(name)),
                ("spaces", Value::Number(path.len() as i64)),
                ("metres", Value::Number((metres / 2.0).round() as i64 * 2)),
            ]),
        );
        self.said_already = 1;
        let look = self.look();
        self.output(vec![t, look], None)
    }

    /// `follow the passage`: on through passage after passage, keeping
    /// straight on at side ways, until it opens into a room, ends, or
    /// turns twice at a fork.
    pub(crate) fn follow_passage(&mut self) -> Output {
        let Place::Room {
            structure,
            mut room,
        } = self.state.place
        else {
            let t = self.say("say.not_here", ctx(&[("words", Value::from("passage"))]));
            return self.output(vec![t], None);
        };
        let mut came: Option<usize> = self.last_link;
        let mut heading = None;
        let (mut spaces, mut metres, mut turns, mut branches) = (0, 0.0, 0, 0);
        loop {
            let space = self.site.structure(structure).interior.rooms[room].space;
            if spaces > 0 && !PASSAGES.contains(&space) {
                break;
            }
            let onward: Vec<Way> = self
                .ways_at(Place::Room { structure, room })
                .into_iter()
                .filter(|w| self.passable(w) && Some(w.link) != came)
                .collect();
            if onward.is_empty() || spaces >= 12 {
                break;
            }
            let next = match heading {
                Some(h) => onward
                    .iter()
                    .find(|w| w.exit == h)
                    .or(if onward.len() == 1 {
                        onward.first()
                    } else {
                        None
                    }),
                None => onward.first(),
            };
            let Some(&w) = next else { break };
            if onward.len() > 1 {
                branches += onward.len() - 1;
            }
            if heading.is_some_and(|h| h != w.exit) {
                turns += 1;
            }
            heading = Some(w.exit);
            came = Some(w.link);
            metres += w.metres;
            spaces += 1;
            let Place::Room { room: n, .. } = w.to else {
                break;
            };
            room = n;
            self.state.rooms_seen.insert((structure, room));
        }
        if spaces == 0 {
            let t = self.say("say.no_exit", ctx(&[("direction", Value::from("on"))]));
            return self.output(vec![t], None);
        }
        let pace = if self.is_dark() { 30.0 } else { 70.0 };
        self.pass((metres / pace).ceil().max(1.0) as u32);
        self.state.place = Place::Room { structure, room };
        self.last_link = came;
        let t = self.say(
            "move.run",
            ctx(&[
                ("spaces", Value::Number(spaces as i64)),
                ("metres", Value::Number((metres / 2.0).round() as i64 * 2)),
                ("turns", Value::Number(turns)),
                ("branches", Value::Number(branches as i64)),
                (
                    "direction",
                    Value::from(heading.map_or("north".to_string(), |h| label(&h))),
                ),
            ]),
        );
        self.said_already = 1;
        let look = self.look();
        self.output(vec![t, look], None)
    }
}
