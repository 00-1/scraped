//! The mapper bot (D04): explores a great interior the way a careful
//! player with a notebook would, and draws a plan from what it is told.
//!
//! It goes through every way it is shown, looks closely in each space
//! (which finds hidden ways), and places each space on its plan by adding
//! up the moves the game reports (so many metres north and east, up or
//! down). The plan is then checked against the true one: how far off each
//! space's place is, against how far it lies from the entrance, and
//! whether the ways it went through join the spaces they truly join.
//! That the plan comes out true is what proves interiors can be mapped on
//! paper.
//!
//! It plays with a light that never fails and a body kept well: it
//! measures the place, not survival.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Serialize;

use scraped_content::{Pack, Value};

use crate::site::Place;
use crate::Game;

/// What the mapper found.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MapRun {
    pub structure: usize,
    /// Spaces it stood in, and all the spaces there are.
    pub visited: usize,
    pub spaces: usize,
    /// Hidden spaces it found by looking closely.
    pub hidden_found: usize,
    /// Mean and worst error of its plan, as a share of each space's
    /// distance from the entrance (at least 10 m).
    pub error_mean: f64,
    pub error_max: f64,
    /// Ways it went through whose two ends it placed as the true spaces.
    pub ways_walked: usize,
    pub topology_exact: bool,
    /// Game hours it took to see every space it could reach.
    pub hours: f64,
}

/// A space's place on the mapper's plan: metres east, metres north, level.
type Spot = (f64, f64, i8);

/// Maps one interior of a world, starting at its entrance.
pub fn map(pack: &Pack, seed: u64, structure: usize) -> MapRun {
    let mut g = Game::new(seed, pack.clone());
    g.sustain = true;
    g.trace = true;
    g.start();
    g.state.place = Place::Room { structure, room: 0 };
    g.state.pos = g.site.land.structure_pos[structure];
    let start = g.state.minutes;
    let rooms = g.site.structure(structure).interior.rooms.len();
    let mut plan: BTreeMap<usize, Spot> = BTreeMap::from([(0, (0.0, 0.0, 0))]);
    let mut walked: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut tried: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut hidden_found = 0;
    let mut here = 0usize;
    let mut looked: BTreeSet<usize> = BTreeSet::new();
    for _ in 0..20_000 {
        // Light as a lamp would give: the mapper always has one.
        g.forced_light = true;
        if looked.insert(here) {
            let o = g.step("look closer");
            hidden_found += o
                .renders
                .iter()
                .filter(|r| r.trace.slot == "room.found")
                .count();
        }
        // A way from here not yet tried, else the nearest space on the
        // plan that has one, by the ways walked.
        let open: Vec<usize> = g
            .ways()
            .into_iter()
            .filter(|w| !tried.contains(&(here, w.link)))
            .map(|w| w.link)
            .collect();
        if let Some(&link) = open.first() {
            tried.insert((here, link));
            // A closed door is opened first.
            if g.ways().iter().any(|w| {
                w.link == link && w.state == scraped_world::structures::PassageState::Closed
            }) {
                g.step_quiet_door(link);
            }
            let o = g.go_way(link);
            let Place::Room { room: to, .. } = g.state.place else {
                break;
            };
            if to == here {
                continue;
            }
            // Where the report says it went.
            // The latest report (the mapper moves without starting new
            // commands, so earlier renders are still there).
            let report = o
                .renders
                .iter()
                .rev()
                .find(|r| r.trace.slot == "move.report");
            if let Some(r) = report {
                let num = |k: &str| match r.vars.get(k) {
                    Some(Value::Number(n)) => *n as f64,
                    _ => 0.0,
                };
                let dir = match r.vars.get("direction") {
                    Some(Value::Text(t)) => t.clone(),
                    _ => String::new(),
                };
                let (x, y, l) = plan[&here];
                let level = l + match dir.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => 0,
                };
                plan.entry(to)
                    .or_insert((x + num("east"), y + num("north"), level));
                walked.insert((here.min(to), here.max(to)));
            }
            here = to;
            continue;
        }
        // Back to the nearest planned space with untried ways.
        let planned: BTreeSet<usize> = plan.keys().copied().collect();
        let next = route_to_untried(&g, structure, here, &planned, &tried);
        match next {
            Some(path) => {
                for link in path {
                    g.forced_light = true;
                    g.go_way(link);
                }
                if let Place::Room { room, .. } = g.state.place {
                    here = room;
                }
            }
            None => break,
        }
    }
    let hours = f64::from(g.state.minutes - start) / 60.0;
    // Check the plan against the truth.
    let st = g.site.structure(structure);
    let (ex, ey) = st.interior.rooms[0].centre();
    let mut errors = Vec::new();
    for (&r, &(x, y, l)) in &plan {
        let (tx, ty) = st.interior.rooms[r].centre();
        let (dx, dy) = (tx - ex, -(ty - ey));
        let off = ((x - dx).powi(2) + (y - dy).powi(2)).sqrt();
        let far = (dx * dx + dy * dy).sqrt().max(10.0);
        let level_ok = l == st.interior.rooms[r].level;
        errors.push(if level_ok { off / far } else { 1.0 });
    }
    let topology_exact = walked.iter().all(|&(a, b)| {
        st.interior
            .links
            .iter()
            .any(|l| (l.a.min(l.b), l.a.max(l.b)) == (a, b))
    });
    MapRun {
        structure,
        visited: plan.len(),
        spaces: rooms,
        hidden_found,
        error_mean: errors.iter().sum::<f64>() / errors.len().max(1) as f64,
        error_max: errors.iter().copied().fold(0.0, f64::max),
        ways_walked: walked.len(),
        topology_exact,
        hours,
    }
}

/// The ways (links) to walk from `from` to the nearest space on the plan
/// with a way not yet tried, through ways already walked.
fn route_to_untried(
    g: &Game,
    structure: usize,
    from: usize,
    planned: &BTreeSet<usize>,
    tried: &BTreeSet<(usize, usize)>,
) -> Option<Vec<usize>> {
    let mut prev: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    let mut q = VecDeque::from([from]);
    let mut seen = BTreeSet::from([from]);
    while let Some(r) = q.pop_front() {
        let ways = g.ways_at(Place::Room { structure, room: r });
        let untried = ways.iter().any(|w| !tried.contains(&(r, w.link)));
        if untried && r != from {
            let mut path = Vec::new();
            let mut at = r;
            while at != from {
                let (p, link) = prev[&at];
                path.push(link);
                at = p;
            }
            path.reverse();
            return Some(path);
        }
        for w in ways {
            let Place::Room { room: n, .. } = w.to else {
                continue;
            };
            // Any way it knows of that can be gone through, to a space on
            // its plan.
            let usable = planned.contains(&n)
                && w.state == scraped_world::structures::PassageState::Open
                && !w.against
                && w.passage != Some(scraped_world::structures::Passage::Window)
                && !w.collapsed
                && !g.under_water(w.to);
            if usable && seen.insert(n) {
                prev.insert(n, (r, w.link));
                q.push_back(n);
            }
        }
    }
    None
}
