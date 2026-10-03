//! Date parsing for Indian accounting exports.

use chrono::{Datelike, Duration, NaiveDate};

/// Parse `2025-04-01`, `01-04-2025`, `01/04/2025`, `01.04.2025`, `1-Apr-2025`, `1-Apr-25`, `01 Apr 2025`.
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    // Tally XML dates: YYYYMMDD.
    if s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
            return Some(d);
        }
    }
    const FORMATS: &[&str] = &[
        "%Y-%m-%d", "%d-%m-%Y", "%d/%m/%Y", "%d.%m.%Y", "%d-%b-%Y", "%d-%b-%y", "%d %b %Y",
        "%d/%b/%Y", "%Y/%m/%d",
    ];
    for f in FORMATS {
        if let Ok(d) = NaiveDate::parse_from_str(s, f) {
            // `%Y` also accepts a 2-digit year ("25" -> year 0025); treat it as 20xx.
            if d.year() < 100 {
                return d.with_year(2000 + d.year());
            }
            return Some(d);
        }
    }
    // Spreadsheet serial number.
    if let Ok(n) = s.parse::<f64>() {
        return from_excel_serial(n);
    }
    None
}

pub fn from_excel_serial(n: f64) -> Option<NaiveDate> {
    if !(1.0..2_958_466.0).contains(&n) {
        return None;
    }
    NaiveDate::from_ymd_opt(1899, 12, 30)?.checked_add_signed(Duration::days(n.trunc() as i64))
}

/// Financial year label like `2025-26` -> (1 Apr 2025, 31 Mar 2026).
pub fn parse_fy(label: &str) -> Option<(NaiveDate, NaiveDate)> {
    let start_year: i32 = label.trim().split(['-', '/']).next()?.trim().parse().ok()?;
    let start = NaiveDate::from_ymd_opt(start_year, 4, 1)?;
    let end = NaiveDate::from_ymd_opt(start_year + 1, 3, 31)?;
    Some((start, end))
}

pub fn fy_label(start: NaiveDate) -> String {
    format!("{}-{:02}", start.year(), (start.year() + 1) % 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats() {
        let d = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        for s in [
            "2025-04-01",
            "01-04-2025",
            "01/04/2025",
            "1-Apr-2025",
            "1-Apr-25",
            "45748",
        ] {
            assert_eq!(parse_date(s), Some(d), "{s}");
        }
        assert_eq!(
            parse_fy("2025-26").unwrap().1,
            NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()
        );
        assert_eq!(fy_label(d), "2025-26");
    }
}
