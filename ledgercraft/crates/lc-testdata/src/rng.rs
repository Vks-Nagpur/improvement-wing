//! Small deterministic RNG (xorshift64*), so every scenario is reproducible from its seed.

use lc_core::Money;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(hi >= lo);
        lo + (self.next_u64() % ((hi - lo + 1) as u64)) as i64
    }
    pub fn chance(&mut self, pct: u32) -> bool {
        (self.next_u64() % 100) < pct as u64
    }
    /// Random amount in rupees range, with random paise.
    pub fn money(&mut self, lo_rupees: i64, hi_rupees: i64) -> Money {
        Money(self.range(lo_rupees * 100, hi_rupees * 100))
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() as i64 - 1) as usize]
    }
}
