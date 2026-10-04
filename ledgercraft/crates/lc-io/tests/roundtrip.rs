//! Books written to Excel/CSV and read back must give exactly the same answers.

use lc_core::rules::RulesPack;
use lc_core::{analyse, Engagement};
use lc_io::export::{export, ExportOptions, Mode, SignOff};
use lc_io::read::{read_trial_balance, read_vouchers};
use lc_io::write_inputs::{write_far, write_trial_balance, write_vouchers_csv};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ledgercraft-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn every_scenario_survives_excel_and_csv() {
    for s in lc_testdata::scenarios::all() {
        let d = tmp(&s.name);
        let e = &s.engagement;
        write_trial_balance(&e.cy, &d.join("tb.xlsx")).unwrap();
        write_trial_balance(e.py.as_ref().unwrap(), &d.join("py.xlsx")).unwrap();
        write_vouchers_csv(&e.vouchers, &d.join("v.csv")).unwrap();

        let back = Engagement {
            cy: read_trial_balance(&d.join("tb.xlsx")).unwrap(),
            py: Some(read_trial_balance(&d.join("py.xlsx")).unwrap()),
            vouchers: read_vouchers(&d.join("v.csv")).unwrap(),
            ..e.clone()
        };
        assert_eq!(back.cy, e.cy, "{}: TB changed in round trip", s.name);
        assert_eq!(
            back.vouchers, e.vouchers,
            "{}: vouchers changed in round trip",
            s.name
        );
        if let Some(far) = &e.far {
            write_far(far, &d.join("far.xlsx")).unwrap();
            let r = lc_io::read::read_far(&d.join("far.xlsx"), far.basis).unwrap();
            assert_eq!(
                &r, far,
                "{}: fixed asset register changed in round trip",
                s.name
            );
        }

        let a = analyse(&back, &RulesPack::builtin());
        let got: BTreeSet<String> = lc_testdata::problem_keys(&a);
        assert_eq!(
            got, s.expected.keys,
            "{}: findings after round trip",
            s.name
        );
        assert_eq!(a.statements.profit.0, s.expected.profit_cy, "{}", s.name);
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn export_is_versioned_and_signing_copy_needs_no_blockers() {
    let root = tmp("export");
    let mut ok = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 3, 6, 4, 2);
    let pending = analyse(&ok.engagement, &RulesPack::builtin());
    ok.engagement = lc_testdata::confirm_all(&ok.engagement, &pending);
    let a = analyse(&ok.engagement, &RulesPack::builtin());
    let signoff = SignOff {
        udin: "26123456ABCDEF1234".into(),
        place: "Nagpur".into(),
        ..Default::default()
    };
    let mut opt = ExportOptions {
        mode: Mode::Signing,
        ..Default::default()
    };
    // Unanswered disclosures refuse a final copy; they are never assumed nil.
    let err = export(&root, &ok.engagement, &a, &signoff, &opt).unwrap_err();
    assert!(err.contains("Not answered yet: Related parties"), "{err}");
    for k in ["contingent", "related_parties", "msme"] {
        opt.report
            .disclosures
            .answers
            .insert(k.into(), "nil".into());
    }
    let d1 = export(&root, &ok.engagement, &a, &signoff, &opt)
        .unwrap()
        .dir;
    // Word output: a valid package carrying the statements.
    let docx = std::fs::read(d1.join("Financial_Statements.docx")).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(docx)).unwrap();
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
    assert!(xml.contains("Balance Sheet as at") && xml.contains("w:tbl"));
    let d2 = export(&root, &ok.engagement, &a, &signoff, &opt)
        .unwrap()
        .dir;
    assert_ne!(d1, d2, "second export must not overwrite the first");
    for f in [
        "Financial_Statements.pdf",
        "Financial_Statements.xlsx",
        "Financial_Statements_preview.html",
        "Auditor_Reference_Workbook.xlsx",
        "export-manifest.json",
    ] {
        assert!(d1.join(f).exists(), "{f} missing");
    }
    let html = std::fs::read_to_string(d1.join("Financial_Statements_preview.html")).unwrap();
    assert!(html.contains("UDIN: 26123456ABCDEF1234") && !html.contains("class=\"boxed draft\""));

    let bad = lc_testdata::scenarios::excel_import_errors();
    let ab = analyse(&bad.engagement, &RulesPack::builtin());
    assert!(
        export(&root, &bad.engagement, &ab, &signoff, &opt).is_err(),
        "signing copy must be refused with blockers"
    );
    let draft = export(
        &root,
        &bad.engagement,
        &ab,
        &signoff,
        &ExportOptions::default(),
    )
    .unwrap()
    .dir;
    assert!(
        std::fs::read_to_string(draft.join("Financial_Statements_preview.html"))
            .unwrap()
            .contains("boxed draft")
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tax_audit_helper_lists_the_planted_cases() {
    let root = tmp("taxaudit");
    let s = lc_testdata::scenarios::firm_with_glitches();
    let a = analyse(&s.engagement, &RulesPack::builtin());
    let ex = export(
        &root,
        &s.engagement,
        &a,
        &SignOff::default(),
        &ExportOptions::default(),
    )
    .unwrap();
    let path = ex.dir.join("Tax_Audit_Helper_Form_3CD.xlsx");
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(&path).unwrap();
    let names = calamine::Reader::sheet_names(&book);
    for want in [
        "Index",
        "18 Depreciation",
        "21(d) Cash payments",
        "26 Statutory dues",
        "31 Loans",
        "31(ba) Cash receipts",
        "34 TDS check",
        "40 Ratios",
        "44 GST break-up",
    ] {
        assert!(names.contains(&want.to_string()), "missing sheet {want}");
    }
    let cash = calamine::Reader::worksheet_range(&mut book, "21(d) Cash payments").unwrap();
    let n = a
        .findings
        .iter()
        .filter(|f| f.code == "CASH_PAYMENT_LIMIT" || f.code == "CASH_ASSET_PURCHASE")
        .count();
    assert_eq!(
        cash.rows()
            .skip(4)
            .filter(|r| !r[0].to_string().is_empty())
            .count(),
        n
    );
    let loans = calamine::Reader::worksheet_range(&mut book, "31 Loans").unwrap();
    assert!(loans.rows().any(|r| r[0].to_string().contains("Loan from")));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn messy_hand_made_trial_balance_is_read() {
    let s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 10, 6, 4);
    let d = tmp("messy");
    let p = d.join("messy.xlsx");
    lc_io::write_inputs::write_trial_balance_messy(
        &s.engagement.cy,
        "Clean Traders",
        "31-03-2026",
        &p,
    )
    .unwrap();
    let tb = read_trial_balance(&p).unwrap();
    let want: Vec<(String, String, lc_core::Money)> = s
        .engagement
        .cy
        .ledgers
        .iter()
        .map(|l| (l.name.clone(), l.group.clone(), l.closing))
        .collect();
    let got: Vec<(String, String, lc_core::Money)> = tb
        .ledgers
        .iter()
        .map(|l| (l.name.clone(), l.group.clone(), l.closing))
        .collect();
    assert_eq!(got, want);
    let _ = std::fs::remove_dir_all(&d);
}
