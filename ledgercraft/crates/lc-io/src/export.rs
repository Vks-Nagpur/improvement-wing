//! One-click export: writes the selected outputs into a new, never-overwritten
//! version folder, atomically (temp folder renamed when complete), with a
//! manifest of SHA-256 hashes.
//!
//! Pipeline: analysis → `lc_core::report::build` (content, rounding) →
//! renderers (PDF / Excel / HTML share one design system).

use chrono::Local;
use lc_core::checks::Finding;
use lc_core::model::Engagement;
use lc_core::report::{Report, ReportOptions};
use lc_core::rules::Severity;
use lc_core::{Analysis, Money};
use rust_xlsxwriter::{Format, FormatBorder, Workbook, Worksheet};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub use lc_core::report::{SignOff, Signatory};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Draft,
    Signing,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub mode: Mode,
    pub pdf: bool,
    pub statements_xlsx: bool,
    pub statements_html: bool,
    pub auditor_workbook: bool,
    pub json: bool,
    pub report: ReportOptions,
    /// Manual adjustments already applied to the books (listed in the workbook).
    pub adjustments: Vec<lc_core::adjust::Adjustment>,
    pub adjustment_effects: Vec<lc_core::adjust::Applied>,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            mode: Mode::Draft,
            pdf: true,
            statements_xlsx: true,
            statements_html: true,
            auditor_workbook: true,
            json: true,
            report: ReportOptions::default(),
            adjustments: Vec::new(),
            adjustment_effects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Exported {
    pub dir: PathBuf,
    /// Items the preparer still has to attend to (not printed).
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
struct Manifest<'a> {
    app: &'a str,
    app_version: &'a str,
    rules_version: &'a str,
    format_pack: &'a str,
    entity: &'a str,
    financial_year: String,
    mode: &'a str,
    created: String,
    files: Vec<(String, String)>,
}

fn safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Build the printable report for these options.
pub fn report(eng: &Engagement, a: &Analysis, signoff: &SignOff, opt: &ExportOptions) -> Report {
    let mut ro = opt.report.clone();
    ro.draft = opt.mode == Mode::Draft;
    lc_core::report::build(eng, a, &ro, signoff)
}

/// Export to `<root>/<entity>/FY <yyyy-yy>/<Draft|Final>-<date>_v<n>/`.
pub fn export(
    root: &Path,
    eng: &Engagement,
    a: &Analysis,
    signoff: &SignOff,
    opt: &ExportOptions,
) -> Result<Exported, String> {
    if opt.mode == Mode::Signing && !a.printable {
        let n = a.count(Severity::Blocker);
        return Err(format!("Signing copy refused: {n} item(s) marked 'Must fix' are still open. Export as draft or fix them first."));
    }
    let rep = report(eng, a, signoff, opt);
    let fy = lc_core::date::fy_label(eng.fy_start);
    let base = root.join(safe(&eng.entity_name)).join(format!("FY {fy}"));
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let stamp = Local::now().format("%Y-%m-%d").to_string();
    let prefix = if opt.mode == Mode::Signing {
        "Final"
    } else {
        "Draft"
    };
    let mut n = 1;
    let final_dir = loop {
        let p = base.join(format!("{prefix}-{stamp}_v{n}"));
        if !p.exists() {
            break p;
        }
        n += 1;
    };
    let tmp = base.join(format!(".tmp-{}-{}", std::process::id(), n));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;

    let result = (|| -> Result<(), String> {
        let draft = opt.mode == Mode::Draft;
        if opt.pdf {
            fs::write(
                tmp.join("Financial_Statements.pdf"),
                crate::render::pdf::render(&rep)?,
            )
            .map_err(|e| e.to_string())?;
        }
        if opt.statements_xlsx {
            fs::write(
                tmp.join("Financial_Statements.xlsx"),
                crate::render::xlsx::render(&rep)?,
            )
            .map_err(|e| e.to_string())?;
        }
        if opt.statements_html {
            fs::write(
                tmp.join("Financial_Statements_preview.html"),
                crate::render::html::render(&rep),
            )
            .map_err(|e| e.to_string())?;
        }
        if opt.auditor_workbook {
            write_auditor_workbook(&tmp.join("Auditor_Reference_Workbook.xlsx"), eng, a, opt)?;
        }
        if opt.json {
            fs::write(
                tmp.join("analysis.json"),
                serde_json::to_string_pretty(a).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        let mut files = Vec::new();
        let mut names: Vec<_> = fs::read_dir(&tmp)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        names.sort();
        for name in names {
            let bytes = fs::read(tmp.join(&name)).map_err(|e| e.to_string())?;
            files.push((
                name.to_string_lossy().to_string(),
                format!("{:x}", Sha256::digest(&bytes)),
            ));
        }
        let m = Manifest {
            app: "LedgerCraft",
            app_version: env!("CARGO_PKG_VERSION"),
            rules_version: &a.rules_version,
            format_pack: &a.statements.pack_id,
            entity: &eng.entity_name,
            financial_year: fy.clone(),
            mode: if draft { "draft" } else { "signing" },
            created: Local::now().to_rfc3339(),
            files,
        };
        fs::write(
            tmp.join("export-manifest.json"),
            serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&tmp);
        return Err(e);
    }
    fs::rename(&tmp, &final_dir).map_err(|e| e.to_string())?;
    Ok(Exported {
        dir: final_dir,
        warnings: rep.warnings,
    })
}

fn x<T>(r: Result<T, rust_xlsxwriter::XlsxError>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

#[allow(dead_code)]
struct Fmts {
    title: Format,
    bold: Format,
    head: Format,
    num: Format,
    num_bold: Format,
    num_total: Format,
    wrap: Format,
}

fn fmts() -> Fmts {
    let nf = "#,##0.00;(#,##0.00);\"-\"";
    Fmts {
        title: Format::new().set_bold().set_font_size(12),
        bold: Format::new().set_bold(),
        head: Format::new()
            .set_bold()
            .set_border_bottom(FormatBorder::Thin)
            .set_text_wrap(),
        num: Format::new().set_num_format(nf),
        num_bold: Format::new().set_num_format(nf).set_bold(),
        num_total: Format::new()
            .set_num_format(nf)
            .set_bold()
            .set_border_top(FormatBorder::Thin)
            .set_border_bottom(FormatBorder::Double),
        wrap: Format::new().set_text_wrap(),
    }
}

fn finding_sheet(
    ws: &mut Worksheet,
    f: &Fmts,
    items: &[&Finding],
    expert: bool,
) -> Result<(), String> {
    let mut heads = vec![
        "Status",
        "Check",
        "Date",
        "Voucher(s)",
        "Ledger",
        "Amount",
        "What we found",
        "Suggested action",
    ];
    if expert {
        heads.push("Legal reference");
    }
    heads.push("Auditor remarks");
    for (c, h) in heads.iter().enumerate() {
        x(ws.write_string_with_format(0, c as u16, *h, &f.head))?;
    }
    for (i, it) in items.iter().enumerate() {
        let r = i as u32 + 1;
        x(ws.write_string(r, 0, it.severity.label()))?;
        x(ws.write_string(r, 1, &it.title))?;
        if let Some(d) = it.date {
            x(ws.write_string(r, 2, d.format("%d-%m-%Y").to_string()))?;
        }
        x(ws.write_string(r, 3, it.voucher.clone().unwrap_or_default()))?;
        x(ws.write_string(r, 4, it.ledger.clone().unwrap_or_default()))?;
        if let Some(a) = it.amount {
            x(ws.write_number_with_format(r, 5, a.as_f64(), &f.num))?;
        }
        x(ws.write_string_with_format(r, 6, &it.message, &f.wrap))?;
        x(ws.write_string_with_format(r, 7, it.suggestion.clone().unwrap_or_default(), &f.wrap))?;
        if expert {
            x(ws.write_string_with_format(r, 8, &it.legal_ref, &f.wrap))?;
        }
    }
    for (c, w) in [
        (0, 10.0),
        (1, 30.0),
        (2, 11.0),
        (3, 26.0),
        (4, 28.0),
        (5, 15.0),
        (6, 70.0),
        (7, 45.0),
        (8, 40.0),
        (9, 30.0),
    ] {
        ws.set_column_width(c, w).ok();
    }
    ws.set_freeze_panes(1, 0).ok();
    ws.autofilter(0, 0, items.len() as u32, heads.len() as u16 - 1)
        .ok();
    Ok(())
}

fn write_auditor_workbook(
    path: &Path,
    eng: &Engagement,
    a: &Analysis,
    opt: &ExportOptions,
) -> Result<(), String> {
    let f = fmts();
    let mut wb = Workbook::new();
    {
        let ws = x(wb.add_worksheet().set_name("Summary"))?;
        x(ws.write_string_with_format(
            0,
            0,
            format!(
                "{} – Auditor Reference Workbook (FY {})",
                eng.entity_name,
                lc_core::date::fy_label(eng.fy_start)
            ),
            &f.title,
        ))?;
        x(ws.write_string(1, 0, "Flags for the auditor's review. LedgerCraft does not decide disallowances; the auditor does."))?;
        x(ws.write_string(2, 0, format!("Rules version: {}", a.rules_version)))?;
        for (c, h) in ["Status", "Check", "Count", "Legal reference"]
            .iter()
            .enumerate()
        {
            x(ws.write_string_with_format(4, c as u16, *h, &f.head))?;
        }
        let mut codes: Vec<(&str, &Finding, usize)> = Vec::new();
        for it in &a.findings {
            match codes.iter_mut().find(|(c, _, _)| *c == it.code) {
                Some(e) => e.2 += 1,
                None => codes.push((it.code.as_str(), it, 1)),
            }
        }
        for (i, (_, it, n)) in codes.iter().enumerate() {
            let r = 5 + i as u32;
            x(ws.write_string(r, 0, it.severity.label()))?;
            x(ws.write_string(r, 1, &it.title))?;
            x(ws.write_number(r, 2, *n as f64))?;
            x(ws.write_string_with_format(r, 3, &it.legal_ref, &f.wrap))?;
        }
        if codes.is_empty() {
            x(ws.write_string(5, 1, "No issues found."))?;
        }
        ws.set_column_width(0, 10).ok();
        ws.set_column_width(1, 50).ok();
        ws.set_column_width(3, 70).ok();
    }
    let all: Vec<&Finding> = a.findings.iter().collect();
    finding_sheet(
        x(wb.add_worksheet().set_name("All findings"))?,
        &f,
        &all,
        true,
    )?;
    let groups: [(&str, &[&str]); 5] = [
        (
            "Cash limits",
            &[
                "CASH_PAYMENT_LIMIT",
                "CASH_ASSET_PURCHASE",
                "CASH_RECEIPT_LIMIT",
                "NEGATIVE_CASH",
            ],
        ),
        (
            "Loans 269SS-269T",
            &[
                "LOAN_ACCEPTED_CASH",
                "LOAN_ACCEPTED_JOURNAL",
                "LOAN_REPAID_CASH",
                "MISGROUP_LOAN",
            ],
        ),
        (
            "Opening balances",
            &[
                "OPENING_DIFF",
                "OPENING_TOTAL_DIFF",
                "OPENING_MISSING_LEDGER",
                "OPENING_NEW_LEDGER",
                "PL_OPENING",
            ],
        ),
        (
            "Grouping",
            &[
                "MISGROUP_LOAN",
                "MISGROUP_OD",
                "MISGROUP_TAX",
                "CAPITAL_IN_EXPENSE",
                "MISC_EXP_ASSET",
                "ABNORMAL_BALANCE",
                "RECLASS_DEBTOR_CR",
                "RECLASS_CREDITOR_DR",
                "RECLASS_BANK_CR",
                "SUSPENSE_BALANCE",
            ],
        ),
        (
            "Data quality",
            &[
                "TB_UNBALANCED",
                "UNKNOWN_GROUP",
                "UNMAPPED",
                "VOUCHER_UNBALANCED",
                "VOUCHER_UNKNOWN_LEDGER",
                "VOUCHER_TB_MISMATCH",
                "VOUCHER_OUTSIDE_PERIOD",
                "DUPLICATE_VOUCHER",
                "CASH_CR_BALANCE",
                "STATEMENT_NOT_BALANCED",
            ],
        ),
    ];
    for (name, codes) in groups {
        let items: Vec<&Finding> = a
            .findings
            .iter()
            .filter(|it| codes.contains(&it.code.as_str()))
            .collect();
        finding_sheet(x(wb.add_worksheet().set_name(name))?, &f, &items, true)?;
    }
    {
        let ws = x(wb.add_worksheet().set_name("Loan register"))?;
        x(ws.write_string_with_format(0, 0, "Loans and deposits taken – helper for Form 3CD clause 31 (amounts owed shown positive)", &f.title))?;
        let heads = [
            "Lender (ledger)",
            "Group",
            "Exempt (bank etc.)",
            "Opening",
            "Accepted – bank",
            "Accepted – cash",
            "Accepted – journal",
            "Principal accepted",
            "Interest credited",
            "TDS on interest",
            "Repaid – bank",
            "Repaid – cash",
            "Repaid – journal",
            "Closing",
            "Maximum outstanding",
            "Squared up",
        ];
        for (c, h) in heads.iter().enumerate() {
            x(ws.write_string_with_format(2, c as u16, *h, &f.head))?;
        }
        for (i, l) in a.loans.iter().enumerate() {
            let r = 3 + i as u32;
            x(ws.write_string(r, 0, &l.ledger))?;
            x(ws.write_string(r, 1, &l.group))?;
            x(ws.write_string(r, 2, if l.exempt { "Yes" } else { "" }))?;
            let nums: [Money; 12] = [
                l.opening,
                l.accepted_bank,
                l.accepted_cash,
                l.accepted_journal,
                l.principal_accepted(),
                l.interest_credited,
                l.tds_on_interest,
                l.repaid_bank,
                l.repaid_cash,
                l.repaid_journal,
                l.closing,
                l.max_outstanding,
            ];
            for (j, v) in nums.iter().enumerate() {
                x(ws.write_number_with_format(r, 3 + j as u16, v.as_f64(), &f.num))?;
            }
            x(ws.write_string(r, 15, if l.squared_up { "Yes" } else { "No" }))?;
        }
        ws.set_column_width(0, 32).ok();
        for c in 1..16 {
            ws.set_column_width(c, 15).ok();
        }
        ws.set_freeze_panes(3, 1).ok();
    }
    if !opt.adjustments.is_empty() {
        let ws = x(wb.add_worksheet().set_name("Adjustments"))?;
        x(ws.write_string_with_format(0, 0, "Manual adjustments passed in LedgerCraft (not in the books; pass the same entries in the books)", &f.title))?;
        let heads = [
            "No.",
            "Type",
            "Status",
            "Narration",
            "Ledger",
            "Debit",
            "Credit",
            "Balance before",
            "Balance after",
            "New ledger",
        ];
        for (c, h) in heads.iter().enumerate() {
            x(ws.write_string_with_format(2, c as u16, *h, &f.head))?;
        }
        let mut r = 3u32;
        for adj in &opt.adjustments {
            for l in adj.lines.iter().filter(|l| !l.amount.is_zero()) {
                x(ws.write_number(r, 0, adj.id as f64))?;
                x(ws.write_string(r, 1, adj.kind.label()))?;
                x(ws.write_string(
                    r,
                    2,
                    if adj.active {
                        "Applied"
                    } else {
                        "Switched off"
                    },
                ))?;
                x(ws.write_string_with_format(r, 3, &adj.narration, &f.wrap))?;
                x(ws.write_string(r, 4, &l.ledger))?;
                let (dr, cr) = if l.amount.0 < 0 {
                    (0.0, -l.amount.as_f64())
                } else {
                    (l.amount.as_f64(), 0.0)
                };
                x(ws.write_number_with_format(r, 5, dr, &f.num))?;
                x(ws.write_number_with_format(r, 6, cr, &f.num))?;
                if let Some(e) = opt.adjustment_effects.iter().find(|e| {
                    e.id == adj.id
                        && lc_core::model::norm_name(&e.ledger)
                            == lc_core::model::norm_name(&l.ledger)
                }) {
                    x(ws.write_number_with_format(r, 7, e.before.as_f64(), &f.num))?;
                    x(ws.write_number_with_format(r, 8, e.after.as_f64(), &f.num))?;
                    if e.created {
                        x(ws.write_string(r, 9, l.new_group.as_deref().unwrap_or("Yes")))?;
                    }
                }
                r += 1;
            }
        }
        ws.set_column_width(3, 40).ok();
        ws.set_column_width(4, 30).ok();
        for c in [1u16, 2, 5, 6, 7, 8, 9] {
            ws.set_column_width(c, 15).ok();
        }
        ws.set_freeze_panes(3, 0).ok();
    }
    x(wb.save(path))
}
