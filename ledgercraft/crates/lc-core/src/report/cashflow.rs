//! Cash Flow Statement – indirect method (AS 3).
//!
//! Two ways of finding investing and financing cash flows:
//! * **Transactions** (preferred): when the day book explains the whole
//!   movement in cash and bank, each cash/bank line is attributed to the
//!   other ledgers of its voucher. Purchases and sales of assets, borrowings
//!   taken and repaid, capital introduced and withdrawn, finance costs paid
//!   and interest received are then the actual cash amounts.
//! * **Balances** (fallback): from the movement in balance-sheet heads. Such
//!   lines are marked "derived" and the statement says so.
//!
//! Taxes paid are always derived from the tax expense and the movement in tax
//! balances (advance tax, TDS, provision for tax), never taken as the expense.
//! Operating cash flow is shown by the indirect method; any difference between
//! it and the cash actually left over after investing and financing is shown
//! as a separate line for review (non-cash transactions, rounding), never hidden.

use crate::facts::YearFacts;
use crate::mapping::Head;
use crate::model::{norm_name, Engagement};
use crate::money::Money;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq)]
pub enum CfLine {
    Heading(String),
    Item(String, Money),
    Subtotal(String, Money),
    Total(String, Money),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CashFlow {
    pub lines: Vec<CfLine>,
    /// "transactions" or "balances".
    pub method: String,
    /// Points the preparer must review (inferred classifications, differences).
    pub review: Vec<String>,
}

/// Whether a cash flow statement is required (TRUTH-MODEL.md §10).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Applicability {
    /// REQUIRED, NOT_REQUIRED, VOLUNTARY, REQUIRES_REVIEW or NOT_SUPPORTED.
    pub status: String,
    pub reason: String,
    pub source: String,
}

/// Applicability is decided by law that LedgerCraft has not verified, so the
/// answer is "requires review" with the reason; it never says "required".
pub fn applicability(eng: &Engagement) -> Applicability {
    if eng.entity_type.is_company() {
        Applicability {
            status: "REQUIRES_REVIEW".into(),
            reason: "Companies: a cash flow statement forms part of the financial statements, with exemptions for some companies (for example one person, small and dormant companies). Whether this company is exempt is for you to decide.".into(),
            source: "Companies Act, 2013, s.2(40) and Schedule III (not verified against the official text)".into(),
        }
    } else {
        Applicability {
            status: "REQUIRES_REVIEW".into(),
            reason: "Non-corporate entities and LLPs: whether AS 3 applies depends on the entity's level under the ICAI criteria; it may also be prepared voluntarily.".into(),
            source: "ICAI Accounting Standards applicability for non-company entities (not verified against the official text)".into(),
        }
    }
}

/// Income-tax balances (not GST): advance tax, TDS/TCS receivable, provision
/// for tax, MAT credit, deferred tax. Found by ledger name (inferred).
pub fn is_tax_ledger(head: Head, name: &str) -> bool {
    if head == Head::TaxExpense {
        return true;
    }
    if !matches!(
        head,
        Head::StLoansAdvances
            | Head::LtLoansAdvances
            | Head::OtherCurrentAssets
            | Head::OtherNcAssets
            | Head::StProvisions
            | Head::LtProvisions
            | Head::OtherCurrentLiabilities
            | Head::OtherLtLiabilities
    ) {
        return false;
    }
    let n = norm_name(name);
    if n.contains("gst")
        || n.contains("goods and services")
        || n.contains("payable") && n.contains("tds")
    {
        // GST, and TDS deducted by the entity and payable to the government, are not taxes on its income.
        return false;
    }
    [
        "income tax",
        "advance tax",
        "self assessment",
        "provision for tax",
        "tax provision",
        "tds receivable",
        "tds recoverable",
        "tcs receivable",
        "mat credit",
        "deferred tax",
        "tax refund",
        "tax deducted at source",
    ]
    .iter()
    .any(|k| n.contains(k))
}

fn is_interest_or_dividend(name: &str) -> bool {
    let n = norm_name(name);
    n.contains("interest") || n.contains("dividend")
}

/// Cash attributed to each ledger by the day book, split into inflows and outflows.
#[derive(Debug, Default)]
struct Flows {
    by_ledger: HashMap<String, (Money, Money)>,
    net: Money,
}

fn transaction_flows(eng: &Engagement, cash: &BTreeSet<String>) -> Flows {
    let mut f = Flows::default();
    for v in eng
        .vouchers
        .iter()
        .filter(|v| v.date >= eng.fy_start && v.date <= eng.fy_end)
    {
        let c: Money = v
            .lines
            .iter()
            .filter(|l| cash.contains(&norm_name(&l.ledger)))
            .map(|l| l.amount)
            .sum();
        if c.is_zero() {
            continue;
        }
        f.net += c;
        for l in v
            .lines
            .iter()
            .filter(|l| !cash.contains(&norm_name(&l.ledger)))
        {
            // A balanced voucher: the other lines carry the cash, opposite sign.
            let e = -l.amount;
            let slot = f.by_ledger.entry(norm_name(&l.ledger)).or_default();
            if e.0 > 0 {
                slot.0 += e;
            } else {
                slot.1 += e;
            }
        }
    }
    f
}

/// `cy`/`py` are rounded to the presentation unit `g`; `cy_exact`/`py_exact`
/// are exact (to judge whether the day book is complete).
pub fn build(
    eng: &Engagement,
    cy: &YearFacts,
    py: &YearFacts,
    cy_exact: &YearFacts,
    py_exact: &YearFacts,
    g: i64,
) -> Result<CashFlow, Money> {
    use CfLine::*;
    let is_company = eng.entity_type.is_company();
    let d = |h: Head| cy.head(h) - py.head(h);
    let amt = |f: &YearFacts, h: Head, name: &str| -> Money {
        f.lines
            .get(&h)
            .and_then(|v| v.iter().find(|l| l.name == name))
            .map(|l| l.amount)
            .unwrap_or_default()
    };
    let mut review: Vec<String> = Vec::new();

    // Ledger -> head, and the tax ledgers.
    let mut head_of: HashMap<String, Head> = HashMap::new();
    let mut tax: BTreeMap<Head, BTreeSet<String>> = BTreeMap::new();
    for f in [py, cy] {
        for (h, v) in &f.lines {
            for l in v {
                head_of.insert(norm_name(&l.name), *h);
                if is_tax_ledger(*h, &l.name) && *h != Head::TaxExpense {
                    tax.entry(*h).or_default().insert(l.name.clone());
                }
            }
        }
    }
    let d_tax = |h: Head| -> Money {
        tax.get(&h)
            .map(|s| s.iter().map(|n| amt(cy, h, n) - amt(py, h, n)).sum())
            .unwrap_or_default()
    };
    if tax.values().any(|s| !s.is_empty()) {
        review.push(format!(
            "Income-tax balances identified by name and kept out of working capital: {}. Check the list.",
            tax.values().flatten().cloned().collect::<Vec<_>>().join(", ")
        ));
    }

    // Can the day book explain the whole movement in cash and bank?
    let cash: BTreeSet<String> = cy
        .lines
        .get(&Head::CashBank)
        .into_iter()
        .chain(py.lines.get(&Head::CashBank))
        .flatten()
        .map(|l| norm_name(&l.name))
        .collect();
    let flows = transaction_flows(eng, &cash);
    let exact_move = cy_exact.head(Head::CashBank) - py_exact.head(Head::CashBank);
    let by_tx = !eng.vouchers.is_empty() && flows.net == exact_move;
    if !eng.vouchers.is_empty() && !by_tx {
        review.push(format!(
            "The day book does not explain the whole movement in cash and bank (day book {}, balances {}); investing and financing figures are derived from balance movements.",
            flows.net, exact_move
        ));
    }
    let r = |m: Money| -> Money { crate::units::round_to(m, g) };
    // Inflows and outflows of the ledgers under some heads (non-tax only).
    let sum_heads = |heads: &[Head], pick: &dyn Fn(&str) -> bool| -> (Money, Money) {
        let mut i = Money::ZERO;
        let mut o = Money::ZERO;
        for (n, (inf, outf)) in &flows.by_ledger {
            if let Some(h) = head_of.get(n) {
                if heads.contains(h) && !is_tax_ledger(*h, n) && pick(n) {
                    i += *inf;
                    o += *outf;
                }
            }
        }
        (r(i), r(o))
    };

    let pbt = cy.pbt();
    let dep = cy.head(Head::Depreciation);
    let fin = cy.head(Head::FinanceCosts);
    let int_inc: Money = cy
        .lines
        .get(&Head::OtherIncome)
        .map(|v| {
            v.iter()
                .filter(|l| is_interest_or_dividend(&l.name))
                .map(|l| l.amount)
                .sum()
        })
        .unwrap_or_default();
    if !int_inc.is_zero() {
        review.push("Interest and dividend income identified by ledger name and shown under investing activities. Check the classification.".into());
    }
    let rem = if is_company {
        Money::ZERO
    } else {
        cy.head(Head::PartnersRemuneration)
    };

    // ---- B and C ------------------------------------------------------------
    let mut b_items: Vec<(String, Money)> = Vec::new();
    let mut c_items: Vec<(String, Money)> = Vec::new();
    let profit = cy.profit();
    let owners_bal = d(Head::Capital)
        - if cy.profit_to == Head::Capital {
            profit
        } else {
            Money::ZERO
        }
        - rem;
    let reserves_bal = d(Head::ReservesSurplus)
        - if cy.profit_to == Head::ReservesSurplus {
            profit
        } else {
            Money::ZERO
        };
    if by_tx {
        let (ppe_in, ppe_out) = sum_heads(&[Head::Ppe, Head::Intangibles, Head::Cwip], &|_| true);
        let (inv_in, inv_out) =
            sum_heads(&[Head::NcInvestments, Head::CurrentInvestments], &|_| true);
        let (ln_in, ln_out) = sum_heads(&[Head::LtLoansAdvances], &|_| true);
        let (int_in, _) = sum_heads(&[Head::OtherIncome], &|n| is_interest_or_dividend(n));
        b_items.extend([
            (
                "Purchase of property, plant and equipment and intangible assets".to_string(),
                ppe_out,
            ),
            (
                "Proceeds from sale of property, plant and equipment".to_string(),
                ppe_in,
            ),
            ("Purchase of investments".to_string(), inv_out),
            ("Proceeds from sale of investments".to_string(), inv_in),
            (
                "Long-term loans, advances and deposits (net)".to_string(),
                ln_in + ln_out,
            ),
            ("Interest and dividends received".to_string(), int_in),
        ]);
        let (bor_in, bor_out) = sum_heads(&[Head::LtBorrowings, Head::StBorrowings], &|_| true);
        let (cap_in, cap_out) = sum_heads(&[Head::Capital], &|_| true);
        let (res_in, res_out) = sum_heads(&[Head::ReservesSurplus], &|_| true);
        let (_, fin_out) = sum_heads(&[Head::FinanceCosts], &|_| true);
        c_items.extend([
            ("Proceeds from borrowings".to_string(), bor_in),
            ("Repayment of borrowings".to_string(), bor_out),
        ]);
        if is_company {
            c_items.push((
                "Proceeds from issue of share capital".into(),
                cap_in + cap_out,
            ));
        } else {
            c_items.push(("Capital introduced by owners".into(), cap_in));
            c_items.push(("Capital withdrawn by owners (drawings)".into(), cap_out));
        }
        c_items.push(("Other movements in reserves".into(), res_in + res_out));
        c_items.push(("Finance costs paid".into(), fin_out));
    } else {
        b_items.extend([
            (
                "Purchase of property, plant and equipment and intangible assets, net (derived)"
                    .to_string(),
                -(d(Head::Ppe) + d(Head::Intangibles) + d(Head::Cwip) + dep),
            ),
            (
                "(Purchase) / sale of investments, net (derived)".to_string(),
                -(d(Head::NcInvestments) + d(Head::CurrentInvestments)),
            ),
            (
                "Long-term loans, advances and deposits, net (derived)".to_string(),
                -(d(Head::LtLoansAdvances) - d_tax(Head::LtLoansAdvances)),
            ),
            (
                "Interest and dividends received (derived)".to_string(),
                int_inc,
            ),
        ]);
        c_items.extend([
            (
                "Proceeds from / (repayment of) long-term borrowings, net (derived)".to_string(),
                d(Head::LtBorrowings),
            ),
            (
                "Proceeds from / (repayment of) short-term borrowings, net (derived)".to_string(),
                d(Head::StBorrowings),
            ),
            ("Finance costs paid (derived)".to_string(), -fin),
            (
                if is_company {
                    "Proceeds from issue of share capital (derived)"
                } else {
                    "Capital introduced / (withdrawn) by owners, net (derived)"
                }
                .to_string(),
                owners_bal,
            ),
            (
                "Other movements in reserves, net (derived)".to_string(),
                reserves_bal,
            ),
        ]);
    }
    let b: Money = b_items.iter().map(|x| x.1).sum();
    let c: Money = c_items.iter().map(|x| x.1).sum();

    // ---- A (indirect) ----------------------------------------------------------
    let mut out = Vec::new();
    out.push(Heading("A. Cash flow from operating activities".into()));
    out.push(Item("Net profit before tax".into(), pbt));
    out.push(Heading("Adjustments for:".into()));
    let mut ops = pbt;
    for (label, v) in [
        ("Depreciation and amortisation expense", dep),
        ("Finance costs", fin),
        ("Interest and dividend income", -int_inc),
        (
            "Partners' remuneration and interest credited to capital accounts",
            rem,
        ),
    ] {
        if !v.is_zero() {
            out.push(Item(label.into(), v));
            ops += v;
        }
    }
    out.push(Subtotal(
        "Operating profit before working capital changes".into(),
        ops,
    ));
    out.push(Heading("Changes in working capital:".into()));
    let wc: [(&str, Money); 10] = [
        (
            "(Increase) / decrease in inventories",
            -d(Head::Inventories),
        ),
        (
            "(Increase) / decrease in trade receivables",
            -d(Head::TradeReceivables),
        ),
        (
            "(Increase) / decrease in short-term loans and advances",
            -(d(Head::StLoansAdvances) - d_tax(Head::StLoansAdvances)),
        ),
        (
            "(Increase) / decrease in other current assets",
            -(d(Head::OtherCurrentAssets) - d_tax(Head::OtherCurrentAssets)),
        ),
        (
            "(Increase) / decrease in other non-current assets",
            -(d(Head::OtherNcAssets) - d_tax(Head::OtherNcAssets)),
        ),
        (
            "Increase / (decrease) in trade payables",
            d(Head::TradePayables),
        ),
        (
            "Increase / (decrease) in other current liabilities",
            d(Head::OtherCurrentLiabilities) - d_tax(Head::OtherCurrentLiabilities),
        ),
        (
            "Increase / (decrease) in short-term provisions",
            d(Head::StProvisions) - d_tax(Head::StProvisions),
        ),
        (
            "Increase / (decrease) in long-term provisions",
            d(Head::LtProvisions) - d_tax(Head::LtProvisions),
        ),
        (
            "Increase / (decrease) in other long-term liabilities",
            d(Head::OtherLtLiabilities) - d_tax(Head::OtherLtLiabilities),
        ),
    ];
    let mut gen = ops;
    for (label, v) in wc {
        if !v.is_zero() {
            out.push(Item(label.into(), v));
            gen += v;
        }
    }
    out.push(Subtotal("Cash generated from operations".into(), gen));
    // Taxes paid = expense − increase in tax liabilities + increase in tax assets.
    let tax_assets = d_tax(Head::StLoansAdvances)
        + d_tax(Head::LtLoansAdvances)
        + d_tax(Head::OtherCurrentAssets)
        + d_tax(Head::OtherNcAssets);
    let tax_liabs = d_tax(Head::StProvisions)
        + d_tax(Head::LtProvisions)
        + d_tax(Head::OtherCurrentLiabilities)
        + d_tax(Head::OtherLtLiabilities);
    let tax_paid = -(cy.head(Head::TaxExpense) - tax_liabs + tax_assets);
    if !tax_paid.is_zero() {
        out.push(Item("Income taxes paid (net of refunds)".into(), tax_paid));
    }
    let mut a = gen + tax_paid;
    // Cash actually left for operations once investing and financing are known.
    let net_actual = cy.head(Head::CashBank) - py.head(Head::CashBank);
    let a_actual = net_actual - b - c;
    let diff = a_actual - a;
    if !diff.is_zero() {
        let small = diff.abs().0 <= g.max(1) * 20;
        out.push(Item(
            if small {
                "Rounding differences".into()
            } else {
                "Non-cash transactions and other differences (review)".into()
            },
            diff,
        ));
        if !small {
            review.push(format!(
                "Operating cash flow by the indirect method differs by {diff} from the cash left after investing and financing. Usually assets bought on credit, loans settled by journal or other non-cash transactions; review and disclose them."
            ));
        }
        a += diff;
    }
    out.push(Subtotal(
        "Net cash from / (used in) operating activities (A)".into(),
        a,
    ));

    out.push(Heading("B. Cash flow from investing activities".into()));
    for (label, v) in &b_items {
        if !v.is_zero() {
            out.push(Item(label.clone(), *v));
        }
    }
    out.push(Subtotal(
        "Net cash from / (used in) investing activities (B)".into(),
        b,
    ));
    out.push(Heading("C. Cash flow from financing activities".into()));
    for (label, v) in &c_items {
        if !v.is_zero() {
            out.push(Item(label.clone(), *v));
        }
    }
    out.push(Subtotal(
        "Net cash from / (used in) financing activities (C)".into(),
        c,
    ));

    let net = a + b + c;
    let open = py.head(Head::CashBank);
    let close = cy.head(Head::CashBank);
    out.push(Subtotal(
        "Net increase / (decrease) in cash and cash equivalents (A + B + C)".into(),
        net,
    ));
    out.push(Item(
        "Cash and cash equivalents at the beginning of the year".into(),
        open,
    ));
    out.push(Total(
        "Cash and cash equivalents at the end of the year".into(),
        open + net,
    ));
    if open + net != close {
        return Err(close - (open + net));
    }
    Ok(CashFlow {
        lines: out,
        method: if by_tx {
            "transactions".into()
        } else {
            "balances".into()
        },
        review,
    })
}
