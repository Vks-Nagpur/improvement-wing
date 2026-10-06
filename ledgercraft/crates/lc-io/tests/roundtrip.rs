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
    // Income-tax rates as the book basis need an explicit confirmation.
    let err = export(&root, &ok.engagement, &a, &signoff, &opt).unwrap_err();
    assert!(
        err.contains("Book depreciation is computed at income-tax rates"),
        "{err}"
    );
    opt.report.depreciation_basis_confirmed = true;
    // Accounting blockers closed is not enough: legal content must be verified by a person.
    let err = export(&root, &ok.engagement, &a, &signoff, &opt).unwrap_err();
    assert!(err.contains("readiness was not assessed"), "{err}");
    let items = lc_core::legal::applicable(
        &ok.engagement,
        &RulesPack::builtin(),
        &lc_core::statements::FormatPack::of(&ok.engagement),
        &lc_core::far::DepPack::builtin(),
        lc_core::legal::Scope {
            tax_audit: opt.tax_audit,
            depreciation: ok.engagement.far.is_some(),
        },
    );
    opt.legal = Some(lc_core::legal::readiness(items.clone(), &[]));
    let err = export(&root, &ok.engagement, &a, &signoff, &opt).unwrap_err();
    assert!(err.contains("legal content not verified (0 of"), "{err}");
    let recs: Vec<lc_core::legal::Verification> = items
        .iter()
        .map(|it| lc_core::legal::Verification {
            item_id: it.id.clone(),
            content_hash: it.hash.clone(),
            authority: "test".into(),
            document_title: "test".into(),
            provision: "test".into(),
            official_source: "test".into(),
            verified_on: "2026-10-07".into(),
            verified_by: "test".into(),
            ..Default::default()
        })
        .collect();
    opt.legal = Some(lc_core::legal::readiness(items, &recs));
    // Signing details must be complete.
    let err = export(&root, &ok.engagement, &a, &signoff, &opt).unwrap_err();
    assert!(
        err.contains("at least one signatory") && err.contains("date"),
        "{err}"
    );
    let signoff = SignOff {
        signatories: vec![lc_core::report::Signatory {
            name: "A Partner".into(),
            designation: "Partner".into(),
            ..Default::default()
        }],
        date: "07-10-2026".into(),
        ..signoff
    };
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
        "31(a)(b)(c) Loans",
        "31(ba)(bc) Cash receipts",
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
    let loans = calamine::Reader::worksheet_range(&mut book, "31(a)(b)(c) Loans").unwrap();
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

#[test]
fn form26_helper_never_uses_form_3cd_clause_numbers() {
    let root = tmp("taxaudit26");
    let mut s = lc_testdata::scenarios::firm_with_glitches();
    // The same books, a Tax Year under the Income-tax Act, 2025.
    let shift = |d: chrono::NaiveDate| d.with_year(d.year() + 1).unwrap();
    use chrono::Datelike;
    s.engagement.fy_start = shift(s.engagement.fy_start);
    s.engagement.fy_end = shift(s.engagement.fy_end);
    for v in s.engagement.vouchers.iter_mut() {
        v.date = shift(v.date);
    }
    let a = analyse(&s.engagement, &RulesPack::builtin());
    let ex = export(
        &root,
        &s.engagement,
        &a,
        &SignOff::default(),
        &ExportOptions::default(),
    )
    .unwrap();
    assert!(!ex.dir.join("Tax_Audit_Helper_Form_3CD.xlsx").exists());
    let path = ex.dir.join("Tax_Audit_Helper_Form_26.xlsx");
    let mut book: calamine::Xlsx<_> = calamine::open_workbook(&path).unwrap();
    let names = calamine::Reader::sheet_names(&book);
    for n in &names {
        assert!(
            !n.chars().next().unwrap().is_ascii_digit(),
            "clause-numbered sheet in Form 26 helper: {n}"
        );
    }
    let index = calamine::Reader::worksheet_range(&mut book, "Index").unwrap();
    let text: String = index
        .rows()
        .flat_map(|r| r.iter().map(|c| c.to_string()))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("not fully mapped") && !text.contains("Clause 21"),
        "{text}"
    );
    // Loan findings carry no Form 3CD wording in a Form 26 year.
    for f in a
        .findings
        .iter()
        .filter(|f| f.code.starts_with("LOAN_") || f.code.starts_with("CASH_"))
    {
        let all = format!(
            "{} {} {}",
            f.message,
            f.suggestion.clone().unwrap_or_default(),
            f.legal_ref
        );
        assert!(!all.contains("3CD") && !all.contains("1961"), "{all}");
    }
}

#[test]
fn hostile_names_never_become_spreadsheet_formulas() {
    let mut s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 10, 6, 4);
    let evil = "=HYPERLINK(\"http://x.invalid\",\"click\")";
    s.engagement.entity_name = "=1+1 Traders".into();
    let old = s.engagement.cy.ledgers[0].name.clone();
    for tb in std::iter::once(&mut s.engagement.cy).chain(s.engagement.py.iter_mut()) {
        for l in tb.ledgers.iter_mut().filter(|l| l.name == old) {
            l.name = evil.into();
        }
    }
    for v in s.engagement.vouchers.iter_mut() {
        v.narration = "@SUM(1+1)".into();
        for l in v.lines.iter_mut().filter(|l| l.ledger == old) {
            l.ledger = evil.into();
        }
    }
    let d = tmp("evil");
    // CSV: guarded on write, restored exactly on read.
    write_vouchers_csv(&s.engagement.vouchers, &d.join("v.csv")).unwrap();
    let raw = std::fs::read_to_string(d.join("v.csv")).unwrap();
    assert!(
        !raw.lines()
            .skip(1)
            .any(|l| l.split(',').any(|c| c.trim_matches('"').starts_with('='))),
        "unguarded formula in CSV"
    );
    assert_eq!(
        read_vouchers(&d.join("v.csv")).unwrap(),
        s.engagement.vouchers
    );
    // Excel outputs: text cells only, no formula anywhere.
    let a = analyse(&s.engagement, &RulesPack::builtin());
    let ex = export(
        &d,
        &s.engagement,
        &a,
        &SignOff::default(),
        &ExportOptions::default(),
    )
    .unwrap();
    for f in [
        "Financial_Statements.xlsx",
        "Auditor_Reference_Workbook.xlsx",
        "Tax_Audit_Helper_Form_3CD.xlsx",
    ] {
        let mut book: calamine::Xlsx<_> = calamine::open_workbook(ex.dir.join(f)).unwrap();
        for sheet in calamine::Reader::sheet_names(&book) {
            let fr = calamine::Reader::worksheet_formula(&mut book, &sheet).unwrap();
            assert!(
                fr.used_cells().all(|(_, _, v)| v.is_empty()),
                "{f}/{sheet} has a formula"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn preview_html_escapes_names() {
    let mut s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 10, 6, 4);
    s.engagement.entity_name = "<script>alert(1)</script> Traders".into();
    let n = s.engagement.cy.ledgers.len() - 1;
    let old = s.engagement.cy.ledgers[n].name.clone();
    s.engagement.cy.ledgers[n].name = format!("<img src=x onerror=alert(2)> {old}");
    let a = analyse(&s.engagement, &RulesPack::builtin());
    let rep = lc_io::export::report(
        &s.engagement,
        &a,
        &SignOff::default(),
        &ExportOptions::default(),
    );
    let html = lc_io::render::html::render(&rep);
    assert!(
        !html.contains("<script>alert") && !html.contains("<img src=x"),
        "unescaped name in preview"
    );
    assert!(html.contains("&lt;script&gt;") || html.contains("&lt;SCRIPT&gt;"));
}

fn zip_text(bytes: &[u8], prefix: &str) -> String {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut out = String::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        if f.name().starts_with(prefix) {
            f.read_to_string(&mut out).unwrap();
        }
    }
    out
}

#[test]
fn draft_mark_is_in_the_body_of_every_format_and_absent_from_final() {
    let s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Company, 3, 8, 5, 3);
    let a = analyse(&s.engagement, &RulesPack::builtin());
    let mut rep = lc_io::export::report(
        &s.engagement,
        &a,
        &SignOff::default(),
        &ExportOptions::default(),
    );
    rep.meta.draft = true;
    // A long note with "&" must not push the mark out of Excel's header.
    rep.meta.draft_note = format!("open items & questions {}", "x".repeat(400));
    let mark = "DRAFT – for discussion only";

    let pages = lc_io::render::pdf::page_texts(&rep).unwrap();
    assert!(pages.len() > 1);
    for (i, p) in pages.iter().enumerate() {
        assert!(p.contains("DRAFT"), "PDF page {} has no DRAFT mark", i + 1);
    }

    let html = lc_io::render::html::render(&rep);
    let body = &html[html.find("<body").unwrap()..];
    assert!(body.contains(mark), "HTML body has no visible draft text");

    let docx = lc_io::render::docx::render(&rep).unwrap();
    assert!(zip_text(&docx, "word/document.xml").contains(mark));
    assert!(zip_text(&docx, "word/header").contains("DRAFT"));

    let xlsx = lc_io::render::xlsx::render(&rep).unwrap();
    let mut book: calamine::Xlsx<_> =
        <calamine::Xlsx<_> as calamine::Reader<_>>::new(std::io::Cursor::new(xlsx.clone()))
            .unwrap();
    let sheets = calamine::Reader::sheet_names(&book);
    for sheet in &sheets {
        let r = calamine::Reader::worksheet_range(&mut book, sheet).unwrap();
        let first = r
            .get_value((0, 0))
            .map(|v| v.to_string())
            .unwrap_or_default();
        assert!(first.starts_with(mark), "{sheet}: first row is {first:?}");
    }
    let sheet_xml = zip_text(&xlsx, "xl/worksheets/sheet");
    assert_eq!(
        sheet_xml.matches("<oddHeader>").count(),
        sheets.len(),
        "every sheet keeps its print header"
    );
    assert!(sheet_xml.matches("DRAFT (").count() >= sheets.len());
    // Print setup: A4, fit to width, titles repeated, print area set.
    assert!(sheet_xml.contains("paperSize=\"9\"") && sheet_xml.contains("fitToHeight=\"0\""));
    let wb_xml = zip_text(&xlsx, "xl/workbook.xml");
    assert_eq!(wb_xml.matches("_xlnm.Print_Titles").count(), sheets.len());
    assert_eq!(wb_xml.matches("_xlnm.Print_Area").count(), sheets.len());

    // Final copy: no draft text anywhere.
    rep.meta.draft = false;
    rep.meta.draft_note.clear();
    for p in lc_io::render::pdf::page_texts(&rep).unwrap() {
        assert!(!p.contains("DRAFT"));
    }
    assert!(!lc_io::render::html::render(&rep).contains(mark));
    let docx = lc_io::render::docx::render(&rep).unwrap();
    assert!(!zip_text(&docx, "word/").contains("DRAFT"));
    let xlsx = lc_io::render::xlsx::render(&rep).unwrap();
    assert!(!zip_text(&xlsx, "xl/").contains("DRAFT"));
}
