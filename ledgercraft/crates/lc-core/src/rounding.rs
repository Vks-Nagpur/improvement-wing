//! Rounding that keeps totals exact (largest-remainder method).
//!
//! Each figure is rounded to the presentation step; if the rounded figures no
//! longer add up to the rounded target, the difference is moved – one step at a
//! time – to the figures that lost (or gained) most in rounding. Totals are then
//! always the plain sum of the printed figures.

use crate::money::Money;
use crate::units::round_to;

/// Round `values` to multiples of `g` so that they sum exactly to `target`
/// (which must itself be a multiple of `g`).
pub fn round_to_target(values: &[Money], g: i64, target: Money) -> Vec<Money> {
    assert!(g >= 1);
    assert_eq!(
        target.paise() % g,
        0,
        "target must be a multiple of the rounding step"
    );
    let mut out: Vec<Money> = values.iter().map(|v| round_to(*v, g)).collect();
    if values.is_empty() {
        assert!(target.is_zero(), "no figures to carry a non-zero total");
        return out;
    }
    let sum: Money = out.iter().copied().sum();
    let mut steps = (target - sum).paise() / g;
    if steps == 0 {
        return out;
    }
    // Remainder each figure lost in rounding (positive = rounded down).
    let mut order: Vec<usize> = (0..values.len()).collect();
    let up = steps > 0;
    order.sort_by(|&a, &b| {
        let ra = (values[a] - out[a]).paise();
        let rb = (values[b] - out[b]).paise();
        let (ka, kb) = if up { (rb, ra) } else { (ra, rb) };
        ka.cmp(&kb).then(a.cmp(&b))
    });
    let mut k = 0;
    while steps != 0 {
        let i = order[k % order.len()];
        if up {
            out[i] += Money(g);
            steps -= 1;
        } else {
            out[i] -= Money(g);
            steps += 1;
        }
        k += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_total() {
        // 3 × 33.33 = 99.99 → in rupees: 33 + 33 + 33 = 99, target 100 → one becomes 34.
        let v = vec![Money(3333), Money(3333), Money(3334)];
        let r = round_to_target(&v, 100, Money(10_000));
        assert_eq!(r.iter().copied().sum::<Money>(), Money(10_000));
        assert_eq!(r, vec![Money(3300), Money(3300), Money(3400)]);
    }

    #[test]
    fn signed_zero_sum() {
        let v = vec![Money(150), Money(150), Money(-300)];
        let r = round_to_target(&v, 100, Money::ZERO);
        assert_eq!(r.iter().copied().sum::<Money>(), Money::ZERO);
    }

    #[test]
    fn random_sets_always_hit_target() {
        let mut x: u64 = 12345;
        for _ in 0..2000 {
            let n = (x % 40) as usize + 1;
            let mut vals = Vec::new();
            for _ in 0..n {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                vals.push(Money((x % 20_000_000) as i64 - 10_000_000));
            }
            let total: Money = vals.iter().copied().sum();
            for g in [100, 100_000, 10_000_000] {
                let t = round_to(total, g);
                let r = round_to_target(&vals, g, t);
                assert_eq!(r.iter().copied().sum::<Money>(), t);
                for (a, b) in vals.iter().zip(&r) {
                    assert!((a.paise() - b.paise()).abs() <= g * 2, "moved too far");
                    assert_eq!(b.paise() % g, 0);
                }
            }
        }
    }
}
