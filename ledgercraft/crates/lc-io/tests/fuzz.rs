//! Fuzz-style tests (J04): valid sample inputs are damaged at random (bytes
//! flipped, cut, repeated, special characters and huge numbers inserted)
//! and fed to every reader. A reader may refuse the input, but it must never
//! crash. Seeds are fixed so any failure repeats; a longer run is possible
//! with `LC_FUZZ_ROUNDS=20000`.

use lc_core::Money;
use lc_testdata::rng::Rng;
use std::path::{Path, PathBuf};

fn rounds() -> usize {
    std::env::var("LC_FUZZ_ROUNDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400)
}

const TOKENS: &[&str] = &[
    "<",
    ">",
    "&",
    "&#0;",
    "&#x1F;",
    "\"",
    "'",
    ",",
    "^",
    "\n",
    "\r\n",
    "\0",
    "-",
    "(",
    ")",
    "99999999999999999999",
    "-9223372036854775808",
    "1e308",
    "NaN",
    "₹",
    "١٢٣",
    "🧾",
    "=SUM(A1)",
    "</ENVELOPE>",
    "<A><A><A><A>",
    "{",
    "}",
    "[",
    "]",
    ":",
    "null",
];

fn mutate(rng: &mut Rng, src: &[u8]) -> Vec<u8> {
    let mut v = src.to_vec();
    for _ in 0..rng.range(1, 6) {
        let len = v.len().max(1) as i64;
        let at = rng.range(0, len) as usize;
        match rng.range(0, 6) {
            0 if !v.is_empty() => {
                let i = at.min(v.len() - 1);
                v[i] ^= 1 << rng.range(0, 7);
            }
            1 => {
                let t = rng.pick(TOKENS).as_bytes();
                let at = at.min(v.len());
                v.splice(at..at, t.iter().copied());
            }
            2 if !v.is_empty() => {
                let end = (at + rng.range(1, 40) as usize).min(v.len());
                v.drain(at.min(end)..end);
            }
            3 if !v.is_empty() => {
                let s = at.min(v.len() - 1);
                let end = (s + rng.range(1, 80) as usize).min(v.len());
                let piece = v[s..end].to_vec();
                for _ in 0..rng.range(1, 4) {
                    v.splice(end..end, piece.iter().copied());
                }
            }
            4 => v.truncate(at),
            _ => v.push(rng.range(0, 255) as u8),
        }
    }
    v
}

fn dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lc-fuzz-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run `f` on damaged copies of `sample`; any panic fails with the input saved.
fn fuzz_file(name: &str, ext: &str, sample: &[u8], seed: u64, f: impl Fn(&Path)) {
    let d = dir(name);
    let mut rng = Rng::new(seed);
    for i in 0..rounds() {
        let bytes = mutate(&mut rng, sample);
        let p = d.join(format!("input.{ext}"));
        std::fs::write(&p, &bytes).unwrap();
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&p))).is_err() {
            let keep = d.join(format!("crash-{i}.{ext}"));
            std::fs::write(&keep, &bytes).unwrap();
            panic!(
                "{name}: crashed on round {i}; input kept at {}",
                keep.display()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

const VOUCHER_XML: &str = r#"<ENVELOPE><BODY><DATA><COLLECTION>
<VOUCHER VCHTYPE="Payment"><DATE>20250415</DATE><VOUCHERNUMBER>12</VOUCHERNUMBER>
<VOUCHERTYPENAME>Payment</VOUCHERTYPENAME><NARRATION>Rent &amp; electricity</NARRATION>
<ALLLEDGERENTRIES.LIST><LEDGERNAME>Rent</LEDGERNAME><AMOUNT>-15000.00</AMOUNT></ALLLEDGERENTRIES.LIST>
<ALLLEDGERENTRIES.LIST><LEDGERNAME>Cash</LEDGERNAME><AMOUNT>15000.00</AMOUNT></ALLLEDGERENTRIES.LIST>
</VOUCHER></COLLECTION></DATA></BODY></ENVELOPE>"#;

#[test]
fn tally_xml_never_crashes() {
    let mut rng = Rng::new(1);
    for _ in 0..rounds() * 3 {
        let bytes = mutate(&mut rng, VOUCHER_XML.as_bytes());
        let r = std::panic::catch_unwind(|| {
            let text = lc_io::tally::sanitize(&bytes);
            if let Ok(root) = lc_io::tally::parse_xml(&text) {
                fn walk(n: &lc_io::tally::Node) {
                    if n.tag.eq_ignore_ascii_case("VOUCHER") {
                        let _ = lc_io::tally::parse_voucher(n);
                    }
                    n.children.iter().for_each(walk);
                }
                walk(&root);
            }
        });
        assert!(r.is_ok(), "crash on {:?}", String::from_utf8_lossy(&bytes));
    }
}

#[test]
fn amounts_in_text_never_crash() {
    let mut rng = Rng::new(2);
    for _ in 0..rounds() * 10 {
        let s = String::from_utf8_lossy(&mutate(&mut rng, b"-12,34,567.89 Dr")).to_string();
        let r = std::panic::catch_unwind(|| {
            let _ = Money::parse(&s);
            let _ = lc_io::tally::tally_amount(&s);
        });
        assert!(r.is_ok(), "crash on {s:?}");
    }
}

#[test]
fn csv_books_never_crash() {
    let s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 3, 2, 1);
    let d = dir("csv-src");
    lc_io::write_inputs::write_vouchers_csv(&s.engagement.vouchers[..12], &d.join("v.csv"))
        .unwrap();
    let sample = std::fs::read(d.join("v.csv")).unwrap();
    fuzz_file("vouchers", "csv", &sample, 3, |p| {
        let _ = lc_io::read::read_vouchers(p);
    });
    let tb = "Trial Balance\nParticulars,Group,Opening,Debit,Credit,Closing\nCash,Cash-in-Hand,1000.00,,,1000.00 Dr\nCapital,Capital Account,(1000.00),,,1000.00 Cr\n";
    fuzz_file("tb", "csv", tb.as_bytes(), 4, |p| {
        let _ = lc_io::read::read_trial_balance(p);
    });
    let bank = "Date,Narration,Chq./Ref.No.,Withdrawal Amt.,Deposit Amt.,Closing Balance\n01/04/2025,NEFT,123,,5000.00,5000.00\n02/04/2025,CHQ,124,1200.00,,3800.00\n";
    fuzz_file("bank", "csv", bank.as_bytes(), 5, |p| {
        let _ = lc_io::read::read_bank_statement(p);
    });
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn excel_files_never_crash() {
    let s = lc_testdata::scenarios::clean(lc_testdata::scenarios::Kind::Firm, 1, 3, 2, 1);
    let d = dir("xlsx-src");
    lc_io::write_inputs::write_trial_balance(&s.engagement.cy, &d.join("tb.xlsx")).unwrap();
    let sample = std::fs::read(d.join("tb.xlsx")).unwrap();
    fuzz_file("tb-xlsx", "xlsx", &sample, 6, |p| {
        let _ = lc_io::read::read_trial_balance(p);
    });
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn portal_files_never_crash() {
    let json = br#"{"data":{"gstin":"27AAAAA1111A1Z5","rtnprd":"032026","docdata":{"b2b":[{"ctin":"27BBBBB2222B1Z5","trdnm":"SHREE GANESH","inv":[{"inum":"SG/101","dt":"05-03-2026","val":11800,"itcavl":"Y","items":[{"txval":10000,"igst":0,"cgst":900,"sgst":900,"cess":0}]}]}]}}}"#;
    fuzz_file("2b", "json", json, 7, |p| {
        let _ = lc_io::portal::read_gstr2b(p);
    });
    let txt = "PART-I - Details of Tax Deducted at Source\nSr. No.^Name of Deductor^TAN of Deductor^^^^^Total Amount Paid / Credited(Rs.)^Total Tax Deducted(Rs.)^Total TDS Deposited(Rs.)\n1^OM SAI ENTERPRISES^NGPO01234E^^^^^500000.00^10000.00^10000.00\n";
    fuzz_file("26as", "txt", txt.as_bytes(), 8, |p| {
        let _ = lc_io::portal::read_26as(p);
    });
}
