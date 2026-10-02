//! Writes books in the LedgerCraft standard template (used for practice data
//! and to round-trip test the readers).

use lc_core::model::{TrialBalance, Voucher};
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

pub fn write_trial_balance(tb: &TrialBalance, path: &Path) -> Result<(), String> {
    let mut wb = Workbook::new();
    let bold = Format::new().set_bold();
    let num = Format::new().set_num_format("#,##0.00");
    let ws = wb
        .add_worksheet()
        .set_name("Trial Balance")
        .map_err(|e| e.to_string())?;
    let heads = [
        "Ledger",
        "Group",
        "Opening",
        "Closing",
        "Closing Stock",
        "Tags",
    ];
    for (c, h) in heads.iter().enumerate() {
        ws.write_string_with_format(0, c as u16, *h, &bold)
            .map_err(|e| e.to_string())?;
    }
    for (i, l) in tb.ledgers.iter().enumerate() {
        let r = i as u32 + 1;
        ws.write_string(r, 0, &l.name).map_err(|e| e.to_string())?;
        ws.write_string(r, 1, &l.group).map_err(|e| e.to_string())?;
        ws.write_number_with_format(r, 2, l.opening.as_f64(), &num)
            .map_err(|e| e.to_string())?;
        ws.write_number_with_format(r, 3, l.closing.as_f64(), &num)
            .map_err(|e| e.to_string())?;
        if let Some(cs) = l.closing_stock {
            ws.write_number_with_format(r, 4, cs.as_f64(), &num)
                .map_err(|e| e.to_string())?;
        }
        if !l.tags.is_empty() {
            ws.write_string(r, 5, l.tags.join(","))
                .map_err(|e| e.to_string())?;
        }
    }
    ws.set_column_width(0, 40).ok();
    ws.set_column_width(1, 28).ok();
    ws.set_column_width(2, 16).ok();
    ws.set_column_width(3, 16).ok();
    ws.set_column_width(4, 16).ok();
    let g = wb
        .add_worksheet()
        .set_name("Groups")
        .map_err(|e| e.to_string())?;
    g.write_string_with_format(0, 0, "Group", &bold)
        .map_err(|e| e.to_string())?;
    g.write_string_with_format(0, 1, "Parent", &bold)
        .map_err(|e| e.to_string())?;
    for (i, (c, p)) in tb.groups.iter().enumerate() {
        g.write_string(i as u32 + 1, 0, c)
            .map_err(|e| e.to_string())?;
        g.write_string(i as u32 + 1, 1, p)
            .map_err(|e| e.to_string())?;
    }
    wb.save(path).map_err(|e| e.to_string())
}

/// CSV (handles millions of lines; Excel sheets stop at 1,048,576 rows).
pub fn write_vouchers_csv(vouchers: &[Voucher], path: &Path) -> Result<(), String> {
    let mut w = csv::Writer::from_path(path).map_err(|e| e.to_string())?;
    w.write_record([
        "Date",
        "Voucher Type",
        "Voucher No",
        "Ledger",
        "Debit",
        "Credit",
        "Narration",
    ])
    .map_err(|e| e.to_string())?;
    for v in vouchers {
        for l in &v.lines {
            let (dr, cr) = if l.amount.is_cr() {
                (String::new(), (-l.amount).fmt_plain())
            } else {
                (l.amount.fmt_plain(), String::new())
            };
            w.write_record([
                v.date.format("%d-%m-%Y").to_string(),
                v.vtype.clone(),
                v.number.clone(),
                l.ledger.clone(),
                dr,
                cr,
                v.narration.clone(),
            ])
            .map_err(|e| e.to_string())?;
        }
    }
    w.flush().map_err(|e| e.to_string())
}

trait Plain {
    fn fmt_plain(self) -> String;
}
impl Plain for lc_core::Money {
    fn fmt_plain(self) -> String {
        let p = self.paise();
        let sign = if p < 0 { "-" } else { "" };
        format!(
            "{sign}{}.{:02}",
            p.unsigned_abs() / 100,
            p.unsigned_abs() % 100
        )
    }
}
