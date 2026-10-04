//! The world on a trajectory, in play: regional drift day by day, the
//! drivers that push it (great inscriptions and the player's largest
//! writing), how it shows locally, change on revisits, seasons and age.

use scraped_content::Value;
use scraped_sim::items::scrape_power;
use scraped_sim::region::{regional_push, Driver};
use scraped_sim::writing::live_of;

use crate::site::{ctx, Place};
use crate::{Game, Output};

/// The player's age when play begins, in years.
// DESIGN-Q: the player starts at 25; a year is 360 days.
pub const START_AGE: u32 = 25;

/// Age bands. Ids for content.
pub const AGES: &[&str] = &["young", "grown", "older", "old", "aged"];

impl Game {
    pub(crate) fn day(&self) -> u32 {
        self.state.minutes / 1440
    }

    pub(crate) fn age(&self) -> u32 {
        START_AGE + self.day() / 360
    }

    pub(crate) fn age_band(&self) -> &'static str {
        match self.age() {
            a if a < 30 => "young",
            a if a < 45 => "grown",
            a if a < 60 => "older",
            a if a < 75 => "old",
            _ => "aged",
        }
    }

    /// The strongest scraper the player carries (0: none).
    pub(crate) fn power(&self) -> u8 {
        self.state
            .carried
            .iter()
            .map(|&t| scrape_power(self.thing(t).kind))
            .max()
            .unwrap_or(0)
    }

    /// How strong a scraper a thing's writing needs: 4 where a great
    /// inscription lies, else 1.
    pub(crate) fn needs_power(&self, thing: usize) -> u8 {
        let layers = self.layers(thing);
        if self.site.greats.iter().any(|g| layers.contains(&g.text)) {
            4
        } else {
            1
        }
    }

    /// The regional pushes acting now: great inscriptions still live on
    /// their surfaces, and the player's region-scale releases.
    pub(crate) fn recompute_drivers(&mut self) {
        let mut drivers = Vec::new();
        for c in &self.claims {
            let great = self.site.greats.iter().find(|g| g.text == c.text);
            let power = self.state.released.get(&c.text).copied().unwrap_or(0);
            let (reach, why) = match (great, power) {
                (Some(g), _) => (g.reach, format!("great inscription ({})", g.kind)), // DEBUG-TEXT
                (None, 4) => (2, format!("player's great release (text {})", c.text)), // DEBUG-TEXT
                (None, 3) => (0, format!("player's regional release (text {})", c.text)), // DEBUG-TEXT
                _ => continue,
            };
            let Some(region) = self.site.regions.at(c.pos) else {
                continue;
            };
            let (variable, target) = regional_push(c.property, c.amount);
            drivers.push(Driver {
                region,
                reach,
                variable,
                target,
                why: format!(
                    "{why}: {}{} {}",
                    if c.negative { "not " } else { "" },
                    c.verb,
                    c.subject
                ), // DEBUG-TEXT
            });
        }
        self.drivers = drivers;
    }

    /// Steps the regions to today.
    pub(crate) fn step_regions(&mut self) {
        let day = self.day();
        if day > self.state.regions.day {
            self.causes = self
                .site
                .regions
                .advance(&mut self.state.regions, day, &self.drivers);
        }
    }

    /// Whether a thing's live layer is one of the great inscriptions.
    pub(crate) fn great_here(&self) -> bool {
        let Place::Room { .. } = self.state.place else {
            return false;
        };
        self.here().into_iter().any(|t| {
            let layers = self.layers(t);
            live_of(&layers, &self.state.scraped)
                .is_some_and(|l| self.site.greats.iter().any(|g| g.text == l))
        })
    }

    /// `wait`, `wait 3 hours`, `wait a day`, `wait 2 weeks`.
    pub(crate) fn wait(&mut self, words: &[String]) -> Output {
        let n = words
            .iter()
            .find_map(|w| w.parse::<u32>().ok())
            .unwrap_or(1)
            .clamp(1, 400);
        let unit = words.iter().find_map(|w| match w.trim_end_matches('s') {
            "minute" => Some(1),
            "hour" => Some(60),
            "day" => Some(1440),
            "week" => Some(7 * 1440),
            "season" => Some(90 * 1440),
            _ => None,
        });
        let minutes = match unit {
            Some(u) => n * u,
            None => 30,
        };
        self.interrupted = false;
        let mut left = minutes;
        while left > 0 && !self.interrupted && self.state.dead.is_none() {
            let step = left.min(60);
            self.advance(step, scraped_sim::body::Activity::Resting);
            left -= step;
        }
        let waited = minutes - left;
        let mut t = self.say(
            "say.wait",
            ctx(&[("minutes", Value::Number(i64::from(waited)))]),
        );
        // Someone who waits a while out of doors, quietly, may see what
        // lives there come out (D06).
        if waited == minutes && waited <= 180 && self.state.place == Place::Outside {
            self.attentive = true;
            let mut facts = Vec::new();
            self.life_facts(crate::attention::Response::Look, &mut facts);
            facts.retain(|f| f.slot == "life.seen");
            if !facts.is_empty() {
                let parts = self.attend(facts, crate::attention::Response::Travel);
                if !parts.is_empty() {
                    t = format!("{t} {}", parts.join(" "));
                }
            }
        }
        self.output(vec![t], None)
    }

    /// Debug: regions and what is pushing them.
    pub(crate) fn regions_truth(&self) -> serde_json::Value {
        let here = self.site.regions.at(self.state.pos);
        serde_json::json!({
            "region": here,
            "vars": here.map(|r| self.state.regions.vars[r]),
            "drivers": self.drivers,
            "causes_here": self.causes.iter().filter(|c| Some(c.region) == here).collect::<Vec<_>>(),
        })
    }

    /// Spoiler view of the regions, `days` ahead.
    pub fn regions_debug(&self, days: u32) -> String {
        scraped_sim::region::debug(&self.site.regions, &self.state.regions, &self.drivers, days)
    }

    pub(crate) fn release_feeling(&mut self, power: u8) -> Option<String> {
        if power < 3 {
            return None;
        }
        let scale = if power >= 4 { "great" } else { "region" };
        Some(self.say("great.release", ctx(&[("scale", Value::from(scale))])))
    }

    pub(crate) fn too_weak(&mut self, thing: usize) -> Output {
        let name = self.thing_name(thing);
        let t = self.say("scrape.too_weak", ctx(&[("thing", Value::from(name))]));
        self.output(vec![t], None)
    }
}
