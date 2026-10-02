//! Rivers, lakes and crossings.
//!
//! Depressions are filled with a tiny slope (priority flood), so every cell
//! has a strictly lower neighbour to drain into and every river reaches the
//! edge of the world: the sea for an island, the gap for a basin. Where the
//! filling is deep, a lake forms.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::Serialize;

use crate::terrain::{Biome, Grid, Terrain};

/// Flow (in cells of rain) above which a channel is a river.
pub const RIVER_FLOW: u32 = 140;
/// Depth of filling, in metres, above which a hollow holds a lake.
const LAKE_DEPTH: f64 = 1.5;
/// Filling shallower than this is ordinary ground; deeper is pooled water.
const SHALLOW: f64 = 0.3;

/// Drainage of a world.
#[derive(Debug, Clone, Serialize)]
pub struct Water {
    /// Index of the cell each cell drains into; `u32::MAX` at the edge.
    pub down: Grid<u32>,
    /// Rain gathered by each cell from everything upstream.
    pub flow: Grid<u32>,
    /// Water surface height after filling hollows.
    #[serde(skip)]
    pub filled: Grid<f64>,
    /// Cells where a river can be waded: narrow and gentle.
    pub ford: Grid<bool>,
    /// Cells in shallow hollows, where water pools instead of running.
    #[serde(skip)]
    pub pooled: Grid<bool>,
}

impl Water {
    /// Traces drainage and turns deep hollows into lakes (updating biomes).
    pub fn generate(t: &mut Terrain) -> Self {
        let s = t.height.size;
        let idx = |x: usize, y: usize| (y * s + x) as u32;
        let key = |h: f64| (h * 1000.0).round() as i64;
        let mut filled = t.height.clone();
        let mut down = Grid::new(s, u32::MAX);
        let mut done = Grid::new(s, false);
        let mut heap = BinaryHeap::new();
        for y in 0..s {
            for x in 0..s {
                if x == 0 || y == 0 || x == s - 1 || y == s - 1 {
                    heap.push(Reverse((key(*t.height.get(x, y)), idx(x, y))));
                    done.set(x, y, true);
                }
            }
        }
        while let Some(Reverse((_, c))) = heap.pop() {
            let (x, y) = (c as usize % s, c as usize / s);
            let here = *filled.get(x, y);
            let nbs: Vec<(usize, usize)> = filled.neighbours(x, y).collect();
            for (nx, ny) in nbs {
                if *done.get(nx, ny) {
                    continue;
                }
                done.set(nx, ny, true);
                // Fill hollows up to just above their outlet, so water always
                // has somewhere lower to go.
                let h = filled.get(nx, ny).max(here + 0.01);
                filled.set(nx, ny, h);
                down.set(nx, ny, c);
                heap.push(Reverse((key(h), idx(nx, ny))));
            }
        }

        // Rain runs downstream, highest cells first.
        let mut order: Vec<u32> = (0..(s * s) as u32).collect();
        order.sort_by_key(|&c| Reverse((key(filled.cells[c as usize]), c)));
        let mut flow = Grid::new(s, 0u32);
        for &c in &order {
            let rain = 1 + (t.moisture.cells[c as usize] * 3.0) as u32;
            flow.cells[c as usize] += rain;
            let d = down.cells[c as usize];
            if d != u32::MAX {
                flow.cells[d as usize] += flow.cells[c as usize];
            }
        }

        // Deep hollows hold lakes; shallow ones stay waterlogged as marsh,
        // where water spreads out instead of running in a channel.
        let mut pooled = Grid::new(s, false);
        for c in 0..s * s {
            let depth = filled.cells[c] - t.height.cells[c];
            if t.height.cells[c] < 0.0 {
                continue;
            }
            if depth > LAKE_DEPTH {
                t.biome.cells[c] = Biome::Lake;
            } else if depth > SHALLOW {
                pooled.cells[c] = true;
                if flow.cells[c] >= RIVER_FLOW {
                    t.biome.cells[c] = Biome::Marsh;
                }
            }
        }

        let mut ford = Grid::new(s, false);
        for y in 0..s {
            for x in 0..s {
                let f = *flow.get(x, y);
                if (RIVER_FLOW..RIVER_FLOW * 4).contains(&f)
                    && t.is_land(x, y)
                    && !*pooled.get(x, y)
                {
                    let here = *t.height.get(x, y);
                    let steep = filled
                        .neighbours(x, y)
                        .any(|(a, b)| (t.height.get(a, b) - here).abs() > 40.0);
                    ford.set(x, y, !steep);
                }
            }
        }
        Water {
            down,
            flow,
            filled,
            ford,
            pooled,
        }
    }

    pub fn is_river(&self, t: &Terrain, x: usize, y: usize) -> bool {
        *self.flow.get(x, y) >= RIVER_FLOW && t.is_land(x, y) && !*self.pooled.get(x, y)
    }

    /// Whether crossing this cell on foot needs a ford or a bridge.
    pub fn needs_crossing(&self, t: &Terrain, x: usize, y: usize) -> bool {
        self.is_river(t, x, y) && !*self.ford.get(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::SIZE;

    #[test]
    fn rivers_run_downhill_to_the_edge() {
        for seed in 0..6 {
            let mut t = Terrain::generate(seed);
            let w = Water::generate(&mut t);
            for start in (0..SIZE * SIZE).step_by(97) {
                let mut c = start;
                let mut steps = 0;
                while w.down.cells[c] != u32::MAX {
                    let next = w.down.cells[c] as usize;
                    assert!(
                        w.filled.cells[next] < w.filled.cells[c],
                        "seed {seed}: uphill at {c}"
                    );
                    c = next;
                    steps += 1;
                    assert!(steps < SIZE * SIZE);
                }
                let (x, y) = (c % SIZE, c / SIZE);
                assert!(
                    x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1,
                    "seed {seed}: stuck at {x},{y}"
                );
            }
        }
    }

    #[test]
    fn every_world_has_rivers() {
        for seed in 0..6 {
            let mut t = Terrain::generate(seed);
            let w = Water::generate(&mut t);
            let river = (0..SIZE * SIZE)
                .filter(|&c| w.is_river(&t, c % SIZE, c / SIZE))
                .count();
            assert!(river > 30, "seed {seed}: {river} river cells");
        }
    }
}
