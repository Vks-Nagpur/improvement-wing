//! Property tests (J03): statements that must hold for many generated books,
//! not just the hand-picked ones. Seeds are fixed so a failure repeats.

use lc_core::adjust::{AdjLine, Adjustment};
use lc_core::rules::{RulesPack, Severity};
use lc_core::{analyse, Money};
use lc_io::export::{export, ExportOptions, Mode, SignOff};
use lc_testdata::rng::Rng;
use lc_testdata::scenarios::{self, Kind};

fn book(seed: u64) -> scenarios::Scenario {
    let mut rng = Rng::new(seed);
    let kind = if seed.is_multiple_of(2) {
        Kind::Company
    } else {
        Kind::Firm
    };
    scenarios::clean(
        kind,
        seed,
        rng.range(2, 12) as usize,
        rng.range(2, 8) as usize,
        rng.range(1, 4) as usize,
    )
}

#[test]
fn generated_books_balance_and_give_the_known_profit() {
    for seed in 1..=24 {
        let s = book(seed);
        let e = &s.engagement;
        let tb: Money = e.cy.ledgers.iter().map(|l| l.closing).sum();
        assert!(tb.is_zero(), "seed {seed}: trial balance off by {tb:?}");
        for v in &e.vouchers {
            let t: Money = v.lines.iter().map(|l| l.amount).sum();
            assert!(t.is_zero(), "seed {seed}: voucher {} off", v.key());
        }
        let a = analyse(e, &RulesPack::builtin());
        let st = &a.statements;
        assert_eq!(st.total_assets, st.total_liabilities, "seed {seed}");
        assert_eq!(st.profit.0, s.expected.profit_cy, "seed {seed}");
    }
}

#[test]
fn balanced_adjustments_keep_double_entry_and_unbalanced_ones_are_refused() {
    for seed in 1..=24 {
        let s = book(seed);
        let mut rng = Rng::new(seed * 7 + 1);
        let names: Vec<String> = s
            .engagement
            .cy
            .ledgers
            .iter()
            .map(|l| l.name.clone())
            .collect();
        let n = rng.range(2, 5) as usize;
        let mut lines: Vec<AdjLine> = (0..n - 1)
            .map(|_| AdjLine {
                ledger: rng.pick(&names).clone(),
                amount: Money(rng.range(-5_000_000, 5_000_000)),
                new_group: None,
            })
            .collect();
        let rest: Money = lines.iter().map(|l| l.amount).sum();
        lines.push(AdjLine {
            ledger: "Audit Adjustment Suspense".into(),
            amount: -rest,
            new_group: Some("Current Liabilities".into()),
        });
        let adj = Adjustment {
            id: 1,
            narration: "generated".into(),
            lines,
            active: true,
            ..Default::default()
        };
        let mut e = s.engagement.clone();
        lc_core::adjust::apply(&mut e, std::slice::from_ref(&adj))
            .unwrap_or_else(|err| panic!("seed {seed}: {err}"));
        let tb: Money = e.cy.ledgers.iter().map(|l| l.closing).sum();
        assert!(tb.is_zero(), "seed {seed}: adjustment broke double entry");
        let a = analyse(&e, &RulesPack::builtin());
        assert_eq!(
            a.statements.total_assets, a.statements.total_liabilities,
            "seed {seed}"
        );

        // One paisa out: refused, and the books are left as they were.
        let mut bad = adj.clone();
        bad.lines[0].amount += Money(1);
        let mut e2 = s.engagement.clone();
        assert!(
            lc_core::adjust::apply(&mut e2, &[bad]).is_err(),
            "seed {seed}"
        );
        assert_eq!(
            e2.cy, s.engagement.cy,
            "seed {seed}: refused entry changed the books"
        );
    }
}

#[test]
fn rounded_figures_always_add_up_to_the_rounded_total() {
    let mut rng = Rng::new(99);
    for _ in 0..2_000 {
        let g = *rng.pick(&[1_i64, 100, 100_000, 10_000_000, 1_000_000_000]);
        let n = rng.range(1, 15) as usize;
        let values: Vec<Money> = (0..n)
            .map(|_| Money(rng.range(-50_000_000_000, 50_000_000_000)))
            .collect();
        let sum: Money = values.iter().copied().sum();
        let target = lc_core::units::round_to(sum, g);
        let out = lc_core::rounding::round_to_target(&values, g, target);
        let got: Money = out.iter().copied().sum();
        assert_eq!(got, target, "g={g} values={values:?}");
        for (o, v) in out.iter().zip(&values) {
            assert_eq!(o.paise() % g, 0);
            assert!(
                (o.paise() - v.paise()).abs() <= g,
                "moved more than one step"
            );
        }
    }
}

#[test]
fn amounts_survive_text_round_trips() {
    let mut rng = Rng::new(5);
    for _ in 0..5_000 {
        let m = Money(rng.range(-1_000_000_000_000_000, 1_000_000_000_000_000));
        let rupees = format!("{}.{:02}", m.paise() / 100, (m.paise() % 100).abs());
        let rupees = if m.paise() < 0 && m.paise() > -100 {
            format!("-{rupees}")
        } else {
            rupees
        };
        assert_eq!(Money::parse(&rupees).unwrap(), m, "{rupees}");
        assert_eq!(
            Money::parse(&m.fmt_indian()).unwrap(),
            m,
            "{}",
            m.fmt_indian()
        );
    }
}

#[test]
fn books_survive_excel_round_trips() {
    let dir = std::env::temp_dir().join(format!("lc-prop-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for seed in 30..=36 {
        let s = book(seed);
        let p = dir.join(format!("tb{seed}.xlsx"));
        lc_io::write_inputs::write_trial_balance(&s.engagement.cy, &p).unwrap();
        let back = lc_io::read::read_trial_balance(&p).unwrap();
        for (a, b) in s.engagement.cy.ledgers.iter().zip(&back.ledgers) {
            assert_eq!(
                (&a.name, a.opening, a.closing),
                (&b.name, b.opening, b.closing),
                "seed {seed}"
            );
        }
        assert_eq!(s.engagement.cy.ledgers.len(), back.ledgers.len());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_final_copy_while_a_blocker_is_open() {
    let ready = lc_core::legal::Readiness {
        ready: true,
        applicable: 1,
        verified: 1,
        ..Default::default()
    };
    let signoff = SignOff {
        signatories: vec![lc_core::report::Signatory {
            name: "A".into(),
            ..Default::default()
        }],
        place: "Nagpur".into(),
        date: "2026-09-30".into(),
        ..Default::default()
    };
    let root = std::env::temp_dir().join(format!("lc-prop-final-{}", std::process::id()));
    let mut seen = 0;
    for s in scenarios::all() {
        let a = analyse(&s.engagement, &RulesPack::builtin());
        if a.count(Severity::Blocker) == 0 {
            continue;
        }
        seen += 1;
        let opt = ExportOptions {
            mode: Mode::Signing,
            legal: Some(ready.clone()),
            ..Default::default()
        };
        let err = export(&root, &s.engagement, &a, &signoff, &opt).unwrap_err();
        assert!(err.contains("Must fix"), "{}: {err}", s.name);
    }
    assert!(seen > 0, "no book with a blocker was tried");
    assert!(!root.exists() || std::fs::read_dir(&root).unwrap().next().is_none());
    let _ = std::fs::remove_dir_all(&root);
}
