//! Analytical charts as self-contained SVG, drawn once and used both in the
//! app (inline) and in the optional PDF annexure. Design rules: one axis,
//! thin marks, 2px surface gap between touching fills, 4px rounded data ends,
//! recessive grid, legend for two or more series plus direct labels, text in
//! ink colours (never the series colour), <title> on every mark for hover.
//! Palettes are validated (categorical slots 1-6 adjacent; ordinal blue ramp).

use crate::engine::Analysis;
use crate::mapping::Head;
use crate::model::{norm_name, Engagement};
use crate::money::Money;
use chrono::Datelike;
use std::collections::BTreeMap;

/// Categorical slots (reference palette, light mode), fixed order.
pub const CATEGORICAL: [&str; 6] = [
    "#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300",
];
/// Ordinal ramp (blue 250 → 650): older balances darker.
pub const ORDINAL: [&str; 5] = ["#86b6ef", "#5598e7", "#2a78d6", "#1c5cab", "#104281"];
const INK: &str = "#1f2a26";
const INK2: &str = "#55615b";
const GRID: &str = "#e3e8e5";
const SURFACE: &str = "#fdfefd";

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// ₹ in lakhs or crores for chart labels.
pub fn compact(m: Money) -> String {
    let r = m.as_f64();
    let a = r.abs();
    let s = if a >= 1e7 {
        format!("{:.2} Cr", r / 1e7)
    } else if a >= 1e5 {
        format!("{:.2} L", r / 1e5)
    } else {
        format!("{:.0}", r)
    };
    format!("₹ {s}")
}

fn open(w: u32, h: u32, font: &str, label: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="{font}" font-size="11" role="img" aria-label="{}">"#,
        esc(label)
    )
}

fn legend(items: &[(&str, &str)], x0: f64, y: f64) -> String {
    let mut s = String::new();
    let mut x = x0;
    for (name, color) in items {
        s += &format!(
            r#"<rect x="{x}" y="{}" width="10" height="10" rx="2" fill="{color}"/><text x="{}" y="{}" fill="{INK2}">{}</text>"#,
            y - 9.0,
            x + 14.0,
            y,
            esc(name)
        );
        x += 22.0 + name.chars().count() as f64 * 6.2;
    }
    s
}

/// Horizontal 100% stacked bars (part-to-whole), one row per period.
pub fn stacked_share(
    title: &str,
    rows: &[(String, Vec<(String, Money)>)],
    font: &str,
) -> Option<String> {
    let cats: Vec<String> = rows.first()?.1.iter().map(|(c, _)| c.clone()).collect();
    let (w, left, right) = (640.0, 120.0, 16.0);
    let bar = 22.0;
    let h = 70.0 + rows.len() as f64 * 46.0;
    let mut s = open(w as u32, h as u32, font, title);
    s += &format!(
        r#"<text x="0" y="16" fill="{INK}" font-size="13" font-weight="600">{}</text>"#,
        esc(title)
    );
    let items: Vec<(&str, &str)> = cats
        .iter()
        .zip(CATEGORICAL.iter())
        .map(|(c, col)| (c.as_str(), *col))
        .collect();
    s += &legend(&items, 0.0, 38.0);
    let span = w - left - right;
    for (ri, (label, parts)) in rows.iter().enumerate() {
        let y = 56.0 + ri as f64 * 46.0;
        s += &format!(
            r#"<text x="0" y="{}" fill="{INK}">{}</text>"#,
            y + 15.0,
            esc(label)
        );
        let total: i64 = parts.iter().map(|(_, m)| m.0.max(0)).sum();
        if total == 0 {
            s += &format!(
                r#"<text x="{left}" y="{}" fill="{INK2}">No figures</text>"#,
                y + 15.0
            );
            continue;
        }
        let mut x = left;
        let shown: Vec<(usize, f64)> = parts
            .iter()
            .enumerate()
            .map(|(i, (_, m))| (i, m.0.max(0) as f64 / total as f64))
            .filter(|(_, f)| *f > 0.0)
            .collect();
        for (k, (i, f)) in shown.iter().enumerate() {
            let width = (span * f - 2.0).max(1.0);
            let last = k + 1 == shown.len();
            let col = CATEGORICAL[*i % CATEGORICAL.len()];
            let tip = format!(
                "{label}: {} {} ({:.1}%)",
                parts[*i].0,
                compact(parts[*i].1),
                f * 100.0
            );
            if last && width > 4.0 {
                s += &format!(
                    r#"<path d="M{x},{y} h{} a4,4 0 0 1 4,4 v{} a4,4 0 0 1 -4,4 h-{} z" fill="{col}"><title>{}</title></path>"#,
                    width - 4.0,
                    bar - 8.0,
                    width - 4.0,
                    esc(&tip)
                );
            } else {
                s += &format!(
                    r#"<rect x="{x}" y="{y}" width="{width}" height="{bar}" fill="{col}"><title>{}</title></rect>"#,
                    esc(&tip)
                );
            }
            if span * f >= 34.0 {
                s += &format!(
                    r#"<text x="{}" y="{}" fill="{INK2}" text-anchor="middle" font-size="10">{:.0}%</text>"#,
                    x + width / 2.0,
                    y + bar + 12.0,
                    f * 100.0
                );
            }
            x += width + 2.0;
        }
    }
    s += "</svg>";
    Some(s)
}

/// Horizontal stacked bars with an ordered (ordinal) colour per bucket.
pub fn stacked_ordinal(
    title: &str,
    buckets: &[String],
    rows: &[(String, Vec<Money>)],
    font: &str,
) -> Option<String> {
    if rows.iter().all(|(_, v)| v.iter().all(|m| m.is_zero())) {
        return None;
    }
    let (w, left, right) = (640.0, 120.0, 90.0);
    let bar = 22.0;
    let h = 70.0 + rows.len() as f64 * 46.0;
    let mut s = open(w as u32, h as u32, font, title);
    s += &format!(
        r#"<text x="0" y="16" fill="{INK}" font-size="13" font-weight="600">{}</text>"#,
        esc(title)
    );
    let items: Vec<(&str, &str)> = buckets
        .iter()
        .zip(ORDINAL.iter())
        .map(|(b, c)| (b.as_str(), *c))
        .collect();
    s += &legend(&items, 0.0, 38.0);
    let max: i64 = rows
        .iter()
        .map(|(_, v)| v.iter().map(|m| m.0.abs()).sum::<i64>())
        .max()
        .unwrap_or(1)
        .max(1);
    let span = w - left - right;
    for (ri, (label, vals)) in rows.iter().enumerate() {
        let y = 56.0 + ri as f64 * 46.0;
        s += &format!(
            r#"<text x="0" y="{}" fill="{INK}">{}</text>"#,
            y + 15.0,
            esc(label)
        );
        let mut x = left;
        let nz: Vec<(usize, &Money)> = vals
            .iter()
            .enumerate()
            .filter(|(_, m)| !m.is_zero())
            .collect();
        let total: i64 = vals.iter().map(|m| m.0.abs()).sum();
        for (k, (i, m)) in nz.iter().enumerate() {
            let width = (span * m.0.abs() as f64 / max as f64 - 2.0).max(1.0);
            let col = ORDINAL[(*i).min(ORDINAL.len() - 1)];
            let tip = format!(
                "{label}, {}: {} ({:.1}%)",
                buckets.get(*i).map(|b| b.as_str()).unwrap_or(""),
                compact(m.abs()),
                m.0.abs() as f64 * 100.0 / total.max(1) as f64
            );
            if k + 1 == nz.len() && width > 4.0 {
                s += &format!(
                    r#"<path d="M{x},{y} h{} a4,4 0 0 1 4,4 v{} a4,4 0 0 1 -4,4 h-{} z" fill="{col}"><title>{}</title></path>"#,
                    width - 4.0,
                    bar - 8.0,
                    width - 4.0,
                    esc(&tip)
                );
            } else {
                s += &format!(
                    r#"<rect x="{x}" y="{y}" width="{width}" height="{bar}" fill="{col}"><title>{}</title></rect>"#,
                    esc(&tip)
                );
            }
            x += width + 2.0;
        }
        s += &format!(
            r#"<text x="{}" y="{}" fill="{INK}">{}</text>"#,
            x + 6.0,
            y + 15.0,
            esc(&compact(Money(total)))
        );
    }
    s += "</svg>";
    Some(s)
}

/// Lines over months: one axis, 2px lines, 8px markers, end labels.
pub fn lines(
    title: &str,
    months: &[String],
    series: &[(String, Vec<Money>)],
    font: &str,
) -> Option<String> {
    if months.len() < 2 || series.iter().all(|(_, v)| v.iter().all(|m| m.is_zero())) {
        return None;
    }
    let (w, h) = (640.0, 260.0);
    let (left, right, top, bottom) = (64.0, 96.0, 52.0, 30.0);
    let mut s = open(w as u32, h as u32, font, title);
    s += &format!(
        r#"<text x="0" y="16" fill="{INK}" font-size="13" font-weight="600">{}</text>"#,
        esc(title)
    );
    let items: Vec<(&str, &str)> = series
        .iter()
        .zip(CATEGORICAL.iter())
        .map(|((n, _), c)| (n.as_str(), *c))
        .collect();
    s += &legend(&items, 0.0, 38.0);
    let max = series
        .iter()
        .flat_map(|(_, v)| v.iter().map(|m| m.0))
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let min = series
        .iter()
        .flat_map(|(_, v)| v.iter().map(|m| m.0))
        .min()
        .unwrap_or(0)
        .min(0) as f64;
    let ph = h - top - bottom;
    let pw = w - left - right;
    let ypos = |v: f64| top + ph - (v - min) / (max - min) * ph;
    let xpos = |i: usize| left + pw * i as f64 / (months.len() - 1) as f64;
    for k in 0..=4 {
        let v = min + (max - min) * k as f64 / 4.0;
        let y = ypos(v);
        s += &format!(
            r#"<line x1="{left}" x2="{}" y1="{y}" y2="{y}" stroke="{GRID}" stroke-width="1"/><text x="{}" y="{}" fill="{INK2}" text-anchor="end" font-size="10">{}</text>"#,
            left + pw,
            left - 6.0,
            y + 3.0,
            esc(&compact(Money(v as i64)))
        );
    }
    for (i, m) in months.iter().enumerate() {
        s += &format!(
            r#"<text x="{}" y="{}" fill="{INK2}" text-anchor="middle" font-size="10">{}</text>"#,
            xpos(i),
            h - 10.0,
            esc(m)
        );
    }
    for (si, (name, vals)) in series.iter().enumerate() {
        let col = CATEGORICAL[si % CATEGORICAL.len()];
        let pts: Vec<String> = vals
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{:.1},{:.1}", xpos(i), ypos(m.0 as f64)))
            .collect();
        s += &format!(
            r#"<polyline points="{}" fill="none" stroke="{col}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>"#,
            pts.join(" ")
        );
        for (i, m) in vals.iter().enumerate() {
            let tip = format!("{} {}: {}", months[i], name, compact(*m));
            s += &format!(
                r#"<g><circle cx="{:.1}" cy="{:.1}" r="4" fill="{col}" stroke="{SURFACE}" stroke-width="2"/><circle cx="{:.1}" cy="{:.1}" r="10" fill="transparent"><title>{}</title></circle></g>"#,
                xpos(i),
                ypos(m.0 as f64),
                xpos(i),
                ypos(m.0 as f64),
                esc(&tip)
            );
        }
        if let Some(last) = vals.last() {
            s += &format!(
                r#"<text x="{}" y="{}" fill="{INK}" font-size="10">{}</text>"#,
                xpos(vals.len() - 1) + 8.0,
                ypos(last.0 as f64) + 3.0,
                esc(name)
            );
        }
    }
    s += "</svg>";
    Some(s)
}

/// The charts of an engagement: expense make-up (this year and last year),
/// ageing of debtors and creditors, and monthly sales and purchases.
pub fn build(eng: &Engagement, a: &Analysis, font: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let parts = |f: &crate::facts::YearFacts| -> Vec<(String, Money)> {
        vec![
            (
                "Materials and stock".into(),
                f.head(Head::Purchases) + f.head(Head::ChangeInInventories),
            ),
            (
                "Employees".into(),
                f.head(Head::EmployeeBenefits)
                    + f.head(Head::PartnersRemuneration)
                    + f.head(Head::ManagerialRemuneration),
            ),
            ("Finance costs".into(), f.head(Head::FinanceCosts)),
            ("Depreciation".into(), f.head(Head::Depreciation)),
            ("Other expenses".into(), f.head(Head::OtherExpenses)),
            ("Tax".into(), f.head(Head::TaxExpense)),
        ]
    };
    let mut rows = vec![("This year".to_string(), parts(&a.facts_cy))];
    if let Some(py) = &a.facts_py {
        rows.push(("Last year".to_string(), parts(py)));
    }
    if let Some(svg) = stacked_share("How the expenses are made up", &rows, font) {
        out.push(("How the expenses are made up".into(), svg));
    }

    // Debtors and creditors have their own age bands: one chart each.
    for (name, title, ag) in [
        (
            "Debtors",
            "Age of debtors (older is darker)",
            &a.ageing_receivables,
        ),
        (
            "Creditors",
            "Age of creditors (older is darker)",
            &a.ageing_payables,
        ),
    ] {
        if let Some(ag) = ag {
            let mut sums = vec![Money::ZERO; ag.bucket_labels.len()];
            for r in &ag.rows {
                for (i, m) in r.buckets.iter().enumerate() {
                    if let Some(s) = sums.get_mut(i) {
                        *s += m.abs();
                    }
                }
            }
            if let Some(svg) =
                stacked_ordinal(title, &ag.bucket_labels, &[(name.to_string(), sums)], font)
            {
                out.push((title.to_string(), svg));
            }
        }
    }

    // Monthly sales and purchases from the day book.
    if !eng.vouchers.is_empty() {
        let head_of: BTreeMap<String, Head> = a
            .mapping
            .iter()
            .filter_map(|m| m.head.map(|h| (norm_name(&m.name), h)))
            .collect();
        let mut months: Vec<chrono::NaiveDate> = Vec::new();
        let (mut y, mut m) = (eng.fy_start.year(), eng.fy_start.month());
        while let Some(d) = chrono::NaiveDate::from_ymd_opt(y, m, 1) {
            if d > eng.fy_end || months.len() >= 18 {
                break;
            }
            months.push(d);
            if m == 12 {
                y += 1;
                m = 1;
            } else {
                m += 1;
            }
        }
        let idx = |date: chrono::NaiveDate| {
            months
                .iter()
                .position(|m| m.year() == date.year() && m.month() == date.month())
        };
        let mut sales = vec![Money::ZERO; months.len()];
        let mut purchases = vec![Money::ZERO; months.len()];
        for v in &eng.vouchers {
            let Some(i) = idx(v.date) else { continue };
            for l in &v.lines {
                match head_of.get(&norm_name(&l.ledger)) {
                    Some(Head::RevenueOps) => sales[i] -= l.amount,
                    Some(Head::Purchases) => purchases[i] += l.amount,
                    _ => {}
                }
            }
        }
        let labels: Vec<String> = months.iter().map(|m| m.format("%b").to_string()).collect();
        if let Some(svg) = lines(
            "Sales and purchases by month",
            &labels,
            &[("Sales".into(), sales), ("Purchases".into(), purchases)],
            font,
        ) {
            out.push(("Sales and purchases by month".into(), svg));
        }
    }
    out
}
