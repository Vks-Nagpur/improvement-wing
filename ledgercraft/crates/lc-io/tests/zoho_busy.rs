//! Zoho Books and BUSY: their export layouts and the Zoho API client.

use lc_core::Money;
use lc_io::read::read_trial_balance_with;
use lc_io::zoho::{apply_account_types, ZohoClient};
use rust_xlsxwriter::Workbook;
use std::thread;

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("lc-zb-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sheet(path: &std::path::Path, name: &str, rows: &[&[&str]]) {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet().set_name(name).unwrap();
    for (r, row) in rows.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            if let Ok(n) = v.replace(',', "").parse::<f64>() {
                ws.write_number(r as u32, c as u16, n).unwrap();
            } else if !v.is_empty() {
                ws.write_string(r as u32, c as u16, *v).unwrap();
            }
        }
    }
    wb.save(path).unwrap();
}

#[test]
fn zoho_trial_balance_export_with_sections_and_types() {
    let d = tmp("zoho");
    let p = d.join("zoho_tb.xlsx");
    sheet(
        &p,
        "Trial Balance",
        &[
            &["ABC Traders"],
            &["Trial Balance", "", ""],
            &["Account", "Account Code", "Net Debit", "Net Credit"],
            &["Accounts Receivable", "", "", ""],
            &["Om Sai Enterprises", "1001", "1,20,000.00", ""],
            &["Bank", "", "", ""],
            &["HDFC Bank", "1100", "50000", ""],
            &["Equity", "", "", ""],
            &["Owner's Capital", "3000", "", "200000"],
            &["Income", "", "", ""],
            &["Sales", "4000", "", "70000.50"],
            &["Expense", "", "", ""],
            &["Rent Expense", "5000", "100000.50", ""],
            &["Total", "", "270000.50", "270000.50"],
        ],
    );
    let tb = read_trial_balance_with(&p, None).unwrap();
    let g: Vec<(&str, &str, Money)> = tb
        .ledgers
        .iter()
        .map(|l| (l.name.as_str(), l.group.as_str(), l.closing))
        .collect();
    assert_eq!(
        g,
        vec![
            (
                "Om Sai Enterprises",
                "Sundry Debtors",
                Money::rupees(120_000)
            ),
            ("HDFC Bank", "Bank Accounts", Money::rupees(50_000)),
            (
                "Owner's Capital",
                "Capital Account",
                Money::rupees(-200_000)
            ),
            ("Sales", "Sales Accounts", Money(-7_000_050)),
            ("Rent Expense", "Indirect Expenses", Money(10_000_050)),
        ]
    );
    assert!(tb
        .ledgers
        .iter()
        .map(|l| l.closing)
        .sum::<Money>()
        .is_zero());
}

#[test]
fn busy_trial_balance_with_account_master() {
    let d = tmp("busy");
    let tbp = d.join("busy_tb.xlsx");
    let mp = d.join("busy_accounts.xlsx");
    sheet(
        &tbp,
        "Sheet1",
        &[
            &["Trial Balance as on 31-03-2026"],
            &["Particulars", "Dr. Amount", "Cr. Amount"],
            &["Cash", "15000", ""],
            &["Ramesh & Sons", "", "15000"],
            &["Grand Total", "15000", "15000"],
        ],
    );
    sheet(
        &mp,
        "List of Accounts",
        &[
            &["Account Name", "Group"],
            &["Cash", "Cash-in-Hand"],
            &["Ramesh & Sons", "Sundry Creditors"],
        ],
    );
    let tb = read_trial_balance_with(&tbp, Some(&mp)).unwrap();
    assert_eq!(tb.ledgers.len(), 2);
    assert_eq!(
        (tb.ledgers[1].group.as_str(), tb.ledgers[1].closing),
        ("Sundry Creditors", Money::rupees(-15_000))
    );
}

#[test]
fn zoho_api_client_refreshes_token_and_pages() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let base = format!("http://{}", server.server_addr().to_ip().unwrap());
    thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let url = req.url().to_string();
            let mut body = String::new();
            std::io::Read::read_to_string(req.as_reader(), &mut body).unwrap();
            let auth = req
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.to_string())
                .unwrap_or_default();
            let reply = if url.starts_with("/oauth/v2/token") {
                assert!(
                    body.contains("grant_type=refresh_token") && body.contains("refresh_token=RT")
                );
                r#"{"access_token":"AT1","expires_in":3600}"#.to_string()
            } else if url.contains("page=1") {
                assert_eq!(auth, "Zoho-oauthtoken AT1");
                assert!(url.contains("organization_id=ORG"));
                r#"{"code":0,"chartofaccounts":[{"account_name":"Om Sai Enterprises","account_type":"accounts_receivable"}],"page_context":{"has_more_page":true}}"#.to_string()
            } else {
                r#"{"code":0,"chartofaccounts":[{"account_name":"GST Output","account_type":"output_tax"}],"page_context":{"has_more_page":false}}"#.to_string()
            };
            req.respond(tiny_http::Response::from_string(reply))
                .unwrap();
        }
    });
    let c = ZohoClient {
        accounts_url: base.clone(),
        api_url: base,
        client_id: "CID".into(),
        client_secret: "SEC".into(),
        refresh_token: "RT".into(),
        organization_id: "ORG".into(),
    };
    let types = c.chart_of_accounts().unwrap();
    assert_eq!(types.len(), 2);
    let mut tb = lc_core::model::TrialBalance {
        ledgers: vec![lc_core::model::Ledger {
            name: "GST Output".into(),
            group: "Other".into(),
            opening: Money::ZERO,
            closing: Money::ZERO,
            closing_stock: None,
            tags: vec![],
        }],
        groups: vec![],
    };
    apply_account_types(&mut tb, &types);
    assert_eq!(tb.ledgers[0].group, "Duties & Taxes");
}
