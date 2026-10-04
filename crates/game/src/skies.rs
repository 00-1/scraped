//! Weather that moves and leaves marks, and the sky (D06).
//!
//! What a front has done is evidence on the land: a ford running deep,
//! storm-felled trees, snow lying on the heights, a lake drawn back from
//! its shore. A front on its way shows as cloud or a fog bank on one side
//! of the sky. All of it goes through the attention model.

use scraped_content::Value;
use scraped_sim::outdoors::{bearing, Pos, BEARINGS};
use scraped_sim::weather::{self, FrontKind};
use scraped_world::terrain::{Biome, SIZE};

use crate::attention::{Fact, Response};
use crate::site::{ctx, Place};
use crate::Game;

/// What the weather has left: ids for content.
pub const MARKS: &[&str] = &[
    "flooded ford",
    "high water",
    "fallen trees",
    "snow dusting",
    "snow lying",
    "deep snow",
    "low lake",
];

/// What is on its way.
pub const COMING: &[&str] = &["rain", "storm", "fog", "snow"];

impl Game {
    /// Cells around the player within `r` cells.
    fn cells_around(&self, r: i64) -> Vec<(usize, usize)> {
        let (x, y) = self.state.pos.cell();
        let mut out = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if (0..SIZE as i64).contains(&nx) && (0..SIZE as i64).contains(&ny) {
                    out.push((nx as usize, ny as usize));
                }
            }
        }
        out
    }

    /// What the weather has done close by, and what is coming.
    pub(crate) fn weather_facts(&mut self, response: Response, out: &mut Vec<Fact>) {
        if self.state.place != Place::Outside || self.forced.is_some() {
            return;
        }
        let now = self.state.minutes;
        let w = &self.site.world;
        let pos = self.state.pos;
        let env = self.env();
        let mut marks: Vec<(&'static str, Option<usize>, f64)> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for (x, y) in self.cells_around(2) {
            let at = Pos::of_cell(x, y);
            let side = bearing(pos, at).filter(|_| pos.dist(at) > 200.0);
            let closed = env.closed(x, y);
            let mark = match closed {
                Some("flood") => Some(("flooded ford", 40.0)),
                Some("fallen trees") => Some(("fallen trees", 36.0)),
                _ if env.is_river(x, y)
                    && weather::rain(w, at, now, 48) >= weather::FLOOD_RAIN / 2 =>
                {
                    Some(("high water", 24.0))
                }
                _ if *w.terrain.biome.get(x, y) == Biome::Lake
                    && weather::lake_low(w, x, y, now) =>
                {
                    Some(("low lake", 34.0))
                }
                _ => None,
            };
            if let Some((m, sal)) = mark {
                if seen.insert(m) {
                    marks.push((m, side, sal));
                }
            }
        }
        // Snow underfoot.
        let (x, y) = pos.cell();
        let depth = weather::snow(w, x, y, now);
        if depth >= 1.0 && *w.terrain.biome.get(x, y) != Biome::Snow {
            let m = if depth >= weather::SNOW_CLOSES {
                "deep snow"
            } else if depth >= 5.0 {
                "snow lying"
            } else {
                "snow dusting"
            };
            marks.push((m, None, if depth >= 5.0 { 30.0 } else { 16.0 }));
        }
        for (m, side, sal) in marks {
            out.push(Fact::new(
                "weather.mark",
                format!("weather-mark:{m}"),
                sal,
                ctx(&[
                    ("mark", Value::from(m)),
                    ("bearing", Value::from(side.map_or("here", |b| BEARINGS[b]))),
                ]),
            ));
        }
        // A front on its way within three hours, seen on one side.
        let dark = self.conditions().1 == "dark";
        let coming = weather::fronts_between(w, now, now + 180)
            .into_iter()
            .filter(|f| f.kind != FrontKind::Heat && !f.covers(pos, now))
            .filter_map(|f| f.over(pos).map(|(a, _)| (a, f)))
            .filter(|(a, _)| *a > now && *a <= now + 180)
            .min_by_key(|(a, f)| (*a, f.id));
        if let Some((at, f)) = coming {
            if !dark || f.kind == FrontKind::Storm {
                let (cx, cy) = f.centre(now);
                let from = bearing(pos, Pos::new(cx as i32, cy as i32)).unwrap_or(0);
                let kind = match f.kind {
                    FrontKind::Storm => "storm",
                    FrontKind::Fog => "fog",
                    _ if weather::mean_air(w, x, y, now) < 1.0 => "snow",
                    _ => "rain",
                };
                let sal = match response {
                    Response::Around => 34.0,
                    _ => 12.0,
                } + if kind == "storm" { 10.0 } else { 0.0 };
                out.push(Fact::new(
                    "weather.coming",
                    format!("weather-coming:{}", f.id),
                    sal,
                    ctx(&[
                        ("kind", Value::from(kind)),
                        ("bearing", Value::from(BEARINGS[from])),
                        ("soon", Value::Bool(at <= now + 60)),
                        ("dark", Value::Bool(dark)),
                    ]),
                ));
            }
        }
    }
}
