//! The backdrop behind the app's text: a reading room by lamplight. Soft
//! warm light drifts very slowly in the dark theme; faint mottling, like
//! old paper, in the light one.
//!
//! Decoration only. It depends on nothing but time and the theme, never on
//! the game (the game has no graphics), and it is kept faint enough that it
//! never costs the text any contrast. Frames are tiny (the app draws them
//! scaled up and blurred by filtering), so painting one costs little.

/// The strongest the backdrop ever gets, as alpha out of 255.
pub const MAX_ALPHA: u32 = 40;

fn hash(x: i32, y: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// A gentler fade than `smooth`, so cells don't show as squares.
fn quintic(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Value noise in 0..1.
fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor(), y.floor());
    let (fx, fy) = (quintic(x - xi), quintic(y - yi));
    let (xi, yi) = (xi as i32, yi as i32);
    let a = hash(xi, yi);
    let b = hash(xi + 1, yi);
    let c = hash(xi, yi + 1);
    let d = hash(xi + 1, yi + 1);
    let top = a + (b - a) * fx;
    let bottom = c + (d - c) * fx;
    top + (bottom - top) * fy
}

/// Three octaves of noise, in 0..1.
/// Each octave is turned a little, so no grid lines line up.
fn fbm(x: f32, y: f32) -> f32 {
    let (x2, y2) = (x * 1.6 - y * 1.2 + 5.2, x * 1.2 + y * 1.6 + 1.3);
    let (x3, y3) = (x2 * 1.6 - y2 * 1.2 + 9.1, x2 * 1.2 + y2 * 1.6 + 7.7);
    (noise(x, y) * 4.0 + noise(x2, y2) * 2.0 + noise(x3, y3)) / 7.0
}

/// A slow rise and fall between 0 and 1 (a triangle eased smooth).
fn breathe(t: f32, period: f32) -> f32 {
    let p = (t / period).rem_euclid(1.0);
    smooth(if p < 0.5 { p * 2.0 } else { 2.0 - p * 2.0 })
}

/// Paints one frame: `out` is `w × h` ARGB pixels, row by row; `t` is in
/// seconds.
pub fn paint(out: &mut [u32], w: usize, h: usize, t: f32, dark: bool) {
    let aspect = h as f32 / w.max(1) as f32;
    // DESIGN-Q: the lamp's colour and the paper's ink; a minute-long breath.
    let (r, g, b) = if dark { (227, 164, 99) } else { (118, 86, 48) };
    let pulse = 0.85 + 0.15 * breathe(t, 61.0);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32 / w.max(1) as f32, y as f32 / h.max(1) as f32);
            let (u, v) = (fx * 1.7, fy * 1.7 * aspect);
            let a = fbm(u + t * 0.009, v + t * 0.006);
            let c = fbm(u * 0.6 - t * 0.007 + 3.0, v * 0.6 + t * 0.004 + 11.0);
            let n = smooth(((a * 0.6 + c * 0.4 - 0.3) / 0.45).clamp(0.0, 1.0));
            let shape = if dark {
                // A lamp's glow from above, stirred by the drift.
                let (dx, dy) = (fx - 0.5, (fy - 0.15) * aspect * 0.55);
                let lamp = (1.0 - (dx * dx + dy * dy) * 2.2).max(0.0);
                lamp * lamp * (0.45 + 0.55 * n)
            } else {
                // Mottled paper, a touch darker towards the edges.
                let (dx, dy) = (fx - 0.5, fy - 0.5);
                let edge = ((dx * dx + dy * dy) * 2.0).min(1.0);
                n * 0.55 + edge * edge * 0.45
            };
            let alpha = ((shape * pulse).clamp(0.0, 1.0) * MAX_ALPHA as f32) as u32;
            out[y * w + x] = (alpha << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faint_varied_and_moving() {
        let (w, h) = (36, 72);
        let mut a = vec![0; w * h];
        let mut b = vec![0; w * h];
        for dark in [false, true] {
            paint(&mut a, w, h, 10.0, dark);
            let alphas: Vec<u32> = a.iter().map(|p| p >> 24).collect();
            assert!(alphas.iter().all(|&x| x <= MAX_ALPHA));
            assert!(alphas.iter().max() > alphas.iter().min(), "it has texture");
            paint(&mut b, w, h, 10.0, dark);
            assert_eq!(a, b, "the same moment paints the same");
            paint(&mut b, w, h, 40.0, dark);
            assert_ne!(a, b, "it drifts");
        }
        // Slow: a tenth of a second changes almost nothing.
        paint(&mut a, w, h, 100.0, true);
        paint(&mut b, w, h, 100.1, true);
        let moved = a
            .iter()
            .zip(&b)
            .filter(|(x, y)| ((*x >> 24) as i32 - (*y >> 24) as i32).abs() > 1)
            .count();
        assert!(moved * 20 < w * h, "{moved} pixels jumped");
    }
}
