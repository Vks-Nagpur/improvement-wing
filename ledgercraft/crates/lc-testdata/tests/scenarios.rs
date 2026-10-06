//! Ground-truth tests: the engine must find exactly the planted problems.

use lc_core::rules::RulesPack;
use lc_core::{analyse, Analysis, Money};
use lc_testdata::scenarios::{self, Kind, Scenario};
use std::collections::BTreeSet;

fn run(s: &Scenario) -> Analysis {
    analyse(&s.engagement, &RulesPack::builtin())
}

/// The scenario after the user confirmed every placement.
fn confirmed(s: &Scenario) -> Analysis {
    let a = run(s);
    analyse(
        &lc_testdata::confirm_all(&s.engagement, &a),
        &RulesPack::builtin(),
    )
}

fn assert_exact(s: &Scenario, a: &Analysis) {
    let got: BTreeSet<String> = lc_testdata::problem_keys(a);
    let missing: Vec<_> = s.expected.keys.difference(&got).collect();
    let extra: Vec<_> = got.difference(&s.expected.keys).collect();
    if !missing.is_empty() || !extra.is_empty() {
        let detail: Vec<String> = a
            .findings
            .iter()
            .filter(|f| !s.expected.keys.contains(&f.key) && !f.code.starts_with("MAPPING_"))
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
    assert!(!a.printable, "unconfirmed placements block a signing copy");
    assert!(
        confirmed(&s).printable,
        "after confirming placements the clean books can be signed"
    );
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
    assert!(
        confirmed(&s).printable,
        "warnings alone must not block a signing copy"
    );
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

#[test]
fn placements_need_confirmation_and_reopen_when_the_books_change() {
    use lc_core::mapping::MapStatus;
    let s = scenarios::clean(Kind::Company, 11, 8, 6, 3);
    let a = run(&s);
    // Fresh books: certain groups are placed by rule, ambiguous ones wait for the user.
    assert!(a.mapping.iter().any(|m| m.status == MapStatus::Rule));
    let pending: Vec<_> = a
        .mapping
        .iter()
        .filter(|m| m.status == MapStatus::Suggested)
        .collect();
    assert!(!pending.is_empty());
    assert!(a.findings.iter().any(|f| f.code == "MAPPING_UNCONFIRMED"));
    // Every Suggested line is either from an ambiguous group, a name, or a side move.
    for m in &pending {
        assert!(!m.status_reason.is_empty());
    }
    // Confirmed: nothing pending, the year can be signed.
    let e = lc_testdata::confirm_all(&s.engagement, &a);
    let b = analyse(&e, &RulesPack::builtin());
    assert!(b.mapping.iter().all(|m| m.status == MapStatus::Confirmed));
    assert!(b.printable);
    // Next year the same ledger sits in another group: the earlier choice is re-opened.
    let mut e2 = e.clone();
    let target = e2
        .cy
        .ledgers
        .iter_mut()
        .find(|l| l.group == "Sundry Creditors")
        .unwrap();
    target.group = "Current Liabilities".into();
    let c = analyse(&e2, &RulesPack::builtin());
    let m = c
        .mapping
        .iter()
        .find(|m| m.group == "Current Liabilities" && m.status == MapStatus::Review)
        .unwrap();
    assert!(m.status_reason.contains("Group changed"));
    assert!(c.findings.iter().any(|f| f.code == "MAPPING_REVIEW"));
    assert!(!c.printable);
}

#[test]
fn clean_llp_and_huf_have_no_findings() {
    for s in [scenarios::clean_llp(), scenarios::clean_huf()] {
        let a = run(&s);
        assert_exact(&s, &a);
        assert_statements(&s, &a);
        assert!(
            confirmed(&s).printable,
            "{}: after confirming placements the books can be signed",
            s.name
        );
    }
}

#[test]
fn branch_books_combine_and_cancel_out() {
    use lc_core::consolidate::{merge, Unit};
    let bb = scenarios::branch_books();
    let mut units = vec![Unit {
        name: "Head office".into(),
        tb: bb.head_office.cy.clone(),
        vouchers: bb.head_office.vouchers.clone(),
    }];
    for (n, e) in &bb.branches {
        units.push(Unit {
            name: n.clone(),
            tb: e.cy.clone(),
            vouchers: e.vouchers.clone(),
        });
    }
    let (tb, vouchers, notes) = merge(&units);
    assert_eq!(notes.inter_branch_difference, Money::ZERO);
    assert_eq!(notes.eliminated.len(), 2, "{:?}", notes.eliminated);
    let mut eng = bb.head_office.clone();
    eng.cy = tb;
    eng.vouchers = vouchers;
    eng.consolidation = Some(notes);
    let a = analyse(&eng, &RulesPack::builtin());
    let other: Vec<_> = lc_testdata::problem_keys(&a).into_iter().collect();
    assert!(other.is_empty(), "unexpected findings: {other:?}");
    assert_eq!(a.statements.profit.0, bb.profit_cy);
    assert_eq!(a.statements.total_assets, a.statements.total_liabilities);
}

#[test]
fn company_managerial_remuneration_is_its_own_head_and_old_choices_reopen() {
    use lc_core::mapping::{Head, MapStatus};
    let mut s = scenarios::clean(Kind::Company, 2, 25, 15, 12);
    let key = lc_core::model::norm_name("Directors' Remuneration");
    // A choice remembered under the old shared head.
    s.engagement
        .mapping_memory
        .insert(key.clone(), "PARTNERS_REMUNERATION".into());
    s.engagement.mapping_context.insert(key, "x".into());
    let a = run(&s);
    let m = a
        .mapping
        .iter()
        .find(|m| m.name == "Directors' Remuneration")
        .unwrap();
    assert_eq!(m.head, Some(Head::ManagerialRemuneration));
    assert_eq!(m.status, MapStatus::Review, "{}", m.status_reason);
    assert_eq!(
        a.statements.profit.0, s.expected.profit_cy,
        "profit unchanged"
    );
    let line = a
        .statements
        .profit_loss
        .iter()
        .find(|r| r.label.contains("Managerial remuneration"))
        .unwrap();
    assert!(line.cy.unwrap_or_default().is_dr());
    // A firm never shows managerial remuneration.
    let mut f = scenarios::clean(Kind::Firm, 1, 10, 6, 4);
    f.engagement.mapping_memory.insert(
        lc_core::model::norm_name("Partners' Remuneration"),
        "MANAGERIAL_REMUNERATION".into(),
    );
    let a = run(&f);
    let m = a
        .mapping
        .iter()
        .find(|m| m.name == "Partners' Remuneration")
        .unwrap();
    assert_eq!(m.head, Some(Head::PartnersRemuneration));
    assert_eq!(m.status, MapStatus::Review);
}
