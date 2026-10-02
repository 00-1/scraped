//! Seeded randomness with platform-independent helpers.
//!
//! Determinism is non-negotiable, so every random decision goes through this
//! wrapper. It only ever pulls raw `u32`s from ChaCha8 and does its own range
//! reduction, so results never depend on pointer width (wasm32 vs x86_64) or
//! on how a `rand` version happens to implement `gen_range`.

use rand_chacha::rand_core::{Rng as _, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Independent random streams, one per generation stage.
///
/// Each stage draws from its own ChaCha stream, so changing how many numbers
/// one stage consumes never shifts the output of another stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Phonology,
    Morphology,
    Syntax,
    Lexicon,
    /// The recurring people named across a corpus.
    Cast,
    /// One stream per inscription, so `--count 10` is a prefix of `--count 40`.
    Inscription(u32),
    Script,
    Numerals,
    /// Fused affix forms (a difficulty dial).
    Fusion,
    /// Fixed words of the potent register.
    Register,
    /// The sound changes leading to an era.
    SoundChange(u32),
    /// Everything else that happens between eras: replaced words, eroded
    /// affixes, script change.
    Evolution(u32),
    /// World generation stages, numbered by the world crate.
    World(u32),
}

impl Stream {
    fn id(self) -> u64 {
        match self {
            Stream::Phonology => 1,
            Stream::Morphology => 3,
            Stream::Syntax => 4,
            Stream::Lexicon => 5,
            Stream::Cast => 6,
            Stream::Inscription(i) => 1_000 + u64::from(i),
            Stream::Script => 7,
            Stream::Fusion => 10,
            Stream::Numerals => 8,
            Stream::Register => 9,
            Stream::SoundChange(e) => 100 + u64::from(e),
            Stream::Evolution(e) => 200 + u64::from(e),
            Stream::World(n) => 500 + u64::from(n),
        }
    }
}

/// A deterministic random source for one generation stage.
pub struct Rng {
    inner: ChaCha8Rng,
}

impl Rng {
    /// Creates the generator for `stream` of `seed`.
    pub fn new(seed: u64, stream: Stream) -> Self {
        // Build the 32-byte key ourselves rather than relying on
        // `seed_from_u64`, whose expansion is a library detail.
        let mut key = [0u8; 32];
        key[..8].copy_from_slice(&seed.to_le_bytes());
        key[8..16].copy_from_slice(b"scraped!");
        let mut inner = ChaCha8Rng::from_seed(key);
        inner.set_stream(stream.id());
        Rng { inner }
    }

    /// Uniform integer in `0..n`. Panics if `n == 0`.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "below(0)");
        // Rejection sampling: discard the incomplete top block so every
        // residue is equally likely.
        let zone = u32::MAX - (u32::MAX % n);
        loop {
            let x = self.inner.next_u32();
            if x < zone {
                return x % n;
            }
        }
    }

    /// Uniform index into a slice of length `len`.
    pub fn index(&mut self, len: usize) -> usize {
        let n = u32::try_from(len).expect("slice too long for rng");
        self.below(n) as usize
    }

    /// Uniform integer in the inclusive range `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        assert!(lo <= hi);
        lo + self.below(hi - lo + 1)
    }

    /// True with probability `percent`/100.
    pub fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }

    /// Picks a uniformly random element.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.index(items.len())]
    }

    /// Picks an index with probability proportional to `weights[i]`.
    /// Panics if all weights are zero.
    pub fn weighted_index(&mut self, weights: &[u32]) -> usize {
        let total: u32 = weights.iter().sum();
        let mut roll = self.below(total);
        for (i, &w) in weights.iter().enumerate() {
            if roll < w {
                return i;
            }
            roll -= w;
        }
        unreachable!("roll exceeded total weight")
    }

    /// Picks a value from `(value, weight)` pairs.
    pub fn weighted<T: Copy>(&mut self, options: &[(T, u32)]) -> T {
        let weights: Vec<u32> = options.iter().map(|&(_, w)| w).collect();
        options[self.weighted_index(&weights)].0
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.index(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED: [u32; 5] = [491, 801, 330, 180, 18];

    #[test]
    fn same_seed_same_numbers() {
        let mut a = Rng::new(7, Stream::Lexicon);
        let mut b = Rng::new(7, Stream::Lexicon);
        for _ in 0..100 {
            assert_eq!(a.below(1000), b.below(1000));
        }
    }

    #[test]
    fn streams_are_independent() {
        let mut a = Rng::new(7, Stream::Lexicon);
        let mut b = Rng::new(7, Stream::Syntax);
        let xs: Vec<u32> = (0..20).map(|_| a.below(1_000_000)).collect();
        let ys: Vec<u32> = (0..20).map(|_| b.below(1_000_000)).collect();
        assert_ne!(xs, ys);
    }

    #[test]
    fn known_values_are_stable() {
        // Pinned output: if this changes, every seed's language changes.
        let mut r = Rng::new(42, Stream::Phonology);
        let xs: Vec<u32> = (0..5).map(|_| r.below(1000)).collect();
        assert_eq!(xs, PINNED);
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Rng::new(1, Stream::Cast);
        for n in 1..50 {
            for _ in 0..20 {
                assert!(r.below(n) < n);
            }
        }
    }

    #[test]
    fn weighted_never_picks_zero_weight() {
        let mut r = Rng::new(3, Stream::Cast);
        for _ in 0..200 {
            let i = r.weighted_index(&[0, 5, 0, 1]);
            assert!(i == 1 || i == 3);
        }
    }
}
