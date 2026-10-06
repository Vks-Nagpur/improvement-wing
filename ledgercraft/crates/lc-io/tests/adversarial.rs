//! Hostile and broken input files: refused with a reason, never a crash or a
//! silent partial import.

use lc_io::read::{read_trial_balance, read_vouchers};
use std::io::Write;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lc-adv-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn zip_bomb_workbook_is_refused_before_opening() {
    let d = tmp("bomb");
    let p = d.join("tb.xlsx");
    let f = std::fs::File::create(&p).unwrap();
    let mut z = zip::ZipWriter::new(f);
    z.start_file(
        "xl/worksheets/sheet1.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    let chunk = vec![b'0'; 1 << 20];
    for _ in 0..80 {
        z.write_all(&chunk).unwrap();
    }
    z.finish().unwrap();
    let e = read_trial_balance(&p).unwrap_err();
    assert!(e.contains("refused") || e.contains("unpack"), "{e}");
}

#[test]
fn corrupted_and_fake_workbooks_give_a_reason() {
    let d = tmp("corrupt");
    for (name, bytes) in [
        ("a.xlsx", b"PK\x03\x04 not really a zip".to_vec()),
        ("b.xlsx", vec![0u8; 100]),
        ("c.xls", b"<html>nope</html>".to_vec()),
    ] {
        let p = d.join(name);
        std::fs::write(&p, bytes).unwrap();
        assert!(read_trial_balance(&p).is_err(), "{name}");
    }
}

#[test]
fn tally_xml_that_is_deep_malformed_or_cut_short_is_refused() {
    let deep = "<A>".repeat(5000) + &"</A>".repeat(5000);
    assert!(lc_io::tally::parse_xml(&deep)
        .unwrap_err()
        .contains("nested too deeply"));
    assert!(lc_io::tally::parse_xml("<ENVELOPE><BODY>").is_err());
    assert!(lc_io::tally::parse_xml("<ENVELOPE></BODY>").is_err());
    assert!(lc_io::tally::parse_xml("</A>").is_err());
    // An external entity is not fetched or expanded.
    let r = lc_io::tally::parse_xml(
        "<!DOCTYPE x [<!ENTITY e SYSTEM \"file:///etc/passwd\">]><A>&e;</A>",
    );
    assert!(r
        .map(|n| !n.children.iter().any(|c| c.text.contains("root:")))
        .unwrap_or(true));
}

#[test]
fn csv_edge_cases_never_panic_and_never_half_import() {
    let d = tmp("csv");
    let cases: &[(&str, &str)] = &[
        ("huge.csv", "Ledger,Group,Closing\nCash,Cash-in-Hand,99999999999999999999999\nCapital,Capital Account,-1\n"),
        ("dup.csv", "Ledger,Ledger,Group,Closing\nCash,Cash,Cash-in-Hand,10\nCapital,Capital,Capital Account,-10\n"),
        ("formula.csv", "Ledger,Group,Closing\n=cmd|' /C calc'!A0,Cash-in-Hand,10\nCapital,Capital Account,-10\n"),
        ("unicode.csv", "Ledger,Group,Closing\n\u{202E}hsaC\u{200B},Cash-in-Hand,10\nपूंजी खाता,Capital Account,-10\n"),
        ("noheader.csv", "a,b,c\n1,2,3\n"),
        ("empty.csv", ""),
    ];
    for (name, text) in cases {
        let p = d.join(name);
        std::fs::write(&p, text).unwrap();
        let r = std::panic::catch_unwind(|| read_trial_balance(&p));
        assert!(r.is_ok(), "{name} panicked");
        if let Ok(Ok(tb)) = r {
            // Accepted files carry every ledger row, nothing half-read.
            assert_eq!(tb.ledgers.len(), 2, "{name}");
        }
    }
    let p = d.join("v.csv");
    std::fs::write(
        &p,
        "Date,Voucher No,Ledger,Debit,Credit\n31-02-2026,1,Cash,10,\n",
    )
    .unwrap();
    assert!(
        read_vouchers(&p).unwrap_err().contains("row"),
        "bad date must name the row"
    );
}

#[test]
fn skipped_rows_are_reported_with_reasons() {
    let s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 10, 6, 4);
    let d = tmp("diag");
    let p = d.join("messy.xlsx");
    lc_io::write_inputs::write_trial_balance_messy(
        &s.engagement.cy,
        "Clean Traders",
        "31-03-2026",
        &p,
    )
    .unwrap();
    let (r, rep) = lc_io::diag::collect(|| read_trial_balance(&p));
    let tb = r.unwrap();
    assert_eq!(rep.accepted, tb.ledgers.len());
    assert!(rep.skipped.iter().any(|x| x.reason.contains("title line")));
    assert!(rep.skipped.iter().any(|x| x.reason.contains("total row")));
}
