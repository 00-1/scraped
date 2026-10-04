//! Natural wonders (D06): oddities with natural causes, placed where the
//! land makes them. Marsh lights over warm bogs, dunes that boom in the
//! wind, a tidal bore running up a river mouth, steam rising from basalt,
//! a gorge that throws back every sound, mirages over salt and sand,
//! aurora over the cold north, a sea shore that glows at night, fogbows,
//! an arch that sings in the wind.
//!
//! They give the world wonder of its own and make magic harder to spot:
//! not every strange thing is writing. None of them has a writing cause.

use serde::Serialize;

use crate::features::Feature;
use crate::geology::{Geology, Rock};
use crate::history::Cell;
use crate::terrain::{Biome, Terrain, SIZE};
use crate::water::Water;

/// Every kind, with when it shows. Ids for content.
pub const KINDS: &[&str] = &[
    "marsh lights",
    "booming dunes",
    "tidal bore",
    "steam vents",
    "echoing gorge",
    "mirage",
    "aurora",
    "glowing shore",
    "fogbow",
    "singing arch",
];

/// One natural wonder.
#[derive(Debug, Clone, Serialize)]
pub struct Phenomenon {
    pub id: usize,
    pub kind: &'static str,
    pub cell: Cell,
    /// Why it is here, in the land's terms ("warm bog", "dune sand",
    /// "river mouth"...): never writing.
    pub cause: &'static str,
    /// The feature it belongs to, if any (a gorge, an arch).
    pub feature: Option<usize>,
}

/// The natural causes a phenomenon may have.
pub const CAUSES: &[&str] = &[
    "warm bog",
    "dune sand",
    "river mouth",
    "basalt heat",
    "gorge walls",
    "hot flat ground",
    "cold north",
    "warm sea",
    "fog and sun",
    "wind through the arch",
];

/// A candidate: kind, cause, score, cell and feature.
type Cand = (&'static str, &'static str, u64, usize, usize, Option<usize>);

fn mix(parts: &[u64]) -> u64 {
    let mut v: u64 = 0x51ed_270b_2730_e4b1;
    for &p in parts {
        v ^= p;
        v = v.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        v ^= v >> 31;
    }
    v
}

/// Places the wonders of a world.
// DESIGN-Q: at most two of each kind (one aurora), at least 15 cells
// (4.5 km) apart; candidates scored by how strongly the land makes them.
pub fn place(
    seed: u64,
    t: &Terrain,
    w: &Water,
    g: &Geology,
    features: &[Feature],
) -> Vec<Phenomenon> {
    // (kind, cause, score, x, y, feature)
    let mut cands: Vec<Cand> = Vec::new();
    let biome = |x: usize, y: usize| *t.biome.get(x, y);
    let near = |x: usize, y: usize, r: i64, f: &dyn Fn(usize, usize) -> bool| -> bool {
        for dy in -r..=r {
            for dx in -r..=r {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if (0..SIZE as i64).contains(&nx)
                    && (0..SIZE as i64).contains(&ny)
                    && f(nx as usize, ny as usize)
                {
                    return true;
                }
            }
        }
        false
    };
    let mut coldest: Option<(f64, usize, usize)> = None;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let h = mix(&[seed, x as u64, y as u64]) % 1000;
            let temp = *t.temperature.get(x, y);
            let b = biome(x, y);
            match b {
                Biome::Marsh if temp > 6.0 => {
                    cands.push(("marsh lights", "warm bog", 1000 + h, x, y, None))
                }
                Biome::Desert => {
                    cands.push(("booming dunes", "dune sand", 800 + h, x, y, None));
                    if temp > 16.0 {
                        cands.push(("mirage", "hot flat ground", 600 + h, x, y, None));
                    }
                }
                Biome::Shore if temp > 14.0 && near(x, y, 1, &|a, c| biome(a, c) == Biome::Sea) => {
                    cands.push(("glowing shore", "warm sea", 700 + h, x, y, None))
                }
                _ => {}
            }
            if w.is_river(t, x, y) && near(x, y, 1, &|a, c| biome(a, c) == Biome::Sea) {
                cands.push(("tidal bore", "river mouth", 1200 + h, x, y, None));
            }
            if t.is_land(x, y) && g.at(x, y) == Rock::Basalt && *t.height.get(x, y) > 200.0 {
                cands.push(("steam vents", "basalt heat", 500 + h, x, y, None));
            }
            if matches!(b, Biome::Marsh | Biome::Shore)
                && near(x, y, 2, &|a, c| biome(a, c) == Biome::Sea)
            {
                cands.push(("fogbow", "fog and sun", 300 + h, x, y, None));
            }
            if t.is_land(x, y) && coldest.is_none_or(|(c, _, _)| temp < c) {
                coldest = Some((temp, x, y));
            }
        }
    }
    for (i, f) in features.iter().enumerate() {
        let (x, y) = (f.cell.ux(), f.cell.uy());
        let h = mix(&[seed, 0xfea7, i as u64]) % 1000;
        match f.kind {
            "gorge" => cands.push(("echoing gorge", "gorge walls", 1500 + h, x, y, Some(f.id))),
            "natural arch" => cands.push((
                "singing arch",
                "wind through the arch",
                1500 + h,
                x,
                y,
                Some(f.id),
            )),
            "salt flat" => cands.push(("mirage", "hot flat ground", 1400 + h, x, y, Some(f.id))),
            "warm spring" => cands.push(("steam vents", "basalt heat", 1300 + h, x, y, Some(f.id))),
            _ => {}
        }
    }
    // Aurora over the cold north, if the land has any cold.
    if let Some((temp, x, y)) = coldest {
        if temp < 2.0 {
            cands.push(("aurora", "cold north", 2000, x, y, None));
        }
    }
    cands.sort_by(|a, b| b.2.cmp(&a.2).then(a.3.cmp(&b.3)).then(a.4.cmp(&b.4)));
    let mut out: Vec<Phenomenon> = Vec::new();
    for (kind, cause, _, x, y, feature) in cands {
        let max = if kind == "aurora" { 1 } else { 2 };
        if out.iter().filter(|p| p.kind == kind).count() >= max {
            continue;
        }
        let close = out.iter().any(|p| {
            let (dx, dy) = (p.cell.ux() as i64 - x as i64, p.cell.uy() as i64 - y as i64);
            dx * dx + dy * dy < 15 * 15
        });
        if close {
            continue;
        }
        out.push(Phenomenon {
            id: out.len(),
            kind,
            cell: Cell::new(x, y),
            cause,
            feature,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::World;

    #[test]
    fn worlds_have_five_wonders_with_natural_causes() {
        for seed in [1u64, 2, 3, 42, 9001] {
            let w = World::generate(seed);
            assert!(w.phenomena.len() >= 5, "seed {seed}: {:?}", w.phenomena);
            for p in &w.phenomena {
                assert!(super::CAUSES.contains(&p.cause));
                assert!(super::KINDS.contains(&p.kind));
            }
        }
    }
}
