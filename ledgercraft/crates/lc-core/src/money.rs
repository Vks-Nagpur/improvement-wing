//! Exact money type. Stored as integer paise; Dr is positive, Cr negative.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Money(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyError(pub String);

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MoneyError {}

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn rupees(r: i64) -> Money {
        Money(r * 100)
    }
    pub const fn paise(self) -> i64 {
        self.0
    }
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
    pub fn abs(self) -> Money {
        Money(self.0.abs())
    }
    pub fn is_dr(self) -> bool {
        self.0 > 0
    }
    pub fn is_cr(self) -> bool {
        self.0 < 0
    }
    /// Value for display / spreadsheet output only. Never used in calculations.
    pub fn as_f64(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Convert a spreadsheet number to paise. Rejects values with more than
    /// two decimals (beyond floating-point noise) instead of silently rounding.
    pub fn from_f64_checked(v: f64) -> Result<Money, MoneyError> {
        if !v.is_finite() {
            return Err(MoneyError(format!("not a finite number: {v}")));
        }
        let scaled = v * 100.0;
        let rounded = scaled.round();
        if (scaled - rounded).abs() > 0.001 {
            return Err(MoneyError(format!(
                "amount {v} has more than 2 decimal places"
            )));
        }
        if rounded.abs() > 9.0e15 {
            return Err(MoneyError(format!("amount {v} is too large")));
        }
        Ok(Money(rounded as i64))
    }

    /// Parse text such as `1,23,456.50`, `₹ 500`, `(1,000)`, `1,234.00 Dr`, `-75`, `500 Cr`.
    pub fn parse(s: &str) -> Result<Money, MoneyError> {
        let mut t: String = s
            .chars()
            .filter(|c| !matches!(c, ',' | '₹' | ' ' | '\u{a0}'))
            .collect();
        if t.is_empty() || t == "-" {
            return Ok(Money::ZERO);
        }
        let lower = t.to_ascii_lowercase();
        let mut sign = 1i64;
        if lower.ends_with("dr") {
            t.truncate(t.len() - 2);
        } else if lower.ends_with("cr") {
            t.truncate(t.len() - 2);
            sign = -sign;
        }
        if t.starts_with("rs.") || t.starts_with("Rs.") {
            t = t[3..].to_string();
        }
        if t.starts_with('(') && t.ends_with(')') {
            t = t[1..t.len() - 1].to_string();
            sign = -sign;
        }
        if let Some(rest) = t.strip_prefix('-') {
            t = rest.to_string();
            sign = -sign;
        }
        let (int_part, frac_part) = match t.split_once('.') {
            Some((i, f)) => (i, f),
            None => (t.as_str(), ""),
        };
        if int_part.is_empty() && frac_part.is_empty() {
            return Err(MoneyError(format!("not an amount: '{s}'")));
        }
        if !int_part.chars().all(|c| c.is_ascii_digit())
            || !frac_part.chars().all(|c| c.is_ascii_digit())
        {
            return Err(MoneyError(format!("not an amount: '{s}'")));
        }
        if frac_part.len() > 2 && frac_part[2..].chars().any(|c| c != '0') {
            return Err(MoneyError(format!(
                "amount '{s}' has more than 2 decimal places"
            )));
        }
        let rupees: i64 = if int_part.is_empty() {
            0
        } else {
            int_part
                .parse()
                .map_err(|_| MoneyError(format!("amount too large: '{s}'")))?
        };
        let mut frac = frac_part.chars().take(2).collect::<String>();
        while frac.len() < 2 {
            frac.push('0');
        }
        let paise: i64 = frac.parse().unwrap_or(0);
        let total = rupees
            .checked_mul(100)
            .and_then(|r| r.checked_add(paise))
            .ok_or_else(|| MoneyError(format!("amount too large: '{s}'")))?;
        Ok(Money(sign * total))
    }

    /// Indian grouping, e.g. `12,34,567.89`; negatives in brackets.
    pub fn fmt_indian(self) -> String {
        let neg = self.0 < 0;
        let v = self.0.unsigned_abs();
        let rupees = v / 100;
        let paise = v % 100;
        let digits = rupees.to_string();
        let grouped = if digits.len() <= 3 {
            digits
        } else {
            let (head, last3) = digits.split_at(digits.len() - 3);
            let mut parts = Vec::new();
            let head_bytes = head.as_bytes();
            let mut i = head_bytes.len();
            while i > 0 {
                let start = i.saturating_sub(2);
                parts.push(&head[start..i]);
                i = start;
            }
            parts.reverse();
            format!("{},{}", parts.join(","), last3)
        };
        if neg {
            format!("({grouped}.{paise:02})")
        } else {
            format!("{grouped}.{paise:02}")
        }
    }

    /// `1,234.00 Dr` / `1,234.00 Cr` style.
    pub fn fmt_drcr(self) -> String {
        if self.0 < 0 {
            format!("{} Cr", Money(-self.0).fmt_indian())
        } else {
            format!("{} Dr", self.fmt_indian())
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.fmt_indian())
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        Money(self.0.checked_add(o.0).expect("money overflow"))
    }
}
impl Sub for Money {
    type Output = Money;
    fn sub(self, o: Money) -> Money {
        Money(self.0.checked_sub(o.0).expect("money overflow"))
    }
}
impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(-self.0)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        *self = *self + o;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        *self = *self - o;
    }
}
impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + b)
    }
}
impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + *b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variants() {
        assert_eq!(Money::parse("1,23,456.50").unwrap(), Money(12345650));
        assert_eq!(Money::parse("₹ 500").unwrap(), Money(50000));
        assert_eq!(Money::parse("(1,000)").unwrap(), Money(-100000));
        assert_eq!(Money::parse("1,234.00 Dr").unwrap(), Money(123400));
        assert_eq!(Money::parse("1,234.00 Cr").unwrap(), Money(-123400));
        assert_eq!(Money::parse("-75").unwrap(), Money(-7500));
        assert_eq!(Money::parse(".5").unwrap(), Money(50));
        assert_eq!(Money::parse("").unwrap(), Money::ZERO);
        assert_eq!(Money::parse("10.500").unwrap(), Money(1050));
        assert!(Money::parse("10.505").is_err());
        assert!(Money::parse("abc").is_err());
    }

    #[test]
    fn from_float() {
        assert_eq!(Money::from_f64_checked(0.1 + 0.2).unwrap(), Money(30));
        assert_eq!(Money::from_f64_checked(-1234.56).unwrap(), Money(-123456));
        assert!(Money::from_f64_checked(1.234).is_err());
    }

    #[test]
    fn indian_format() {
        assert_eq!(Money(123456789).fmt_indian(), "12,34,567.89");
        assert_eq!(Money(-100000).fmt_indian(), "(1,000.00)");
        assert_eq!(Money(5).fmt_indian(), "0.05");
        assert_eq!(Money::rupees(10000000).fmt_indian(), "1,00,00,000.00");
        assert_eq!(Money(-123400).fmt_drcr(), "1,234.00 Cr");
    }
}
