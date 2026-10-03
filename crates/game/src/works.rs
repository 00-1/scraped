//! Works and calendar doors in play (D05): machines worked part by part
//! in order, each showing whether it can move, and doors that open only
//! on their festival day. Puzzles of observation and order, not reading.

use scraped_content::Value;
use scraped_sim::outdoors::label;

use crate::site::{ctx, Place};
use crate::{Game, Output};

/// Days in the year (four seasons).
const YEAR: u32 = 4 * scraped_sim::region::SEASON_DAYS;

impl Game {
    /// The works a part belongs to, and its step.
    fn part_of(&self, i: usize) -> Option<(usize, usize)> {
        let w = self.site.fixtures.mechanisms[i].controls?;
        let step = self
            .site
            .fixtures
            .works
            .get(w)?
            .steps
            .iter()
            .position(|&s| s == i)?;
        Some((w, step))
    }

    /// A part's state: running once worked, ready when every part before
    /// it runs, else idle.
    pub(crate) fn part_state(&self, i: usize) -> &'static str {
        let Some((w, step)) = self.part_of(i) else {
            return "idle";
        };
        let steps = &self.site.fixtures.works[w].steps;
        if self.state.sim.moved.contains(&i) {
            "running"
        } else if steps[..step]
            .iter()
            .all(|s| self.state.sim.moved.contains(s))
        {
            "ready"
        } else {
            "idle"
        }
    }

    /// Whether a works has run to its end.
    pub(crate) fn works_done(&self, w: usize) -> bool {
        self.site.fixtures.works[w]
            .steps
            .iter()
            .all(|s| self.state.sim.moved.contains(s))
    }

    /// Working one part: it moves only when the parts before it run; when
    /// the last moves, the sealed way opens.
    pub(crate) fn work_part(&mut self, i: usize) -> Output {
        let kind = label(&self.site.fixtures.mechanisms[i].kind);
        match self.part_state(i) {
            "running" => {
                let c = ctx(&[
                    ("kind", Value::from(kind)),
                    ("state", Value::from("running")),
                ]);
                let t = self.say("mech.already", c);
                self.output(vec![t], None)
            }
            "idle" => {
                // What it waits on: the first part before it not running.
                let (w, step) = self.part_of(i).expect("a part");
                let needs = self.site.fixtures.works[w].steps[..step]
                    .iter()
                    .find(|s| !self.state.sim.moved.contains(s))
                    .map(|&s| label(&self.site.fixtures.mechanisms[s].kind))
                    .unwrap_or_default();
                self.pass(2);
                let c = ctx(&[
                    ("kind", Value::from(kind)),
                    ("needs", Value::from(needs)),
                    ("family", Value::from(self.site.fixtures.works[w].family)),
                ]);
                let t = self.say("mech.idle", c);
                self.output(vec![t], None)
            }
            _ => {
                self.pass(5);
                self.state.sim.moved.insert(i);
                let (w, _) = self.part_of(i).expect("a part");
                let c = ctx(&[
                    ("kind", Value::from(kind)),
                    ("state", Value::from("running")),
                ]);
                let mut parts = vec![self.say("mech.operate", c)];
                if self.works_done(w) {
                    let c = ctx(&[("family", Value::from(self.site.fixtures.works[w].family))]);
                    parts.push(self.say("works.done", c));
                }
                self.output(parts, None)
            }
        }
    }

    /// The day of the year (0 to 359).
    pub(crate) fn day_of_year(&self) -> u32 {
        (self.state.minutes / 1440) % YEAR
    }

    /// Why a way is sealed shut, if it is: "works" (a gate the works
    /// raise) or "calendar" (a door that opens only on its day).
    pub(crate) fn sealed(&self, structure: usize, link: usize) -> Option<&'static str> {
        let f = &self.site.fixtures;
        if let Some(w) = f
            .works
            .iter()
            .position(|w| w.structure == structure && w.seal == link)
        {
            return (!self.works_done(w)).then_some("works");
        }
        let c = f
            .calendar
            .iter()
            .find(|c| c.structure == structure && c.link == link)?;
        let d = self.day_of_year();
        let off = (d + YEAR - c.day) % YEAR;
        (off > 1 && off < YEAR - 1).then_some("calendar")
    }

    /// `examine the calendar stone`: its marks, and the one notch cut
    /// deeper than the rest, at the day its temple's door opens.
    pub(crate) fn examine_calendar(&mut self, t: usize) -> Option<Output> {
        let Place::Room { structure, .. } = self.thing(t).home else {
            return None;
        };
        let day = self
            .site
            .fixtures
            .calendar
            .iter()
            .find(|c| c.structure == structure)?
            .day;
        self.pass(2);
        let season =
            scraped_sim::region::SEASONS[(day / scraped_sim::region::SEASON_DAYS) as usize % 4];
        let part = ["early", "middle", "late"][((day % 90) / 30) as usize];
        let now = self.day_of_year();
        let c = ctx(&[
            ("season", Value::from(season)),
            ("part", Value::from(part)),
            (
                "today",
                Value::Bool((now + YEAR - day) % YEAR <= 1 || (day + YEAR - now) % YEAR <= 1),
            ),
        ]);
        let tx = self.say("calendar.notch", c);
        Some(self.output(vec![tx], None))
    }
}
