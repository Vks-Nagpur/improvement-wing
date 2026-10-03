//! One-click import from TallyPrime / Tally.ERP 9 over its XML-over-HTTP
//! interface (default port 9000; enable it in Tally: F1 Help > Settings >
//! Connectivity / Advanced Configuration).
//!
//! Tally conventions handled here:
//! * amounts: debit is negative, credit positive (inverted to LedgerCraft's Dr +);
//! * dates: YYYYMMDD; reserved parent "Primary" may arrive as "&#4; Primary";
//! * stock-in-hand ledgers report closing stock as their closing balance – it is
//!   moved to `closing_stock` so the trial balance still adds to zero.

use chrono::{Datelike, Months, NaiveDate};
use lc_core::groups::{Class, GroupResolver};
use lc_core::model::{Ledger, TrialBalance, Voucher, VoucherLine};
use lc_core::Money;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TallyClient {
    pub url: String,
    pub timeout: Duration,
}

impl Default for TallyClient {
    fn default() -> Self {
        TallyClient {
            url: "http://localhost:9000".into(),
            timeout: Duration::from_secs(120),
        }
    }
}

/// A minimal XML element tree.
#[derive(Debug, Default, Clone)]
pub struct Node {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub text: String,
    pub children: Vec<Node>,
}

impl Node {
    pub fn attr(&self, k: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(a, _)| a.eq_ignore_ascii_case(k))
            .map(|(_, v)| v.as_str())
    }
    pub fn child(&self, tag: &str) -> Option<&Node> {
        self.children
            .iter()
            .find(|c| c.tag.eq_ignore_ascii_case(tag))
    }
    pub fn child_text(&self, tag: &str) -> String {
        self.child(tag)
            .map(|c| c.text.trim().to_string())
            .unwrap_or_default()
    }
    /// All descendants with this tag (depth-first).
    pub fn find_all<'a>(&'a self, tag: &str, out: &mut Vec<&'a Node>) {
        for c in &self.children {
            if c.tag.eq_ignore_ascii_case(tag) {
                out.push(c);
            } else {
                c.find_all(tag, out);
            }
        }
    }
}

/// Decode Tally's response (UTF-8 or UTF-16) and remove the control-character
/// references Tally emits (e.g. `&#4;`) that are not valid XML 1.0.
pub fn sanitize(bytes: &[u8]) -> String {
    let text = if bytes.len() >= 2
        && ((bytes[0] == 0xFF && bytes[1] == 0xFE)
            || (bytes.len() > 3 && bytes[1] == 0 && bytes[3] == 0))
    {
        let start = if bytes[0] == 0xFF { 2 } else { 0 };
        let u16s: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(p) = rest.find("&#") {
        out.push_str(&rest[..p]);
        let tail = &rest[p + 2..];
        match tail.find(';') {
            Some(e) if e <= 6 => {
                let code = &tail[..e];
                let n = if let Some(h) = code.strip_prefix('x') {
                    u32::from_str_radix(h, 16).ok()
                } else {
                    code.parse::<u32>().ok()
                };
                match n {
                    Some(n) if n < 32 && n != 9 && n != 10 && n != 13 => {}
                    _ => out.push_str(&rest[p..p + 3 + e]),
                }
                rest = &tail[e + 1..];
            }
            _ => {
                out.push_str("&#");
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .collect()
}

pub fn parse_xml(xml: &str) -> Result<Node, String> {
    let mut r = Reader::from_str(xml);
    r.config_mut().trim_text(true);
    let mut stack: Vec<Node> = vec![Node {
        tag: "#root".into(),
        ..Default::default()
    }];
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => {
                let mut n = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    let v = a
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    n.attrs
                        .push((String::from_utf8_lossy(a.key.as_ref()).into_owned(), v));
                }
                stack.push(n);
            }
            Ok(Event::Empty(e)) => {
                let mut n = Node {
                    tag: String::from_utf8_lossy(e.name().as_ref()).into_owned(),
                    ..Default::default()
                };
                for a in e.attributes().flatten() {
                    let v = a
                        .unescape_value()
                        .map(|v| v.into_owned())
                        .unwrap_or_default();
                    n.attrs
                        .push((String::from_utf8_lossy(a.key.as_ref()).into_owned(), v));
                }
                stack.last_mut().unwrap().children.push(n);
            }
            Ok(Event::Text(t)) => {
                let s = t
                    .unescape()
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&t).into_owned());
                stack.last_mut().unwrap().text.push_str(&s);
            }
            Ok(Event::CData(t)) => stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&String::from_utf8_lossy(&t)),
            Ok(Event::End(_)) => {
                let n = stack.pop().ok_or("unbalanced XML")?;
                stack.last_mut().ok_or("unbalanced XML")?.children.push(n);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                return Err(format!(
                    "Tally response is not valid XML at byte {}: {e}",
                    r.buffer_position()
                ))
            }
        }
    }
    if stack.len() != 1 {
        return Err("Tally response ended early (incomplete XML)".into());
    }
    Ok(stack.pop().unwrap())
}

fn tdate(d: NaiveDate) -> String {
    d.format("%Y%m%d").to_string()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// XML request for a custom collection (inline TDL), optionally filtered to a period.
pub fn collection_request(
    id: &str,
    object: &str,
    fetch: &str,
    company: Option<&str>,
    period: Option<(NaiveDate, NaiveDate)>,
    date_filter: bool,
) -> String {
    let mut vars = String::from("<SVEXPORTFORMAT>$$SysName:XML</SVEXPORTFORMAT>");
    if let Some(c) = company {
        vars.push_str(&format!("<SVCURRENTCOMPANY>{}</SVCURRENTCOMPANY>", esc(c)));
    }
    if let Some((f, t)) = period {
        vars.push_str(&format!(
            "<SVFROMDATE>{}</SVFROMDATE><SVTODATE>{}</SVTODATE>",
            tdate(f),
            tdate(t)
        ));
    }
    let (filters, system) = if date_filter {
        ("<FILTERS>LCInPeriod</FILTERS>", "<SYSTEM TYPE=\"Formulae\" NAME=\"LCInPeriod\">$Date &gt;= ##SVFromDate AND $Date &lt;= ##SVToDate</SYSTEM>")
    } else {
        ("", "")
    };
    format!(
        "<ENVELOPE><HEADER><VERSION>1</VERSION><TALLYREQUEST>Export</TALLYREQUEST><TYPE>Collection</TYPE><ID>{id}</ID></HEADER>\
<BODY><DESC><STATICVARIABLES>{vars}</STATICVARIABLES><TDL><TDLMESSAGE>\
<COLLECTION NAME=\"{id}\" ISMODIFY=\"No\"><TYPE>{object}</TYPE><FETCH>{fetch}</FETCH>{filters}</COLLECTION>{system}\
</TDLMESSAGE></TDL></DESC></BODY></ENVELOPE>"
    )
}

/// Tally amount text → LedgerCraft amount (Dr positive).
pub fn tally_amount(s: &str) -> Result<Money, String> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(Money::ZERO);
    }
    let lower = t.to_ascii_lowercase();
    if lower.ends_with("dr") || lower.ends_with("cr") {
        return Money::parse(t).map_err(|e| e.to_string());
    }
    // Plain numbers follow Tally's sign: debit negative.
    let m = Money::parse(t.split('@').next().unwrap_or(t)).map_err(|e| e.to_string())?;
    Ok(-m)
}

fn name_of(n: &Node) -> String {
    n.attr("NAME")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| n.child_text("NAME"))
}

impl TallyClient {
    pub fn new(host: &str, port: u16) -> TallyClient {
        TallyClient {
            url: format!("http://{host}:{port}"),
            ..Default::default()
        }
    }

    fn post(&self, body: &str) -> Result<Node, String> {
        let agent = ureq::AgentBuilder::new().timeout(self.timeout).build();
        let resp = agent
            .post(&self.url)
            .set("Content-Type", "text/xml; charset=utf-8")
            .send_string(body)
            .map_err(|e| format!("Cannot reach Tally at {}: {e}. Check that Tally is open, a company is loaded and the HTTP/XML server is enabled on this port.", self.url))?;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes)
            .map_err(|e| format!("Reading Tally response: {e}"))?;
        let root = parse_xml(&sanitize(&bytes))?;
        let mut errs = Vec::new();
        root.find_all("LINEERROR", &mut errs);
        if let Some(e) = errs.first() {
            return Err(format!("Tally reported: {}", e.text.trim()));
        }
        Ok(root)
    }

    /// Connection check with a plain-language diagnosis.
    pub fn check(&self) -> Result<Vec<String>, String> {
        self.companies()
    }

    pub fn companies(&self) -> Result<Vec<String>, String> {
        let root = self.post(&collection_request(
            "LCCompanies",
            "Company",
            "Name",
            None,
            None,
            false,
        ))?;
        let mut v = Vec::new();
        root.find_all("COMPANY", &mut v);
        Ok(v.into_iter()
            .map(name_of)
            .filter(|n| !n.is_empty())
            .collect())
    }

    pub fn groups(&self, company: &str) -> Result<Vec<(String, String)>, String> {
        let root = self.post(&collection_request(
            "LCGroups",
            "Group",
            "Name, Parent",
            Some(company),
            None,
            false,
        ))?;
        let mut v = Vec::new();
        root.find_all("GROUP", &mut v);
        Ok(v.into_iter()
            .map(|g| (name_of(g), g.child_text("PARENT")))
            .filter(|(n, p)| !n.is_empty() && !p.is_empty() && !p.eq_ignore_ascii_case("primary"))
            .collect())
    }

    /// Trial balance for the period, with the company's group tree.
    pub fn trial_balance(
        &self,
        company: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<TrialBalance, String> {
        let groups = self.groups(company)?;
        let root = self.post(&collection_request(
            "LCLedgers",
            "Ledger",
            "Name, Parent, OpeningBalance, ClosingBalance",
            Some(company),
            Some((from, to)),
            false,
        ))?;
        let mut v = Vec::new();
        root.find_all("LEDGER", &mut v);
        let mut tb = TrialBalance {
            ledgers: Vec::new(),
            groups,
        };
        let res = GroupResolver::new(&tb);
        for l in v {
            let name = name_of(l);
            if name.is_empty() {
                continue;
            }
            let mut parent = l.child_text("PARENT");
            if parent.is_empty() && lc_core::model::norm_name(&name).starts_with("profit & loss") {
                parent = "Profit & Loss A/c".into();
            }
            let opening = tally_amount(&l.child_text("OPENINGBALANCE"))
                .map_err(|e| format!("{name}: opening balance: {e}"))?;
            let mut closing = tally_amount(&l.child_text("CLOSINGBALANCE"))
                .map_err(|e| format!("{name}: closing balance: {e}"))?;
            let mut closing_stock = None;
            if res.resolve(&parent).ok() == Some(Class::StockInHand) {
                closing_stock = Some(closing);
                closing = opening;
            }
            tb.ledgers.push(Ledger {
                name,
                group: parent,
                opening,
                closing,
                closing_stock,
                tags: vec![],
            });
        }
        Ok(tb)
    }

    /// Day book for the period, fetched month by month (large books stay responsive).
    pub fn vouchers(
        &self,
        company: &str,
        from: NaiveDate,
        to: NaiveDate,
        mut progress: impl FnMut(NaiveDate, usize),
    ) -> Result<Vec<Voucher>, String> {
        let mut out = Vec::new();
        let mut start = from;
        while start <= to {
            let month_end = start
                .with_day(1)
                .and_then(|d| d.checked_add_months(Months::new(1)))
                .and_then(|d| d.pred_opt())
                .unwrap_or(to)
                .min(to);
            let req = collection_request(
                "LCVouchers",
                "Voucher",
                "Date, VoucherTypeName, VoucherNumber, Narration, AllLedgerEntries.LedgerName, AllLedgerEntries.Amount, LedgerEntries.LedgerName, LedgerEntries.Amount",
                Some(company),
                Some((start, month_end)),
                true,
            );
            let root = self.post(&req)?;
            let mut v = Vec::new();
            root.find_all("VOUCHER", &mut v);
            for x in v {
                out.push(parse_voucher(x)?);
            }
            progress(month_end, out.len());
            start = month_end.succ_opt().unwrap_or(to);
            if month_end == to {
                break;
            }
        }
        Ok(out)
    }
}

pub fn parse_voucher(x: &Node) -> Result<Voucher, String> {
    let date_s = x.child_text("DATE");
    let date = lc_core::date::parse_date(&date_s)
        .ok_or_else(|| format!("voucher date '{date_s}' not understood"))?;
    let mut lines = Vec::new();
    for c in &x.children {
        let t = c.tag.to_ascii_uppercase();
        if t == "ALLLEDGERENTRIES.LIST" || t == "LEDGERENTRIES.LIST" {
            let ledger = c.child_text("LEDGERNAME");
            if ledger.is_empty() {
                continue;
            }
            let amount = tally_amount(&c.child_text("AMOUNT"))
                .map_err(|e| format!("voucher {}: {e}", x.child_text("VOUCHERNUMBER")))?;
            lines.push(VoucherLine { ledger, amount });
        }
    }
    Ok(Voucher {
        date,
        number: x.child_text("VOUCHERNUMBER"),
        vtype: {
            let t = x.child_text("VOUCHERTYPENAME");
            if t.is_empty() {
                x.attr("VCHTYPE").unwrap_or("").to_string()
            } else {
                t
            }
        },
        narration: x.child_text("NARRATION"),
        lines,
    })
}

/// Full engagement data for a financial year (and last year's trial balance).
pub fn import_year(
    client: &TallyClient,
    company: &str,
    fy_start: NaiveDate,
    fy_end: NaiveDate,
    with_vouchers: bool,
    progress: impl FnMut(NaiveDate, usize),
) -> Result<(TrialBalance, Option<TrialBalance>, Vec<Voucher>), String> {
    let cy = client.trial_balance(company, fy_start, fy_end)?;
    let py_start = fy_start
        .checked_sub_months(Months::new(12))
        .unwrap_or(fy_start);
    let py_end = fy_start.pred_opt().unwrap_or(fy_start);
    let py = client
        .trial_balance(company, py_start, py_end)
        .ok()
        .filter(|t| t.ledgers.iter().any(|l| !l.closing.is_zero()));
    let vouchers = if with_vouchers {
        client.vouchers(company, fy_start, fy_end, progress)?
    } else {
        Vec::new()
    };
    Ok((cy, py, vouchers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_and_parse_tally_quirks() {
        let raw = b"<ENVELOPE><BODY><DATA><COLLECTION><GROUP NAME=\"Sundry Debtors\"><PARENT>&#4; Current Assets</PARENT></GROUP>\
<LEDGER NAME=\"Cash\"><PARENT>Cash-in-Hand</PARENT><OPENINGBALANCE>-5000.00</OPENINGBALANCE><CLOSINGBALANCE>1,234.50 Cr</CLOSINGBALANCE></LEDGER>\
<LEDGER NAME=\"A &amp; B Traders\"><PARENT>Sundry Creditors</PARENT><OPENINGBALANCE>250.00</OPENINGBALANCE></LEDGER></COLLECTION></DATA></BODY></ENVELOPE>";
        let root = parse_xml(&sanitize(raw)).unwrap();
        let mut g = Vec::new();
        root.find_all("GROUP", &mut g);
        assert_eq!(g[0].child_text("PARENT"), "Current Assets");
        let mut l = Vec::new();
        root.find_all("LEDGER", &mut l);
        assert_eq!(name_of(l[1]), "A & B Traders");
        assert_eq!(
            tally_amount(&l[0].child_text("OPENINGBALANCE")).unwrap(),
            Money::rupees(5_000)
        );
        assert_eq!(
            tally_amount(&l[0].child_text("CLOSINGBALANCE")).unwrap(),
            Money(-123_450)
        );
        assert_eq!(tally_amount("250.00").unwrap(), Money::rupees(-250));
        // UTF-16 little-endian with BOM.
        let mut u16 = vec![0xFF, 0xFE];
        for c in "<A>x</A>".encode_utf16() {
            u16.extend_from_slice(&c.to_le_bytes());
        }
        assert_eq!(parse_xml(&sanitize(&u16)).unwrap().children[0].text, "x");
    }

    #[test]
    fn request_is_well_formed() {
        let r = collection_request(
            "LCVouchers",
            "Voucher",
            "Date",
            Some("ABC & Co"),
            Some((
                NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 4, 30).unwrap(),
            )),
            true,
        );
        let root = parse_xml(&r).unwrap();
        let mut v = Vec::new();
        root.find_all("SVCURRENTCOMPANY", &mut v);
        assert_eq!(v[0].text, "ABC & Co");
        assert!(r.contains("<SVFROMDATE>20250401</SVFROMDATE>"));
    }
}
