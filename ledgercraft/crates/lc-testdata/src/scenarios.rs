//! Scenarios from basic to complex, each with ground truth.

use crate::builder::{Builder, Expected};
use crate::rng::Rng;
use chrono::NaiveDate;
use lc_core::model::{Engagement, EntityType};
use lc_core::Money;
use std::collections::HashMap;

pub struct Scenario {
    pub name: String,
    pub description: String,
    pub engagement: Engagement,
    pub expected: Expected,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Firm,
    Company,
}

pub const CASH: &str = "Cash";
pub const BANK: &str = "HDFC Bank Current A/c";
pub const SALES: &str = "Sales - Local";
pub const PURCHASES: &str = "Purchases - Local";
pub const PLAC: &str = "Profit & Loss A/c";

const CASH_EXPENSES: &[&str] = &[
    "Office Expenses",
    "Printing & Stationery",
    "Telephone Expenses",
    "Staff Welfare",
    "Conveyance",
];

fn r(x: i64) -> Money {
    Money::rupees(x)
}

fn dmy(d: NaiveDate) -> String {
    d.format("%d-%m-%Y").to_string()
}

struct Sim {
    kind: Kind,
    customers: Vec<String>,
    suppliers: Vec<String>,
    ops_per_day: usize,
    cash_rcpt_today: HashMap<String, Money>,
}

/// Create the standard ledgers with openings. Capital is balanced later by `balance_capital`.
fn base_ledgers(b: &mut Builder, rng: &mut Rng, kind: Kind, n_cust: usize, n_sup: usize) -> Sim {
    let cust_group = match kind {
        Kind::Firm => "Debtors - Local",
        Kind::Company => "Debtors - Export",
    };
    b.group(cust_group, "Sundry Debtors");
    b.group("Administrative Expenses", "Indirect Expenses");
    b.group("Travel & Conveyance", "Administrative Expenses");

    match kind {
        Kind::Firm => {
            b.ledger("Partner A - Capital", "Capital Account", r(-500_000));
            b.ledger("Partner B - Capital", "Capital Account", Money::ZERO);
            b.ledger("Partners' Remuneration", "Indirect Expenses", Money::ZERO);
            b.ledger(
                "Interest on Partners' Capital",
                "Indirect Expenses",
                Money::ZERO,
            );
        }
        Kind::Company => {
            b.ledger("Equity Share Capital", "Capital Account", r(-1_000_000));
            b.ledger("Securities Premium", "Reserves & Surplus", Money::ZERO);
            b.ledger(
                "Directors' Remuneration",
                "Administrative Expenses",
                Money::ZERO,
            );
            b.ledger("Income Tax Expense", "Indirect Expenses", Money::ZERO);
            b.ledger("Provision for Income Tax", "Provisions", Money::ZERO);
            b.ledger("Advance Tax", "Loans & Advances (Asset)", Money::ZERO);
            b.ledger("HDFC Bank Term Loan", "Secured Loans", r(-800_000));
            b.ledger("Interest on Term Loan", "Indirect Expenses", Money::ZERO);
            b.ledger("Investment in Mutual Funds", "Investments", r(200_000));
            b.ledger("Security Deposit - Office", "Deposits (Asset)", r(150_000));
            b.ledger("Accounting Software", "Fixed Assets", r(60_000));
            b.ledger(
                "Capital Work-in-Progress - Factory",
                "Fixed Assets",
                Money::ZERO,
            );
            b.ledger("Travelling Expenses", "Travel & Conveyance", Money::ZERO);
            b.ledger("Stock-in-Trade - Warehouse B", "Stock-in-Hand", Money::ZERO);
        }
    }
    b.ledger(PLAC, "Profit & Loss A/c", Money::ZERO);
    b.ledger(CASH, "Cash-in-Hand", r(60_000));
    b.ledger(BANK, "Bank Accounts", r(900_000));
    b.ledger(SALES, "Sales Accounts", Money::ZERO);
    b.ledger(PURCHASES, "Purchase Accounts", Money::ZERO);
    b.ledger("Freight Inward", "Direct Expenses", Money::ZERO);
    for e in CASH_EXPENSES {
        b.ledger(e, "Indirect Expenses", Money::ZERO);
    }
    for e in [
        "Rent",
        "Electricity Charges",
        "Salary",
        "Bank Charges",
        "Depreciation",
        "Interest on Unsecured Loans",
    ] {
        b.ledger(e, "Indirect Expenses", Money::ZERO);
    }
    b.ledger("Interest Received", "Indirect Incomes", Money::ZERO);
    b.ledger("Plant & Machinery", "Fixed Assets", r(800_000));
    b.ledger("Furniture & Fixtures", "Fixed Assets", r(150_000));
    b.ledger("TDS Payable", "Duties & Taxes", Money::ZERO);
    b.ledger(
        "Unsecured Loan - Rajesh Kumar",
        "Unsecured Loans",
        r(-300_000),
    );

    let os = rng.money(200_000, 400_000);
    let cs = rng.money(250_000, 450_000);
    let py_os = rng.money(150_000, 350_000);
    b.stock("Stock-in-Trade", "Stock-in-Hand", os, cs, py_os);
    if kind == Kind::Company {
        // Second warehouse: opening already zero, closing stock planned.
        b.set_opening("Stock-in-Trade - Warehouse B", r(50_000));
    }

    let mut customers = Vec::new();
    for i in 1..=n_cust {
        let n = format!("Customer {i:02}");
        b.ledger(&n, cust_group, rng.money(5_000, 150_000));
        customers.push(n);
    }
    let mut suppliers = Vec::new();
    for i in 1..=n_sup {
        let n = format!("Supplier {i:02}");
        b.ledger(&n, "Sundry Creditors", -rng.money(5_000, 120_000));
        suppliers.push(n);
    }

    // Previous-year income and expenses (for comparatives).
    b.py_pl(SALES, "Sales Accounts", -rng.money(4_000_000, 6_000_000));
    b.py_pl(
        PURCHASES,
        "Purchase Accounts",
        rng.money(2_500_000, 3_200_000),
    );
    b.py_pl("Rent", "Indirect Expenses", r(300_000));
    b.py_pl("Salary", "Indirect Expenses", rng.money(400_000, 600_000));
    b.py_pl(
        "Electricity Charges",
        "Indirect Expenses",
        rng.money(50_000, 90_000),
    );
    b.py_pl(
        "Office Expenses",
        "Indirect Expenses",
        rng.money(30_000, 60_000),
    );
    b.py_pl(
        "Depreciation",
        "Indirect Expenses",
        rng.money(100_000, 150_000),
    );

    Sim {
        kind,
        customers,
        suppliers,
        ops_per_day: 0,
        cash_rcpt_today: HashMap::new(),
    }
}

/// Register that ties to the base ledgers (books on Income-tax rates, WDV).
fn base_far(b: &Builder, extra: Vec<lc_core::far::Asset>) -> lc_core::far::Register {
    use lc_core::far::{Asset, BookBasis, Register};
    let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
    let mut assets = vec![
        Asset {
            name: "CNC Lathe".into(),
            ledger: "Plant & Machinery".into(),
            book_class: "plant_general".into(),
            it_block: "plant_general".into(),
            put_to_use: d(2021, 6, 1),
            cost: r(1_000_000),
            opening_acc_dep: r(200_000),
            sold_on: None,
            sale_value: Money::ZERO,
            useful_life_years: None,
        },
        Asset {
            name: "Office furniture".into(),
            ledger: "Furniture & Fixtures".into(),
            book_class: "furniture".into(),
            it_block: "furniture".into(),
            put_to_use: d(2022, 4, 10),
            cost: r(200_000),
            opening_acc_dep: r(50_000),
            sold_on: None,
            sale_value: Money::ZERO,
            useful_life_years: None,
        },
    ];
    assets.extend(extra);
    assert_eq!(
        b.bal("Plant & Machinery"),
        r(800_000),
        "register assumes the standard opening block"
    );
    Register {
        assets,
        it_opening: [
            ("plant_general".to_string(), r(800_000)),
            ("furniture".to_string(), r(150_000)),
        ]
        .into_iter()
        .collect(),
        basis: BookBasis::IncomeTaxRates,
    }
}

fn balance_capital(b: &mut Builder, kind: Kind) {
    let name = match kind {
        Kind::Firm => "Partner B - Capital",
        Kind::Company => PLAC,
    };
    let cur = b.bal(name);
    let total = b.opening_total();
    b.set_opening(name, cur - total);
    assert!(b.opening_total().is_zero());
}

/// Post a voucher, nudging the amount by 1 paisa until it is unique for the day.
fn post_unique(
    b: &mut Builder,
    d: NaiveDate,
    vt: &str,
    amt: Money,
    mk: impl Fn(Money) -> Vec<(String, Money)>,
) -> String {
    let mut a = amt;
    loop {
        let lines = mk(a);
        let refs: Vec<(&str, Money)> = lines.iter().map(|(n, m)| (n.as_str(), *m)).collect();
        if let Some(k) = b.try_v(d, vt, &refs) {
            return k;
        }
        a += Money(1);
    }
}

fn two(dr: &str, cr: &str, a: Money) -> Vec<(String, Money)> {
    vec![(dr.to_string(), a), (cr.to_string(), -a)]
}

/// Make sure cash covers a planned payment of `amt` (withdraw from bank if needed).
fn ensure_cash(b: &mut Builder, d: NaiveDate, amt: Money) {
    if b.bal(CASH) < amt + r(20_000) {
        let w = amt + r(50_000);
        post_unique(b, d, "Contra", w, |a| two(CASH, BANK, a));
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Start,
    End,
}

fn simulate(
    b: &mut Builder,
    rng: &mut Rng,
    sim: &mut Sim,
    hook: &mut dyn FnMut(&mut Builder, i64, Phase),
) {
    let days = (b.fy_end - b.fy_start).num_days() + 1;
    for dn in 0..days {
        let d = b.day(dn);
        sim.cash_rcpt_today.clear();
        hook(b, dn, Phase::Start);
        if dn % 30 == 15 {
            monthly(b, rng, sim, d);
        }
        for _ in 0..sim.ops_per_day {
            daily_op(b, rng, sim, d);
        }
        if dn == days - 1 {
            year_end(b, sim, d);
        }
        hook(b, dn, Phase::End);
    }
}

fn monthly(b: &mut Builder, rng: &mut Rng, sim: &mut Sim, d: NaiveDate) {
    post_unique(b, d, "Payment", r(25_000), |a| two("Rent", BANK, a));
    post_unique(b, d, "Payment", rng.money(40_000, 55_000), |a| {
        two("Salary", BANK, a)
    });
    post_unique(b, d, "Payment", rng.money(4_000, 8_000), |a| {
        two("Electricity Charges", BANK, a)
    });
    // Interest credited to the lender, net of TDS (not a fresh loan).
    let int = r(3_000);
    let tds = r(300);
    b.v(
        d,
        "Journal",
        &[
            ("Interest on Unsecured Loans", int),
            ("Unsecured Loan - Rajesh Kumar", -(int - tds)),
            ("TDS Payable", -tds),
        ],
    );
    let pending = -b.bal("TDS Payable");
    if pending.is_dr() {
        post_unique(b, d, "Payment", pending, |a| two("TDS Payable", BANK, a));
    }
    if sim.kind == Kind::Company {
        post_unique(b, d, "Payment", r(40_000), |a| {
            two("HDFC Bank Term Loan", BANK, a)
        });
        post_unique(b, d, "Payment", rng.money(6_000, 7_000), |a| {
            two("Interest on Term Loan", BANK, a)
        });
        post_unique(b, d, "Journal", rng.money(500, 4_000), |a| {
            two("Travelling Expenses", BANK, a)
        });
    }
}

fn daily_op(b: &mut Builder, rng: &mut Rng, sim: &mut Sim, d: NaiveDate) {
    let op = rng.range(0, 99);
    match op {
        0..=21 => {
            let c = rng.pick(&sim.customers).clone();
            post_unique(b, d, "Sales", rng.money(5_000, 200_000), |a| {
                two(&c, SALES, a)
            });
        }
        22..=33 => {
            post_unique(b, d, "Sales", rng.money(500, 50_000), |a| {
                two(CASH, SALES, a)
            });
        }
        34..=50 => {
            let c = rng.pick(&sim.customers).clone();
            let due = b.bal(&c);
            if due > r(1_000) {
                let amt = Money(rng.range(100_000, due.paise()));
                post_unique(b, d, "Receipt", amt, |a| two(BANK, &c, a.min(due)));
            }
        }
        51..=56 => {
            let c = rng.pick(&sim.customers).clone();
            let due = b.bal(&c);
            let today = sim.cash_rcpt_today.get(&c).copied().unwrap_or_default();
            let amt = rng.money(500, 19_000);
            if due > amt + r(10) && today + amt < r(150_000) {
                post_unique(b, d, "Receipt", amt, |a| two(CASH, &c, a));
                *sim.cash_rcpt_today.entry(c).or_default() += amt;
            }
        }
        57..=74 => {
            let s = rng.pick(&sim.suppliers).clone();
            post_unique(b, d, "Purchase", rng.money(5_000, 120_000), |a| {
                two(PURCHASES, &s, a)
            });
        }
        75..=86 => {
            let s = rng.pick(&sim.suppliers).clone();
            let due = -b.bal(&s);
            if due > r(1_000) && b.bal(BANK) > due + r(200_000) {
                let amt = Money(rng.range(100_000, due.paise()));
                post_unique(b, d, "Payment", amt, |a| two(&s, BANK, a.min(due)));
            }
        }
        87..=96 => {
            let e = *rng.pick(CASH_EXPENSES);
            let amt = rng.money(100, 9_999);
            if b.bal(CASH) < amt + r(15_000) {
                if b.bal(BANK) > r(300_000) {
                    post_unique(b, d, "Contra", r(50_000), |a| two(CASH, BANK, a));
                } else {
                    return;
                }
            }
            post_unique(b, d, "Payment", amt, |a| two(e, CASH, a));
        }
        97..=98 => {
            post_unique(b, d, "Payment", rng.money(10, 500), |a| {
                two("Bank Charges", BANK, a)
            });
        }
        _ => {
            post_unique(b, d, "Receipt", rng.money(1_000, 9_000), |a| {
                two(BANK, "Interest Received", a)
            });
        }
    }
}

fn year_end(b: &mut Builder, sim: &Sim, d: NaiveDate) {
    let pm = Money(b.bal("Plant & Machinery").paise() * 15 / 100);
    let ff = Money(b.bal("Furniture & Fixtures").paise() * 10 / 100);
    b.v(
        d,
        "Journal",
        &[
            ("Depreciation", pm + ff),
            ("Plant & Machinery", -pm),
            ("Furniture & Fixtures", -ff),
        ],
    );
    match sim.kind {
        Kind::Firm => {
            b.v(
                d,
                "Journal",
                &[
                    ("Partners' Remuneration", r(240_000)),
                    ("Partner A - Capital", r(-120_000)),
                    ("Partner B - Capital", r(-120_000)),
                ],
            );
            b.v(
                d,
                "Journal",
                &[
                    ("Interest on Partners' Capital", r(60_000)),
                    ("Partner A - Capital", r(-30_000)),
                    ("Partner B - Capital", r(-30_000)),
                ],
            );
        }
        Kind::Company => {
            b.v(
                d,
                "Journal",
                &[("Directors' Remuneration", r(360_000)), (BANK, r(-360_000))],
            );
            b.v(
                d,
                "Payment",
                &[("Advance Tax", r(150_000)), (BANK, r(-150_000))],
            );
            b.v(
                d,
                "Journal",
                &[
                    ("Income Tax Expense", r(180_000)),
                    ("Provision for Income Tax", r(-180_000)),
                ],
            );
            b.v(
                d,
                "Payment",
                &[
                    ("Capital Work-in-Progress - Factory", r(400_000)),
                    (BANK, r(-400_000)),
                ],
            );
            b.v(
                d,
                "Payment",
                &[
                    ("Investment in Mutual Funds", r(100_000)),
                    (BANK, r(-100_000)),
                ],
            );
        }
    }
}

fn top_up_bank(b: &mut Builder, kind: Kind) {
    // Keep the bank positive at year end in clean books (owners bring funds).
    let bank = b.bal(BANK);
    if bank < r(100_000) {
        let need = r(100_000) - bank;
        let d = b.fy_end;
        let cap = match kind {
            Kind::Firm => "Partner A - Capital",
            Kind::Company => "Equity Share Capital",
        };
        post_unique(b, d, "Receipt", need, |a| two(BANK, cap, a));
    }
}

/// Clean books: the engine must report nothing.
pub fn clean(kind: Kind, seed: u64, n_cust: usize, n_sup: usize, ops_per_day: usize) -> Scenario {
    let (entity, name) = match kind {
        Kind::Firm => (EntityType::Firm, "Clean Traders (Partnership Firm)"),
        Kind::Company => (EntityType::Company, "Clean Industries Private Limited"),
    };
    let mut b = Builder::new(name, entity, 2025);
    let mut rng = Rng::new(seed);
    let mut sim = base_ledgers(&mut b, &mut rng, kind, n_cust, n_sup);
    sim.ops_per_day = ops_per_day;
    if kind == Kind::Firm {
        b.far = Some(base_far(&b, vec![]));
    }
    balance_capital(&mut b, kind);
    simulate(&mut b, &mut rng, &mut sim, &mut |_, _, _| {});
    top_up_bank(&mut b, kind);
    let (engagement, expected) = b.finish();
    Scenario {
        name: format!(
            "clean_{}_{seed}",
            if kind == Kind::Firm {
                "firm"
            } else {
                "company"
            }
        ),
        description: format!(
            "Clean {} books, {} vouchers/day, no planted errors",
            if kind == Kind::Firm {
                "partnership firm"
            } else {
                "company"
            },
            ops_per_day
        ),
        engagement,
        expected,
    }
}

/// Partnership firm with ~30 planted glitches, each with its expected finding.
pub fn firm_with_glitches() -> Scenario {
    let mut b = Builder::new("Glitchy Traders (Partnership Firm)", EntityType::Firm, 2025);
    let mut rng = Rng::new(7);
    let mut sim = base_ledgers(&mut b, &mut rng, Kind::Firm, 12, 8);
    sim.ops_per_day = 6;

    // Extra ledgers for planted cases.
    b.ledger("Loan from Suresh Kumar", "Sundry Creditors", Money::ZERO);
    b.ledger("Shree Ganesh Hardware", "Sundry Creditors", Money::ZERO);
    b.ledger("Balaji Traders", "Sundry Creditors", Money::ZERO);
    b.ledger("Repairs & Maintenance", "Indirect Expenses", Money::ZERO);
    b.ledger_tagged(
        "Shree Roadlines",
        "Sundry Creditors",
        Money::ZERO,
        &["transporter"],
    );
    for n in [
        "Loan from Mahesh Patil",
        "Loan from Anil Joshi",
        "Loan from Kiran Rao",
        "Loan from Prakash Jain",
    ] {
        b.ledger(n, "Unsecured Loans", Money::ZERO);
    }
    b.ledger("Loan from Ravi Mehta", "Unsecured Loans", r(-18_000));
    b.ledger("Loan from Vinod Shah", "Unsecured Loans", r(-50_000));
    b.ledger("Loan from Sunil Patel", "Unsecured Loans", r(-12_000));
    b.ledger("Om Sai Enterprises", "Sundry Debtors", Money::ZERO);
    b.ledger("Sai Krupa Stores", "Sundry Debtors", Money::ZERO);
    b.ledger("Patil Agro Industries", "Sundry Debtors", Money::ZERO);
    b.ledger("State Bank of India CC A/c", "Bank Accounts", Money::ZERO);
    b.ledger("Suspense A/c", "Suspense A/c", Money::ZERO);
    b.ledger("GST Payable - Old", "Sundry Creditors", Money::ZERO);
    b.ledger("Laptop Purchase", "Indirect Expenses", Money::ZERO);
    b.ledger("Old Debtor - Renamed", "Sundry Debtors", r(7_000));
    b.not_in_py("Old Debtor - Renamed");
    b.py_only("Old Debtor", "Sundry Debtors", r(7_000));
    // Furniture bought on day 190 (8 October 2025) is used for less than 180 days:
    // the register gives half rate (₹900); the books charge full rate on it.
    let added = lc_core::far::Asset {
        name: "Workstations".into(),
        ledger: "Furniture & Fixtures".into(),
        book_class: "furniture".into(),
        it_block: "furniture".into(),
        put_to_use: b.day(190),
        cost: r(18_000),
        opening_acc_dep: Money::ZERO,
        sold_on: None,
        sale_value: Money::ZERO,
        useful_life_years: None,
    };
    b.far = Some(base_far(&b, vec![added]));
    b.expect("FAR_DEP_MISMATCH:depreciation");
    b.expect("FAR_TB_MISMATCH:Furniture & Fixtures");
    balance_capital(&mut b, Kind::Firm);

    // Opening-balance edits after last year was closed.
    b.tweak_opening("Customer 03", r(5_000));
    b.tweak_opening("Partner B - Capital", r(-5_000));
    b.expect("OPENING_DIFF:Customer 03");
    b.expect("OPENING_DIFF:Partner B - Capital");
    b.tweak_opening("Rent", r(5_000));
    b.tweak_opening(PLAC, r(-5_000));
    b.expect("PL_OPENING:Rent");
    b.expect(format!("OPENING_DIFF:{PLAC}"));
    b.expect("OPENING_NEW_LEDGER:Old Debtor - Renamed");
    b.expect("OPENING_MISSING_LEDGER:Old Debtor");
    // Static mis-groupings.
    for k in [
        "MISGROUP_LOAN:Loan from Suresh Kumar",
        "MISGROUP_OD:State Bank of India CC A/c",
        "RECLASS_BANK_CR:State Bank of India CC A/c",
        "SUSPENSE_BALANCE:Suspense A/c",
        "MISGROUP_TAX:GST Payable - Old",
        "CAPITAL_IN_EXPENSE:Laptop Purchase",
        "RECLASS_DEBTOR_CR:Patil Agro Industries",
    ] {
        b.expect(k);
    }

    let mut hook = |b: &mut Builder, dn: i64, ph: Phase| {
        let d = b.day(dn);
        if ph == Phase::End {
            if dn == 170 {
                // Deposit more cash than available: cash negative at day end.
                let over = b.bal(CASH) + r(10_000);
                b.v(d, "Contra", &[(BANK, over), (CASH, -over)]);
                b.expect(format!("NEGATIVE_CASH:{CASH} {}", dmy(d)));
            }
            return;
        }
        match dn {
            171 => {
                b.v(d, "Contra", &[(CASH, r(60_000)), (BANK, r(-60_000))]);
            }
            30 => {
                b.v(
                    d,
                    "Receipt",
                    &[(BANK, r(50_000)), ("Loan from Suresh Kumar", r(-50_000))],
                );
            }
            200 => {
                ensure_cash(b, d, r(25_000));
                let k = b.v(
                    d,
                    "Payment",
                    &[("Loan from Suresh Kumar", r(25_000)), (CASH, r(-25_000))],
                );
                b.expect(format!("LOAN_REPAID_CASH:{k} Loan from Suresh Kumar"));
                // Duplicate sales invoice.
                b.v(
                    d,
                    "Sales",
                    &[
                        ("Customer 07", Money(4_712_345)),
                        (SALES, Money(-4_712_345)),
                    ],
                );
                let k2 = b.v_raw(
                    d,
                    "Sales",
                    &[
                        ("Customer 07", Money(4_712_345)),
                        (SALES, Money(-4_712_345)),
                    ],
                );
                b.expect(format!("DUPLICATE_VOUCHER:{k2}"));
            }
            40 => {
                b.v(
                    d,
                    "Purchase",
                    &[
                        (PURCHASES, r(20_000)),
                        ("Shree Ganesh Hardware", r(-20_000)),
                    ],
                );
                b.v(
                    d,
                    "Purchase",
                    &[(PURCHASES, r(15_000)), ("Balaji Traders", r(-15_000))],
                );
                b.v(
                    d,
                    "Purchase",
                    &[
                        ("Freight Inward", r(80_000)),
                        ("Shree Roadlines", r(-80_000)),
                    ],
                );
            }
            45 => {
                ensure_cash(b, d, r(13_500));
                b.v(
                    d,
                    "Payment",
                    &[("Shree Ganesh Hardware", r(6_000)), (CASH, r(-6_000))],
                );
                b.v(
                    d,
                    "Payment",
                    &[("Shree Ganesh Hardware", r(7_500)), (CASH, r(-7_500))],
                );
                b.expect(format!(
                    "CASH_PAYMENT_LIMIT:{} Shree Ganesh Hardware",
                    dmy(d)
                ));
            }
            46 => {
                ensure_cash(b, d, r(9_000));
                b.v(
                    d,
                    "Payment",
                    &[("Balaji Traders", r(9_000)), (CASH, r(-9_000))],
                );
            }
            50 => {
                ensure_cash(b, d, Money(1_250_000));
                let k = b.v(
                    d,
                    "Payment",
                    &[
                        ("Repairs & Maintenance", Money(1_250_000)),
                        (CASH, Money(-1_250_000)),
                    ],
                );
                b.expect(format!("CASH_PAYMENT_LIMIT:{k} Repairs & Maintenance"));
            }
            51 => {
                ensure_cash(b, d, r(10_000));
                b.v(
                    d,
                    "Payment",
                    &[("Repairs & Maintenance", r(10_000)), (CASH, r(-10_000))],
                );
            }
            60 => {
                ensure_cash(b, d, r(30_000));
                b.v(
                    d,
                    "Payment",
                    &[("Shree Roadlines", r(30_000)), (CASH, r(-30_000))],
                );
            }
            61 => {
                ensure_cash(b, d, r(40_000));
                b.v(
                    d,
                    "Payment",
                    &[("Shree Roadlines", r(40_000)), (CASH, r(-40_000))],
                );
                b.expect(format!("CASH_PAYMENT_LIMIT:{} Shree Roadlines", dmy(d)));
            }
            70 => {
                let k = b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(25_000)), ("Loan from Mahesh Patil", r(-25_000))],
                );
                b.expect(format!("LOAN_ACCEPTED_CASH:{k} Loan from Mahesh Patil"));
            }
            80 => {
                b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(15_000)), ("Loan from Anil Joshi", r(-15_000))],
                );
            }
            85 => {
                b.v(
                    d,
                    "Receipt",
                    &[
                        (CASH, Money(1_999_900)),
                        ("Loan from Kiran Rao", Money(-1_999_900)),
                    ],
                );
            }
            90 => {
                let k = b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(10_000)), ("Loan from Anil Joshi", r(-10_000))],
                );
                b.expect(format!("LOAN_ACCEPTED_CASH:{k} Loan from Anil Joshi"));
            }
            100 => {
                // Interest credited to lender: must NOT count as loan accepted.
                b.v(
                    d,
                    "Journal",
                    &[
                        ("Interest on Unsecured Loans", r(5_000)),
                        ("Loan from Ravi Mehta", r(-5_000)),
                    ],
                );
            }
            110 => {
                // Principal 18,000 + 1,500 = 19,500 (< 20,000) -> no 269SS finding.
                b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(1_500)), ("Loan from Ravi Mehta", r(-1_500))],
                );
            }
            120 => {
                // Balance with interest 24,500 (>= 20,000) -> 269T finding.
                ensure_cash(b, d, r(2_000));
                let k = b.v(
                    d,
                    "Payment",
                    &[("Loan from Ravi Mehta", r(2_000)), (CASH, r(-2_000))],
                );
                b.expect(format!("LOAN_REPAID_CASH:{k} Loan from Ravi Mehta"));
            }
            130 => {
                ensure_cash(b, d, r(15_000));
                let k = b.v(
                    d,
                    "Payment",
                    &[("Loan from Vinod Shah", r(15_000)), (CASH, r(-15_000))],
                );
                b.expect(format!("LOAN_REPAID_CASH:{k} Loan from Vinod Shah"));
            }
            135 => {
                ensure_cash(b, d, r(12_000));
                b.v(
                    d,
                    "Payment",
                    &[("Loan from Sunil Patel", r(12_000)), (CASH, r(-12_000))],
                );
            }
            140 => {
                let k = b.v(
                    d,
                    "Journal",
                    &[("Rent", r(30_000)), ("Loan from Prakash Jain", r(-30_000))],
                );
                b.expect(format!("LOAN_ACCEPTED_JOURNAL:{k} Loan from Prakash Jain"));
            }
            150 => {
                b.v(
                    d,
                    "Sales",
                    &[("Om Sai Enterprises", r(300_000)), (SALES, r(-300_000))],
                );
                b.v(
                    d,
                    "Sales",
                    &[("Sai Krupa Stores", r(250_000)), (SALES, r(-250_000))],
                );
                b.v(
                    d,
                    "Sales",
                    &[("Patil Agro Industries", r(10_000)), (SALES, r(-10_000))],
                );
            }
            155 => {
                b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(120_000)), ("Om Sai Enterprises", r(-120_000))],
                );
                b.v(
                    d,
                    "Receipt",
                    &[(CASH, r(90_000)), ("Om Sai Enterprises", r(-90_000))],
                );
                b.expect(format!("CASH_RECEIPT_LIMIT:{} Om Sai Enterprises", dmy(d)));
                b.v(
                    d,
                    "Receipt",
                    &[(BANK, r(25_000)), ("Patil Agro Industries", r(-25_000))],
                );
            }
            156 => {
                b.v(
                    d,
                    "Receipt",
                    &[
                        (CASH, Money(19_999_900)),
                        ("Sai Krupa Stores", Money(-19_999_900)),
                    ],
                );
            }
            160 => {
                let k = b.v(d, "Sales", &[(CASH, r(250_000)), (SALES, r(-250_000))]);
                b.expect(format!("CASH_RECEIPT_LIMIT:{k} {SALES}"));
                b.v(
                    d,
                    "Payment",
                    &[
                        (PURCHASES, r(100_000)),
                        ("State Bank of India CC A/c", r(-100_000)),
                    ],
                );
                b.v(
                    d,
                    "Journal",
                    &[
                        ("Office Expenses", Money(123_400)),
                        ("Suspense A/c", Money(-123_400)),
                    ],
                );
                b.v(
                    d,
                    "Journal",
                    &[("Customer 06", r(1_800)), ("GST Payable - Old", r(-1_800))],
                );
                b.v(
                    d,
                    "Payment",
                    &[("Laptop Purchase", r(65_000)), (BANK, r(-65_000))],
                );
            }
            190 => {
                ensure_cash(b, d, r(18_000));
                b.v(
                    d,
                    "Payment",
                    &[("Furniture & Fixtures", r(18_000)), (CASH, r(-18_000))],
                );
                b.expect(format!(
                    "CASH_ASSET_PURCHASE:{} Furniture & Fixtures",
                    dmy(d)
                ));
            }
            _ => {}
        }
    };
    simulate(&mut b, &mut rng, &mut sim, &mut hook);
    top_up_bank(&mut b, Kind::Firm);
    let (engagement, expected) = b.finish();
    Scenario {
        name: "firm_with_glitches".into(),
        description: "Partnership firm with planted opening differences, wrong groupings, cash-limit, loan (principal vs interest) and voucher errors".into(),
        engagement,
        expected,
    }
}

/// Typical problems of a hand-made Excel import.
pub fn excel_import_errors() -> Scenario {
    let mut b = Builder::new(
        "Excel Import Errors (Proprietorship)",
        EntityType::Proprietor,
        2025,
    );
    let mut rng = Rng::new(11);
    let mut sim = base_ledgers(&mut b, &mut rng, Kind::Firm, 5, 4);
    sim.ops_per_day = 2;
    b.ledger("Sundry Deposits", "Deposits Misc", Money::ZERO);
    balance_capital(&mut b, Kind::Firm);
    let mut hook = |b: &mut Builder, dn: i64, ph: Phase| {
        if ph != Phase::Start {
            return;
        }
        let d = b.day(dn);
        match dn {
            20 => {
                let k = b.v_raw(d, "Payment", &[("Rent", r(1_000)), (BANK, r(-900))]);
                b.expect(format!("VOUCHER_UNBALANCED:{k}"));
            }
            25 => {
                b.v_with_ghost(d, "Payment", "Misc Ledger X", r(300), CASH);
                b.expect("VOUCHER_UNKNOWN_LEDGER:Misc Ledger X");
            }
            30 => {
                b.v(
                    d,
                    "Payment",
                    &[("Sundry Deposits", r(10_000)), (BANK, r(-10_000))],
                );
                b.expect("UNKNOWN_GROUP:Sundry Deposits");
            }
            _ => {}
        }
    };
    simulate(&mut b, &mut rng, &mut sim, &mut hook);
    top_up_bank(&mut b, Kind::Firm);
    let late = b.fy_end + chrono::Duration::days(2);
    let k = b.v(late, "Payment", &[("Rent", r(2_000)), (BANK, r(-2_000))]);
    b.expect(format!("VOUCHER_OUTSIDE_PERIOD:{k}"));
    b.tweak_closing("Rent", r(500));
    b.expect("VOUCHER_TB_MISMATCH:Rent");
    b.expect("TB_UNBALANCED:closing");
    let (engagement, expected) = b.finish();
    Scenario {
        name: "excel_import_errors".into(),
        description: "Unbalanced voucher, unknown ledger, unknown group, edited TB closing, voucher outside the year".into(),
        engagement,
        expected,
    }
}

/// All scenarios used by the test-suite and the `ledgercraft scenarios` command.
pub fn all() -> Vec<Scenario> {
    vec![
        clean(Kind::Firm, 1, 10, 6, 4),
        clean(Kind::Company, 2, 25, 15, 12),
        firm_with_glitches(),
        excel_import_errors(),
        clean_llp(),
        clean_huf(),
    ]
}

/// Rename ledgers everywhere (both trial balances and the day book).
fn rename_ledgers(e: &mut Engagement, renames: &[(&str, &str)]) {
    let new = |n: &str| {
        renames
            .iter()
            .find(|(o, _)| *o == n)
            .map(|(_, x)| x.to_string())
    };
    for tb in std::iter::once(&mut e.cy).chain(e.py.iter_mut()) {
        for l in tb.ledgers.iter_mut() {
            if let Some(x) = new(&l.name) {
                l.name = x;
            }
        }
    }
    for v in e.vouchers.iter_mut() {
        for l in v.lines.iter_mut() {
            if let Some(x) = new(&l.ledger) {
                l.ledger = x;
            }
        }
    }
}

/// Clean LLP books: partners' contribution instead of capital.
pub fn clean_llp() -> Scenario {
    let mut s = clean(Kind::Firm, 3, 8, 5, 3);
    s.engagement.entity_type = EntityType::Llp;
    s.engagement.entity_name = "Clean Ventures LLP".into();
    rename_ledgers(
        &mut s.engagement,
        &[
            ("Partner A - Capital", "Designated Partner A - Contribution"),
            ("Partner B - Capital", "Designated Partner B - Contribution"),
        ],
    );
    s.name = "clean_llp".into();
    s.description = "Clean LLP books, partners' contribution, no planted errors".into();
    s
}

/// Clean HUF books: the Karta's capital account, no partners.
pub fn clean_huf() -> Scenario {
    let mut s = clean(Kind::Firm, 4, 6, 4, 2);
    s.engagement.entity_type = EntityType::Huf;
    s.engagement.entity_name = "Ramesh Kumar Sharma (HUF)".into();
    rename_ledgers(
        &mut s.engagement,
        &[
            ("Partner A - Capital", "HUF Capital Account"),
            ("Partner B - Capital", "HUF Corpus Account"),
            ("Partners' Remuneration", "Salary to Karta"),
            ("Interest on Partners' Capital", "Interest on Karta Capital"),
        ],
    );
    s.name = "clean_huf".into();
    s.description = "Clean HUF books, Karta's capital, no planted errors".into();
    s
}

/// A head office and one branch, each with its own books. The inter-branch
/// accounts ("Pune Branch A/c" in the head office, "Head Office A/c" in the
/// branch) must cancel out when the books are combined.
pub struct BranchBooks {
    pub name: String,
    pub description: String,
    pub head_office: Engagement,
    pub branches: Vec<(String, Engagement)>,
    /// Profit of the combined books (head office + branches).
    pub profit_cy: Money,
}

pub fn branch_books() -> BranchBooks {
    const SENT: i64 = 200_000;
    let mut b = Builder::new("Branch Traders (Partnership Firm)", EntityType::Firm, 2025);
    let mut rng = Rng::new(5);
    let mut sim = base_ledgers(&mut b, &mut rng, Kind::Firm, 8, 5);
    sim.ops_per_day = 3;
    b.far = Some(base_far(&b, vec![]));
    b.ledger("Pune Branch A/c", "Branch / Divisions", Money::ZERO);
    b.not_in_py("Pune Branch A/c");
    balance_capital(&mut b, Kind::Firm);
    let mut hook = |b: &mut Builder, dn: i64, ph: Phase| {
        if ph == Phase::Start && dn == 10 {
            let d = b.day(dn);
            b.v(
                d,
                "Payment",
                &[("Pune Branch A/c", r(SENT)), (BANK, r(-SENT))],
            );
        }
    };
    simulate(&mut b, &mut rng, &mut sim, &mut hook);
    top_up_bank(&mut b, Kind::Firm);
    let (ho, ho_exp) = b.finish();

    // The branch: started this year with funds from the head office.
    let mut br = Builder::new("Pune Branch", EntityType::Firm, 2025);
    br.with_py = false;
    br.ledger("Head Office A/c", "Branch / Divisions", Money::ZERO);
    br.ledger(PLAC, "Profit & Loss A/c", Money::ZERO);
    br.ledger("Cash", "Cash-in-Hand", Money::ZERO);
    br.ledger("SBI Pune Current A/c", "Bank Accounts", Money::ZERO);
    br.ledger(SALES, "Sales Accounts", Money::ZERO);
    br.ledger(PURCHASES, "Purchase Accounts", Money::ZERO);
    br.ledger("Rent", "Indirect Expenses", Money::ZERO);
    br.ledger("Salary", "Indirect Expenses", Money::ZERO);
    let bank = "SBI Pune Current A/c";
    let d = br.day(10);
    br.v(
        d,
        "Receipt",
        &[(bank, r(SENT)), ("Head Office A/c", r(-SENT))],
    );
    let mut rng = Rng::new(55);
    for m in 0..12 {
        let d = br.day(15 + m * 30);
        let s = rng.money(60_000, 90_000);
        post_unique(&mut br, d, "Sales", s, |a| two(bank, SALES, a));
        let p = rng.money(30_000, 50_000);
        post_unique(&mut br, d, "Purchase", p, |a| two(PURCHASES, bank, a));
        post_unique(&mut br, d, "Payment", r(8_000), |a| two("Rent", bank, a));
        post_unique(&mut br, d, "Payment", r(12_000), |a| two("Salary", bank, a));
    }
    let (branch, br_exp) = br.finish();
    BranchBooks {
        name: "branch_books".into(),
        description:
            "Head office and Pune branch: combine them; the inter-branch accounts cancel out".into(),
        head_office: ho,
        branches: vec![("Pune Branch".into(), branch)],
        profit_cy: ho_exp.profit_cy + br_exp.profit_cy,
    }
}
