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
    read_trial_balance_with(path, None)
}

const CLOSING_SIGNED: &[&str] = &[
    "closing",
    "closing balance",
    "balance",
    "closing balance (dr/cr)",
];
const CLOSING_DR: &[&str] = &[
    "closing dr",
    "closing debit",
    "debit",
    "dr",
    "dr. amount",
    "dr amount",
    "net debit",
    "debit amount",
];
const CLOSING_CR: &[&str] = &[
    "closing cr",
    "closing credit",
    "credit",
    "cr",
    "cr. amount",
    "cr amount",
    "net credit",
    "credit amount",
];
const TYPE_COL: &[&str] = &["account type", "account group", "type", "category"];

/// Zoho Books account types (API values or display names) → equivalent standard group.
pub fn zoho_type_group(t: &str) -> Option<&'static str> {
    let k = t.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    Some(match k.as_str() {
        "cash" => "Cash-in-Hand",
        "bank" => "Bank Accounts",
        "accounts_receivable" => "Sundry Debtors",
        "accounts_payable" => "Sundry Creditors",
        "fixed_asset" => "Fixed Assets",
        "stock" | "inventory" => "Stock-in-Hand",
        "other_current_asset"
        | "other_asset"
        | "payment_clearing"
        | "prepaid_card"
        | "input_tax" => "Current Assets",
        "other_current_liability" | "credit_card" | "other_liability" => "Current Liabilities",
        "output_tax" | "overseas_tax_payable" | "tax_payable" => "Duties & Taxes",
        "long_term_liability" => "Loans (Liability)",
        "equity" => "Capital Account",
        "income" | "operating_income" => "Sales Accounts",
        "other_income" => "Indirect Incomes",
        "cost_of_goods_sold" => "Purchase Accounts",
        "expense" | "other_expense" | "operating_expense" => "Indirect Expenses",
        _ => return None,
    })
}

/// Trial balance from the LedgerCraft template, Tally/BUSY/Zoho Excel exports.
/// The group of each ledger comes from (in order): a Group column, an account
/// type column (Zoho), the separate account master (`master`, e.g. BUSY "List
/// of Accounts" with Account Name + Group), or the section heading row above it.
pub fn read_trial_balance_with(path: &Path, master: Option<&Path>) -> Result<TrialBalance, String> {
    let t = read_table(path, &["Trial Balance", "TB"], LEDGER)?;
    let lc = t.col(LEDGER).ok_or("no Ledger / Account column")?;
    let gc = t.col(GROUP);
    let type_c = t.col(TYPE_COL);
    let has_closing = t.col(CLOSING_SIGNED).is_some()
        || t.col(CLOSING_DR).is_some()
        || t.col(CLOSING_CR).is_some();
    if !has_closing {
        return Err(format!(
            "{}: no Closing / Debit / Credit column",
            path.display()
        ));
    }
    if t.col(&["date", "voucher date"]).is_some()
        && t.col(&[
            "voucher no",
            "voucher no.",
            "vch no",
            "vch no.",
            "voucher number",
            "voucher type",
            "vch type",
        ])
        .is_some()
    {
        return Err(
            "this looks like a day book (it has Date and Voucher columns), not a trial balance"
                .into(),
        );
    }
    let mut master_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if let Some(m) = master {
        let mt = read_table(m, &["List of Accounts", "Accounts", "Masters"], LEDGER)?;
        let ml = mt
            .col(LEDGER)
            .ok_or("account master: no Account Name column")?;
        let mg = mt.col(GROUP).ok_or("account master: no Group column")?;
        for row in &mt.rows {
            master_map.insert(
                lc_core::model::norm_name(mt.get(row, Some(ml))),
                mt.get(row, Some(mg)).to_string(),
            );
        }
    }
    let stock_c = t.col(&["closing stock", "closing stock value"]);
    let tags_c = t.col(&["tags", "tag"]);
    let mut ledgers = Vec::new();
    let mut section = String::new();
    for (row, &line) in t.rows.iter().zip(&t.source_rows) {
        let name = t.get(row, Some(lc)).to_string();
        let lower = name.to_ascii_lowercase();
        if name.is_empty() {
            if row.iter().any(|c| !c.is_empty()) {
                crate::diag::skip(line, "no ledger name", row);
            }
            continue;
        }
        if lower == "total"
            || lower == "grand total"
            || lower.starts_with("total for")
            || lower.starts_with("total ")
        {
            crate::diag::skip(line, "total row (worked out again by LedgerCraft)", row);
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
            CLOSING_SIGNED,
            CLOSING_DR,
            CLOSING_CR,
            "closing",
            line,
        )?;
        let row_has_amount = row.iter().enumerate().any(|(i, c)| {
            i != lc
                && !c.is_empty()
                && Money::parse(c).is_ok()
                && Some(i) != gc
                && Some(i) != type_c
        });
        let explicit_group = gc
            .map(|c| t.get(row, Some(c)).to_string())
            .filter(|g| !g.is_empty());
        let typed_group = type_c
            .map(|c| t.get(row, Some(c)))
            .and_then(zoho_type_group)
            .map(String::from);
        let master_group = master_map.get(&lc_core::model::norm_name(&name)).cloned();
        if explicit_group.is_none()
            && typed_group.is_none()
            && master_group.is_none()
            && !row_has_amount
        {
            // A section heading such as "Accounts Receivable" in an exported report.
            section = zoho_type_group(&name)
                .map(String::from)
                .unwrap_or_else(|| name.clone());
            crate::diag::skip(
                line,
                "section heading (used as the group of the ledgers below it)",
                row,
            );
            continue;
        }
        let group = explicit_group
            .or(typed_group)
            .or(master_group)
            .unwrap_or_else(|| section.clone());
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
        crate::diag::accept();
        ledgers.push(Ledger {
            name,
            group,
            opening,
            closing,
            closing_stock,
            tags,
        });
    }
    if !ledgers.is_empty() {
        let mut seen = std::collections::HashSet::new();
        let dups = ledgers
            .iter()
            .filter(|l| !seen.insert(lc_core::model::norm_name(&l.name)))
            .count();
        if dups * 20 > ledgers.len() {
            return Err(format!("{dups} ledger names repeat; a trial balance lists each ledger once (is this a day book or ledger report?)"));
        }
        if ledgers.iter().all(|l| l.group.trim().is_empty()) {
            return Err("no group for any ledger: add a Group column, or supply the account master (e.g. BUSY List of Accounts)".into());
        }
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
        crate::diag::accept();
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

/// Fixed asset register: sheet "Fixed Assets" (one row per asset) and optional
/// sheet "IT Opening" (block, opening written down value).
pub fn read_far(
    path: &Path,
    basis: lc_core::far::BookBasis,
) -> Result<lc_core::far::Register, String> {
    use lc_core::far::{Asset, Register};
    let t = read_table(
        path,
        &["Fixed Assets", "FAR", "Assets"],
        &["asset", "asset name", "description"],
    )?;
    let name_c = t
        .col(&["asset", "asset name", "description"])
        .ok_or("Fixed Assets: no Asset column")?;
    let ledger_c = t
        .col(&["ledger", "ledger name", "account"])
        .ok_or("Fixed Assets: no Ledger column")?;
    let class_c = t.col(&["schedule ii class", "book class", "class"]);
    let block_c = t
        .col(&["it block", "block", "income tax block"])
        .ok_or("Fixed Assets: no IT Block column")?;
    let date_c = t
        .col(&["put to use", "date put to use", "date of use", "date"])
        .ok_or("Fixed Assets: no 'Put to use' date column")?;
    let cost_c = t
        .col(&["cost", "original cost", "gross cost"])
        .ok_or("Fixed Assets: no Cost column")?;
    let acc_c = t.col(&[
        "opening accumulated depreciation",
        "accumulated depreciation",
        "acc dep",
    ]);
    let sold_c = t.col(&["sold on", "date of sale", "sale date"]);
    let sale_c = t.col(&["sale value", "sale proceeds"]);
    let life_c = t.col(&["useful life", "useful life years", "life"]);
    let mut assets = Vec::new();
    for (row, &line) in t.rows.iter().zip(&t.source_rows) {
        let name = t.get(row, Some(name_c)).to_string();
        if name.is_empty() {
            continue;
        }
        let money = |c: Option<usize>, what: &str| -> Result<Money, String> {
            let s = t.get(row, c);
            if s.is_empty() {
                Ok(Money::ZERO)
            } else {
                Money::parse(s).map_err(|e| format!("Fixed Assets row {line}: {what}: {e}"))
            }
        };
        let put = parse_date(t.get(row, Some(date_c)))
            .ok_or_else(|| format!("Fixed Assets row {line}: cannot read 'put to use' date"))?;
        let sold =
            match t.get(row, sold_c) {
                "" => None,
                s => Some(parse_date(s).ok_or_else(|| {
                    format!("Fixed Assets row {line}: cannot read sale date '{s}'")
                })?),
            };
        let life = match t.get(row, life_c) {
            "" => None,
            s => Some(s.parse::<f64>().map_err(|_| {
                format!("Fixed Assets row {line}: useful life '{s}' is not a number")
            })?),
        };
        assets.push(Asset {
            name,
            ledger: t.get(row, Some(ledger_c)).to_string(),
            book_class: t.get(row, class_c).to_string(),
            it_block: t.get(row, Some(block_c)).to_string(),
            put_to_use: put,
            cost: money(Some(cost_c), "cost")?,
            opening_acc_dep: money(acc_c, "accumulated depreciation")?,
            sold_on: sold,
            sale_value: money(sale_c, "sale value")?,
            useful_life_years: life,
        });
    }
    let mut it_opening = std::collections::BTreeMap::new();
    if let Some(o) = read_optional_sheet(path, "IT Opening", &["block", "it block"])? {
        let b = o
            .col(&["block", "it block"])
            .ok_or("IT Opening: no Block column")?;
        let w = o
            .col(&["opening wdv", "opening written down value", "wdv"])
            .ok_or("IT Opening: no Opening WDV column")?;
        for row in &o.rows {
            let k = o.get(row, Some(b));
            if !k.is_empty() {
                *it_opening.entry(k.to_string()).or_insert(Money::ZERO) +=
                    Money::parse(o.get(row, Some(w))).map_err(|e| format!("IT Opening: {e}"))?;
            }
        }
    }
    Ok(Register {
        assets,
        it_opening,
        basis,
    })
}

/// A bank statement as banks export it (Excel or CSV): date, narration,
/// cheque / reference, withdrawal and deposit (or amount with Dr/Cr), balance.
/// Lines above the column headings (account details) are skipped.
pub fn read_bank_statement(path: &Path) -> Result<Vec<lc_core::bankrec::BankLine>, String> {
    let t = read_table(
        path,
        &["Statement", "Bank Statement", "Transactions"],
        &[
            "narration",
            "description",
            "particulars",
            "transaction details",
            "remarks",
            "details",
        ],
    )?;
    let dc = t
        .col(&[
            "date",
            "txn date",
            "transaction date",
            "tran date",
            "value date",
            "posting date",
            "value dt",
            "txn. date",
        ])
        .ok_or("no Date column (looked for Date, Txn Date, Transaction Date, Value Date)")?;
    let nc = t.col(&[
        "narration",
        "description",
        "particulars",
        "transaction details",
        "remarks",
        "details",
    ]);
    let rc = t.col(&[
        "chq/ref no",
        "chq./ref.no.",
        "chq / ref no",
        "chq no",
        "cheque no",
        "cheque no.",
        "cheque number",
        "ref no",
        "ref no.",
        "reference",
        "reference no",
        "chq/ref number",
        "instrument no",
    ]);
    let wc = t.col(&[
        "withdrawal",
        "withdrawals",
        "withdrawal amt",
        "withdrawal amt.",
        "withdrawal amount",
        "debit",
        "debits",
        "debit amount",
        "dr",
        "withdrawal (dr)",
        "debit (rs)",
    ]);
    let pc = t.col(&[
        "deposit",
        "deposits",
        "deposit amt",
        "deposit amt.",
        "deposit amount",
        "credit",
        "credits",
        "credit amount",
        "cr",
        "deposit (cr)",
        "credit (rs)",
    ]);
    let ac = t.col(&["amount", "transaction amount", "amount (rs)"]);
    let tc = t.col(&["dr/cr", "dr / cr", "cr/dr", "type", "txn type"]);
    let bc = t.col(&[
        "balance",
        "closing balance",
        "running balance",
        "balance (rs)",
        "available balance",
    ]);
    if wc.is_none() && pc.is_none() && ac.is_none() {
        return Err("no Withdrawal / Deposit (or Amount) columns".into());
    }
    let num = |s: &str, line: usize| -> Result<Money, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(Money::ZERO);
        }
        let low = s.to_ascii_lowercase();
        let digits: String = s
            .chars()
            .filter(|c| {
                c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '(' || *c == ')' || *c == ','
            })
            .collect();
        let m = Money::parse(&digits).map_err(|e| format!("row {line}: {e}"))?;
        Ok(if low.ends_with("dr") {
            -m.abs()
        } else if low.ends_with("cr") {
            m.abs()
        } else {
            m
        })
    };
    let mut out = Vec::new();
    for (row, &line) in t.rows.iter().zip(&t.source_rows) {
        let ds = t.get(row, Some(dc));
        let Some(date) = parse_date(ds) else {
            // Opening / closing / total lines without a date are not transactions.
            continue;
        };
        let amount = if let Some(a) = ac {
            let m = num(t.get(row, Some(a)), line)?.abs();
            let kind = t.get(row, tc).to_ascii_lowercase();
            if kind.starts_with("dr") || kind.starts_with("d") && !kind.starts_with("dep") {
                -m
            } else {
                m
            }
        } else {
            num(t.get(row, pc), line)?.abs() - num(t.get(row, wc), line)?.abs()
        };
        if amount.is_zero() {
            continue;
        }
        let balance = match bc {
            Some(_) => {
                let b = t.get(row, bc);
                if b.trim().is_empty() {
                    None
                } else {
                    Some(num(b, line)?)
                }
            }
            None => None,
        };
        out.push(lc_core::bankrec::BankLine {
            date,
            narration: t.get(row, nc).to_string(),
            reference: t.get(row, rc).to_string(),
            amount,
            balance,
        });
    }
    if out.is_empty() {
        return Err("no transactions found in the statement".into());
    }
    Ok(out)
}
