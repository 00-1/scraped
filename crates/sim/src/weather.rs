//! Weather that moves (D06): fronts of rain, storms, fog banks and spells
//! of heat that cross the land with the prevailing wind, and what they
//! leave behind.
//!
//! Each week has its own fronts, drawn from the seed, the season and how
//! wet the land is. A front is a circle moving in a straight line, so
//! whether it covers a place at a moment, and for how long it rained there
//! over the last days, are worked out exactly from its path (only `+ - * /`
//! and `sqrt`). Nothing is stored: the same seed and minute always give
//! the same sky.
//!
//! Consequences follow from the record: fords run too deep after heavy
//! rain, snow lies on the heights and closes passes until it melts,
//! storms bring down trees across forest ways, lakes draw back in a dry
//! summer, and (from M07) hard frost makes ice that bears.

use serde::Serialize;

use scraped_world::terrain::{Biome, SIZE};
use scraped_world::World;

use crate::outdoors::{hash, time_of_day, unit, Pos, CELL};
use crate::region::{season, season_offset, SEASON_DAYS};

const WEEK: u32 = 7 * 1440;

/// What a front brings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FrontKind {
    /// Heat lies under everything else.
    Heat,
    Fog,
    Rain,
    Storm,
}

/// A weather system crossing the land.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Front {
    /// Unique in the world: week and index.
    pub id: u64,
    pub kind: FrontKind,
    /// The minute it forms and how long it lasts.
    pub start: u32,
    pub minutes: u32,
    /// Its centre when it forms, in metres.
    pub x: f64,
    pub y: f64,
    /// Metres it moves each minute.
    pub vx: f64,
    pub vy: f64,
    pub radius: f64,
}

impl Front {
    /// Where its centre is at a minute.
    pub fn centre(&self, minutes: u32) -> (f64, f64) {
        let t = f64::from(minutes.saturating_sub(self.start));
        (self.x + self.vx * t, self.y + self.vy * t)
    }

    pub fn active(&self, minutes: u32) -> bool {
        minutes >= self.start && minutes < self.start + self.minutes
    }

    /// Whether it covers a point at a minute.
    pub fn covers(&self, p: Pos, minutes: u32) -> bool {
        if !self.active(minutes) {
            return false;
        }
        let (cx, cy) = self.centre(minutes);
        let (dx, dy) = (f64::from(p.x) - cx, f64::from(p.y) - cy);
        dx * dx + dy * dy <= self.radius * self.radius
    }

    /// The minutes during which it covers a point, as (from, to), or
    /// `None` if it never does.
    pub fn over(&self, p: Pos) -> Option<(u32, u32)> {
        // |r + v t|² = R²: a quadratic in t.
        let (rx, ry) = (self.x - f64::from(p.x), self.y - f64::from(p.y));
        let a = self.vx * self.vx + self.vy * self.vy;
        let b = 2.0 * (rx * self.vx + ry * self.vy);
        let c = rx * rx + ry * ry - self.radius * self.radius;
        let (t0, t1) = if a < 1e-9 {
            if c > 0.0 {
                return None;
            }
            (0.0, f64::from(self.minutes))
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 {
                return None;
            }
            let s = disc.sqrt();
            ((-b - s) / (2.0 * a), (-b + s) / (2.0 * a))
        };
        let t0 = t0.max(0.0);
        let t1 = t1.min(f64::from(self.minutes));
        (t1 > t0).then(|| (self.start + t0 as u32, self.start + t1 as u32))
    }

    /// Minutes it covered a point within `[from, to)`.
    pub fn overlap(&self, p: Pos, from: u32, to: u32) -> u32 {
        match self.over(p) {
            Some((a, b)) => b.min(to).saturating_sub(a.max(from)),
            None => 0,
        }
    }
}

/// How wet the land is on the whole, 0–1, from a sample of cells.
fn wetness(w: &World) -> f64 {
    let mut sum = 0.0;
    let mut n = 0.0;
    for y in (4..SIZE).step_by(20) {
        for x in (4..SIZE).step_by(20) {
            if w.terrain.is_land(x, y) {
                sum += *w.terrain.moisture.get(x, y);
                n += 1.0;
            }
        }
    }
    if n == 0.0 {
        0.5
    } else {
        sum / n
    }
}

/// The fronts that form in a week.
// DESIGN-Q: two to five rain fronts a week by how wet the land is (one
// more in autumn and winter); storms mostly in autumn and winter; fog
// banks in spring and autumn; spells of heat in summer. Fronts move with
// the prevailing wind, give or take a point of the compass.
pub fn fronts(w: &World, week: u32) -> Vec<Front> {
    let season = season(week * WEEK + WEEK / 2);
    let wet = wetness(w);
    let h = |k: u64| hash(&[w.seed, 0xf407, u64::from(week), k]);
    let rains = 1 + (wet * 4.0) as u64 + u64::from(season >= 2);
    let storms = match season {
        2 | 3 => 1 + h(1) % 2,
        _ => h(1) % 2,
    };
    let fogs = match season {
        0 | 2 => 1 + h(2) % 2,
        _ => h(2) % 2,
    };
    let heats = if season == 1 { 1 + h(3) % 2 } else { 0 };
    let wind = ((w.terrain.wind + 22) / 45 % 8) as usize;
    let span = (SIZE as f64) * f64::from(CELL);
    let mut out = Vec::new();
    let mut make = |kind: FrontKind, k: u64| {
        let r = h(100 + k);
        let (speed, radius, hours) = match kind {
            FrontKind::Rain => (
                100.0 + (r % 100) as f64,
                8000.0 + (r >> 8) as f64 % 8000.0,
                10 + (r >> 20) % 20,
            ),
            FrontKind::Storm => (
                250.0 + (r % 150) as f64,
                4000.0 + (r >> 8) as f64 % 4000.0,
                4 + (r >> 20) % 7,
            ),
            FrontKind::Fog => (
                30.0 + (r % 30) as f64,
                5000.0 + (r >> 8) as f64 % 7000.0,
                6 + (r >> 20) % 9,
            ),
            FrontKind::Heat => (
                20.0,
                20000.0 + (r >> 8) as f64 % 10000.0,
                48 + (r >> 20) % 48,
            ),
        };
        let bearing = (wind + 7 + ((r >> 30) % 3) as usize) % 8;
        let (ux, uy) = unit(bearing);
        let minutes = (hours * 60) as u32;
        // Fog forms in the evening; the rest at any hour.
        let day = (r >> 36) % 7;
        let start = week * WEEK
            + day as u32 * 1440
            + match kind {
                FrontKind::Fog => 19 * 60 + (r >> 40) as u32 % 180,
                _ => (r >> 40) as u32 % 1440,
            };
        // The path crosses the land: the centre passes near the middle of
        // the map halfway through its life, off to one side or the other.
        let side = ((r >> 48) % 1000) as f64 / 1000.0 - 0.5;
        let mid = (span / 2.0 - uy * side * span, span / 2.0 + ux * side * span);
        let half = f64::from(minutes) / 2.0;
        out.push(Front {
            id: u64::from(week) * 64 + k,
            kind,
            start,
            minutes,
            x: mid.0 - ux * speed * half,
            y: mid.1 - uy * speed * half,
            vx: ux * speed,
            vy: uy * speed,
            radius,
        });
    };
    let mut k = 0;
    for (kind, n) in [
        (FrontKind::Rain, rains),
        (FrontKind::Storm, storms),
        (FrontKind::Fog, fogs),
        (FrontKind::Heat, heats),
    ] {
        for _ in 0..n {
            make(kind, k);
            k += 1;
        }
    }
    out
}

/// Fronts that may act in `[from, to)`: those formed in the weeks it
/// spans and the week before (no front lasts longer than a week).
pub fn fronts_between(w: &World, from: u32, to: u32) -> Vec<Front> {
    let first = (from / WEEK).saturating_sub(1);
    let last = to.saturating_sub(1) / WEEK;
    (first..=last)
        .flat_map(|wk| fronts(w, wk))
        .filter(|f| f.start < to && f.start + f.minutes > from)
        .collect()
}

/// The day's mean air at a cell in this season, for snow and melt.
pub fn mean_air(w: &World, x: usize, y: usize, minutes: u32) -> f64 {
    *w.terrain.temperature.get(x, y) + season_offset(minutes)
}

/// Whether rain falls as snow at a cell.
fn cold(w: &World, x: usize, y: usize, minutes: u32) -> bool {
    mean_air(w, x, y, minutes) < 1.0
}

/// The front over a point now, the strongest if several.
pub fn front_at(w: &World, pos: Pos, minutes: u32) -> Option<Front> {
    fronts_between(w, minutes, minutes + 1)
        .into_iter()
        .filter(|f| f.covers(pos, minutes))
        // Fog banks only hold through the night and morning.
        .filter(|f| {
            f.kind != FrontKind::Fog
                || matches!(
                    time_of_day(minutes),
                    "night" | "dawn" | "morning" | "evening"
                )
        })
        .max_by_key(|f| (f.kind, f.id))
}

/// The weather at a place and time: "clear", "rain", "storm", "snow" or
/// "fog".
pub fn weather(w: &World, pos: Pos, minutes: u32) -> &'static str {
    let (x, y) = pos.cell();
    match front_at(w, pos, minutes).map(|f| f.kind) {
        Some(FrontKind::Rain | FrontKind::Storm) if cold(w, x, y, minutes) => "snow",
        Some(FrontKind::Rain) => "rain",
        Some(FrontKind::Storm) => "storm",
        Some(FrontKind::Fog) => "fog",
        _ => {
            // Mist of the place's own: wet ground at dawn and night.
            let mut m = *w.terrain.moisture.get(x, y);
            if matches!(
                w.terrain.biome.get(x, y),
                Biome::Marsh | Biome::Shore | Biome::Lake
            ) {
                m += 0.2;
            }
            let r = (hash(&[w.seed, 0x5745_4154, u64::from(minutes / 180)]) % 1000) as f64 / 1000.0;
            if matches!(time_of_day(minutes), "dawn" | "night") && r < m * 0.2 {
                "fog"
            } else {
                "clear"
            }
        }
    }
}

/// Degrees a spell of heat adds at a place now.
pub fn heat(w: &World, pos: Pos, minutes: u32) -> f64 {
    let hot = fronts_between(w, minutes, minutes + 1)
        .iter()
        .any(|f| f.kind == FrontKind::Heat && f.covers(pos, minutes));
    if hot {
        6.0
    } else {
        0.0
    }
}

/// Minutes of rain over a point in the last `hours`, storms counted
/// double; snow does not count.
pub fn rain(w: &World, pos: Pos, minutes: u32, hours: u32) -> u32 {
    let from = minutes.saturating_sub(hours * 60);
    rain_in(&fronts_between(w, from, minutes), w, pos, minutes, hours)
}

/// [`rain`] over fronts already gathered.
pub fn rain_in(fs: &[Front], w: &World, pos: Pos, minutes: u32, hours: u32) -> u32 {
    let from = minutes.saturating_sub(hours * 60);
    let (x, y) = pos.cell();
    if cold(w, x, y, minutes) {
        return 0;
    }
    fs.iter()
        .map(|f| match f.kind {
            FrontKind::Rain => f.overlap(pos, from, minutes),
            FrontKind::Storm => 2 * f.overlap(pos, from, minutes),
            _ => 0,
        })
        .sum()
}

// ---------- consequences ----------

/// Rain (minutes, storms double) over two days that puts fords in flood.
// DESIGN-Q: five and a half hours of rain in two days floods a ford; it runs
// shallow again once the rain has passed out of that window.
pub const FLOOD_RAIN: u32 = 330;
/// Snow (cm) that closes high ground.
pub const SNOW_CLOSES: f64 = 20.0;
/// Rain (minutes) in thirty summer days below which lakes draw back.
pub const DROUGHT_RAIN: u32 = 240;

/// Whether a ford at a cell runs too deep to wade.
pub fn ford_flooded(w: &World, x: usize, y: usize, minutes: u32) -> bool {
    *w.water.ford.get(x, y) && rain(w, Pos::of_cell(x, y), minutes, 48) >= FLOOD_RAIN
}

/// [`ford_flooded`] over fronts already gathered.
pub fn ford_flooded_in(fs: &[Front], w: &World, x: usize, y: usize, minutes: u32) -> bool {
    *w.water.ford.get(x, y) && rain_in(fs, w, Pos::of_cell(x, y), minutes, 48) >= FLOOD_RAIN
}

/// Snow lying at a cell, in cm: what fell in the last ten days, less what
/// has melted since.
// DESIGN-Q: snow falls at 3 cm an hour (5 in a storm) and melts at a
// fifth of a cm an hour for each degree of the day's mean above freezing.
pub fn snow(w: &World, x: usize, y: usize, minutes: u32) -> f64 {
    let from = minutes.saturating_sub(10 * 1440);
    snow_in(&fronts_between(w, from, minutes), w, x, y, minutes)
}

/// [`snow`] over fronts already gathered.
pub fn snow_in(fs: &[Front], w: &World, x: usize, y: usize, minutes: u32) -> f64 {
    if !cold(w, x, y, minutes) && mean_air(w, x, y, minutes) > 6.0 {
        return 0.0;
    }
    let pos = Pos::of_cell(x, y);
    let from = minutes.saturating_sub(10 * 1440);
    let mut depth = 0.0;
    for &f in fs {
        let rate = match f.kind {
            FrontKind::Rain => 3.0,
            FrontKind::Storm => 5.0,
            _ => continue,
        };
        let Some((a, b)) = f.over(pos) else { continue };
        let (a, b) = (a.max(from), b.min(minutes));
        if b <= a || !cold(w, x, y, a) {
            continue;
        }
        let fell = f64::from(b - a) / 60.0 * rate;
        let warm = mean_air(w, x, y, minutes).max(0.0);
        let melted = f64::from(minutes - b) / 60.0 * warm * 0.2;
        depth += (fell - melted).max(0.0);
    }
    depth
}

/// Whether a cell is high ground that snow can close.
pub fn high_ground(w: &World, x: usize, y: usize) -> bool {
    matches!(w.terrain.biome.get(x, y), Biome::Rock | Biome::Tundra)
        || *w.terrain.height.get(x, y) > 900.0
}

/// Whether snow has closed a cell.
pub fn snowbound(w: &World, x: usize, y: usize, minutes: u32) -> bool {
    let from = minutes.saturating_sub(10 * 1440);
    snowbound_in(&fronts_between(w, from, minutes), w, x, y, minutes)
}

/// [`snowbound`] over fronts already gathered.
pub fn snowbound_in(fs: &[Front], w: &World, x: usize, y: usize, minutes: u32) -> bool {
    high_ground(w, x, y) && w.terrain.is_land(x, y) && snow_in(fs, w, x, y, minutes) >= SNOW_CLOSES
}

/// Whether storm-felled trees lie across a cell: storms in the last two
/// weeks bring down trees in about one forest patch in twenty they cross.
// DESIGN-Q: windthrow lies for two weeks and cannot be climbed through.
pub fn windthrow(w: &World, x: usize, y: usize, minutes: u32) -> bool {
    let from = minutes.saturating_sub(14 * 1440);
    windthrow_in(&fronts_between(w, from, minutes), w, x, y, minutes)
}

/// [`windthrow`] over fronts already gathered.
pub fn windthrow_in(fs: &[Front], w: &World, x: usize, y: usize, minutes: u32) -> bool {
    if !matches!(w.terrain.biome.get(x, y), Biome::Forest | Biome::Pine) {
        return false;
    }
    let pos = Pos::of_cell(x, y);
    let from = minutes.saturating_sub(14 * 1440);
    fs.iter().any(|f| {
        f.kind == FrontKind::Storm
            && f.over(pos).is_some_and(|(a, _)| a < minutes && a >= from)
            && hash(&[w.seed, 0x7ee5, f.id, (x / 2) as u64, (y / 2) as u64]).is_multiple_of(20)
    })
}

/// Whether a lake has drawn back from its shore after a dry summer.
pub fn lake_low(w: &World, x: usize, y: usize, minutes: u32) -> bool {
    let s = season(minutes);
    if !(s == 1 || s == 2) || minutes < 30 * 1440 {
        return false;
    }
    let pos = Pos::of_cell(x, y);
    rain(w, pos, minutes, 30 * 24) < DROUGHT_RAIN
}

/// The fronts any consequence at a minute can depend on: those of the
/// last three weeks.
pub fn recent(w: &World, minutes: u32) -> Vec<Front> {
    fronts_between(w, minutes.saturating_sub(21 * 1440), minutes + 1)
}

/// What closes a cell now, if weather does: "flood", "snow" or
/// "fallen trees".
pub fn closed_in(
    fs: &[Front],
    w: &World,
    x: usize,
    y: usize,
    minutes: u32,
) -> Option<&'static str> {
    if ford_flooded_in(fs, w, x, y, minutes) {
        Some("flood")
    } else if windthrow_in(fs, w, x, y, minutes) {
        Some("fallen trees")
    } else if snowbound_in(fs, w, x, y, minutes) {
        Some("snow")
    } else {
        None
    }
}

/// What the weather did in a 30-day month that changed routes or places:
/// (kind, area, week) for each flooded ford, snow-closed height, windthrow
/// and drawn-back lake, counted once per area of about 5 km and week.
/// Sampled every other day at noon; for the depth metrics.
pub fn episodes(w: &World, month: u32) -> std::collections::BTreeSet<(&'static str, usize, u32)> {
    let mut out = std::collections::BTreeSet::new();
    let t0 = month * 30 * 1440;
    for d in (0..30).step_by(2) {
        let t = t0 + d * 1440 + 720;
        let fs = fronts_between(w, t.saturating_sub(21 * 1440), t + 1);
        for y in (0..SIZE).step_by(2) {
            for x in (0..SIZE).step_by(2) {
                let area = (y / 16) * SIZE + x / 16;
                let kind = if *w.terrain.biome.get(x, y) == Biome::Lake {
                    lake_low(w, x, y, t).then_some("low lake")
                } else {
                    closed_in(&fs, w, x, y, t)
                };
                if let Some(k) = kind {
                    out.insert((k, area, t / WEEK));
                }
            }
        }
    }
    out
}

/// How many days into the year a minute falls.
pub fn day_of_year(minutes: u32) -> u32 {
    minutes / 1440 % (4 * SEASON_DAYS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_front_covers_what_its_path_says() {
        let w = World::generate(3);
        for f in fronts(&w, 5) {
            let p = Pos::new(24000, 24000);
            match f.over(p) {
                Some((a, b)) => {
                    let mid = (a + b) / 2;
                    assert!(f.covers(p, mid), "{f:?}");
                    if a > f.start + 2 {
                        assert!(!f.covers(p, a - 2));
                    }
                }
                None => {
                    for t in (f.start..f.start + f.minutes).step_by(30) {
                        assert!(!f.covers(p, t));
                    }
                }
            }
        }
    }

    #[test]
    fn weather_moves_across_the_land() {
        // A front reaches one side of the map before the other.
        let w = World::generate(1);
        let f = fronts(&w, 2)
            .into_iter()
            .find(|f| f.kind == FrontKind::Rain)
            .unwrap();
        let (a, b) = (f.centre(f.start), f.centre(f.start + f.minutes - 1));
        let moved = ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt();
        assert!(moved > 10000.0, "{moved}");
    }

    #[test]
    fn weather_is_deterministic_and_varied() {
        let w = World::generate(9);
        let p = Pos::new(20000, 22000);
        let mut kinds = std::collections::BTreeSet::new();
        for h in 0..(60 * 24) {
            let t = h * 60;
            assert_eq!(weather(&w, p, t), weather(&w, p, t));
            kinds.insert(weather(&w, p, t));
        }
        assert!(kinds.len() >= 3, "{kinds:?}");
    }

    #[test]
    fn weather_changes_routes_and_places_every_month() {
        for seed in [1u64, 2] {
            let w = World::generate(seed);
            let n: usize = (0..12).map(|m| episodes(&w, m).len()).sum();
            assert!(n / 12 >= 3, "seed {seed}: {n} in a year");
        }
    }
}
