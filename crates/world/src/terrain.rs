//! Land and climate: height, temperature, moisture and biomes.
//!
//! Only basic arithmetic and square roots are used, never `sin`, `exp` or
//! `powf`: those can differ in the last bit between platforms, and the same
//! seed must give the same world everywhere.

use serde::Serialize;

use scraped_lang::rng::{Rng, Stream};

/// Cells per side.
pub const SIZE: usize = 160;
/// Metres per cell side.
// DESIGN-Q: 160 cells of 300 m is 48 km across, two to three days' walk on
// rough ground rather than the "week's walk" the spec suggests. A larger map
// costs generation time in the browser; revisit with movement (M06).
pub const CELL_METRES: u32 = 300;

/// A square grid of values, row-major.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Grid<T> {
    pub size: usize,
    pub cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    pub fn new(size: usize, fill: T) -> Self {
        Grid {
            size,
            cells: vec![fill; size * size],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> &T {
        &self.cells[y * self.size + x]
    }

    pub fn set(&mut self, x: usize, y: usize, v: T) {
        let s = self.size;
        self.cells[y * s + x] = v;
    }

    /// The eight neighbours of a cell that lie on the grid.
    pub fn neighbours(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        const D: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let s = self.size as i32;
        D.iter().filter_map(move |&(dx, dy)| {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            (nx >= 0 && ny >= 0 && nx < s && ny < s).then_some((nx as usize, ny as usize))
        })
    }
}

/// Overall shape of the land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// Land surrounded by sea.
    Island,
    /// A valley ringed by mountains, drained through one gap.
    Basin,
}

/// Broad kinds of land. Descriptions are content slots (M06); these are ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Biome {
    Sea,
    Lake,
    Shore,
    Marsh,
    Grassland,
    Scrub,
    Desert,
    Forest,
    Pine,
    Tundra,
    Rock,
    Snow,
}

impl Biome {
    pub fn is_water(self) -> bool {
        matches!(self, Biome::Sea | Biome::Lake)
    }
}

/// Height, climate and biomes of a world.
#[derive(Debug, Clone, Serialize)]
pub struct Terrain {
    pub shape: Shape,
    /// Metres above sea level; below 0 is sea.
    pub height: Grid<f64>,
    /// Degrees Celsius, yearly mean.
    pub temperature: Grid<f64>,
    /// 0 (desert) to 1 (very wet).
    pub moisture: Grid<f64>,
    pub biome: Grid<Biome>,
    /// Direction the prevailing wind blows towards, in degrees (0 = north).
    pub wind: u32,
}

/// Deterministic lattice noise: a hash of integer coordinates.
fn lattice(seed: u64, x: i64, y: i64) -> f64 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    h ^= h >> 31;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 29;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in [0, 1].
fn value_noise(seed: u64, x: f64, y: f64) -> f64 {
    let (xi, yi) = (x.floor(), y.floor());
    let (tx, ty) = (smooth(x - xi), smooth(y - yi));
    let (xi, yi) = (xi as i64, yi as i64);
    let a = lattice(seed, xi, yi);
    let b = lattice(seed, xi + 1, yi);
    let c = lattice(seed, xi, yi + 1);
    let d = lattice(seed, xi + 1, yi + 1);
    let top = a + (b - a) * tx;
    let bottom = c + (d - c) * tx;
    top + (bottom - top) * ty
}

/// Fractal noise: several octaves of value noise, in about [0, 1].
pub(crate) fn fbm(seed: u64, x: f64, y: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed.wrapping_add(u64::from(o) * 7919), x * freq, y * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

impl Terrain {
    pub fn generate(seed: u64) -> Self {
        let mut rng = Rng::new(seed, Stream::World(1));
        let shape = if rng.chance(55) {
            Shape::Island
        } else {
            Shape::Basin
        };
        let noise_seed = u64::from(rng.below(u32::MAX)) << 20 | seed & 0xfffff;
        let n = SIZE as f64;
        // The basin drains through a gap in one side.
        let gap_side = rng.below(4);
        let gap_at = 0.3 + 0.4 * f64::from(rng.below(1000)) / 1000.0;

        let mut height = Grid::new(SIZE, 0.0);
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (u, v) = (x as f64 / n, y as f64 / n);
                let base = fbm(noise_seed, u * 6.0, v * 6.0, 5);
                let ridges = 1.0 - (2.0 * fbm(noise_seed ^ 0xabc, u * 3.0, v * 3.0, 3) - 1.0).abs();
                let (dx, dy) = (u - 0.5, v - 0.5);
                let r = (dx * dx + dy * dy).sqrt() * 2.0;
                let h = match shape {
                    Shape::Island => {
                        // Land in the middle, falling to sea at the edges.
                        let mask = 1.0 - r * r;
                        (base * 0.7 + ridges * 0.3) * 1600.0 * mask.max(-0.5) - 250.0 + mask * 600.0
                    }
                    Shape::Basin => {
                        // Mountains at the rim, a lowland in the middle, and
                        // one valley carved out through the rim.
                        let rim = (r * r * r).min(1.4);
                        let h = 60.0 + base * 300.0 + ridges * 250.0 * rim + rim * 900.0;
                        let (gx, gy) = match gap_side {
                            0 => (gap_at, 0.0),
                            1 => (1.0, gap_at),
                            2 => (gap_at, 1.0),
                            _ => (0.0, gap_at),
                        };
                        let (sx, sy) = (gx - 0.5, gy - 0.5);
                        let along = ((dx * sx + dy * sy) / (sx * sx + sy * sy)).clamp(0.0, 1.0);
                        let (px, py) = (dx - sx * along, dy - sy * along);
                        let off = (px * px + py * py).sqrt();
                        let valley = 20.0 + (1.0 - along) * 70.0 + off * 2500.0;
                        h.min(valley)
                    }
                };
                // Fine relief gathers runoff into channels; on a perfectly
                // smooth slope water would run in parallel sheets.
                let detail = (fbm(noise_seed ^ 0xd37a, u * 28.0, v * 28.0, 2) - 0.5) * 90.0;
                height.set(x, y, h + if h > 0.0 { detail } else { 0.0 });
            }
        }
        erode(&mut height);

        let wind = rng.below(8) * 45;
        let temperature = temperature(&height, &mut rng);
        let moisture = moisture(&height, wind, noise_seed);
        let biome = biomes(&height, &temperature, &moisture);
        Terrain {
            shape,
            height,
            temperature,
            moisture,
            biome,
            wind,
        }
    }

    pub fn is_land(&self, x: usize, y: usize) -> bool {
        !self.biome.get(x, y).is_water()
    }
}

/// Erosion-lite: a few passes that soften peaks and widen valleys.
fn erode(h: &mut Grid<f64>) {
    for _ in 0..2 {
        let prev = h.clone();
        for y in 0..h.size {
            for x in 0..h.size {
                let here = *prev.get(x, y);
                let nb: Vec<f64> = prev
                    .neighbours(x, y)
                    .map(|(a, b)| *prev.get(a, b))
                    .collect();
                let mean = nb.iter().sum::<f64>() / nb.len() as f64;
                // Material slides from steep high points into low ones.
                h.set(x, y, here + (mean - here) * 0.35);
            }
        }
    }
}

/// Colder to the north and with altitude (about 6.5 °C per 1000 m).
fn temperature(h: &Grid<f64>, rng: &mut Rng) -> Grid<f64> {
    let warmth = 8.0 + f64::from(rng.below(14));
    let mut t = Grid::new(h.size, 0.0);
    for y in 0..h.size {
        for x in 0..h.size {
            let latitude = y as f64 / h.size as f64; // 0 north, 1 south
            let alt = h.get(x, y).max(0.0);
            t.set(x, y, warmth - 6.0 + latitude * 12.0 - alt * 0.0065);
        }
    }
    t
}

/// Wind carries moisture inland; rising ground wrings it out, leaving a
/// rain shadow behind high ridges.
fn moisture(h: &Grid<f64>, wind: u32, noise_seed: u64) -> Grid<f64> {
    let s = h.size;
    let mut m = Grid::new(s, 0.0);
    // Walk each line of cells along the wind direction.
    let (dx, dy): (i32, i32) = match wind {
        0 => (0, -1),
        45 => (1, -1),
        90 => (1, 0),
        135 => (1, 1),
        180 => (0, 1),
        225 => (-1, 1),
        270 => (-1, 0),
        _ => (-1, -1),
    };
    let starts: Vec<(i32, i32)> = (0..s as i32)
        .flat_map(|i| {
            let mut v = Vec::new();
            if dx > 0 {
                v.push((0, i));
            } else if dx < 0 {
                v.push((s as i32 - 1, i));
            }
            if dy > 0 {
                v.push((i, 0));
            } else if dy < 0 {
                v.push((i, s as i32 - 1));
            }
            v
        })
        .collect();
    let mut visits = Grid::new(s, 0u32);
    for (sx, sy) in starts {
        let (mut x, mut y) = (sx, sy);
        let mut carried = 1.0f64;
        let mut last = *h.get(x as usize, y as usize);
        while x >= 0 && y >= 0 && x < s as i32 && y < s as i32 {
            let (ux, uy) = (x as usize, y as usize);
            let here = *h.get(ux, uy);
            if here < 0.0 {
                carried = (carried + 0.08).min(1.0); // the sea refills the air
            }
            let rise = (here - last).max(0.0);
            let rain = (carried * (0.02 + rise / 400.0)).min(carried);
            carried -= rain * 0.6;
            let wet = (carried * 0.7 + rain * 3.0).min(1.0);
            m.set(ux, uy, m.get(ux, uy) + wet);
            visits.set(ux, uy, visits.get(ux, uy) + 1);
            last = here;
            x += dx;
            y += dy;
        }
    }
    for y in 0..s {
        for x in 0..s {
            let v = *visits.get(x, y);
            let base = if v > 0 {
                m.get(x, y) / f64::from(v)
            } else {
                0.5
            };
            let n = fbm(noise_seed ^ 0x5eed, x as f64 / 20.0, y as f64 / 20.0, 3);
            m.set(x, y, (base * 0.8 + n * 0.3).clamp(0.0, 1.0));
        }
    }
    m
}

fn biomes(h: &Grid<f64>, t: &Grid<f64>, m: &Grid<f64>) -> Grid<Biome> {
    let mut b = Grid::new(h.size, Biome::Grassland);
    for y in 0..h.size {
        for x in 0..h.size {
            let (alt, temp, wet) = (*h.get(x, y), *t.get(x, y), *m.get(x, y));
            let biome = if alt < 0.0 {
                Biome::Sea
            } else if alt < 12.0 {
                if wet > 0.65 {
                    Biome::Marsh
                } else {
                    Biome::Shore
                }
            } else if temp < -4.0 {
                Biome::Snow
            } else if alt > 1400.0 {
                Biome::Rock
            } else if temp < 1.0 {
                Biome::Tundra
            } else if wet < 0.22 {
                if temp > 14.0 {
                    Biome::Desert
                } else {
                    Biome::Scrub
                }
            } else if wet < 0.42 {
                Biome::Grassland
            } else if temp < 6.0 {
                Biome::Pine
            } else if wet > 0.75 && alt < 60.0 {
                Biome::Marsh
            } else {
                Biome::Forest
            };
            b.set(x, y, biome);
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_varied() {
        let a = Terrain::generate(1);
        let b = Terrain::generate(1);
        assert_eq!(a.height, b.height);
        let shapes: std::collections::BTreeSet<String> = (0..12)
            .map(|s| format!("{:?}", Terrain::generate(s).shape))
            .collect();
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn islands_have_sea_at_the_edge_and_land_in_the_middle() {
        for seed in 0..20 {
            let t = Terrain::generate(seed);
            if t.shape != Shape::Island {
                continue;
            }
            assert_eq!(*t.biome.get(0, 0), Biome::Sea, "seed {seed}");
            let land = t.biome.cells.iter().filter(|b| !b.is_water()).count();
            assert!(
                land > SIZE * SIZE / 5,
                "seed {seed}: only {land} land cells"
            );
        }
    }

    #[test]
    fn several_biomes_appear() {
        let kinds: std::collections::BTreeSet<Biome> = (0..6)
            .flat_map(|s| Terrain::generate(s).biome.cells)
            .collect();
        assert!(kinds.len() >= 7, "{kinds:?}");
    }
}
