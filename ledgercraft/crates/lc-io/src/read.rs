//! Readers for the LedgerCraft standard Excel/CSV templates (also accepts
//! common Tally/BUSY export column names).

use crate::table::{read_optional_sheet, read_table, Table};
use lc_core::date::parse_date;
use lc_core::model::{Ledger, TrialBalance, Voucher, VoucherLine};
use lc_core::Money;
use std::path::Path;

const LEDGER: &[&str] = &[
    "ledger",
    "ledger name",
    "particulars",
    "account",
    "account name",
    "name",
];
const GROUP: &[&str] = &["group", "under", "parent", "group name", "parent group"];

fn amount(
    t: &Table,
    row: &[String],
    signed: &[&str],
    dr: &[&str],
    cr: &[&str],
    what: &str,
    line: usize,
) -> Result<Money, String> {
    let parse = |s: &str| Money::parse(s).map_err(|e| format!("row {line}: {what}: {e}"));
    if let Some(c) = t.col(signed) {
        return parse(t.get(row, Some(c)));
    }
    let d = t
        .col(dr)
        .map(|c| parse(t.get(row, Some(c))))
        .transpose()?
        .unwrap_or_default();
    let k = t
        .col(cr)
        .map(|c| parse(t.get(row, Some(c))))
        .transpose()?
        .unwrap_or_default();
    Ok(d.abs() - k.abs())
}

pub fn read_trial_balance(path: &Path) -> Result<TrialBalance, String> {
    let t = read_table(path, &["Trial Balance", "TB"], LEDGER)?;
    let lc = t.col(LEDGER).ok_or("no Ledger column")?;
    let gc = t
        .col(GROUP)
        .ok_or("no Group column (needed to place each ledger)")?;
    let has_closing = t
        .col(&[
            "closing",
            "closing balance",
            "balance",
            "closing dr",
            "closing debit",
        ])
        .is_some();
    if !has_closing {
        return Err(format!("{}: no Closing column", path.display()));
    }
    let stock_c = t.col(&["closing stock", "closing stock value"]);
    let tags_c = t.col(&["tags", "tag"]);
    let mut ledgers = Vec::new();
    for (row, &line) in t.rows.iter().zip(&t.source_rows) {
        let name = t.get(row, Some(lc)).to_string();
        if name.is_empty()
            || name.eq_ignore_ascii_case("total")
            || name.eq_ignore_ascii_case("grand total")
        {
            continue;
        }
        let opening = amount(
            &t,
            row,
            &["opening", "opening balance"],
            &["opening dr", "opening debit"],
            &["opening cr", "opening credit"],
            "opening",
            line,
        )?;
        let closing = amount(
            &t,
            row,
            &["closing", "closing balance", "balance"],
            &["closing dr", "closing debit"],
            &["closing cr", "closing credit"],
            "closing",
            line,
        )?;
        let closing_stock = match stock_c {
            Some(c) if !t.get(row, Some(c)).is_empty() => Some(
                Money::parse(t.get(row, Some(c)))
                    .map_err(|e| format!("row {line}: closing stock: {e}"))?,
            ),
            _ => None,
        };
        let tags = t
            .get(row, tags_c)
            .split([',', ';'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        ledgers.push(Ledger {
            name,
            group: t.get(row, Some(gc)).to_string(),
            opening,
            closing,
            closing_stock,
            tags,
        });
    }
    let mut groups = Vec::new();
    if let Some(g) = read_optional_sheet(path, "Groups", &["group", "group name"])? {
        let a = g
            .col(&["group", "group name"])
            .ok_or("Groups sheet: no Group column")?;
        let p = g
            .col(&["parent", "under", "parent group"])
            .ok_or("Groups sheet: no Parent column")?;
        for row in &g.rows {
            let (child, parent) = (g.get(row, Some(a)), g.get(row, Some(p)));
            if !child.is_empty() && !parent.is_empty() {
                groups.push((child.to_string(), parent.to_string()));
            }
        }
    }
    Ok(TrialBalance { ledgers, groups })
}

/// One row per voucher line. Rows with the same date + type + number form one voucher.
pub fn read_vouchers(path: &Path) -> Result<Vec<Voucher>, String> {
    let t = read_table(
        path,
        &["Vouchers", "Day Book", "Daybook"],
        &["ledger", "ledger name", "particulars"],
    )?;
    let dc = t.col(&["date", "voucher date"]).ok_or("no Date column")?;
    let nc = t
        .col(&[
            "voucher no",
            "voucher no.",
            "vch no",
            "vch no.",
            "number",
            "voucher number",
        ])
        .ok_or("no Voucher No column")?;
    let tc = t.col(&["voucher type", "vch type", "type"]);
    let lc = t.col(LEDGER).ok_or("no Ledger column")?;
    let narr = t.col(&["narration", "remarks"]);
    let mut out: Vec<Voucher> = Vec::new();
    for (row, &line) in t.rows.iter().zip(&t.source_rows) {
        let date = parse_date(t.get(row, Some(dc)))
            .ok_or_else(|| format!("row {line}: cannot read date '{}'", t.get(row, Some(dc))))?;
        let number = t.get(row, Some(nc)).to_string();
        let vtype = t.get(row, tc).to_string();
        let amt = amount(
            &t,
            row,
            &["amount"],
            &["debit", "dr", "debit amount"],
            &["credit", "cr", "credit amount"],
            "amount",
            line,
        )?;
        let vl = VoucherLine {
            ledger: t.get(row, Some(lc)).to_string(),
            amount: amt,
        };
        let same = out
            .last()
            .map(|v| v.date == date && v.number == number && v.vtype == vtype)
            .unwrap_or(false);
        if same {
            out.last_mut().unwrap().lines.push(vl);
        } else {
            out.push(Voucher {
                date,
                number,
                vtype,
                narration: t.get(row, narr).to_string(),
                lines: vec![vl],
            });
        }
    }
    Ok(out)
}
