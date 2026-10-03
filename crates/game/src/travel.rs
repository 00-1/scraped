//! The open world in play: looking out over the land, and travelling across
//! it by landmark, heading, edge or remembered name.

use scraped_content::{Context, Value};

use crate::parser::{resolve, Candidate, Resolution};
use crate::site::{ctx, label, Place};
use crate::{Game, Output, Sighting, Target, Travelled};
use scraped_sim::body::Activity;
use scraped_sim::outdoors::{
    self, bearing, distance_band, duration_band, hash, parse_bearing, rotate, rough_metres,
    rough_minutes, sight_range, unit, InView, Pos, Way, ARRIVE, BEARINGS, EDGES, EYE, LOCAL, STEP,
};

/// How far `head north` goes before stopping to look, in steps.
// DESIGN-Q: a heading walks about 3 km (or until something new comes into
// view, or water bars the way).
const HEADING_STEPS: usize = 10;
/// How far one `follow` goes, in cells.
// DESIGN-Q: following an edge goes up to about 12 km per command.
const FOLLOW_CELLS: usize = 40;
/// The longest single journey, in steps.
const MAX_STEPS: usize = 400;

/// What a journey is trying to do.
enum Goal {
    To(Pos),
    Heading(usize),
    Along(Vec<Pos>),
}

impl Game {
    /// Weather, light, and how far the eye reaches, here and now.
    pub(crate) fn conditions(&self) -> (&'static str, &'static str, f64) {
        self.conditions_at(self.state.pos, self.state.minutes)
    }

    pub(crate) fn conditions_at(
        &self,
        pos: Pos,
        minutes: u32,
    ) -> (&'static str, &'static str, f64) {
        if let Some((w, l)) = self.forced {
            return (w, l, sight_range(w, l));
        }
        let w = outdoors::weather(&self.site.world, pos, minutes);
        let l = outdoors::outdoor_light(minutes);
        (w, l, sight_range(w, l))
    }

    pub(crate) fn town_here(&self) -> Option<usize> {
        self.site.land.town(&self.site.world, self.state.pos)
    }

    /// Whether a spot outdoors is part of where the player stands: the same
    /// settlement, or within a few hundred metres.
    pub(crate) fn local(&self, p: Pos) -> bool {
        match self.town_here() {
            Some(t) => self.site.land.town(&self.site.world, p) == Some(t),
            None => p.dist(self.state.pos) <= f64::from(LOCAL),
        }
    }

    /// Buildings the player can walk straight into from here.
    pub(crate) fn local_structures(&self) -> Vec<usize> {
        let town = self.town_here();
        self.site
            .world
            .structures
            .iter()
            .filter(|st| match town {
                Some(t) => st.settlement == Some(t),
                None => {
                    self.site.land.structure_pos[st.id].dist(self.state.pos) <= f64::from(LOCAL)
                }
            })
            .map(|st| st.id)
            .collect()
    }

    fn view_from(&self, pos: Pos, range: f64) -> Vec<InView> {
        let town = self.site.land.town(&self.site.world, pos);
        self.site.land.in_view(&self.site.world, pos, range, town)
    }

    pub(crate) fn in_view(&self) -> Vec<InView> {
        self.view_from(self.state.pos, self.conditions().2)
    }

    /// Marks everything in view, and the settlement underfoot, as seen.
    pub(crate) fn see_around(&mut self) {
        for v in self.in_view() {
            self.state.seen.insert(v.landmark);
        }
        if let Some(t) = self.town_here() {
            if let Some(i) = self
                .site
                .land
                .landmarks
                .iter()
                .position(|l| l.settlement == Some(t))
            {
                self.state.seen.insert(i);
            }
        }
    }

    pub(crate) fn landmark_name(&mut self, i: usize) -> String {
        let c = self.site.landmark_vars(i);
        self.stable("land.name", c, 3_000_000 + i as u64)
    }

    pub(crate) fn edge_name(&mut self, e: usize) -> String {
        let c = ctx(&[("kind", Value::from(EDGES[e]))]);
        self.stable("land.edge_name", c, 4_000_000 + e as u64)
    }

    pub(crate) fn edge_phrase(&mut self, e: usize, b: Option<usize>) -> String {
        let name = self.edge_name(e);
        let c = ctx(&[
            ("name", Value::from(name)),
            ("kind", Value::from(EDGES[e])),
            ("side", Value::from(b.map_or("here", |b| BEARINGS[b]))),
        ]);
        self.say("land.edge", c)
    }

    /// A landmark as offered in "which do you mean?": by its name, which
    /// way and how far, so alike landmarks read apart.
    pub(crate) fn landmark_choice(&mut self, i: usize) -> String {
        match self.in_view().into_iter().find(|v| v.landmark == i) {
            Some(v) => self.landmark_phrase(&v),
            None => self.landmark_name(i),
        }
    }

    pub(crate) fn landmark_phrase(&mut self, v: &InView) -> String {
        let name = self.landmark_name(v.landmark);
        let c = ctx(&[
            ("name", Value::from(name)),
            (
                "kind",
                Value::from(self.site.land.landmarks[v.landmark].kind),
            ),
            ("bearing", Value::from(BEARINGS[v.bearing])),
            ("distance", Value::from(distance_band(v.metres))),
        ]);
        self.say("land.landmark", c)
    }

    /// Things outdoors the parser can name: landmarks in view, places the
    /// player named, and edges close by.
    pub(crate) fn outdoor_targets(&mut self) -> Vec<Candidate<Target>> {
        let mut out = Vec::new();
        for v in self.in_view() {
            let name = self.landmark_name(v.landmark);
            let kind = self.site.land.landmarks[v.landmark].kind;
            let b = BEARINGS[v.bearing];
            let parts: Vec<&str> = ["north", "south", "east", "west"]
                .into_iter()
                .filter(|p| b.contains(p))
                .collect();
            // Trait words name it too, whatever words the text used.
            let t = &self.site.land.landmarks[v.landmark].traits;
            let mut extra = vec![kind, b, t.mark, t.mark2];
            extra.retain(|w| !w.is_empty());
            // Every trait names it, loosely, as do the halves of a
            // compound bearing ("west" for the north-west).
            let mut loose = parts;
            loose.extend([t.shape, t.cover, t.setting, t.condition, t.height]);
            if t.walls {
                loose.push("walled");
            }
            loose.retain(|w| !w.is_empty());
            out.push(Candidate::new(Target::Landmark(v.landmark), &name, &extra).loosely(&loose));
        }
        for (i, (name, _)) in self.state.names.clone().iter().enumerate() {
            out.push(Candidate::new(Target::Named(i), name, &["place"]));
        }
        for (e, _) in self.site.land.edges_near(self.state.pos) {
            let name = self.edge_name(e);
            out.push(Candidate::new(Target::Edge(e), &name, &[EDGES[e]]));
        }
        out
    }

    pub(crate) fn outdoor_summary(&mut self) -> (Vec<Sighting>, Vec<String>, Option<String>) {
        let view = self.in_view();
        let landmarks = view
            .iter()
            .map(|v| Sighting {
                name: self.landmark_name(v.landmark),
                bearing: BEARINGS[v.bearing].to_string(),
                distance: distance_band(v.metres).to_string(),
            })
            .collect();
        let edges = self
            .site
            .land
            .edges_near(self.state.pos)
            .into_iter()
            .map(|(e, _)| EDGES[e].to_string())
            .collect();
        let (w, l, _) = self.conditions();
        (landmarks, edges, Some(format!("{w} {l}")))
    }

    /// Examining something far off: where it lies, as the eye judges.
    pub(crate) fn describe_far(&mut self, t: Target) -> String {
        match t {
            Target::Landmark(i) => {
                let l = self.site.land.landmarks[i].pos;
                let v = InView {
                    landmark: i,
                    bearing: bearing(self.state.pos, l).unwrap_or(0),
                    metres: self.state.pos.dist(l),
                    score: 0.0,
                };
                self.landmark_phrase(&v)
            }
            Target::Edge(e) => {
                let b = self
                    .site
                    .land
                    .edges_near(self.state.pos)
                    .into_iter()
                    .find(|(k, _)| *k == e)
                    .and_then(|(_, b)| b);
                self.edge_phrase(e, b)
            }
            _ => {
                let name = self.target_name(t);
                self.say("travel.unseen", ctx(&[("words", Value::from(name))]))
            }
        }
    }

    fn outdoors_only(&mut self, verb: &str) -> Option<Output> {
        if self.state.place == Place::Outside {
            return None;
        }
        let t = self.say("travel.indoors", ctx(&[("verb", Value::from(verb))]));
        Some(self.output(vec![t], None))
    }

    pub(crate) fn go_landmark(&mut self, i: usize) -> Output {
        let l = &self.site.land.landmarks[i];
        let pos = l.pos;
        self.travel_to(pos, Some(Target::Landmark(i)), "walk")
    }

    /// `head north`, or `head for the hill`.
    pub(crate) fn head(&mut self, words: &[String]) -> Output {
        if let Some(o) = self.outdoors_only("head") {
            return o;
        }
        match words.iter().find_map(|w| parse_bearing(w)) {
            Some(b) => self.head_toward(b),
            None => {
                let rest: Vec<String> = words
                    .iter()
                    .filter(|w| !matches!(w.as_str(), "for" | "towards" | "toward"))
                    .cloned()
                    .collect();
                self.with_target("go", &rest)
            }
        }
    }

    pub(crate) fn head_toward(&mut self, b: usize) -> Output {
        if let Some(o) = self.outdoors_only("head") {
            return o;
        }
        self.journey(Goal::Heading(b), None, "head", None)
    }

    /// `follow the river downstream`, `follow road north`.
    pub(crate) fn follow(&mut self, words: &[String]) -> Output {
        if let Some(o) = self.outdoors_only("follow") {
            return o;
        }
        let mut way = Way::Onward;
        let mut rest = Vec::new();
        for w in words {
            match w.as_str() {
                "upstream" | "up" => way = Way::Upstream,
                "downstream" | "down" => way = Way::Downstream,
                _ => match parse_bearing(w) {
                    Some(b) => way = Way::Toward(b),
                    None => rest.push(w.clone()),
                },
            }
        }
        let cands: Vec<Candidate<Target>> = self
            .outdoor_targets()
            .into_iter()
            .filter(|c| matches!(c.target, Target::Edge(_)))
            .collect();
        let chosen = if rest.iter().all(|w| matches!(w.as_str(), "the" | "a")) && cands.len() == 1 {
            Resolution::One(cands[0].target)
        } else {
            resolve(&rest, &cands, self.state.it.as_ref())
        };
        match chosen {
            Resolution::One(Target::Edge(e)) => {
                self.state.it = Some(Target::Edge(e));
                self.follow_edge(e, way)
            }
            Resolution::Many(_) | Resolution::One(_) | Resolution::None => {
                let t = self.say(
                    "travel.no_edge",
                    ctx(&[("words", Value::from(rest.join(" ")))]),
                );
                self.output(vec![t], None)
            }
        }
    }

    pub(crate) fn follow_edge(&mut self, e: usize, way: Way) -> Output {
        let water = e <= 1;
        let way = match way {
            Way::Onward if water => Way::Downstream,
            Way::Upstream | Way::Downstream if !water => Way::Onward,
            w => w,
        };
        let env = self.env();
        let (path, end) = self.site.land.follow(
            &self.site.world,
            e,
            self.state.pos,
            way,
            FOLLOW_CELLS,
            &|x, y| env.obstacle(x, y),
        );
        if path.len() < 2 {
            let c = ctx(&[
                ("edge", Value::from(EDGES[e])),
                ("by", Value::from(end.unwrap_or("end"))),
            ]);
            let t = self.say("travel.edge_end", c);
            return self.output(vec![t], None);
        }
        self.journey(Goal::Along(path), None, "follow", Some((e, end)))
    }

    pub(crate) fn go_back(&mut self) -> Output {
        if let Some(o) = self.outdoors_only("back") {
            return o;
        }
        match self.state.trail.pop() {
            Some(p) => self.journey(Goal::To(p), None, "back", None),
            None => {
                let t = self.say("travel.back_none", Context::new());
                self.output(vec![t], None)
            }
        }
    }

    /// `name this place the gap`: remembered by the player, and reachable
    /// by `go to the gap` from anywhere.
    pub(crate) fn name_place(&mut self, words: &[String]) -> Output {
        if let Some(o) = self.outdoors_only("name") {
            return o;
        }
        let skip = ["this", "place", "spot", "here", "as", "it"];
        let name: Vec<&str> = words
            .iter()
            .map(String::as_str)
            .skip_while(|w| skip.contains(w))
            .collect();
        let name = name
            .join(" ")
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();
        if name.is_empty() {
            let t = self.say(
                "say.name_bad",
                ctx(&[("input", Value::from(words.join(" ")))]),
            );
            return self.output(vec![t], None);
        }
        self.state.names.retain(|(n, _)| *n != name);
        self.state.names.push((name.clone(), self.state.pos));
        let t = self.say("say.named", ctx(&[("name", Value::from(name))]));
        self.output(vec![t], None)
    }

    /// Walks to a known point: by route when it can be seen or the way is
    /// marked by landmarks, by dead reckoning (and drift) when not.
    pub(crate) fn travel_to(&mut self, goal: Pos, dest: Option<Target>, mode: &str) -> Output {
        if goal.dist(self.state.pos) <= ARRIVE {
            let name = dest.map(|d| self.target_name(d)).unwrap_or_default();
            let t = self.say("travel.already", ctx(&[("name", Value::from(name))]));
            return self.output(vec![t], None);
        }
        self.journey(Goal::To(goal), dest, mode, None)
    }

    fn journey(
        &mut self,
        goal: Goal,
        dest: Option<Target>,
        mode: &str,
        edge: Option<(usize, Option<&'static str>)>,
    ) -> Output {
        let start = self.state.pos;
        let start_minutes = self.state.minutes;
        let env = self.env();
        let route = match &goal {
            Goal::To(p) => self
                .site
                .land
                .route_by(&self.site.world, start, *p, &|x, y| env.passable(x, y)),
            _ => None,
        };
        let (waypoints, max) = match &goal {
            Goal::To(_) => match route {
                Some(path) => (path, MAX_STEPS),
                None => {
                    let name = dest.map(|d| self.target_name(d)).unwrap_or_default();
                    let t = self.say("travel.no_route", ctx(&[("name", Value::from(name))]));
                    return self.output(vec![t], None);
                }
            },
            Goal::Heading(_) => (Vec::new(), HEADING_STEPS),
            Goal::Along(path) => (path.clone(), MAX_STEPS),
        };
        if mode != "back" {
            self.state.trail.push(start);
            if self.state.trail.len() > 64 {
                self.state.trail.remove(0);
            }
        }
        self.see_around();
        let dest_landmark = match dest {
            Some(Target::Landmark(i)) => Some(i),
            _ => None,
        };
        let following = matches!(goal, Goal::Along(_));
        let mut pos = start;
        let (mut bx, mut by) = (f64::from(start.x), f64::from(start.y));
        let mut err: i32 = 0;
        let mut blind = 0;
        let mut carry = 0.0;
        let mut wi = 0;
        self.interrupted = false;
        let mut event: Option<(&str, Context)> = None;
        for step in 0..max {
            let now = self.state.minutes;
            let (weather, light, range) = self.conditions_at(pos, now);
            let view = self.view_from(pos, range);
            // DESIGN-Q: in clear daylight outside woods the sun keeps the
            // player's heading true even with no landmark in view.
            let (cx, cy) = pos.cell();
            let sun = weather == "clear"
                && light == "daylight"
                && !matches!(
                    self.site.world.terrain.biome.get(cx, cy),
                    scraped_world::terrain::Biome::Forest | scraped_world::terrain::Biome::Pine
                );
            let goal_seen = match goal {
                Goal::To(g) => self
                    .site
                    .land
                    .sight(&self.site.world, pos, EYE, g, 10.0, range),
                _ => false,
            };
            let guided = following || goal_seen || sun || !view.is_empty();
            if guided {
                // Landmarks show where the player really is.
                err = 0;
                bx = f64::from(pos.x);
                by = f64::from(pos.y);
            } else {
                blind += 1;
                err = (err
                    + (hash(&[self.seed(), u64::from(start_minutes), step as u64]) % 25) as i32
                    - 12)
                    .clamp(-90, 90);
            }
            // Where the player means to go next.
            let aim = match &goal {
                Goal::Heading(b) => {
                    let (ux, uy) = unit(*b);
                    (bx + ux * STEP, by + uy * STEP)
                }
                Goal::To(_) | Goal::Along(_) => {
                    while wi < waypoints.len()
                        && (f64::from(waypoints[wi].x) - bx).abs() < 1.0
                        && (f64::from(waypoints[wi].y) - by).abs() < 1.0
                    {
                        wi += 1;
                    }
                    let Some(w) = waypoints.get(wi) else { break };
                    (f64::from(w.x), f64::from(w.y))
                }
            };
            let (dx, dy) = (aim.0 - bx, aim.1 - by);
            let d = (dx * dx + dy * dy).sqrt();
            if d < 1.0 {
                wi += 1;
                continue;
            }
            let len = d.min(STEP);
            let intended = (dx / d, dy / d);
            let actual = rotate(intended, err);
            let next = pos.moved(actual.0 * len, actual.1 * len);
            let (nx, ny) = next.cell();
            let obstacle = self.env().obstacle(nx, ny).filter(|o| {
                // Following a river keeps to its bank.
                !(*o == "river" && edge.is_some_and(|(e, _)| self.site.land.has_edge(nx, ny, e)))
            });
            if let Some(by_what) = obstacle {
                let c = ctx(&[
                    ("by", Value::from(by_what)),
                    (
                        "bearing",
                        Value::from(bearing(pos, next).map_or("north", |b| BEARINGS[b])),
                    ),
                ]);
                event = Some(("travel.blocked", c));
                break;
            }
            carry += self.site.land.walk_minutes(&self.site.world, pos, next)
                * self.state.body.slowness();
            pos = next;
            self.state.pos = pos;
            let whole = carry.floor();
            carry -= whole;
            self.advance(whole as u32, Activity::Walking);
            if self.state.dead.is_some() || self.interrupted {
                break;
            }
            bx += intended.0 * len;
            by += intended.1 * len;
            if guided {
                bx = f64::from(pos.x);
                by = f64::from(pos.y);
            }
            if let Some(t) = self.site.land.town(&self.site.world, pos) {
                if let Some(i) = self
                    .site
                    .land
                    .landmarks
                    .iter()
                    .position(|l| l.settlement == Some(t))
                {
                    if Some(i) != dest_landmark && self.state.seen.insert(i) {
                        let name = self.landmark_name(i);
                        let c = ctx(&[
                            ("name", Value::from(name)),
                            ("kind", Value::from(self.site.land.landmarks[i].kind)),
                            (
                                "bearing",
                                Value::from(
                                    bearing(pos, self.site.land.landmarks[i].pos)
                                        .map_or("north", |b| BEARINGS[b]),
                                ),
                            ),
                            ("distance", Value::from("near")),
                        ]);
                        event = Some(("travel.interrupt", c));
                        break;
                    }
                }
            }
            // Anything new in sight stops the walk.
            let now = self.state.minutes;
            let (_, _, range) = self.conditions_at(pos, now);
            let fresh: Vec<InView> = self
                .view_from(pos, range)
                .into_iter()
                .filter(|v| !self.state.seen.contains(&v.landmark))
                .collect();
            let mut stop = None;
            for v in &fresh {
                self.state.seen.insert(v.landmark);
                let kind = self.site.land.landmarks[v.landmark].kind;
                if stop.is_none()
                    && !matches!(kind, "hill" | "mountain")
                    && Some(v.landmark) != dest_landmark
                {
                    stop = Some(v.clone());
                }
            }
            if let Some(v) = stop {
                let name = self.landmark_name(v.landmark);
                let c = ctx(&[
                    ("name", Value::from(name)),
                    (
                        "kind",
                        Value::from(self.site.land.landmarks[v.landmark].kind),
                    ),
                    ("bearing", Value::from(BEARINGS[v.bearing])),
                    ("distance", Value::from(distance_band(v.metres))),
                ]);
                event = Some(("travel.interrupt", c));
                break;
            }
        }
        if event.is_none() && self.state.dead.is_none() && !self.interrupted {
            match (&goal, edge) {
                (Goal::To(g), _) => {
                    if pos.dist(*g) <= ARRIVE {
                        pos = *g;
                        if let Some(d) = dest {
                            let name = self.target_name(d);
                            event = Some(("travel.arrive", ctx(&[("name", Value::from(name))])));
                        }
                    } else if wi >= waypoints.len() {
                        let name = dest.map(|d| self.target_name(d)).unwrap_or_default();
                        event = Some(("travel.not_there", ctx(&[("name", Value::from(name))])));
                    }
                }
                (Goal::Along(path), Some((e, Some(by_what)))) if wi >= path.len() => {
                    event = Some((
                        "travel.edge_end",
                        ctx(&[
                            ("edge", Value::from(EDGES[e])),
                            ("by", Value::from(by_what)),
                        ]),
                    ));
                }
                _ => {}
            }
        }
        let moved = pos != start;
        self.state.pos = pos;
        let total = self.state.minutes - start_minutes;
        let believed = Pos::new(bx.floor() as i32, by.floor() as i32);
        let mut parts = Vec::new();
        if moved {
            let metres = start.dist(believed);
            let b = bearing(start, believed).filter(|_| metres >= 50.0);
            self.last_travel = Some(Travelled {
                bearing: b.map_or(String::new(), |b| BEARINGS[b].to_string()),
                metres: rough_metres(metres),
                minutes: rough_minutes(total),
            });
            let c = ctx(&[
                ("mode", Value::from(mode)),
                ("bearing", Value::from(b.map_or("nowhere", |b| BEARINGS[b]))),
                ("distance", Value::from(distance_band(metres))),
                ("metres", Value::Number(rough_metres(metres))),
                ("duration", Value::from(duration_band(total))),
                ("minutes", Value::Number(rough_minutes(total))),
                ("edge", Value::from(edge.map_or("none", |(e, _)| EDGES[e]))),
            ]);
            parts.push(self.say("travel.report", c));
            if blind >= 2 {
                let (w, l, _) = self.conditions();
                let c = ctx(&[
                    ("weather", Value::from(w)),
                    ("light", Value::from(l)),
                    ("biome", Value::from(self.site.biome_at(pos))),
                ]);
                parts.push(self.say("travel.lost", c));
            }
        }
        if let Some((slot, c)) = event {
            parts.push(self.say(slot, c));
        }
        if moved && self.state.dead.is_none() {
            parts.push(self.describe(crate::attention::Response::Arrival));
        }
        let truth = serde_json::json!({
            "pos": [pos.x, pos.y],
            "cell": pos.cell(),
            "believed": [believed.x, believed.y],
            "blind_steps": blind,
            "town": self.site.land.town(&self.site.world, pos),
            "label": label(&self.site.world.terrain.biome.get(pos.cell().0, pos.cell().1)),
        });
        self.output(parts, Some(truth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraped_content::Pack;

    fn pack() -> Pack {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "toml") {
                files.push((
                    p.file_name().unwrap().to_string_lossy().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
        Pack::load(&files).0
    }

    fn game(seed: u64, forced: Option<(&'static str, &'static str)>) -> Game {
        let mut g = Game::new(seed, pack());
        g.forced = forced;
        g.start();
        g
    }

    const CLEAR: Option<(&str, &str)> = Some(("clear", "daylight"));

    #[test]
    fn sight_is_symmetric_and_blocked_by_terrain() {
        let g = game(42, CLEAR);
        let (w, land) = (&g.site.world, &g.site.land);
        let mut blocked = 0;
        for i in 0..400u64 {
            let h = hash(&[i, 1]);
            let a = Pos::new((h % 48_000) as i32, ((h >> 20) % 48_000) as i32);
            let b = Pos::new(((h >> 40) % 48_000) as i32, (hash(&[i, 2]) % 48_000) as i32);
            let ab = land.sight(w, a, EYE, b, EYE, 20_000.0);
            assert_eq!(ab, land.sight(w, b, EYE, a, EYE, 20_000.0));
            // A ridge well above the line between them hides each from the other.
            let mid = Pos::new((a.x + b.x) / 2, (a.y + b.y) / 2);
            if land.ground(mid) > land.ground(a).max(land.ground(b)) + 50.0 {
                assert!(!ab);
                blocked += 1;
            }
        }
        assert!(blocked > 0, "the test found no ridges");
    }

    /// Repeats `go` to a landmark until there, as a player would.
    fn walk_to(g: &mut Game, i: usize) -> bool {
        let goal = g.site.land.landmarks[i].pos;
        for _ in 0..30 {
            g.go_landmark(i);
            if g.state.pos == goal || g.state.pos.dist(goal) <= ARRIVE {
                return true;
            }
        }
        false
    }

    #[test]
    fn go_to_a_visible_landmark_arrives_in_clear_daylight() {
        for seed in [1, 42, 7] {
            let mut g = game(seed, CLEAR);
            let fresh = g.state.clone();
            // Look out from the start and from every landmark.
            let mut spots = vec![g.state.pos];
            spots.extend(g.site.land.landmarks.iter().map(|l| l.pos));
            let mut tried = 0;
            for from in spots.into_iter().take(12) {
                g.state = fresh.clone();
                g.state.pos = from;
                let view: Vec<usize> = g.in_view().iter().take(3).map(|v| v.landmark).collect();
                for i in view {
                    g.state = fresh.clone();
                    g.state.pos = from;
                    let goal = g.site.land.landmarks[i].pos;
                    if g.site.land.route(&g.site.world, from, goal).is_none() {
                        continue;
                    }
                    tried += 1;
                    assert!(
                        walk_to(&mut g, i),
                        "seed {seed}: never reached landmark {i}"
                    );
                }
            }
            assert!(tried >= 5, "seed {seed}: too few journeys tried");
        }
    }

    fn offset(o: &Output) -> Option<(i64, i64)> {
        let t = o.truth.as_ref()?;
        let p = (t["pos"][0].as_i64()?, t["pos"][1].as_i64()?);
        let b = (t["believed"][0].as_i64()?, t["believed"][1].as_i64()?);
        Some((p.0 - b.0, p.1 - b.1))
    }

    #[test]
    fn drift_needs_blindness_and_is_reproducible() {
        let run = |forced| {
            let mut g = game(42, forced);
            g.spoil = true;
            let mut worst = 0i64;
            for d in [
                "head north",
                "head east",
                "head south",
                "head west",
                "head north",
            ] {
                let o = g.step(d);
                if let Some((dx, dy)) = offset(&o) {
                    worst = worst.max(dx.abs() + dy.abs());
                }
            }
            (worst, g.state.pos)
        };
        let (clear, _) = run(CLEAR);
        assert_eq!(clear, 0, "no drift with the sun and landmarks");
        let (fog, at) = run(Some(("fog", "dark")));
        assert!(fog > 0, "fog at night must drift");
        assert_eq!(run(Some(("fog", "dark"))).1, at, "drift is seeded");
    }

    #[test]
    fn bearing_reports_match_geometry_in_clear_conditions() {
        let mut g = game(7, CLEAR);
        for d in [
            "head northeast",
            "head south",
            "head west",
            "follow road",
            "go back",
        ] {
            let from = g.state.pos;
            let o = g.step(d);
            let Some(t) = o.state.travelled else { continue };
            let to = g.state.pos;
            let real = from.dist(to);
            assert!(
                (t.metres as f64 - real).abs() <= 251.0,
                "{d}: {} vs {real}",
                t.metres
            );
            if real >= 50.0 {
                assert_eq!(t.bearing, BEARINGS[bearing(from, to).unwrap()], "{d}");
            }
        }
    }

    /// A surveyor that knows only what it is told: it picks where to go
    /// from what is in view, and sums travel reports (bearing and rough
    /// distance) into a map, which is checked against the truth.
    #[test]
    fn a_surveyor_can_map_the_world_from_reports() {
        let mut errors = Vec::new();
        for seed in [1, 42, 7] {
            let mut g = game(seed, CLEAR);
            let origin = g.state.pos;
            let mut est = (0.0f64, 0.0f64);
            let mut walked = 0.0;
            let mut been = std::collections::BTreeSet::new();
            let mut towns = std::collections::BTreeSet::new();
            for turn in 0..30 {
                let view = g.in_view();
                let pick = view
                    .iter()
                    .filter(|v| !been.contains(&v.landmark))
                    .filter(|v| {
                        let l = &g.site.land.landmarks[v.landmark];
                        g.site
                            .land
                            .route(&g.site.world, g.state.pos, l.pos)
                            .is_some()
                    })
                    .min_by_key(|v| {
                        (
                            g.site.land.landmarks[v.landmark].settlement.is_none(),
                            v.metres as i64,
                        )
                    });
                let before = g.state.pos;
                let o = match pick {
                    Some(v) => {
                        been.insert(v.landmark);
                        g.go_landmark(v.landmark)
                    }
                    None => g.head_toward([0, 2, 4, 6][turn % 4]),
                };
                walked += before.dist(g.state.pos);
                if let Some(t) = o.state.travelled {
                    if let Some(b) = BEARINGS.iter().position(|x| *x == t.bearing) {
                        let (ux, uy) = unit(b);
                        est.0 += ux * t.metres as f64;
                        est.1 += uy * t.metres as f64;
                    }
                }
                // Arrived in a settlement: check the surveyor's map against it.
                if let Some(t) = g.site.land.town(&g.site.world, g.state.pos) {
                    if g.state.pos == g.site.town_pos(t) && towns.insert(t) {
                        let truth = (
                            f64::from(g.state.pos.x - origin.x),
                            f64::from(g.state.pos.y - origin.y),
                        );
                        let off = ((est.0 - truth.0).powi(2) + (est.1 - truth.1).powi(2)).sqrt();
                        errors.push((seed, off, walked));
                    }
                }
            }
        }
        // Dead reckoning drifts with distance walked; a paper map is only
        // as good as that.
        assert!(
            errors.len() >= 6,
            "the surveyor reached too few settlements: {errors:?}"
        );
        let rel: Vec<f64> = errors.iter().map(|e| e.1 / e.2.max(2000.0)).collect();
        let mean = rel.iter().sum::<f64>() / rel.len() as f64;
        let worst = rel.iter().fold(0.0f64, |a, &b| a.max(b));
        assert!(
            mean <= 0.15 && worst <= 0.35,
            "map errors (seed, metres off, metres walked): {errors:?}"
        );
    }

    #[test]
    fn a_long_journey_is_fast() {
        let mut g = game(42, CLEAR);
        let start = g.state.pos;
        let far = (0..g.site.land.landmarks.len())
            .filter(|&i| {
                g.site
                    .land
                    .route(&g.site.world, start, g.site.land.landmarks[i].pos)
                    .is_some()
            })
            .max_by_key(|&i| start.dist2(g.site.land.landmarks[i].pos))
            .unwrap();
        let t = std::time::Instant::now();
        let ok = walk_to(&mut g, far);
        let took = t.elapsed();
        assert!(ok, "never arrived");
        let limit = if cfg!(debug_assertions) { 20.0 } else { 2.0 };
        assert!(took.as_secs_f64() < limit, "took {took:?}");
        eprintln!(
            "crossed {:.1} km in {took:?}",
            start.dist(g.state.pos) / 1000.0
        );
    }

    /// Land positions spread over the map, for looking around from.
    fn spots(g: &Game, n: u64) -> Vec<Pos> {
        let w = &g.site.world;
        let size = scraped_world::terrain::SIZE as u64;
        (0..n * 8)
            .map(|k| hash(&[g.seed(), 0x5907, k]))
            .map(|h| ((h % size) as usize, ((h >> 20) % size) as usize))
            .filter(|&(x, y)| w.terrain.is_land(x, y))
            .take(n as usize)
            .map(|(x, y)| Pos::of_cell(x, y))
            .collect()
    }

    #[test]
    fn alike_landmarks_in_view_read_apart() {
        for seed in 1..=10 {
            let mut g = game(seed, CLEAR);
            for p in spots(&g, 30) {
                g.state.pos = p;
                let view = g.in_view();
                let names: Vec<String> = view.iter().map(|v| g.landmark_name(v.landmark)).collect();
                let mut seen = std::collections::BTreeSet::new();
                for n in &names {
                    assert!(
                        seen.insert(n.clone()),
                        "seed {seed}: two '{n}' in view: {names:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn landmarks_read_the_same_from_everywhere() {
        let mut g = game(42, CLEAR);
        let mut names: std::collections::BTreeMap<usize, String> = Default::default();
        for p in spots(&g, 30) {
            g.state.pos = p;
            for v in g.in_view() {
                let n = g.landmark_name(v.landmark);
                assert_eq!(names.entry(v.landmark).or_insert_with(|| n.clone()), &n);
            }
        }
    }

    #[test]
    fn qualified_references_resolve() {
        use crate::parser::{resolve, tokens, Resolution};
        let mut checked = 0;
        for seed in 1..=10 {
            let mut g = game(seed, CLEAR);
            for p in spots(&g, 30) {
                g.state.pos = p;
                let view = g.in_view();
                let cands = g.outdoor_targets();
                let find = |words: &str| resolve(&tokens(words), &cands, None);
                for (a, va) in view.iter().enumerate() {
                    let la = &g.site.land.landmarks[va.landmark];
                    let alike: Vec<&InView> = view
                        .iter()
                        .filter(|v| g.site.land.landmarks[v.landmark].kind == la.kind)
                        .collect();
                    if alike.len() < 2 {
                        continue;
                    }
                    let me = Target::Landmark(va.landmark);
                    // By the trait that sets it apart.
                    let t = &la.traits;
                    let by_trait = format!("the {} {} {}", t.mark, t.mark2, la.kind);
                    assert_eq!(
                        find(&by_trait),
                        Resolution::One(me),
                        "seed {seed}: {by_trait}"
                    );
                    // By bearing, when no alike landmark shares it.
                    let b = BEARINGS[va.bearing];
                    if alike.iter().filter(|v| v.bearing == va.bearing).count() == 1 {
                        let by_way = format!("the {} to the {b}", la.kind);
                        assert_eq!(find(&by_way), Resolution::One(me), "seed {seed}: {by_way}");
                    }
                    // By ordinal, in the order they are offered.
                    let k = alike
                        .iter()
                        .position(|v| v.landmark == va.landmark)
                        .unwrap();
                    let ordinals = ["first", "second", "third", "fourth", "fifth"];
                    if k < ordinals.len() {
                        let by_order = format!("the {} {}", ordinals[k], la.kind);
                        assert_eq!(
                            find(&by_order),
                            Resolution::One(me),
                            "seed {seed}: {by_order}"
                        );
                    }
                    let _ = a;
                    checked += 1;
                }
            }
        }
        assert!(checked > 50, "too few alike landmarks to test: {checked}");
    }
}
