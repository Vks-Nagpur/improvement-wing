//! Presentation units (₹, hundreds, thousands, lakhs, millions, crores) and
//! number formatting. Display only – calculations always use exact paise.

use crate::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    #[default]
    Rupees,
    Hundreds,
    Thousands,
    Lakhs,
    Millions,
    Crores,
}

impl Unit {
    pub fn paise(self) -> i64 {
        match self {
            Unit::Rupees => 100,
            Unit::Hundreds => 10_000,
            Unit::Thousands => 100_000,
            Unit::Lakhs => 10_000_000,
            Unit::Millions => 100_000_000,
            Unit::Crores => 1_000_000_000,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Unit::Rupees => "₹",
            Unit::Hundreds => "₹ in hundreds",
            Unit::Thousands => "₹ in thousands",
            Unit::Lakhs => "₹ in lakhs",
            Unit::Millions => "₹ in millions",
            Unit::Crores => "₹ in crores",
        }
    }
    /// Wording for "(All amounts in …)".
    pub fn long(self) -> &'static str {
        match self {
            Unit::Rupees => "₹",
            Unit::Hundreds => "₹ hundreds",
            Unit::Thousands => "₹ thousands",
            Unit::Lakhs => "₹ lakhs",
            Unit::Millions => "₹ millions",
            Unit::Crores => "₹ crores",
        }
    }
    pub fn parse(s: &str) -> Option<Unit> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "rupees" | "rs" | "inr" | "₹" => Unit::Rupees,
            "hundreds" => Unit::Hundreds,
            "thousands" | "000" => Unit::Thousands,
            "lakhs" | "lakh" | "lacs" => Unit::Lakhs,
            "millions" | "million" => Unit::Millions,
            "crores" | "crore" => Unit::Crores,
            _ => return None,
        })
    }
    /// Units Schedule III (Division I) allows for a company of this turnover:
    /// below ₹100 crore – hundreds, thousands, lakhs, millions (or decimals thereof);
    /// ₹100 crore or more – lakhs, millions, crores (or decimals thereof).
    pub fn allowed_for_company(self, turnover: Money) -> bool {
        let hundred_crore = Money::rupees(1_000_000_000);
        if turnover.abs() >= hundred_crore {
            matches!(self, Unit::Lakhs | Unit::Millions | Unit::Crores)
        } else {
            !matches!(self, Unit::Crores)
        }
    }
}

/// Smallest step (in paise) that a figure is rounded to.
pub fn granularity(unit: Unit, decimals: u8) -> i64 {
    let g = unit.paise() / 10i64.pow(decimals.min(2) as u32);
    g.max(1)
}

/// Round half away from zero to a multiple of `g` paise.
pub fn round_to(m: Money, g: i64) -> Money {
    if g <= 1 {
        return m;
    }
    let v = m.paise();
    let q = v / g;
    let r = v % g;
    let q = if 2 * r.abs() >= g { q + v.signum() } else { q };
    Money(q * g)
}

/// Format an already-rounded amount in the unit: Indian digit grouping,
/// negatives in brackets, nil as "-".
pub fn fmt_amount(m: Money, unit: Unit, decimals: u8) -> String {
    if m.is_zero() {
        return "-".into();
    }
    let decimals = decimals.min(2);
    let scale = unit.paise() / 10i64.pow(decimals as u32);
    let v = round_to(m, scale.max(1)).paise() / scale.max(1);
    let neg = v < 0;
    let a = v.unsigned_abs();
    let div = 10u64.pow(decimals as u32);
    let int = a / div;
    let frac = a % div;
    let digits = int.to_string();
    let grouped = if digits.len() <= 3 {
        digits
    } else {
        let (head, last3) = digits.split_at(digits.len() - 3);
        let mut parts = Vec::new();
        let mut i = head.len();
        while i > 0 {
            let s = i.saturating_sub(2);
            parts.push(&head[s..i]);
            i = s;
        }
        parts.reverse();
        format!("{},{}", parts.join(","), last3)
    };
    let body = if decimals > 0 {
        format!("{grouped}.{frac:0width$}", width = decimals as usize)
    } else {
        grouped
    };
    if neg {
        format!("({body})")
    } else {
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounding_and_format() {
        assert_eq!(round_to(Money(149), 100), Money(100));
        assert_eq!(round_to(Money(150), 100), Money(200));
        assert_eq!(round_to(Money(-150), 100), Money(-200));
        assert_eq!(
            fmt_amount(Money::rupees(12_345_678), Unit::Lakhs, 2),
            "123.46"
        );
        assert_eq!(
            fmt_amount(Money::rupees(-12_345_678), Unit::Rupees, 0),
            "(1,23,45,678)"
        );
        assert_eq!(
            fmt_amount(Money(123_456_789), Unit::Rupees, 2),
            "12,34,567.89"
        );
        assert_eq!(fmt_amount(Money::ZERO, Unit::Crores, 2), "-");
        assert_eq!(granularity(Unit::Lakhs, 2), 100_000);
        assert!(!Unit::Thousands.allowed_for_company(Money::rupees(2_000_000_000)));
        assert!(Unit::Thousands.allowed_for_company(Money::rupees(50_000_000)));
    }
}
