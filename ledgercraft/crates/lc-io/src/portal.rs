//! Files downloaded from government portals: GSTR-2B (JSON or the portal's
//! Excel) and Form 26AS (the TRACES text file, or its Excel conversion).
//! Each is summarised per party for `lc_core::recon`.

use crate::table::{raw_rows, read_table};
use lc_core::money::Money;
use lc_core::recon::PortalParty;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

fn money_json(v: Option<&Value>) -> Money {
    match v {
        Some(Value::Number(n)) => Money((n.as_f64().unwrap_or(0.0) * 100.0).round() as i64),
        Some(Value::String(s)) => Money::parse(s).unwrap_or_default(),
        _ => Money::ZERO,
    }
}

fn add(map: &mut BTreeMap<String, PortalParty>, id: &str, name: &str, base: Money, tax: Money) {
    let e = map.entry(id.to_string()).or_insert_with(|| PortalParty {
        id: id.to_string(),
        name: name.to_string(),
        ..Default::default()
    });
    if e.name.is_empty() {
        e.name = name.to_string();
    }
    e.documents += 1;
    e.base += base;
    e.tax += tax;
}

/// Tax of an invoice / note: invoice-level fields, or the sum of its items.
fn tax_and_base(doc: &Value) -> (Money, Money) {
    let fields = |v: &Value| {
        let t = ["igst", "cgst", "sgst", "cess"]
            .iter()
            .map(|k| money_json(v.get(*k)))
            .sum::<Money>();
        (money_json(v.get("txval")), t)
    };
    match doc.get("items").and_then(|i| i.as_array()) {
        Some(items) if !items.is_empty() => items
            .iter()
            .map(fields)
            .fold((Money::ZERO, Money::ZERO), |a, b| (a.0 + b.0, a.1 + b.1)),
        _ => fields(doc),
    }
}

/// GSTR-2B: input tax credit available per supplier (B2B invoices, less
/// credit notes and plus debit notes). Invoices marked ITC not available are left out.
pub fn read_gstr2b(path: &Path) -> Result<Vec<PortalParty>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "json" {
        let v: Value = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("not a GSTR-2B JSON file: {e}"))?;
        let docdata = v
            .pointer("/data/docdata")
            .or_else(|| v.get("docdata"))
            .ok_or(
                "this JSON has no 'docdata' section: download GSTR-2B (JSON) from the GST portal",
            )?;
        let mut map = BTreeMap::new();
        for s in docdata
            .get("b2b")
            .and_then(|x| x.as_array())
            .into_iter()
            .flatten()
        {
            let (id, name) = (
                s.get("ctin").and_then(|x| x.as_str()).unwrap_or(""),
                s.get("trdnm").and_then(|x| x.as_str()).unwrap_or(""),
            );
            for inv in s
                .get("inv")
                .and_then(|x| x.as_array())
                .into_iter()
                .flatten()
            {
                if inv.get("itcavl").and_then(|x| x.as_str()) == Some("N") {
                    continue;
                }
                let (b, t) = tax_and_base(inv);
                add(&mut map, id, name, b, t);
            }
        }
        for s in docdata
            .get("cdnr")
            .and_then(|x| x.as_array())
            .into_iter()
            .flatten()
        {
            let (id, name) = (
                s.get("ctin").and_then(|x| x.as_str()).unwrap_or(""),
                s.get("trdnm").and_then(|x| x.as_str()).unwrap_or(""),
            );
            for nt in s.get("nt").and_then(|x| x.as_array()).into_iter().flatten() {
                if nt.get("itcavl").and_then(|x| x.as_str()) == Some("N") {
                    continue;
                }
                let (b, t) = tax_and_base(nt);
                let credit = nt
                    .get("typ")
                    .and_then(|x| x.as_str())
                    .map(|t| t.eq_ignore_ascii_case("C"))
                    .unwrap_or(true);
                if credit {
                    add(&mut map, id, name, -b, -t);
                } else {
                    add(&mut map, id, name, b, t);
                }
            }
        }
        if map.is_empty() {
            return Err("no B2B invoices found in the GSTR-2B file".into());
        }
        return Ok(map.into_values().collect());
    }
    // Portal Excel: sheet B2B, two header rows (the second has the tax columns).
    let raw = raw_rows(path, &["B2B"])?;
    let h = raw
        .iter()
        .position(|r| r.iter().any(|c| c.to_ascii_lowercase().starts_with("gstin of supplier")))
        .ok_or("no 'GSTIN of supplier' column: use the B2B sheet of GSTR-2B downloaded from the portal")?;
    let width = raw.iter().map(|r| r.len()).max().unwrap_or(0);
    let head: Vec<String> = (0..width)
        .map(|c| {
            let a = raw[h].get(c).cloned().unwrap_or_default();
            let b = raw
                .get(h + 1)
                .and_then(|r| r.get(c))
                .cloned()
                .unwrap_or_default();
            format!("{a} {b}").to_ascii_lowercase()
        })
        .collect();
    let col = |k: &[&str]| head.iter().position(|x| k.iter().any(|w| x.contains(w)));
    let gc = col(&["gstin of supplier"]).ok_or("no GSTIN column")?;
    let nc = col(&["trade/legal name", "trade name", "legal name"]);
    let tv = col(&["taxable value"]);
    let ig = col(&["integrated tax"]);
    let ce = col(&["central tax"]);
    let st = col(&["state/ut tax", "state tax"]);
    let cs = col(&["cess"]);
    let av = col(&["itc availability"]);
    let get =
        |r: &Vec<String>, c: Option<usize>| c.and_then(|c| r.get(c)).cloned().unwrap_or_default();
    let m = |s: String| Money::parse(&s).unwrap_or_default();
    let mut map = BTreeMap::new();
    for r in raw.iter().skip(h + 2) {
        let id = get(r, Some(gc));
        if id.len() != 15 {
            continue;
        }
        if get(r, av).eq_ignore_ascii_case("no") {
            continue;
        }
        let tax = m(get(r, ig)) + m(get(r, ce)) + m(get(r, st)) + m(get(r, cs));
        add(&mut map, &id, &get(r, nc), m(get(r, tv)), tax);
    }
    if map.is_empty() {
        return Err("no B2B invoices found in the sheet".into());
    }
    Ok(map.into_values().collect())
}

fn is_tan(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[..4].iter().all(|c| c.is_ascii_uppercase())
        && b[4..9].iter().all(|c| c.is_ascii_digit())
        && b[9].is_ascii_uppercase()
}

/// Form 26AS, Part I (tax deducted at source): per deductor, amount paid or
/// credited and TDS deposited. Reads the TRACES text file (fields separated
/// by ^) or an Excel/CSV conversion with a 'TAN of Deductor' column.
pub fn read_26as(path: &Path) -> Result<Vec<PortalParty>, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut map: BTreeMap<String, PortalParty> = BTreeMap::new();
    if ext == "txt" {
        let text =
            String::from_utf8_lossy(&std::fs::read(path).map_err(|e| e.to_string())?).to_string();
        let mut part1 = false;
        for line in text.lines() {
            let low = line.to_ascii_lowercase();
            if low.contains("part-i")
                || low.contains("part i") && low.contains("tax deducted at source")
            {
                part1 = true;
            }
            if part1 && (low.contains("part-ii") || low.contains("part ii")) {
                break;
            }
            let f: Vec<&str> = line.split('^').map(|x| x.trim()).collect();
            // Deductor summary rows: Sr. No.^Name^TAN^...^Amount paid^Tax deducted^TDS deposited
            if f.len() < 5 || f[0].is_empty() || !f[0].chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Some(ti) = f.iter().position(|x| is_tan(x)) else {
                continue;
            };
            let nums: Vec<Money> = f
                .iter()
                .skip(ti + 1)
                .filter(|x| !x.is_empty())
                .filter_map(|x| Money::parse(x).ok())
                .collect();
            if nums.len() < 3 {
                continue;
            }
            let n = nums.len();
            let name = f[..ti]
                .iter()
                .rev()
                .find(|x| !x.is_empty() && !x.chars().all(|c| c.is_ascii_digit()))
                .copied()
                .unwrap_or("");
            let e = map.entry(f[ti].to_string()).or_insert_with(|| PortalParty {
                id: f[ti].to_string(),
                name: name.to_string(),
                ..Default::default()
            });
            e.documents += 1;
            e.base += nums[n - 3];
            e.tax += nums[n - 1];
        }
    } else {
        let t = read_table(path, &["Part I", "26AS"], &["tan of deductor"])?;
        let tc = t
            .col(&["tan of deductor"])
            .ok_or("no TAN of Deductor column")?;
        let nc = t.col(&["name of deductor"]);
        let ac = t.col(&[
            "total amount paid / credited(rs.)",
            "total amount paid / credited",
            "amount paid / credited",
            "total amount paid/credited",
        ]);
        let dc = t.col(&[
            "total tds deposited(rs.)",
            "total tds deposited",
            "tds deposited",
            "tax deposited",
        ]);
        for row in &t.rows {
            let tan = t.get(row, Some(tc)).to_string();
            if !is_tan(&tan) {
                continue;
            }
            let e = map.entry(tan.clone()).or_insert_with(|| PortalParty {
                id: tan.clone(),
                name: t.get(row, nc).to_string(),
                ..Default::default()
            });
            e.documents += 1;
            e.base += Money::parse(t.get(row, ac)).unwrap_or_default();
            e.tax += Money::parse(t.get(row, dc)).unwrap_or_default();
        }
    }
    if map.is_empty() {
        return Err("no deductor rows found: download Form 26AS as a text file from TRACES (or its Excel conversion)".into());
    }
    Ok(map.into_values().collect())
}
