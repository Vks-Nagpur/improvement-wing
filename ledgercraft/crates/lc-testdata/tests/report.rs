//! The printed report must cast exactly in every unit and layout.

use lc_core::report::{build, Block, ReportOptions, RowStyle, SignOff, Toggle};
use lc_core::rules::RulesPack;
use lc_core::units::Unit;
use lc_core::{analyse, Money};
use lc_testdata::scenarios::{self, Kind};

fn val(s: &str) -> f64 {
    if s == "-" || s.is_empty() {
        return 0.0;
    }
    let neg = s.starts_with('(');
    let v: f64 = s
        .trim_matches(|c| c == '(' || c == ')')
        .replace(',', "")
        .parse()
        .unwrap_or_else(|_| panic!("not a number: {s}"));
    if neg {
        -v
    } else {
        v
    }
}

fn check(name: &str, eng: &lc_core::Engagement, unit: Unit, decimals: u8) {
    let a = analyse(eng, &RulesPack::builtin());
    let opt = ReportOptions {
        unit,
        decimals,
        cash_flow: Toggle::On,
        ratios: Toggle::On,
        ..Default::default()
    };
    let r = build(eng, &a, &opt, &SignOff::default());
    let eps = 0.5 / 10f64.powi(decimals as i32);
    let bs = r.sections.iter().find(|s| s.id == "balance_sheet").unwrap();
    let Block::Table(t) = &bs.blocks[0] else {
        panic!()
    };
    let totals: Vec<&Vec<String>> = t
        .rows
        .iter()
        .filter(|r| r.style == RowStyle::Total)
        .map(|r| &r.cells)
        .collect();
    assert_eq!(totals.len(), 2, "{name}");
    for (a, b) in totals[0].iter().zip(totals[1].iter()).skip(2) {
        assert_eq!(
            a, b,
            "{name} {unit:?}: balance sheet does not tally after rounding"
        );
    }
    // Every face line equals the total of its note, and each side adds up.
    let notes = r.sections.iter().find(|s| s.id == "notes").unwrap();
    let mut face_lines: Vec<(String, Vec<String>)> = Vec::new();
    for s in r
        .sections
        .iter()
        .filter(|s| s.id == "balance_sheet" || s.id == "profit_and_loss")
    {
        let Block::Table(t) = &s.blocks[0] else {
            panic!()
        };
        for row in &t.rows {
            if !row.cells[1].is_empty() {
                face_lines.push((row.cells[1].clone(), row.cells[2..].to_vec()));
            }
        }
        if s.id == "balance_sheet" {
            let mut side = 0.0;
            for row in &t.rows {
                match row.style {
                    RowStyle::Item => side += val(&row.cells[2]),
                    RowStyle::Total => {
                        assert!(
                            (side - val(&row.cells[2])).abs() < eps,
                            "{name} {unit:?}: side items {side} vs total {}",
                            row.cells[2]
                        );
                        side = 0.0;
                    }
                    _ => {}
                }
            }
        }
    }
    for (no, vals) in face_lines {
        let mut found = false;
        let mut in_note = false;
        for b in &notes.blocks {
            match b {
                Block::Heading { text, level: 2 } => {
                    in_note = text.split_whitespace().next() == Some(no.as_str())
                }
                Block::Table(t) if in_note && !found => {
                    let last = t
                        .rows
                        .iter()
                        .rev()
                        .find(|r| r.style == RowStyle::Total)
                        .unwrap_or_else(|| panic!("{name}: note {no} has no total"));
                    let n = last.cells.len();
                    if t.columns.len() <= 3 {
                        let note_vals = &last.cells[n - vals.len()..];
                        assert_eq!(
                            note_vals,
                            &vals[..],
                            "{name} {unit:?}: note {no} total differs from face"
                        );
                    } else {
                        // Wide schedules (capital movement, PPE): closing balance is the last column.
                        let want = &vals[0];
                        let got = if t.landscape {
                            &last.cells[n - 2]
                        } else {
                            &last.cells[n - 1]
                        };
                        assert_eq!(
                            got, want,
                            "{name} {unit:?}: note {no} schedule total differs from face"
                        );
                    }
                    // Items + subtotal-free rows add up to the total (two-column notes).
                    if t.columns.len() <= 3 {
                        let items: f64 = t
                            .rows
                            .iter()
                            .filter(|r| r.style == RowStyle::Item)
                            .map(|r| val(&r.cells[1]))
                            .sum();
                        let has_sub = t.rows.iter().any(|r| r.style == RowStyle::Subtotal);
                        if !has_sub {
                            assert!(
                                (items - val(&last.cells[1])).abs() < eps,
                                "{name} {unit:?}: note {no} lines {items} vs total {}",
                                last.cells[1]
                            );
                        }
                    }
                    found = true;
                }
                _ => {}
            }
        }
        assert!(found, "{name}: note {no} missing");
    }
    // Cash flow ends at the balance-sheet cash figure.
    if let Some(cf) = r.sections.iter().find(|s| s.id == "cash_flow") {
        let Block::Table(t) = &cf.blocks[0] else {
            panic!()
        };
        let end = &t.rows.last().unwrap().cells[1];
        let bs_cash = t.rows.len();
        let _ = bs_cash;
        let Block::Table(b) = &bs.blocks[0] else {
            panic!()
        };
        let cash_row = b
            .rows
            .iter()
            .find(|r| r.cells[0].contains("Cash and"))
            .unwrap();
        assert_eq!(
            end, &cash_row.cells[2],
            "{name} {unit:?}: cash flow does not end at balance-sheet cash"
        );
    }
    let _ = Money::ZERO;
}

#[test]
fn report_casts_in_every_unit() {
    let cases = vec![
        (
            "clean_firm",
            scenarios::clean(Kind::Firm, 1, 10, 6, 4).engagement,
        ),
        (
            "clean_company",
            scenarios::clean(Kind::Company, 2, 25, 15, 12).engagement,
        ),
        ("glitches", scenarios::firm_with_glitches().engagement),
    ];
    for (name, eng) in &cases {
        for (unit, dec) in [
            (Unit::Rupees, 2),
            (Unit::Rupees, 0),
            (Unit::Thousands, 0),
            (Unit::Lakhs, 2),
            (Unit::Millions, 2),
            (Unit::Crores, 2),
        ] {
            check(name, eng, unit, dec);
        }
    }
    for seed in 200..215 {
        for kind in [Kind::Firm, Kind::Company] {
            let s = scenarios::clean(kind, seed, 8, 5, 3);
            for (unit, dec) in [(Unit::Lakhs, 2), (Unit::Thousands, 0)] {
                check(&s.name, &s.engagement, unit, dec);
            }
        }
    }
}
