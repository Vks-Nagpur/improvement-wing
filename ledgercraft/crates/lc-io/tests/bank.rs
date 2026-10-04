//! Bank statements as banks export them, and the reconciliation built on them.

use lc_core::money::Money;
use lc_io::read::read_bank_statement;

fn tmp(name: &str, body: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("lc-bank-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join(format!("{name}.csv"));
    std::fs::write(&p, body).unwrap();
    p
}

#[test]
fn withdrawal_and_deposit_columns_with_account_details_above() {
    let p = tmp(
        "hdfc",
        "HDFC BANK Ltd.\nAccount No : 50100012345678\nStatement From : 01/03/2026 To : 31/03/2026\n\nDate,Narration,Chq./Ref.No.,Value Dt,Withdrawal Amt.,Deposit Amt.,Closing Balance\n02/03/26,NEFT CR-OM SAI ENTERPRISES,N123456,02/03/26,,\"50,000.00\",\"1,50,000.00\"\n10/03/26,CHQ PAID-654321-SHREE ROADLINES,0000654321,10/03/26,\"20,000.00\",,\"1,30,000.00\"\n25/03/26,CHARGES FOR SMS ALERTS,,25/03/26,17.70,,\"1,29,982.30\"\n,,,,,,\nTotal,,,,\"20,017.70\",\"50,000.00\",\n",
    );
    let lines = read_bank_statement(&p).unwrap();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].amount, Money(5_000_000));
    assert_eq!(lines[1].amount, Money(-2_000_000));
    assert_eq!(lines[1].reference, "0000654321");
    assert_eq!(lines[2].balance, Some(Money(12_998_230)));
}

#[test]
fn amount_with_dr_cr_column() {
    let p = tmp(
        "sbi",
        "Txn Date,Description,Ref No,Amount,Dr/Cr,Balance\n01-04-2025,BY TRANSFER,,1000.00,CR,1000.00 Cr\n05-04-2025,TO CASH,,250.00,DR,750.00 Cr\n",
    );
    let lines = read_bank_statement(&p).unwrap();
    assert_eq!(lines[0].amount, Money(100_000));
    assert_eq!(lines[1].amount, Money(-25_000));
    assert_eq!(lines[1].balance, Some(Money(75_000)));
}
