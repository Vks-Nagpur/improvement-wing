//! A simple in-memory table read from Excel or CSV, with header detection.

use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// 1-based row number in the source file for each data row (for error messages).
    pub source_rows: Vec<usize>,
}

fn cell_text(c: &Data) -> String {
    match c {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else if ((f * 100.0) - (f * 100.0).round()).abs() < 0.001 {
                // Remove binary floating-point noise (e.g. 0.30000000000000004).
                format!("{f:.2}")
            } else {
                format!("{f}")
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => format!("{}", d.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR {e:?}"),
    }
}

/// All cells of a sheet (by preferred names, else the first sheet) or a CSV
/// file, as text, without header detection.
pub fn raw_rows(path: &Path, prefer_sheets: &[&str]) -> Result<Vec<Vec<String>>, String> {
    read_table_raw(path, prefer_sheets)
}

/// Read a sheet (by preferred names, else the first sheet) or a CSV file.
pub fn read_table(
    path: &Path,
    prefer_sheets: &[&str],
    header_hint: &[&str],
) -> Result<Table, String> {
    let raw = read_table_raw(path, prefer_sheets)?;
    table_from_raw(path, raw, header_hint)
}

fn read_table_raw(path: &Path, prefer_sheets: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let raw: Vec<Vec<String>> = if ext == "csv" {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut out = Vec::new();
        for rec in rdr.records() {
            let rec = rec.map_err(|e| format!("{}: {e}", path.display()))?;
            out.push(rec.iter().map(|s| unguard(s.trim())).collect());
        }
        out
    } else {
        check_workbook(path)?;
        let mut wb = open_workbook_auto(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let names = wb.sheet_names().to_vec();
        let pick = prefer_sheets
            .iter()
            .find_map(|p| names.iter().find(|n| n.eq_ignore_ascii_case(p)).cloned())
            .or_else(|| names.first().cloned())
            .ok_or_else(|| format!("{}: workbook has no sheets", path.display()))?;
        let range = wb
            .worksheet_range(&pick)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if range.height() > MAX_ROWS || range.width() > MAX_COLS {
            return Err(format!(
                "{}: the sheet has {} rows and {} columns; LedgerCraft reads at most {MAX_ROWS} rows and {MAX_COLS} columns.",
                path.display(),
                range.height(),
                range.width()
            ));
        }
        range
            .rows()
            .map(|r| r.iter().map(cell_text).collect())
            .collect()
    };
    if raw.len() > MAX_ROWS {
        return Err(format!("{}: more than {MAX_ROWS} rows.", path.display()));
    }
    Ok(raw)
}

fn table_from_raw(
    path: &Path,
    raw: Vec<Vec<String>>,
    header_hint: &[&str],
) -> Result<Table, String> {
    let header_idx = raw
        .iter()
        .position(|r| {
            r.iter().any(|c| {
                header_hint
                    .iter()
                    .any(|h| c.to_ascii_lowercase().trim() == *h)
            })
        })
        .ok_or_else(|| {
            format!(
                "{}: could not find a header row containing one of {:?}",
                path.display(),
                header_hint
            )
        })?;
    let headers: Vec<String> = raw[header_idx]
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    crate::diag::rows(raw.len());
    for (i, r) in raw.iter().enumerate().take(header_idx) {
        if !r.iter().all(|c| c.is_empty()) {
            crate::diag::skip(i + 1, "title line above the header row", r);
        }
    }
    let mut rows = Vec::new();
    let mut source_rows = Vec::new();
    for (i, r) in raw.iter().enumerate().skip(header_idx + 1) {
        if r.iter().all(|c| c.is_empty()) {
            continue;
        }
        rows.push(r.clone());
        source_rows.push(i + 1);
    }
    Ok(Table {
        headers,
        rows,
        source_rows,
    })
}

/// Optional sheet; `Ok(None)` when the workbook has no such sheet (or input is CSV).
pub fn read_optional_sheet(
    path: &Path,
    sheet: &str,
    header_hint: &[&str],
) -> Result<Option<Table>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "csv" {
        return Ok(None);
    }
    check_workbook(path)?;
    let wb = open_workbook_auto(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !wb
        .sheet_names()
        .iter()
        .any(|n| n.eq_ignore_ascii_case(sheet))
    {
        return Ok(None);
    }
    read_table(path, &[sheet], header_hint).map(Some)
}

impl Table {
    pub fn col(&self, aliases: &[&str]) -> Option<usize> {
        aliases
            .iter()
            .find_map(|a| self.headers.iter().position(|h| h == a))
    }
    pub fn get<'a>(&'a self, row: &'a [String], col: Option<usize>) -> &'a str {
        col.and_then(|c| row.get(c))
            .map(|s| s.as_str())
            .unwrap_or("")
    }
}

/// Text written by LedgerCraft's CSV writer with a leading apostrophe so a
/// spreadsheet does not run it as a formula: the apostrophe is removed again.
pub fn unguard(s: &str) -> String {
    match s.strip_prefix('\'') {
        Some(rest) if rest.starts_with(['=', '+', '-', '@']) => rest.to_string(),
        _ => s.to_string(),
    }
}

/// Text that a spreadsheet would treat as a formula gets a leading apostrophe.
pub fn guard(s: &str) -> String {
    if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_string()
    }
}

pub const MAX_ROWS: usize = 3_000_000;
pub const MAX_COLS: usize = 400;
/// Largest total uncompressed size of an Excel/ODS package.
pub const MAX_UNPACKED: u64 = 1 << 30;

/// Excel (.xlsx/.xlsm) and ODS files are zip packages: refuse one that would
/// unpack to an excessive size (a "zip bomb") before it is opened.
pub fn check_workbook(path: &Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "xlsx" | "xlsm" | "ods" | "xlsb") {
        return Ok(());
    }
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let packed = f.metadata().map(|m| m.len()).unwrap_or(0).max(1);
    let mut z = zip::ZipArchive::new(f)
        .map_err(|e| format!("{}: not a valid Excel file ({e})", path.display()))?;
    if z.len() > 10_000 {
        return Err(format!(
            "{}: the file has too many parts ({}).",
            path.display(),
            z.len()
        ));
    }
    let mut total: u64 = 0;
    for i in 0..z.len() {
        let e = z
            .by_index_raw(i)
            .map_err(|e| format!("{}: damaged Excel file ({e})", path.display()))?;
        total = total.saturating_add(e.size());
    }
    if total > MAX_UNPACKED || total / packed > 500 {
        return Err(format!(
            "{}: the file would unpack to {} MB; refused as unsafe or too large.",
            path.display(),
            total >> 20
        ));
    }
    Ok(())
}
