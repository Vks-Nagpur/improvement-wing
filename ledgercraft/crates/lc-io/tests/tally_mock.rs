//! End-to-end: a mock Tally server serves practice books in Tally's XML
//! dialect; the importer must reproduce the books exactly and the engine must
//! find the same planted problems.

use chrono::NaiveDate;
use lc_core::groups::{Class, GroupResolver};
use lc_core::model::{Engagement, TrialBalance};
use lc_core::rules::RulesPack;
use lc_core::{analyse, Money};
use lc_io::tally::{import_year, parse_xml, TallyClient};
use std::collections::BTreeSet;
use std::thread;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Tally writes debit as negative.
fn t(m: Money) -> String {
    let v = -m.paise();
    format!(
        "{}{}.{:02}",
        if v < 0 { "-" } else { "" },
        v.unsigned_abs() / 100,
        v.unsigned_abs() % 100
    )
}

fn ledgers_xml(tb: &TrialBalance) -> String {
    let res = GroupResolver::new(tb);
    let mut x = String::new();
    for l in &tb.ledgers {
        let stock = res.resolve(&l.group).ok() == Some(Class::StockInHand);
        // Tally: a stock ledger opens at opening stock and closes at closing stock.
        let opening = if stock { l.closing } else { l.opening };
        let closing = if stock {
            l.closing_stock.unwrap_or(l.closing)
        } else {
            l.closing
        };
        x.push_str(&format!(
            "<LEDGER NAME=\"{}\" RESERVEDNAME=\"\"><PARENT TYPE=\"String\">{}</PARENT><OPENINGBALANCE TYPE=\"Amount\">{}</OPENINGBALANCE><CLOSINGBALANCE TYPE=\"Amount\">{}</CLOSINGBALANCE></LEDGER>",
            esc(&l.name),
            esc(&l.group),
            t(opening),
            t(closing)
        ));
    }
    x
}

fn serve(eng: Engagement) -> String {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let addr = format!("http://{}", server.server_addr().to_ip().unwrap());
    thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let mut body = String::new();
            std::io::Read::read_to_string(req.as_reader(), &mut body).unwrap();
            let root = parse_xml(&body).unwrap();
            let mut v = Vec::new();
            root.find_all("TYPE", &mut v);
            let object = v
                .iter()
                .map(|n| n.text.clone())
                .find(|t| t != "Collection")
                .unwrap_or_default();
            let mut f = Vec::new();
            root.find_all("SVFROMDATE", &mut f);
            let from = f
                .first()
                .and_then(|n| NaiveDate::parse_from_str(&n.text, "%Y%m%d").ok());
            let mut tt = Vec::new();
            root.find_all("SVTODATE", &mut tt);
            let to = tt
                .first()
                .and_then(|n| NaiveDate::parse_from_str(&n.text, "%Y%m%d").ok());
            let data = match object.as_str() {
                "Company" => "<COMPANY NAME=\"Glitchy Traders\"></COMPANY><COMPANY NAME=\"Other Co\"></COMPANY>".to_string(),
                "Group" => {
                    let mut x = String::from("<GROUP NAME=\"Sundry Debtors\"><PARENT>&#4; Current Assets</PARENT></GROUP><GROUP NAME=\"Capital Account\"><PARENT>&#4; Primary</PARENT></GROUP>");
                    for (g, p) in &eng.cy.groups {
                        x.push_str(&format!("<GROUP NAME=\"{}\"><PARENT>{}</PARENT></GROUP>", esc(g), esc(p)));
                    }
                    x
                }
                "Ledger" => {
                    if from == Some(eng.fy_start) {
                        ledgers_xml(&eng.cy)
                    } else {
                        ledgers_xml(eng.py.as_ref().unwrap())
                    }
                }
                "Voucher" => {
                    let (a, b) = (from.unwrap(), to.unwrap());
                    let mut x = String::new();
                    for v in eng.vouchers.iter().filter(|v| v.date >= a && v.date <= b) {
                        x.push_str(&format!(
                            "<VOUCHER VCHTYPE=\"{0}\"><DATE>{1}</DATE><VOUCHERTYPENAME>{0}</VOUCHERTYPENAME><VOUCHERNUMBER>{2}</VOUCHERNUMBER><NARRATION>{3}</NARRATION>",
                            esc(&v.vtype),
                            v.date.format("%Y%m%d"),
                            esc(&v.number),
                            esc(&v.narration)
                        ));
                        for l in &v.lines {
                            x.push_str(&format!(
                                "<ALLLEDGERENTRIES.LIST><LEDGERNAME>{}</LEDGERNAME><ISDEEMEDPOSITIVE>{}</ISDEEMEDPOSITIVE><AMOUNT>{}</AMOUNT></ALLLEDGERENTRIES.LIST>",
                                esc(&l.ledger),
                                if l.amount.is_dr() { "Yes" } else { "No" },
                                t(l.amount)
                            ));
                        }
                        x.push_str("</VOUCHER>");
                    }
                    x
                }
                _ => String::new(),
            };
            let resp = format!("<ENVELOPE><HEADER><VERSION>1</VERSION><STATUS>1</STATUS></HEADER><BODY><DESC></DESC><DATA><COLLECTION>{data}</COLLECTION></DATA></BODY></ENVELOPE>");
            req.respond(tiny_http::Response::from_string(resp)).unwrap();
        }
    });
    addr
}

#[test]
fn one_click_tally_import_reproduces_books() {
    let s = lc_testdata::scenarios::firm_with_glitches();
    let url = serve(s.engagement.clone());
    let client = TallyClient {
        url,
        ..Default::default()
    };
    assert_eq!(
        client.companies().unwrap(),
        vec!["Glitchy Traders", "Other Co"]
    );
    let mut months = 0;
    let (cy, py, vouchers) = import_year(
        &client,
        "Glitchy Traders",
        s.engagement.fy_start,
        s.engagement.fy_end,
        true,
        |_, _| months += 1,
    )
    .unwrap();
    assert_eq!(months, 12, "day book is fetched month by month");
    let orig = &s.engagement;
    assert_eq!(cy.ledgers.len(), orig.cy.ledgers.len());
    for (a, b) in cy.ledgers.iter().zip(&orig.cy.ledgers) {
        assert_eq!(
            (&a.name, &a.group, a.opening, a.closing, a.closing_stock),
            (&b.name, &b.group, b.opening, b.closing, b.closing_stock)
        );
    }
    assert_eq!(vouchers, orig.vouchers, "vouchers must round-trip exactly");

    // Tags (e.g. transporter) are LedgerCraft's own data, kept per client.
    let mut cy = cy;
    for l in cy.ledgers.iter_mut() {
        l.tags = orig
            .cy
            .ledgers
            .iter()
            .find(|o| o.name == l.name)
            .map(|o| o.tags.clone())
            .unwrap_or_default();
    }
    let eng = Engagement {
        cy,
        py,
        vouchers,
        ..orig.clone()
    };
    let a = analyse(&eng, &RulesPack::builtin());
    let got: BTreeSet<String> = a.findings.iter().map(|f| f.key.clone()).collect();
    assert_eq!(got, s.expected.keys);
    assert_eq!(a.statements.profit.0, s.expected.profit_cy);
}

#[test]
fn clear_message_when_tally_is_not_running() {
    let client = TallyClient {
        url: "http://127.0.0.1:1".into(),
        timeout: std::time::Duration::from_secs(2),
    };
    let e = client.companies().unwrap_err();
    assert!(e.contains("Cannot reach Tally"), "{e}");
}
