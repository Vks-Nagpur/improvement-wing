//! Adversarial books (J02): awkward but possible books. Each case states
//! what LedgerCraft must do: report a named finding, keep the statements
//! balanced, never panic.

use chrono::NaiveDate;
use lc_core::model::{Engagement, Ledger, Voucher, VoucherLine};
use lc_core::rules::RulesPack;
use lc_core::{analyse, Analysis, Money};
use lc_testdata::scenarios::{self, Kind, BANK, CASH};
use std::collections::BTreeSet;

fn r(x: i64) -> Money {
    Money::rupees(x)
}

/// Clean firm books, every placement confirmed.
fn base(with_vouchers: bool) -> Engagement {
    let mut e = scenarios::clean(Kind::Firm, 11, 6, 4, 2).engagement;
    if !with_vouchers {
        e.vouchers.clear();
    }
    let a = analyse(&e, &RulesPack::builtin());
    lc_testdata::confirm_all(&e, &a)
}

fn run(e: &Engagement) -> Analysis {
    analyse(e, &RulesPack::builtin())
}

fn codes(a: &Analysis) -> BTreeSet<String> {
    a.findings.iter().map(|f| f.code.clone()).collect()
}

fn idx(e: &Engagement, name: &str) -> usize {
    e.cy.ledgers.iter().position(|l| l.name == name).unwrap()
}

/// Move `amt` of closing balance from one ledger to another (TB stays balanced).
fn shift(e: &mut Engagement, from: &str, to: &str, amt: Money) {
    let (a, b) = (idx(e, from), idx(e, to));
    e.cy.ledgers[a].closing -= amt;
    e.cy.ledgers[b].closing += amt;
}

fn add_ledger(e: &mut Engagement, name: &str, group: &str, closing: Money) {
    e.cy.ledgers.push(Ledger {
        name: name.into(),
        group: group.into(),
        opening: Money::ZERO,
        closing,
        closing_stock: None,
        tags: vec![],
    });
}

fn tallies(a: &Analysis) {
    assert_eq!(a.statements.total_assets, a.statements.total_liabilities);
}

fn voucher(date: NaiveDate, no: &str, lines: &[(&str, Money)]) -> Voucher {
    Voucher {
        date,
        number: no.into(),
        vtype: "Journal".into(),
        narration: String::new(),
        lines: lines
            .iter()
            .map(|(l, a)| VoucherLine {
                ledger: l.to_string(),
                amount: *a,
            })
            .collect(),
    }
}

/// Run a case: never panics, and the finding codes are exactly `want`.
fn case(name: &str, e: &Engagement, want: &[&str]) -> Analysis {
    let a = std::panic::catch_unwind(|| run(e)).unwrap_or_else(|_| panic!("{name}: panicked"));
    let want: BTreeSet<String> = want.iter().map(|s| s.to_string()).collect();
    assert_eq!(codes(&a), want, "{name}");
    tallies(&a);
    a
}

fn first_in(e: &Engagement, group_prefix: &str) -> String {
    e.cy.ledgers
        .iter()
        .find(|l| l.group.starts_with(group_prefix))
        .unwrap()
        .name
        .clone()
}

#[test]
fn balances_on_the_unusual_side() {
    let e0 = base(false);
    case("clean", &e0, &[]);

    let mut e = e0.clone();
    add_ledger(&mut e, "Zero Ledger", "Indirect Expenses", Money::ZERO);
    case("a ledger with nothing in it", &e, &[]);

    // A confirmed placement whose balance changes side is reopened for review.
    let mut e = e0.clone();
    let b = e.cy.ledgers[idx(&e, BANK)].closing;
    shift(&mut e, BANK, CASH, b + r(5_000));
    case("overdrawn bank", &e, &["MAPPING_REVIEW"]);
    let fresh = Engagement {
        mapping_memory: Default::default(),
        mapping_context: Default::default(),
        ..e.clone()
    };
    assert!(
        codes(&run(&fresh)).contains("RECLASS_BANK_CR"),
        "unconfirmed overdraft is shown as borrowing"
    );

    let sup = first_in(&e0, "Sundry Creditors");
    let mut e = e0.clone();
    let s = e.cy.ledgers[idx(&e, &sup)].closing;
    shift(&mut e, CASH, &sup, -s + r(700));
    case("supplier with a debit balance", &e, &["MAPPING_REVIEW"]);

    let cus = first_in(&e0, "Debtors");
    let mut e = e0.clone();
    let s = e.cy.ledgers[idx(&e, &cus)].closing;
    shift(&mut e, &cus, CASH, s + r(900));
    case("customer with a credit balance", &e, &["MAPPING_REVIEW"]);

    let mut e = e0.clone();
    add_ledger(&mut e, "Sales Returns", "Sales Accounts", r(25_000));
    let c = idx(&e, CASH);
    e.cy.ledgers[c].closing -= r(25_000);
    case("contra ledger: sales returns", &e, &["ABNORMAL_BALANCE"]);
}

#[test]
fn suspense_opening_and_duplicate_names() {
    let e0 = base(false);
    let mut e = e0.clone();
    add_ledger(&mut e, "Suspense A/c", "Suspense A/c", r(1_234));
    let c = idx(&e, CASH);
    e.cy.ledgers[c].closing -= r(1_234);
    case("suspense", &e, &["MAPPING_UNCONFIRMED", "SUSPENSE_BALANCE"]);

    let mut e = e0.clone();
    let c = idx(&e, "Furniture & Fixtures");
    e.cy.ledgers[c].opening += r(100);
    case(
        "opening differs from last year's closing",
        &e,
        &["OPENING_DIFF", "OPENING_TOTAL_DIFF"],
    );

    // Same name, different group: named as such (it used to show only as
    // an opening difference and a review).
    let mut e = e0.clone();
    add_ledger(&mut e, "Office Expenses", "Sundry Creditors", r(-500));
    let c = idx(&e, CASH);
    e.cy.ledgers[c].closing += r(500);
    let a = case(
        "same ledger name in two groups",
        &e,
        &["DUPLICATE_LEDGER", "MAPPING_REVIEW", "OPENING_DIFF"],
    );
    assert_eq!(
        a.findings
            .iter()
            .filter(|f| f.code == "DUPLICATE_LEDGER")
            .count(),
        1
    );
}

#[test]
fn unicode_names_change_nothing() {
    let mut e = base(false);
    let before = run(&e);
    let new = "कार्यालय खर्च – Büro 事務所 🧾";
    for tb in std::iter::once(&mut e.cy).chain(e.py.iter_mut()) {
        for l in tb
            .ledgers
            .iter_mut()
            .filter(|l| l.name == "Office Expenses")
        {
            l.name = new.into();
        }
    }
    let k = lc_core::model::norm_name("Office Expenses");
    if let Some(v) = e.mapping_memory.remove(&k) {
        e.mapping_memory.insert(lc_core::model::norm_name(new), v);
    }
    if let Some(v) = e.mapping_context.remove(&k) {
        e.mapping_context.insert(lc_core::model::norm_name(new), v);
    }
    let a = case("unicode ledger name", &e, &[]);
    assert_eq!(a.statements.profit, before.statements.profit);
    assert_eq!(a.statements.total_assets, before.statements.total_assets);
}

#[test]
fn huge_amounts_are_refused_not_crashed() {
    let e0 = base(true);
    let mut e = e0.clone();
    e.vouchers.push(voucher(
        e.fy_start,
        "X3",
        &[(CASH, Money(i64::MAX - 5)), (BANK, Money(-(i64::MAX - 5)))],
    ));
    let a = case(
        "voucher near the largest number",
        &e,
        &["AMOUNTS_TOO_LARGE"],
    );
    assert!(!a.printable);

    let mut e = base(false);
    let c = idx(&e, CASH);
    e.cy.ledgers[c].closing += Money(i64::MAX / 2);
    let k = idx(&e, "Partner A - Capital");
    e.cy.ledgers[k].closing -= Money(i64::MAX / 2);
    case(
        "balance near the largest number",
        &e,
        &["AMOUNTS_TOO_LARGE"],
    );

    // Large but real (₹50,000 crore) is fine.
    let mut e = base(false);
    let big = Money::rupees(500_000_000_000);
    let c = idx(&e, CASH);
    e.cy.ledgers[c].closing += big;
    let k = idx(&e, "Partner A - Capital");
    e.cy.ledgers[k].closing -= big;
    case("₹50,000 crore", &e, &[]);
}

#[test]
fn vouchers_duplicated_malformed_or_empty() {
    let v0 = base(true);
    case("clean with vouchers", &v0, &[]);

    let mut e = v0.clone();
    let d = e.vouchers[3].clone();
    e.vouchers.push(d);
    case(
        "voucher entered twice",
        &e,
        &["DUPLICATE_VOUCHER", "VOUCHER_TB_MISMATCH"],
    );

    let mut e = v0.clone();
    e.vouchers.push(voucher(e.fy_start, "X1", &[(CASH, r(10))]));
    case(
        "one-sided voucher",
        &e,
        &["VOUCHER_TB_MISMATCH", "VOUCHER_UNBALANCED"],
    );

    let mut e = v0.clone();
    e.vouchers.push(voucher(e.fy_start, "X2", &[]));
    case("voucher with no lines", &e, &[]);
}

#[test]
fn periods_part_year_eighteen_months_and_leap_year() {
    let e0 = base(false);
    let mut e = e0.clone();
    e.fy_start = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
    // Depreciation in the register is for a full year, so it no longer agrees.
    case(
        "six-month period",
        &e,
        &["FAR_DEP_MISMATCH", "FAR_TB_MISMATCH"],
    );

    let mut e = e0.clone();
    e.fy_start = NaiveDate::from_ymd_opt(2024, 10, 1).unwrap();
    e.far = None;
    e.py = None;
    let a = case("eighteen-month period", &e, &[]);
    let rep = lc_core::report::build(&e, &a, &Default::default(), &Default::default());
    let text = serde_json::to_string(&rep).unwrap();
    assert!(text.contains("For the period ended 31 March 2026"));
    assert!(!text.contains("For the year ended"));

    // FY 2027-28 has 29 February 2028.
    let mut l = base(false);
    l.fy_start = NaiveDate::from_ymd_opt(2027, 4, 1).unwrap();
    l.fy_end = NaiveDate::from_ymd_opt(2028, 3, 31).unwrap();
    l.py = None;
    l.far = None;
    // Vouchers net to nil, so each closing equals its opening.
    for x in l.cy.ledgers.iter_mut() {
        x.closing = x.opening;
        x.closing_stock = None;
    }
    l.vouchers = vec![
        voucher(
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(),
            "L1",
            &[(CASH, r(10)), (BANK, r(-10))],
        ),
        voucher(
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap(),
            "L2",
            &[(BANK, r(10)), (CASH, r(-10))],
        ),
    ];
    l.mapping_memory.clear();
    l.mapping_context.clear();
    let l = lc_testdata::confirm_all(&l, &run(&l));
    case("leap day vouchers", &l, &[]);
}
