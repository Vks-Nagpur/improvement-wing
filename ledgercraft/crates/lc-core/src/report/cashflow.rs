//! Cash Flow Statement – indirect method (AS 3), derived from two rounded
//! balance sheets and the Statement of Profit and Loss. Every balance-sheet
//! head is classified exactly once, so the statement always reconciles to the
//! movement in cash and bank balances; `build` proves it before returning.

use crate::facts::YearFacts;
use crate::mapping::Head;
use crate::money::Money;

#[derive(Debug, Clone, PartialEq)]
pub enum CfLine {
    Heading(String),
    Item(String, Money),
    Subtotal(String, Money),
    Total(String, Money),
}

fn interest_income(f: &YearFacts) -> Money {
    f.lines
        .get(&Head::OtherIncome)
        .map(|v| {
            v.iter()
                .filter(|l| l.name.to_ascii_lowercase().contains("interest"))
                .map(|l| l.amount)
                .sum()
        })
        .unwrap_or_default()
}

/// Returns the lines, or the unexplained difference if it fails to reconcile.
pub fn build(cy: &YearFacts, py: &YearFacts, is_company: bool) -> Result<Vec<CfLine>, Money> {
    use CfLine::*;
    let d = |h: Head| cy.head(h) - py.head(h);
    let mut out = Vec::new();

    let pbt = cy.pbt();
    let dep = cy.head(Head::Depreciation);
    let fin = cy.head(Head::FinanceCosts);
    let int_inc = interest_income(cy);
    let rem = if is_company {
        Money::ZERO
    } else {
        cy.head(Head::PartnersRemuneration)
    };

    out.push(Heading("A. Cash flow from operating activities".into()));
    out.push(Item("Net profit before tax".into(), pbt));
    out.push(Heading("Adjustments for:".into()));
    let mut ops = pbt;
    for (label, v) in [
        ("Depreciation and amortisation expense", dep),
        ("Finance costs", fin),
        ("Interest income", -int_inc),
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
            -d(Head::StLoansAdvances),
        ),
        (
            "(Increase) / decrease in other current assets",
            -d(Head::OtherCurrentAssets),
        ),
        (
            "(Increase) / decrease in other non-current assets",
            -d(Head::OtherNcAssets),
        ),
        (
            "Increase / (decrease) in trade payables",
            d(Head::TradePayables),
        ),
        (
            "Increase / (decrease) in other current liabilities",
            d(Head::OtherCurrentLiabilities),
        ),
        (
            "Increase / (decrease) in short-term provisions",
            d(Head::StProvisions),
        ),
        (
            "Increase / (decrease) in long-term provisions",
            d(Head::LtProvisions),
        ),
        (
            "Increase / (decrease) in other long-term liabilities",
            d(Head::OtherLtLiabilities),
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
    let tax = -cy.head(Head::TaxExpense);
    if !tax.is_zero() {
        out.push(Item("Income taxes paid".into(), tax));
    }
    let a = gen + tax;
    out.push(Subtotal(
        "Net cash from / (used in) operating activities (A)".into(),
        a,
    ));

    out.push(Heading("B. Cash flow from investing activities".into()));
    let mut b = Money::ZERO;
    for (label, v) in [
        (
            "Purchase of property, plant and equipment and intangible assets (net)",
            -(d(Head::Ppe) + d(Head::Intangibles) + d(Head::Cwip) + dep),
        ),
        (
            "(Purchase) / sale of investments (net)",
            -(d(Head::NcInvestments) + d(Head::CurrentInvestments)),
        ),
        (
            "Long-term loans, advances and deposits (net)",
            -d(Head::LtLoansAdvances),
        ),
        ("Interest received", int_inc),
    ] {
        if !v.is_zero() {
            out.push(Item(label.into(), v));
            b += v;
        }
    }
    out.push(Subtotal(
        "Net cash from / (used in) investing activities (B)".into(),
        b,
    ));

    out.push(Heading("C. Cash flow from financing activities".into()));
    let profit = cy.profit();
    let (owners_label, owners) = if is_company {
        (
            "Proceeds from issue of share capital",
            d(Head::Capital)
                - if cy.profit_to == Head::Capital {
                    profit
                } else {
                    Money::ZERO
                },
        )
    } else {
        (
            "Capital introduced / (withdrawn) by owners (net)",
            d(Head::Capital)
                - if cy.profit_to == Head::Capital {
                    profit
                } else {
                    Money::ZERO
                }
                - rem,
        )
    };
    let reserves = d(Head::ReservesSurplus)
        - if cy.profit_to == Head::ReservesSurplus {
            profit
        } else {
            Money::ZERO
        };
    let mut c = Money::ZERO;
    for (label, v) in [
        (
            "Proceeds from / (repayment of) long-term borrowings (net)",
            d(Head::LtBorrowings),
        ),
        (
            "Proceeds from / (repayment of) short-term borrowings (net)",
            d(Head::StBorrowings),
        ),
        ("Finance costs paid", -fin),
        (owners_label, owners),
        ("Other movements in reserves (net)", reserves),
    ] {
        if !v.is_zero() {
            out.push(Item(label.into(), v));
            c += v;
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
    Ok(out)
}
