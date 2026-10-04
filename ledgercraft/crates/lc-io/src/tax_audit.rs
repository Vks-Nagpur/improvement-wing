//! Tax audit helper workbook: one sheet per clause that the books can
//! support, laid out like the clause so the auditor can copy, verify and
//! complete it. LedgerCraft fills what the books show; columns it cannot know
//! (PAN, address, GST status, due-date payment) are left for the auditor.
//!
//! Up to FY 2025-26: Form 3CA/3CB with Form 3CD (Income-tax Act, 1961).
//! From Tax Year 2026-27: Form 26 (section 63 of the Income-tax Act, 2025,
//! Rule 47). Form 26 clause numbers are not yet mapped; the sheets carry the
//! Form 3CD subject and the new-Act section.

use lc_core::engine::Analysis;
use lc_core::groups::Class;
use lc_core::mapping::Head;
use lc_core::model::{norm_name, Engagement};
use lc_core::money::Money;
use rust_xlsxwriter::{Format, FormatBorder, Workbook, Worksheet};
use std::path::Path;

fn x<T>(r: Result<T, rust_xlsxwriter::XlsxError>) -> Result<T, String> {
    r.map_err(|e| e.to_string())
}

struct F {
    title: Format,
    sub: Format,
    head: Format,
    num: Format,
    wrap: Format,
    note: Format,
}

fn fmts() -> F {
    let nf = "#,##0.00;(#,##0.00);\"-\"";
    F {
        title: Format::new().set_bold().set_font_size(12),
        sub: Format::new().set_italic().set_font_color("595959"),
        head: Format::new()
            .set_bold()
            .set_text_wrap()
            .set_background_color("EDEDED")
            .set_border(FormatBorder::Thin),
        num: Format::new()
            .set_num_format(nf)
            .set_border(FormatBorder::Thin),
        wrap: Format::new().set_text_wrap().set_border(FormatBorder::Thin),
        note: Format::new()
            .set_text_wrap()
            .set_italic()
            .set_font_color("7F4F00"),
    }
}

/// Which form and Act apply to the year.
pub fn form_for(eng: &Engagement) -> (&'static str, bool) {
    let new_act = eng.fy_start >= chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    if new_act {
        ("Form 26 (section 63, Income-tax Act, 2025)", true)
    } else {
        ("Form 3CD (section 44AB, Income-tax Act, 1961)", false)
    }
}

pub fn file_name(eng: &Engagement) -> &'static str {
    if form_for(eng).1 {
        "Tax_Audit_Helper_Form_26.xlsx"
    } else {
        "Tax_Audit_Helper_Form_3CD.xlsx"
    }
}

fn header(
    ws: &mut Worksheet,
    f: &F,
    title: &str,
    sub: &str,
    cols: &[(&str, f64)],
) -> Result<u32, String> {
    x(ws.write_string_with_format(0, 0, title, &f.title))?;
    x(ws.write_string_with_format(1, 0, sub, &f.sub))?;
    for (c, (h, w)) in cols.iter().enumerate() {
        x(ws.write_string_with_format(3, c as u16, *h, &f.head))?;
        x(ws.set_column_width(c as u16, *w))?;
    }
    x(ws.set_row_height(3, 32))?;
    ws.set_freeze_panes(4, 0).ok();
    Ok(4)
}

fn money(ws: &mut Worksheet, f: &F, r: u32, c: u16, m: Money) -> Result<(), String> {
    x(ws.write_number_with_format(r, c, m.as_f64(), &f.num)).map(|_| ())
}

fn text(ws: &mut Worksheet, f: &F, r: u32, c: u16, t: &str) -> Result<(), String> {
    x(ws.write_string_with_format(r, c, t, &f.wrap)).map(|_| ())
}

fn findings(a: &Analysis, codes: &[&str]) -> Vec<lc_core::checks::Finding> {
    a.findings
        .iter()
        .filter(|f| codes.contains(&f.code.as_str()))
        .cloned()
        .collect()
}

/// (turnover, gross profit, net profit, closing stock) of one year.
type Figures = (Money, Money, Money, Money);
type RatioRow<'a> = (&'a str, Box<dyn Fn(&Figures) -> f64>);

pub fn write(path: &Path, eng: &Engagement, a: &Analysis) -> Result<(), String> {
    let f = fmts();
    let (form, new_act) = form_for(eng);
    let sec = |old: &'static str, new: &'static str| if new_act { new } else { old };
    let mut wb = Workbook::new();

    // Index
    {
        let ws = x(wb.add_worksheet().set_name("Index"))?;
        x(ws.write_string_with_format(
            0,
            0,
            format!("Tax audit helper: {}", eng.entity_name),
            &f.title,
        ))?;
        x(ws.write_string(
            1,
            0,
            format!(
                "Applicable report: {form}. Period: {}.",
                eng.period_phrase()
            ),
        ))?;
        x(ws.write_string_with_format(2, 0, if new_act {
            "Form 26 clause numbers are not yet mapped in LedgerCraft: each sheet names the Form 3CD subject and the Income-tax Act, 2025 section. Verify every figure and complete the blank columns."
        } else {
            "Clause numbers follow published summaries of Form 3CD and are not yet verified against the official form text. Verify every figure and complete the blank columns."
        }, &f.note))?;
        ws.set_column_width(0, 120).ok();
        let rows = [
            "18  Depreciation as per the Income-tax Act (block-wise)",
            "21(d)  Payments otherwise than by account payee cheque / draft / electronic mode above the limit",
            "26  Statutory dues (allowed only on actual payment): balances to check",
            "31(a)/(b)/(c)  Loans and deposits taken and repaid: principal by mode",
            "31(ba)/(bc)  Receipts of Rs 2 lakh or more otherwise than by banking channel",
            "34  Payments on which tax is to be deducted: expense ledgers to check",
            "40  Turnover, gross profit and net profit ratios",
            "44  Break-up of expenditure (GST registration status to be filled)",
        ];
        for (i, r) in rows.iter().enumerate() {
            x(ws.write_string(4 + i as u32, 0, *r))?;
        }
    }

    // 18 Depreciation (Income-tax)
    {
        let ws = x(wb.add_worksheet().set_name("18 Depreciation"))?;
        let mut r = header(
            ws,
            &f,
            "Clause 18: Depreciation as per the Income-tax Act",
            sec("s.32, Income-tax Act, 1961", "s.33, Income-tax Act, 2025"),
            &[
                ("Block", 34.0),
                ("Rate %", 8.0),
                ("Opening WDV", 16.0),
                ("Additions 180 days or more", 16.0),
                ("Additions less than 180 days", 16.0),
                ("Deductions (sale value)", 16.0),
                ("Depreciation", 16.0),
                ("Closing WDV", 16.0),
                ("Short-term capital gain", 16.0),
            ],
        )?;
        match &a.far {
            Some(far) if !far.it.is_empty() => {
                for it in &far.it {
                    text(ws, &f, r, 0, &it.label)?;
                    x(ws.write_number_with_format(r, 1, it.rate, &f.num))?;
                    for (c, m) in [
                        it.opening_wdv,
                        it.additions_180_or_more,
                        it.additions_less_180,
                        it.sale_proceeds,
                        it.depreciation,
                        it.closing_wdv,
                        it.stcg,
                    ]
                    .iter()
                    .enumerate()
                    {
                        money(ws, &f, r, 2 + c as u16, *m)?;
                    }
                    r += 1;
                }
            }
            _ => {
                x(ws.write_string_with_format(
                    r,
                    0,
                    "Fixed asset register not imported: depreciation cannot be worked out.",
                    &f.note,
                ))?;
            }
        }
    }

    // 21(d) cash payments
    {
        let ws = x(wb.add_worksheet().set_name("21(d) Cash payments"))?;
        let mut r = header(ws, &f, "Clause 21(d): Payments above the limit otherwise than by account payee cheque / draft / electronic mode", sec("s.40A(3) and (3A), Rule 6DD", "s.36 (corresponds to s.40A(3))"), &[
            ("Date", 12.0), ("Voucher", 26.0), ("Paid to / ledger", 30.0), ("Amount", 16.0), ("Details", 60.0), ("Rule 6DD exception? (auditor)", 22.0), ("Disallowed amount (auditor)", 18.0),
        ])?;
        for fd in findings(a, &["CASH_PAYMENT_LIMIT", "CASH_ASSET_PURCHASE"]) {
            text(
                ws,
                &f,
                r,
                0,
                &fd.date
                    .map(|d| d.format("%d-%m-%Y").to_string())
                    .unwrap_or_default(),
            )?;
            text(ws, &f, r, 1, fd.voucher.as_deref().unwrap_or(""))?;
            text(ws, &f, r, 2, fd.ledger.as_deref().unwrap_or(""))?;
            money(ws, &f, r, 3, fd.amount.unwrap_or_default().abs())?;
            text(ws, &f, r, 4, &fd.message)?;
            text(ws, &f, r, 5, "")?;
            text(ws, &f, r, 6, "")?;
            r += 1;
        }
        if r == 4 {
            x(ws.write_string_with_format(r, 0, "No cash payment above the limit was found in the day book (a day book is needed for this test).", &f.note))?;
        }
    }

    // 26 statutory dues
    {
        let ws = x(wb.add_worksheet().set_name("26 Statutory dues"))?;
        let mut r = header(
            ws,
            &f,
            "Clause 26: Sums allowed only on actual payment: closing balances to check",
            sec(
                "s.43B",
                "corresponding provision of the Income-tax Act, 2025",
            ),
            &[
                ("Ledger", 34.0),
                ("Group", 22.0),
                ("Opening", 16.0),
                ("Closing", 16.0),
                ("Nature", 30.0),
                ("Paid on or before due date of return? (auditor)", 22.0),
                ("Date of payment (auditor)", 16.0),
            ],
        )?;
        let words: &[(&str, &str)] = &[
            ("gst", "Goods and services tax"),
            ("cgst", "Goods and services tax"),
            ("sgst", "Goods and services tax"),
            ("igst", "Goods and services tax"),
            ("tds", "Tax deducted at source"),
            ("provident", "Provident fund"),
            ("pf", "Provident fund"),
            ("esi", "Employees' state insurance"),
            ("bonus", "Bonus"),
            ("leave", "Leave encashment"),
            ("gratuity", "Gratuity"),
            ("professional tax", "Professional tax"),
            ("excise", "Duty / tax"),
            ("customs", "Duty / tax"),
            ("vat", "Duty / tax"),
            ("cess", "Duty / tax"),
            ("msme", "Payment to micro or small enterprise"),
        ];
        for m in &a.mapping {
            let n = format!(" {} ", norm_name(&m.name));
            let hit = words.iter().find(|(w, _)| n.contains(&format!(" {w} ")));
            let liability = matches!(
                m.class,
                Some(Class::DutiesTaxes)
                    | Some(Class::Provisions)
                    | Some(Class::CurrentLiabilities)
            );
            if let (true, Some((_, nature))) = (liability, hit) {
                let l = eng
                    .cy
                    .ledgers
                    .iter()
                    .find(|l| norm_name(&l.name) == norm_name(&m.name));
                text(ws, &f, r, 0, &m.name)?;
                text(ws, &f, r, 1, &m.group)?;
                money(ws, &f, r, 2, -l.map(|l| l.opening).unwrap_or_default())?;
                money(ws, &f, r, 3, -m.tb_closing)?;
                text(ws, &f, r, 4, nature)?;
                text(ws, &f, r, 5, "")?;
                text(ws, &f, r, 6, "")?;
                r += 1;
            }
        }
        x(ws.write_string_with_format(r + 1, 0, "Listed by ledger name and group; amounts owed shown positive. Add any statutory due that is booked under another name.", &f.note))?;
    }

    // 31 loans
    {
        let ws = x(wb.add_worksheet().set_name("31 Loans"))?;
        let mut r = header(
            ws,
            &f,
            "Clause 31(a), (b), (c): Loans and deposits taken and repaid (principal only)",
            sec("ss.269SS and 269T", "ss.185 and 188"),
            &[
                ("Lender", 30.0),
                ("Address (auditor)", 22.0),
                ("PAN (auditor)", 14.0),
                ("Bank / Government (exempt)", 12.0),
                ("Accepted: bank", 15.0),
                ("Accepted: cash", 15.0),
                ("Accepted: journal", 15.0),
                ("Interest credited", 15.0),
                ("Repaid: bank", 15.0),
                ("Repaid: cash", 15.0),
                ("Repaid: journal", 15.0),
                ("Maximum outstanding", 16.0),
                ("Squared up?", 10.0),
                ("Closing", 15.0),
            ],
        )?;
        for l in &a.loans {
            text(ws, &f, r, 0, &l.ledger)?;
            text(ws, &f, r, 1, "")?;
            text(ws, &f, r, 2, "")?;
            text(ws, &f, r, 3, if l.exempt { "Yes" } else { "" })?;
            for (c, m) in [
                l.accepted_bank,
                l.accepted_cash,
                l.accepted_journal,
                l.interest_credited,
                l.repaid_bank,
                l.repaid_cash,
                l.repaid_journal,
                l.max_outstanding,
            ]
            .iter()
            .enumerate()
            {
                money(ws, &f, r, 4 + c as u16, *m)?;
            }
            text(ws, &f, r, 12, if l.squared_up { "Yes" } else { "No" })?;
            money(ws, &f, r, 13, l.closing)?;
            r += 1;
        }
        x(ws.write_string_with_format(r + 1, 0, "Only the principal accepted or repaid is counted; interest and TDS on interest are kept separate. Journal entries are shown for the auditor to judge the mode.", &f.note))?;
    }

    // 31(ba)/(bc) receipts
    {
        let ws = x(wb.add_worksheet().set_name("31(ba) Cash receipts"))?;
        let mut r = header(
            ws,
            &f,
            "Clause 31(ba)/(bc): Receipts of Rs 2 lakh or more otherwise than by banking channel",
            sec("s.269ST", "s.186"),
            &[
                ("Date", 12.0),
                ("Voucher", 26.0),
                ("Received from / ledger", 30.0),
                ("Amount", 16.0),
                ("Details", 60.0),
                ("Name, address, PAN of payer (auditor)", 30.0),
            ],
        )?;
        for fd in findings(a, &["CASH_RECEIPT_LIMIT"]) {
            text(
                ws,
                &f,
                r,
                0,
                &fd.date
                    .map(|d| d.format("%d-%m-%Y").to_string())
                    .unwrap_or_default(),
            )?;
            text(ws, &f, r, 1, fd.voucher.as_deref().unwrap_or(""))?;
            text(ws, &f, r, 2, fd.ledger.as_deref().unwrap_or(""))?;
            money(ws, &f, r, 3, fd.amount.unwrap_or_default().abs())?;
            text(ws, &f, r, 4, &fd.message)?;
            text(ws, &f, r, 5, "")?;
            r += 1;
        }
        if r == 4 {
            x(ws.write_string_with_format(r, 0, "None found in the day book.", &f.note))?;
        }
    }

    // 34 TDS
    {
        let ws = x(wb.add_worksheet().set_name("34 TDS check"))?;
        let mut r = header(ws, &f, "Clause 34: Expenses on which tax may have to be deducted: list to check against TDS returns", "Sections are indicative; confirm the section, deduction and deposit from the TDS records.", &[
            ("Ledger", 34.0), ("Shown under", 26.0), ("Amount for the year", 16.0), ("Likely section (indicative)", 26.0), ("TDS deducted (auditor)", 16.0), ("TDS deposited in time? (auditor)", 18.0),
        ])?;
        let tds: &[(&str, &str)] = &[
            ("contract", "Payments to contractors"),
            ("contractor", "Payments to contractors"),
            ("labour", "Payments to contractors"),
            ("job work", "Payments to contractors"),
            (
                "professional",
                "Fees for professional or technical services",
            ),
            ("consultancy", "Fees for professional or technical services"),
            ("audit fee", "Fees for professional or technical services"),
            ("legal", "Fees for professional or technical services"),
            ("rent", "Rent"),
            ("commission", "Commission or brokerage"),
            ("brokerage", "Commission or brokerage"),
            ("interest", "Interest other than on securities"),
            ("salary", "Salaries"),
            ("salaries", "Salaries"),
            ("wages", "Salaries"),
            ("freight", "Payments to contractors (transport)"),
            ("transport", "Payments to contractors (transport)"),
        ];
        for m in a.mapping.iter().filter(|m| matches!(m.head, Some(h) if matches!(h, Head::OtherExpenses | Head::EmployeeBenefits | Head::FinanceCosts | Head::Purchases | Head::PartnersRemuneration))) {
            let n = format!(" {} ", norm_name(&m.name));
            if let Some((_, what)) = tds.iter().find(|(w, _)| n.contains(&format!(" {w}"))) {
                text(ws, &f, r, 0, &m.name)?;
                text(ws, &f, r, 1, m.head.map(|h| h.label()).unwrap_or(""))?;
                money(ws, &f, r, 2, m.amount)?;
                text(ws, &f, r, 3, what)?;
                text(ws, &f, r, 4, "")?;
                text(ws, &f, r, 5, "")?;
                r += 1;
            }
        }
    }

    // 40 ratios
    {
        let ws = x(wb.add_worksheet().set_name("40 Ratios"))?;
        header(
            ws,
            &f,
            "Clause 40: Turnover, gross profit and net profit",
            "Gross profit = turnover less purchases, change in inventories and direct expenses.",
            &[
                ("Particulars", 40.0),
                ("This year", 18.0),
                ("Last year", 18.0),
            ],
        )?;
        let figures = |y: &lc_core::facts::YearFacts| {
            let turnover = y.head(Head::RevenueOps);
            let direct: Money = y
                .lines
                .get(&Head::OtherExpenses)
                .map(|ls| {
                    ls.iter()
                        .filter(|l| l.class == Some(Class::DirectExpenses))
                        .map(|l| l.amount)
                        .sum()
                })
                .unwrap_or_default();
            let gp =
                turnover - y.head(Head::Purchases) - y.head(Head::ChangeInInventories) - direct;
            let np = y.profit();
            let inv = y.head(Head::Inventories);
            (turnover, gp, np, inv)
        };
        let cy = figures(&a.facts_cy);
        let py = a.facts_py.as_ref().map(figures);
        let pct = |n: Money, d: Money| {
            if d.is_zero() {
                0.0
            } else {
                n.as_f64() / d.as_f64() * 100.0
            }
        };
        let rows: [RatioRow; 7] = [
            (
                "Turnover (revenue from operations)",
                Box::new(|t| t.0.as_f64()),
            ),
            ("Gross profit", Box::new(|t| t.1.as_f64())),
            (
                "Gross profit / turnover (%)",
                Box::new(move |t| pct(t.1, t.0)),
            ),
            ("Net profit", Box::new(|t| t.2.as_f64())),
            (
                "Net profit / turnover (%)",
                Box::new(move |t| pct(t.2, t.0)),
            ),
            ("Closing stock-in-trade", Box::new(|t| t.3.as_f64())),
            (
                "Stock-in-trade / turnover (%)",
                Box::new(move |t| pct(t.3, t.0)),
            ),
        ];
        for (i, (label, fun)) in rows.iter().enumerate() {
            let r = 4 + i as u32;
            text(ws, &f, r, 0, label)?;
            x(ws.write_number_with_format(r, 1, fun(&cy), &f.num))?;
            match &py {
                Some(p) => x(ws.write_number_with_format(r, 2, fun(p), &f.num)).map(|_| ())?,
                None => text(ws, &f, r, 2, "")?,
            }
        }
    }

    // 44 GST break-up of expenditure
    {
        let ws = x(wb.add_worksheet().set_name("44 GST break-up"))?;
        let mut r = header(ws, &f, "Clause 44: Break-up of total expenditure by GST registration of the supplier", "LedgerCraft lists the expenditure; split each amount by the suppliers' GST status from the purchase records.", &[
            ("Ledger", 34.0), ("Shown under", 26.0), ("Total expenditure", 16.0), ("Registered: relating to exempt goods/services", 16.0),
            ("Registered: composition scheme", 16.0), ("Registered: others", 16.0), ("Total to registered entities", 16.0), ("Unregistered entities", 16.0),
        ])?;
        let mut total = Money::ZERO;
        for m in a.mapping.iter().filter(|m| matches!(m.head, Some(h) if h.nature() == lc_core::groups::Nature::Expense && !matches!(h, Head::Depreciation | Head::TaxExpense | Head::ChangeInInventories | Head::PartnersRemuneration))) {
            if m.amount.is_zero() {
                continue;
            }
            text(ws, &f, r, 0, &m.name)?;
            text(ws, &f, r, 1, m.head.map(|h| h.label()).unwrap_or(""))?;
            money(ws, &f, r, 2, m.amount)?;
            for c in 3..8 {
                text(ws, &f, r, c, "")?;
            }
            total += m.amount;
            r += 1;
        }
        x(ws.write_string_with_format(r, 0, "Total", &f.head))?;
        money(ws, &f, r, 2, total)?;
    }

    x(wb.save(path))
}
