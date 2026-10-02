//! One-click export: writes the selected outputs into a new, never-overwritten
//! version folder, atomically (temp folder renamed when complete), with a
//! manifest of SHA-256 hashes.

use chrono::Local;
use lc_core::checks::Finding;
use lc_core::model::Engagement;
use lc_core::rules::Severity;
use lc_core::statements::{Row, RowKind, Statements};
use lc_core::{Analysis, Money};
use rust_xlsxwriter::{Format, FormatBorder, Workbook, Worksheet};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Sign-off details printed on the statements (UDIN is paste-only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignOff {
    #[serde(default)]
    pub signatories: Vec<String>,
    #[serde(default)]
    pub signatory_title: String,
    #[serde(default)]
    pub auditor_firm: String,
    #[serde(default)]
    pub frn: String,
    #[serde(default)]
    pub auditor_partner: String,
    #[serde(default)]
    pub membership_no: String,
    #[serde(default)]
    pub udin: String,
    #[serde(default)]
    pub place: String,
    #[serde(default)]
    pub date: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Draft,
    Signing,
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub mode: Mode,
    pub statements_xlsx: bool,
    pub statements_html: bool,
    pub auditor_workbook: bool,
    pub json: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            mode: Mode::Draft,
            statements_xlsx: true,
            statements_html: true,
            auditor_workbook: true,
            json: true,
        }
    }
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

/// Export to `<root>/<entity>/FY <yyyy-yy>/<Draft|Final>-<date>_v<n>/`. Returns the folder.
pub fn export(
    root: &Path,
    eng: &Engagement,
    a: &Analysis,
    signoff: &SignOff,
    opt: &ExportOptions,
) -> Result<PathBuf, String> {
    if opt.mode == Mode::Signing && !a.printable {
        let n = a.count(Severity::Blocker);
        return Err(format!("Signing copy refused: {n} item(s) marked 'Must fix' are still open. Export as draft or fix them first."));
    }
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
        if opt.statements_xlsx {
            write_statements_xlsx(
                &tmp.join("Financial_Statements.xlsx"),
                eng,
                a,
                signoff,
                draft,
            )?;
        }
        if opt.statements_html {
            fs::write(
                tmp.join("Financial_Statements_print.html"),
                statements_html(eng, a, signoff, draft),
            )
            .map_err(|e| e.to_string())?;
        }
        if opt.auditor_workbook {
            write_auditor_workbook(&tmp.join("Auditor_Reference_Workbook.xlsx"), eng, a)?;
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
            let hash = format!("{:x}", Sha256::digest(&bytes));
            files.push((name.to_string_lossy().to_string(), hash));
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
    Ok(final_dir)
}

/// Column headings: "As at" for the Balance Sheet, "For the year ended" for P&L and notes.
fn period_heads(eng: &Engagement, as_at: bool) -> (String, String) {
    let lead = if as_at { "As at" } else { "For the year ended" };
    let py_end = eng.fy_start - chrono::Duration::days(1);
    (
        format!("{lead} {}", eng.fy_end.format("%d-%m-%Y")),
        format!("{lead} {}", py_end.format("%d-%m-%Y")),
    )
}

fn note_heads(eng: &Engagement) -> (String, String) {
    let py_end = eng.fy_start - chrono::Duration::days(1);
    (
        eng.fy_end.format("%d-%m-%Y").to_string(),
        py_end.format("%d-%m-%Y").to_string(),
    )
}

fn x<T>(r: Result<T, rust_xlsxwriter::XlsxError>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

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

#[allow(clippy::too_many_arguments)]
fn write_statement(
    ws: &mut Worksheet,
    f: &Fmts,
    eng: &Engagement,
    title: &str,
    rows: &[Row],
    has_py: bool,
    signoff: &SignOff,
    draft: bool,
    as_at: bool,
) -> Result<(), String> {
    let (cyh, pyh) = period_heads(eng, as_at);
    x(ws.write_string_with_format(0, 0, &eng.entity_name, &f.title))?;
    x(ws.write_string_with_format(1, 0, title, &f.bold))?;
    x(ws.write_string(2, 0, "(All amounts in ₹)"))?;
    if draft {
        x(ws.write_string_with_format(2, 3, "DRAFT", &f.bold))?;
    }
    x(ws.write_string_with_format(4, 0, "Particulars", &f.head))?;
    x(ws.write_string_with_format(4, 1, "Note No.", &f.head))?;
    x(ws.write_string_with_format(4, 2, &cyh, &f.head))?;
    if has_py {
        x(ws.write_string_with_format(4, 3, &pyh, &f.head))?;
    }
    let mut r = 5u32;
    for row in rows {
        let (lf, nf) = match row.kind {
            RowKind::Heading | RowKind::SubHeading => (&f.bold, &f.num),
            RowKind::Item => (&f.wrap, &f.num),
            RowKind::Subtotal => (&f.bold, &f.num_bold),
            RowKind::Total => (&f.bold, &f.num_total),
        };
        x(ws.write_string_with_format(r, 0, &row.label, lf))?;
        if let Some(n) = row.note {
            x(ws.write_number(r, 1, n as f64))?;
        }
        if let Some(v) = row.cy {
            x(ws.write_number_with_format(r, 2, v.as_f64(), nf))?;
        }
        if has_py {
            if let Some(v) = row.py {
                x(ws.write_number_with_format(r, 3, v.as_f64(), nf))?;
            }
        }
        r += 1;
    }
    r += 1;
    x(ws.write_string(
        r,
        0,
        "The accompanying notes form an integral part of the financial statements.",
    ))?;
    r += 2;
    for line in signature_lines(eng, signoff) {
        x(ws.write_string(r, 0, &line))?;
        r += 1;
    }
    ws.set_column_width(0, 62).ok();
    ws.set_column_width(1, 9).ok();
    ws.set_column_width(2, 22).ok();
    ws.set_column_width(3, 22).ok();
    Ok(())
}

fn signature_lines(eng: &Engagement, s: &SignOff) -> Vec<String> {
    let blank = |v: &str| {
        if v.trim().is_empty() {
            "____________".to_string()
        } else {
            v.to_string()
        }
    };
    let title = if s.signatory_title.is_empty() {
        match eng.entity_type {
            lc_core::EntityType::Company => "Director",
            lc_core::EntityType::Llp => "Designated Partner",
            lc_core::EntityType::Firm => "Partner",
            lc_core::EntityType::Proprietor => "Proprietor",
            lc_core::EntityType::Huf => "Karta",
            lc_core::EntityType::Aop | lc_core::EntityType::Boi => "Authorised Member",
        }
        .to_string()
    } else {
        s.signatory_title.clone()
    };
    let mut v = vec![
        "As per our report of even date".to_string(),
        format!("For {}", blank(&s.auditor_firm)),
        "Chartered Accountants".to_string(),
        format!("Firm Registration No.: {}", blank(&s.frn)),
        String::new(),
        format!("{} (Partner)", blank(&s.auditor_partner)),
        format!("Membership No.: {}", blank(&s.membership_no)),
        format!("UDIN: {}", blank(&s.udin)),
        String::new(),
        format!("For and on behalf of {}", eng.entity_name),
    ];
    if s.signatories.is_empty() {
        v.push(format!("____________ ({title})"));
        if !matches!(
            eng.entity_type,
            lc_core::EntityType::Proprietor | lc_core::EntityType::Huf
        ) {
            v.push(format!("____________ ({title})"));
        }
    } else {
        for n in &s.signatories {
            v.push(format!("{n} ({title})"));
        }
    }
    v.push(String::new());
    v.push(format!("Place: {}", blank(&s.place)));
    v.push(format!("Date: {}", blank(&s.date)));
    v
}

fn write_statements_xlsx(
    path: &Path,
    eng: &Engagement,
    a: &Analysis,
    signoff: &SignOff,
    draft: bool,
) -> Result<(), String> {
    let st: &Statements = &a.statements;
    let has_py = eng.py.is_some();
    let f = fmts();
    let mut wb = Workbook::new();
    {
        let ws = x(wb.add_worksheet().set_name("Balance Sheet"))?;
        write_statement(
            ws,
            &f,
            eng,
            &st.bs_title,
            &st.balance_sheet,
            has_py,
            signoff,
            draft,
            true,
        )?;
    }
    {
        let ws = x(wb.add_worksheet().set_name("Profit and Loss"))?;
        write_statement(
            ws,
            &f,
            eng,
            &st.pl_title,
            &st.profit_loss,
            has_py,
            signoff,
            draft,
            false,
        )?;
    }
    {
        let ws = x(wb.add_worksheet().set_name("Notes"))?;
        let (cyh, pyh) = note_heads(eng);
        x(ws.write_string_with_format(0, 0, &eng.entity_name, &f.title))?;
        x(ws.write_string_with_format(1, 0, "Notes to the financial statements", &f.bold))?;
        x(ws.write_string(2, 0, "(All amounts in ₹)"))?;
        let mut r = 4u32;
        for n in &st.notes {
            x(ws.write_string_with_format(r, 0, format!("Note {}: {}", n.no, n.title), &f.head))?;
            x(ws.write_string_with_format(r, 2, &cyh, &f.head))?;
            if has_py {
                x(ws.write_string_with_format(r, 3, &pyh, &f.head))?;
            }
            r += 1;
            for l in &n.lines {
                x(ws.write_string(r, 0, &l.label))?;
                x(ws.write_number_with_format(r, 2, l.cy.as_f64(), &f.num))?;
                if let (true, Some(p)) = (has_py, l.py) {
                    x(ws.write_number_with_format(r, 3, p.as_f64(), &f.num))?;
                }
                r += 1;
            }
            x(ws.write_string_with_format(r, 0, "Total", &f.bold))?;
            x(ws.write_number_with_format(r, 2, n.total_cy.as_f64(), &f.num_total))?;
            if let (true, Some(p)) = (has_py, n.total_py) {
                x(ws.write_number_with_format(r, 3, p.as_f64(), &f.num_total))?;
            }
            r += 2;
        }
        ws.set_column_width(0, 62).ok();
        ws.set_column_width(2, 22).ok();
        ws.set_column_width(3, 22).ok();
    }
    {
        let ws = x(wb.add_worksheet().set_name("Mapping"))?;
        for (c, h) in [
            "Ledger",
            "Group",
            "Standard group",
            "Shown under",
            "How mapped",
            "Reclassified by balance side",
            "Amount (Dr+ / Cr-)",
        ]
        .iter()
        .enumerate()
        {
            x(ws.write_string_with_format(0, c as u16, *h, &f.head))?;
        }
        for (i, m) in a.mapping.iter().enumerate() {
            let r = i as u32 + 1;
            x(ws.write_string(r, 0, &m.name))?;
            x(ws.write_string(r, 1, &m.group))?;
            x(ws.write_string(r, 2, m.class.map(|c| c.label()).unwrap_or("NOT RECOGNISED")))?;
            x(ws.write_string(
                r,
                3,
                m.head.map(|h| h.id()).unwrap_or_else(|| "UNMAPPED".into()),
            ))?;
            x(ws.write_string(r, 4, m.source.map(|s| format!("{s:?}")).unwrap_or_default()))?;
            x(ws.write_string(r, 5, if m.reclassified { "Yes" } else { "" }))?;
            x(ws.write_number_with_format(r, 6, m.amount.as_f64(), &f.num))?;
        }
        ws.set_column_width(0, 40).ok();
        ws.set_column_width(1, 26).ok();
        ws.set_column_width(2, 24).ok();
        ws.set_column_width(3, 28).ok();
        ws.set_column_width(6, 18).ok();
    }
    x(wb.save(path))
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

fn write_auditor_workbook(path: &Path, eng: &Engagement, a: &Analysis) -> Result<(), String> {
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
    x(wb.save(path))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn money_cell(v: Option<Money>) -> String {
    match v {
        Some(m) if m.is_zero() => "-".into(),
        Some(m) => m.fmt_indian(),
        None => String::new(),
    }
}

/// Plain, print-ready HTML (A4). Opens in any browser; Print → Save as PDF.
pub fn statements_html(eng: &Engagement, a: &Analysis, signoff: &SignOff, draft: bool) -> String {
    let st = &a.statements;
    let has_py = eng.py.is_some();
    let (ncy, npy) = note_heads(eng);
    let mut h = String::new();
    h.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>");
    h.push_str(&esc(&eng.entity_name));
    h.push_str(" – Financial Statements</title><style>\n@page{size:A4;margin:18mm 15mm}\nbody{font-family:'Times New Roman',serif;font-size:11pt;color:#000;background:#fff}\nh1{font-size:13pt;text-align:center;margin:0}\nh2{font-size:11.5pt;text-align:center;margin:2px 0 8px}\n.unit{text-align:right;font-size:9.5pt}\ntable{width:100%;border-collapse:collapse}\nth{border-bottom:1px solid #000;text-align:right;padding:3px 4px;font-size:10pt}\nth:first-child{text-align:left}\ntd{padding:2px 4px;vertical-align:top}\ntd.n{text-align:right;white-space:nowrap}\ntd.c{text-align:center}\ntr.h td{font-weight:bold;padding-top:6px}\ntr.s td{font-weight:bold}\ntr.t td{font-weight:bold;border-top:1px solid #000;border-bottom:3px double #000}\n.page{page-break-after:always}\n.sig{margin-top:28px;display:flex;justify-content:space-between;font-size:10.5pt}\n.draft{position:fixed;top:40%;left:20%;font-size:80pt;color:rgba(0,0,0,.08);transform:rotate(-30deg)}\n</style></head><body>\n");
    if draft {
        h.push_str("<div class=\"draft\">DRAFT</div>\n");
    }
    let table = |rows: &[Row], title: &str, as_at: bool, h: &mut String| {
        let (cyh, pyh) = period_heads(eng, as_at);
        h.push_str("<div class=\"page\">");
        h.push_str(&format!(
            "<h1>{}</h1><h2>{}</h2><div class=\"unit\">(All amounts in ₹)</div>",
            esc(&eng.entity_name),
            esc(title)
        ));
        h.push_str(&format!(
            "<table><tr><th>Particulars</th><th>Note</th><th>{}</th>{}</tr>",
            esc(&cyh),
            if has_py {
                format!("<th>{}</th>", esc(&pyh))
            } else {
                String::new()
            }
        ));
        for r in rows {
            let cls = match r.kind {
                RowKind::Heading | RowKind::SubHeading => "h",
                RowKind::Subtotal => "s",
                RowKind::Total => "t",
                RowKind::Item => "",
            };
            h.push_str(&format!(
                "<tr class=\"{cls}\"><td>{}</td><td class=\"c\">{}</td><td class=\"n\">{}</td>{}</tr>",
                esc(&r.label),
                r.note.map(|n| n.to_string()).unwrap_or_default(),
                money_cell(r.cy),
                if has_py { format!("<td class=\"n\">{}</td>", money_cell(r.py)) } else { String::new() }
            ));
        }
        h.push_str("</table><p>The accompanying notes form an integral part of the financial statements.</p>");
        let lines = signature_lines(eng, signoff);
        let split = lines
            .iter()
            .position(|l| l.starts_with("For and on behalf"))
            .unwrap_or(lines.len());
        h.push_str("<div class=\"sig\"><div>");
        for l in &lines[..split] {
            h.push_str(&format!("{}<br>", esc(l)));
        }
        h.push_str("</div><div>");
        for l in &lines[split..] {
            h.push_str(&format!("{}<br>", esc(l)));
        }
        h.push_str("</div></div></div>\n");
    };
    table(&st.balance_sheet, &st.bs_title, true, &mut h);
    table(&st.profit_loss, &st.pl_title, false, &mut h);
    h.push_str(&format!("<h1>{}</h1><h2>Notes to the financial statements</h2><div class=\"unit\">(All amounts in ₹)</div>", esc(&eng.entity_name)));
    for n in &st.notes {
        h.push_str(&format!(
            "<table style=\"margin-bottom:12px\"><tr><th>Note {}: {}</th><th>{}</th>{}</tr>",
            n.no,
            esc(&n.title),
            esc(&ncy),
            if has_py {
                format!("<th>{}</th>", esc(&npy))
            } else {
                String::new()
            }
        ));
        for l in &n.lines {
            h.push_str(&format!(
                "<tr><td>{}</td><td class=\"n\">{}</td>{}</tr>",
                esc(&l.label),
                money_cell(Some(l.cy)),
                if has_py {
                    format!("<td class=\"n\">{}</td>", money_cell(l.py))
                } else {
                    String::new()
                }
            ));
        }
        h.push_str(&format!(
            "<tr class=\"t\"><td>Total</td><td class=\"n\">{}</td>{}</tr></table>",
            money_cell(Some(n.total_cy)),
            if has_py {
                format!("<td class=\"n\">{}</td>", money_cell(n.total_py))
            } else {
                String::new()
            }
        ));
    }
    h.push_str("</body></html>\n");
    h
}
