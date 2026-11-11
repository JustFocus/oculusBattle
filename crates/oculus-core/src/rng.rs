//! Small seedable PRNG (SplitMix64). Deterministic and dependency-free.

/// SplitMix64 generator: fast, tiny state, good statistical quality for games.
#[derive(Debug, Clone)]
pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub(crate) fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform float in `[0, 1)`.
    pub(crate) fn next_f32(&mut self) -> f32 {
        // 24 bits fit exactly in an f32 mantissa, so the result is never 1.0.
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform integer in `[0, n)`. Returns 0 when `n` is 0.
    pub(crate) fn below(&mut self, n: usize) -> usize {
        ((u64::from(self.next_u32()) * n as u64) >> 32) as usize
    }

    /// Fisher-Yates shuffle in place.
    pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_ne!(Rng::new(7).next_u32(), Rng::new(8).next_u32());
    }

    #[test]
    fn floats_are_in_unit_interval() {
        let mut rng = Rng::new(1);
        for _ in 0..10_000 {
            let f = rng.next_f32();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = Rng::new(2);
        let mut seen = [false; 5];
        for _ in 0..1000 {
            let v = rng.below(5);
            assert!(v < 5);
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = Rng::new(3);
        let mut items: Vec<u32> = (0..20).collect();
        rng.shuffle(&mut items);
        let mut sorted = items.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_ne!(items, sorted);
    }
}
