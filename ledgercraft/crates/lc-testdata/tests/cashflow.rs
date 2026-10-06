//! Cash flow golden test: a small company book where every figure is known.

use lc_core::model::EntityType;
use lc_core::report::cashflow::{self, CfLine};
use lc_core::rules::RulesPack;
use lc_core::{analyse, Money};
use lc_testdata::builder::Builder;

fn r(x: i64) -> Money {
    Money::rupees(x)
}

fn book() -> lc_core::Engagement {
    let mut b = Builder::new(
        "Golden Cash Flow Private Limited",
        EntityType::Company,
        2025,
    );
    b.ledger("HDFC Bank", "Bank Accounts", r(100_000));
    b.ledger("Equity Share Capital", "Capital Account", r(-100_000));
    b.ledger("Profit & Loss A/c", "Profit & Loss A/c", Money::ZERO);
    for (n, g) in [
        ("Sales", "Sales Accounts"),
        ("Purchases", "Purchase Accounts"),
        ("Salary", "Indirect Expenses"),
        ("Plant & Machinery", "Fixed Assets"),
        ("Furniture", "Fixed Assets"),
        ("Furniture Supplier", "Sundry Creditors"),
        ("HDFC Term Loan", "Secured Loans"),
        ("Interest on Term Loan", "Indirect Expenses"),
        ("Advance Tax", "Loans & Advances (Asset)"),
        ("Provision for Income Tax", "Provisions"),
        ("Income Tax Expense", "Indirect Expenses"),
        ("Depreciation", "Indirect Expenses"),
        ("Interest Received", "Indirect Incomes"),
        ("Customer A", "Sundry Debtors"),
    ] {
        b.ledger(n, g, Money::ZERO);
    }
    let d = b.day(30);
    let bank = "HDFC Bank";
    for (dr, cr, a) in [
        (bank, "Sales", 500_000),
        ("Purchases", bank, 300_000),
        ("Salary", bank, 50_000),
        ("Plant & Machinery", bank, 80_000),
        ("Furniture", "Furniture Supplier", 20_000), // bought on credit: not a cash flow
        (bank, "HDFC Term Loan", 60_000),
        ("HDFC Term Loan", bank, 10_000),
        ("Interest on Term Loan", bank, 5_000),
        ("Advance Tax", bank, 15_000),
        ("Income Tax Expense", "Provision for Income Tax", 18_000),
        ("Depreciation", "Plant & Machinery", 8_000),
        (bank, "Interest Received", 2_000),
        ("Customer A", "Sales", 30_000),
        (bank, "Equity Share Capital", 50_000),
    ] {
        b.v(d, "Journal", &[(dr, r(a)), (cr, r(-a))]);
    }
    b.finish().0
}

fn item(lines: &[CfLine], label: &str) -> Money {
    lines
        .iter()
        .find_map(|l| match l {
            CfLine::Item(t, m) | CfLine::Subtotal(t, m) if t.starts_with(label) => Some(*m),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no line '{label}': {lines:#?}"))
}

#[test]
fn transactions_give_actual_cash_and_non_cash_items_are_shown() {
    let eng = book();
    let a = analyse(&eng, &RulesPack::builtin());
    let py = a.facts_py.clone().unwrap();
    let cf = cashflow::build(&eng, &a.facts_cy, &py, &a.facts_cy, &py, 1).unwrap();
    assert_eq!(cf.method, "transactions");
    let l = &cf.lines;
    assert_eq!(
        item(l, "Purchase of property, plant"),
        r(-80_000),
        "only the bank purchase, not the credit one"
    );
    assert_eq!(item(l, "Interest and dividends received"), r(2_000));
    assert_eq!(item(l, "Proceeds from borrowings"), r(60_000));
    assert_eq!(item(l, "Repayment of borrowings"), r(-10_000));
    assert_eq!(item(l, "Proceeds from issue of share capital"), r(50_000));
    assert_eq!(item(l, "Finance costs paid"), r(-5_000));
    assert_eq!(
        item(l, "Income taxes paid"),
        r(-15_000),
        "advance tax paid, not the expense"
    );
    assert_eq!(
        item(l, "Non-cash transactions"),
        r(-20_000),
        "furniture on credit"
    );
    assert_eq!(
        item(l, "Net cash from / (used in) operating activities (A)"),
        r(135_000)
    );
    assert_eq!(
        item(l, "Net cash from / (used in) investing activities (B)"),
        r(-78_000)
    );
    assert_eq!(
        item(l, "Net cash from / (used in) financing activities (C)"),
        r(95_000)
    );
    assert!(
        cf.review.iter().any(|w| w.contains("non-cash")),
        "{:?}",
        cf.review
    );
}

#[test]
fn without_a_day_book_lines_are_marked_derived_and_taxes_still_come_from_balances() {
    let mut eng = book();
    let a = analyse(&eng, &RulesPack::builtin());
    eng.vouchers.clear();
    let py = a.facts_py.clone().unwrap();
    let cf = cashflow::build(&eng, &a.facts_cy, &py, &a.facts_cy, &py, 1).unwrap();
    assert_eq!(cf.method, "balances");
    assert_eq!(item(&cf.lines, "Income taxes paid"), r(-15_000));
    assert!(cf
        .lines
        .iter()
        .any(|l| matches!(l, CfLine::Item(t, _) if t.contains("(derived)"))));
    assert!(!cf
        .lines
        .iter()
        .any(|l| matches!(l, CfLine::Item(t, _) if t.starts_with("Non-cash"))));
}

#[test]
fn applicability_is_never_claimed_as_required() {
    let eng = book();
    assert_eq!(cashflow::applicability(&eng).status, "REQUIRES_REVIEW");
}
