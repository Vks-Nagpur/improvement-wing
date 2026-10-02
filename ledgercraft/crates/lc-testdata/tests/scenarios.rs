//! Ground-truth tests: the engine must find exactly the planted problems.

use lc_core::rules::RulesPack;
use lc_core::{analyse, Analysis, Money};
use lc_testdata::scenarios::{self, Kind, Scenario};
use std::collections::BTreeSet;

fn run(s: &Scenario) -> Analysis {
    analyse(&s.engagement, &RulesPack::builtin())
}

fn assert_exact(s: &Scenario, a: &Analysis) {
    let got: BTreeSet<String> = a.findings.iter().map(|f| f.key.clone()).collect();
    let missing: Vec<_> = s.expected.keys.difference(&got).collect();
    let extra: Vec<_> = got.difference(&s.expected.keys).collect();
    if !missing.is_empty() || !extra.is_empty() {
        let detail: Vec<String> = a
            .findings
            .iter()
            .filter(|f| !s.expected.keys.contains(&f.key))
            .map(|f| format!("  {} | {}", f.key, f.message))
            .collect();
        panic!(
            "scenario {}:\n missing (not detected): {missing:#?}\n extra (false alarms): {extra:#?}\n extra detail:\n{}",
            s.name,
            detail.join("\n")
        );
    }
}

fn assert_statements(s: &Scenario, a: &Analysis) {
    let st = &a.statements;
    // Presentation: in clean books no face line may be negative, and the
    // numbered subtotal labels of the P&L must all be present.
    if s.expected.keys.is_empty() {
        for r in &st.balance_sheet {
            if r.kind == lc_core::statements::RowKind::Item {
                assert!(
                    !r.cy.unwrap_or_default().is_cr() && !r.py.unwrap_or_default().is_cr(),
                    "{}: negative line '{}' {:?} {:?}",
                    s.name,
                    r.label,
                    r.cy,
                    r.py
                );
            }
        }
    }
    let romans: Vec<&str> = st
        .profit_loss
        .iter()
        .filter_map(|r| r.label.split_once(". ").map(|x| x.0))
        .filter(|p| p.chars().all(|c| "IVX".contains(c)))
        .collect();
    let want = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"];
    assert_eq!(
        romans,
        want[..romans.len()].to_vec(),
        "{}: P&L numbering has gaps",
        s.name
    );
    assert_eq!(
        st.total_assets, st.total_liabilities,
        "{}: balance sheet does not tally",
        s.name
    );
    assert_eq!(
        st.profit.0, s.expected.profit_cy,
        "{}: current-year profit",
        s.name
    );
    assert_eq!(
        st.profit.1, s.expected.profit_py,
        "{}: previous-year profit",
        s.name
    );
    // Every note total equals the face figure it supports.
    for row in st.balance_sheet.iter().chain(&st.profit_loss) {
        if let Some(no) = row.note {
            let n = &st.notes[(no - 1) as usize];
            assert_eq!(
                Some(n.total_cy),
                row.cy,
                "{}: note {} total vs face",
                s.name,
                n.no
            );
            let lines: Money = n.lines.iter().map(|l| l.cy).sum();
            assert_eq!(
                lines, n.total_cy,
                "{}: note {} lines do not add up",
                s.name, n.no
            );
        }
    }
}

#[test]
fn clean_firm_has_no_findings() {
    let s = scenarios::clean(Kind::Firm, 1, 10, 6, 4);
    let a = run(&s);
    assert_exact(&s, &a);
    assert_statements(&s, &a);
    assert!(a.printable);
}

#[test]
fn clean_company_has_no_findings() {
    let s = scenarios::clean(Kind::Company, 2, 25, 15, 12);
    let a = run(&s);
    assert_exact(&s, &a);
    assert_statements(&s, &a);
    let labels: Vec<&str> = a
        .statements
        .balance_sheet
        .iter()
        .map(|r| r.label.as_str())
        .collect();
    for want in [
        "(a) Share capital",
        "(ii) Intangible assets",
        "(iii) Capital work-in-progress",
        "(b) Non-current investments",
        "(d) Short-term provisions",
    ] {
        assert!(labels.contains(&want), "missing line {want}: {labels:?}");
    }
}

#[test]
fn many_random_clean_books_have_no_false_alarms() {
    for seed in 100..130 {
        for kind in [Kind::Firm, Kind::Company] {
            let s = scenarios::clean(kind, seed, 8, 5, 3);
            let a = run(&s);
            assert_exact(&s, &a);
            assert_statements(&s, &a);
        }
    }
}

#[test]
fn every_planted_glitch_is_found() {
    let s = scenarios::firm_with_glitches();
    let a = run(&s);
    assert_exact(&s, &a);
    assert_statements(&s, &a);
    assert!(a.printable, "warnings alone must not block a signing copy");
}

#[test]
fn loan_register_counts_only_principal_as_accepted() {
    let s = scenarios::firm_with_glitches();
    let a = run(&s);
    let ravi = a
        .loans
        .iter()
        .find(|l| l.ledger == "Loan from Ravi Mehta")
        .expect("Ravi in loan register");
    assert_eq!(ravi.opening, Money::rupees(18_000));
    assert_eq!(ravi.interest_credited, Money::rupees(5_000));
    assert_eq!(ravi.principal_accepted(), Money::rupees(1_500));
    assert_eq!(ravi.repaid_cash, Money::rupees(2_000));
    assert_eq!(ravi.max_outstanding, Money::rupees(24_500));
    assert_eq!(ravi.closing, Money::rupees(22_500));

    let rajesh = a
        .loans
        .iter()
        .find(|l| l.ledger == "Unsecured Loan - Rajesh Kumar")
        .unwrap();
    assert!(
        rajesh.principal_accepted().is_zero(),
        "monthly interest must not be treated as loan accepted"
    );
    assert_eq!(rajesh.interest_credited, Money::rupees(2_700 * 12));

    let suresh = a
        .loans
        .iter()
        .find(|l| l.ledger == "Loan from Suresh Kumar")
        .expect("mis-grouped loan still in register");
    assert_eq!(suresh.accepted_bank, Money::rupees(50_000));
    assert_eq!(suresh.repaid_cash, Money::rupees(25_000));
}

#[test]
fn excel_import_errors_are_found() {
    let s = scenarios::excel_import_errors();
    let a = run(&s);
    assert_exact(&s, &a);
    assert!(!a.printable, "blockers must prevent a signing copy");
}

#[test]
fn findings_carry_legal_reference_by_year() {
    let s = scenarios::firm_with_glitches();
    let a = run(&s);
    let f = a
        .findings
        .iter()
        .find(|f| f.code == "LOAN_ACCEPTED_CASH")
        .unwrap();
    assert!(
        f.legal_ref.contains("269SS"),
        "FY 2025-26 uses the 1961 Act reference: {}",
        f.legal_ref
    );

    let mut e = s.engagement.clone();
    e.fy_start = chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap();
    e.fy_end = chrono::NaiveDate::from_ymd_opt(2027, 3, 31).unwrap();
    let a2 = analyse(&e, &RulesPack::builtin());
    let f2 = a2
        .findings
        .iter()
        .find(|f| f.code == "LOAN_ACCEPTED_CASH")
        .unwrap();
    assert!(
        f2.legal_ref.contains("2025"),
        "FY 2026-27 uses the 2025 Act reference: {}",
        f2.legal_ref
    );
}
