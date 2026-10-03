//! The rock under the land (D03): a light layer of rock types by region,
//! so the land has consistent character. Caves sit in limestone, slate
//! is quarried where slate lies, warm springs rise through basalt.
//!
//! Regions come from slow noise, nudged by height: the highest ground is
//! old granite, wet lowlands lie on clay laid down by rivers, and dry
//! country is sandstone. Like the terrain, only `+ - * /` are used.

use serde::Serialize;

use crate::terrain::{fbm, Biome, Grid, Terrain};
use crate::water::Water;

/// A kind of rock. Ids for content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rock {
    Granite,
    Slate,
    Limestone,
    Sandstone,
    Basalt,
    Clay,
}

impl Rock {
    pub const ALL: [Rock; 6] = [
        Rock::Granite,
        Rock::Slate,
        Rock::Limestone,
        Rock::Sandstone,
        Rock::Basalt,
        Rock::Clay,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Rock::Granite => "granite",
            Rock::Slate => "slate",
            Rock::Limestone => "limestone",
            Rock::Sandstone => "sandstone",
            Rock::Basalt => "basalt",
            Rock::Clay => "clay",
        }
    }

    /// Water dissolves it, so it holds caves and sinkholes.
    pub fn soluble(self) -> bool {
        self == Rock::Limestone
    }

    /// Good building or quarrying stone.
    pub fn quarried(self) -> bool {
        matches!(
            self,
            Rock::Granite | Rock::Slate | Rock::Limestone | Rock::Sandstone
        )
    }
}

/// The rock under every cell.
#[derive(Debug, Clone, Serialize)]
pub struct Geology {
    pub rock: Grid<Rock>,
}

impl Geology {
    pub fn generate(seed: u64, t: &Terrain, w: &Water) -> Self {
        let s = t.height.size;
        let n = s as f64;
        let ns = seed ^ 0x6e01_0617;
        let mut rock = Grid::new(s, Rock::Clay);
        for y in 0..s {
            for x in 0..s {
                let (u, v) = (x as f64 / n, y as f64 / n);
                // Two slow fields pick a region's rock; a third, sparse
                // one marks old volcanic ground.
                let a = fbm(ns, u * 3.0, v * 3.0, 3);
                let b = fbm(ns ^ 0x55, u * 3.0, v * 3.0, 3);
                let volcanic = fbm(ns ^ 0xba5a, u * 5.0, v * 5.0, 2);
                let h = *t.height.get(x, y);
                let biome = *t.biome.get(x, y);
                let r = if volcanic > 0.72 {
                    Rock::Basalt
                } else if h > 1100.0 || (h > 600.0 && a > 0.6) {
                    Rock::Granite
                } else if h < 40.0
                    && (*t.moisture.get(x, y) > 0.55
                        || w.is_river(t, x, y)
                        || biome == Biome::Marsh)
                {
                    Rock::Clay
                } else if matches!(biome, Biome::Desert | Biome::Scrub) && b > 0.4 {
                    Rock::Sandstone
                } else if a < 0.45 {
                    Rock::Limestone
                } else if b > 0.55 {
                    Rock::Slate
                } else if b < 0.4 {
                    Rock::Sandstone
                } else {
                    Rock::Limestone
                };
                rock.set(x, y, r);
            }
        }
        Geology { rock }
    }

    pub fn at(&self, x: usize, y: usize) -> Rock {
        *self.rock.get(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn several_rocks_in_regions() {
        let mut t = Terrain::generate(3);
        let w = Water::generate(&mut t);
        let g = Geology::generate(3, &t, &w);
        let kinds: std::collections::BTreeSet<Rock> = g.rock.cells.iter().copied().collect();
        assert!(kinds.len() >= 4, "{kinds:?}");
        // Regions, not salt and pepper: most cells match a neighbour.
        let s = g.rock.size;
        let alike = (1..s - 1)
            .flat_map(|y| (1..s - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| g.at(x, y) == g.at(x + 1, y))
            .count();
        assert!(alike * 10 > (s - 2) * (s - 2) * 8);
    }
}
