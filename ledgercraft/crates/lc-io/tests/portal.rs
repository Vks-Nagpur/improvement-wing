//! GSTR-2B and Form 26AS as downloaded from the portals.

use lc_core::money::Money;
use lc_io::portal::{read_26as, read_gstr2b};

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("lc-portal-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn gstr2b_json_with_items_credit_note_and_itc_not_available() {
    let d = dir("2bjson");
    let p = d.join("2b.json");
    std::fs::write(&p, r#"{"data":{"gstin":"27AAAAA1111A1Z5","rtnprd":"032026","docdata":{
      "b2b":[
        {"ctin":"27BBBBB2222B1Z5","trdnm":"SHREE GANESH HARDWARE","inv":[
          {"inum":"SG/101","dt":"05-03-2026","val":11800,"itcavl":"Y","items":[{"txval":10000,"igst":0,"cgst":900,"sgst":900,"cess":0}]},
          {"inum":"SG/102","dt":"15-03-2026","val":5900,"itcavl":"N","items":[{"txval":5000,"cgst":450,"sgst":450}]}]},
        {"ctin":"29CCCCC3333C1Z5","trdnm":"OM SAI ENTERPRISES","inv":[
          {"inum":"7","dt":"10-03-2026","val":23600,"itcavl":"Y","txval":20000,"igst":3600,"cgst":0,"sgst":0,"cess":0}]}],
      "cdnr":[{"ctin":"27BBBBB2222B1Z5","trdnm":"SHREE GANESH HARDWARE","nt":[{"ntnum":"CN1","typ":"C","itcavl":"Y","items":[{"txval":1000,"cgst":90,"sgst":90}]}]}]
    }}}"#).unwrap();
    let v = read_gstr2b(&p).unwrap();
    let sg = v.iter().find(|x| x.id == "27BBBBB2222B1Z5").unwrap();
    assert_eq!(
        sg.tax,
        Money(180000 - 18000),
        "one invoice with ITC, less the credit note"
    );
    assert_eq!(sg.base, Money(1000000 - 100000));
    let om = v.iter().find(|x| x.name == "OM SAI ENTERPRISES").unwrap();
    assert_eq!(om.tax, Money(360000));
}

#[test]
fn gstr2b_excel_with_two_header_rows() {
    let d = dir("2bxlsx");
    let p = d.join("2b.xlsx");
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet().set_name("B2B").unwrap();
    ws.write_string(0, 0, "Goods and Services Tax - GSTR-2B")
        .unwrap();
    let h1 = [
        "GSTIN of supplier",
        "Trade/Legal name",
        "Invoice details",
        "",
        "",
        "",
        "Place of supply",
        "Supply Attract Reverse Charge",
        "Taxable Value (₹)",
        "Tax Amount",
        "",
        "",
        "",
        "GSTR-1/IFF/GSTR-5 Period",
        "GSTR-1/IFF/GSTR-5 Filing Date",
        "ITC Availability",
    ];
    let h2 = [
        "",
        "",
        "Invoice number",
        "Invoice type",
        "Invoice Date",
        "Invoice Value(₹)",
        "",
        "",
        "",
        "Integrated Tax(₹)",
        "Central Tax(₹)",
        "State/UT Tax(₹)",
        "Cess(₹)",
        "",
        "",
        "",
    ];
    for (c, t) in h1.iter().enumerate() {
        ws.write_string(4, c as u16, *t).unwrap();
    }
    for (c, t) in h2.iter().enumerate() {
        ws.write_string(5, c as u16, *t).unwrap();
    }
    let rows = [
        [
            "27BBBBB2222B1Z5",
            "SHREE GANESH HARDWARE",
            "SG/101",
            "Regular",
            "05/03/2026",
            "11800",
            "Maharashtra",
            "No",
            "10000",
            "0",
            "900",
            "900",
            "0",
            "Mar'26",
            "11/04/2026",
            "Yes",
        ],
        [
            "27BBBBB2222B1Z5",
            "SHREE GANESH HARDWARE",
            "SG/102",
            "Regular",
            "15/03/2026",
            "5900",
            "Maharashtra",
            "No",
            "5000",
            "0",
            "450",
            "450",
            "0",
            "Mar'26",
            "11/04/2026",
            "No",
        ],
    ];
    for (r, row) in rows.iter().enumerate() {
        for (c, t) in row.iter().enumerate() {
            ws.write_string(6 + r as u32, c as u16, *t).unwrap();
        }
    }
    wb.save(&p).unwrap();
    let v = read_gstr2b(&p).unwrap();
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].tax, Money(180000));
    assert_eq!(v[0].documents, 1);
}

#[test]
fn form_26as_text_file_part_one() {
    let d = dir("26as");
    let p = d.join("26as.txt");
    std::fs::write(&p, "File Creation Date^05-04-2026\nPermanent Account Number (PAN)^ABCPS1234D\n\nPART-I - Details of Tax Deducted at Source\nSr. No.^Name of Deductor^TAN of Deductor^^^^^Total Amount Paid / Credited(Rs.)^Total Tax Deducted(Rs.)^Total TDS Deposited(Rs.)\n1^OM SAI ENTERPRISES PRIVATE LIMITED^NGPO01234E^^^^^500000.00^10000.00^10000.00\n^Sr. No.^Section^Transaction Date^Status of Booking^Date of Booking^Remarks^Amount Paid / Credited(Rs.)^Tax Deducted(Rs.)^TDS Deposited(Rs.)\n^1^194C^31-Mar-2026^F^15-May-2026^-^500000.00^10000.00^10000.00\n2^PATIL AGRO INDUSTRIES^PNEP09876F^^^^^100000.00^2000.00^2000.00\nPART-II - Details of Tax Deducted at Source for 15G / 15H\nSr. No.^Name of Deductor^TAN of Deductor\n1^SOMEONE^MUMS11111A^^^^^1.00^0.00^0.00\n").unwrap();
    let v = read_26as(&p).unwrap();
    assert_eq!(v.len(), 2);
    let om = v.iter().find(|x| x.id == "NGPO01234E").unwrap();
    assert_eq!(om.name, "OM SAI ENTERPRISES PRIVATE LIMITED");
    assert_eq!(om.base, Money(50_000_000));
    assert_eq!(om.tax, Money(1_000_000));
}
