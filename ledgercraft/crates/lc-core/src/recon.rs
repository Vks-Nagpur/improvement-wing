//! Party-wise reconciliation of the books with government statements:
//! input tax credit against GSTR-2B (per supplier) and tax deducted against
//! Form 26AS (per deductor). Parties are matched on the name in the books
//! and the name on the portal; a name match is a suggestion the user sees
//! (TRUTH-MODEL.md §4), never a silent decision.

use crate::groups::{Class, GroupResolver};
use crate::model::{norm_name, Engagement};
use crate::money::Money;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One party as the portal reports it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortalParty {
    /// GSTIN of the supplier or TAN of the deductor.
    pub id: String,
    pub name: String,
    pub documents: usize,
    /// Taxable value (2B) or amount paid / credited (26AS).
    pub base: Money,
    /// ITC available (2B) or TDS deposited (26AS).
    pub tax: Money,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BookParty {
    pub ledger: String,
    /// Purchases / sales with the party in the year.
    pub base: Money,
    /// ITC booked (GST) or TDS booked (26AS) on the party's entries.
    pub tax: Money,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReconRow {
    pub ledger: Option<String>,
    pub portal_id: Option<String>,
    pub portal_name: Option<String>,
    pub books_base: Money,
    pub books_tax: Money,
    pub portal_base: Money,
    pub portal_tax: Money,
    /// Portal minus books.
    pub difference: Money,
    /// "exact" (same name), "close" (name looks alike: confirm), or "" when unmatched.
    pub matched_by: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Recon {
    pub kind: String,
    pub rows: Vec<ReconRow>,
    pub books_total: Money,
    pub portal_total: Money,
}

const NOISE: &[&str] = &[
    "m",
    "s",
    "ms",
    "pvt",
    "private",
    "ltd",
    "limited",
    "llp",
    "the",
    "and",
    "co",
    "company",
    "corp",
    "corporation",
    "inc",
    "a",
    "c",
    "ac",
];

fn tokens(name: &str) -> Vec<String> {
    norm_name(name)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty() && !NOISE.contains(t))
        .map(|t| t.to_string())
        .collect()
}

/// How alike two party names are: 1.0 identical after removing noise words.
pub fn name_score(a: &str, b: &str) -> f64 {
    let (ta, tb) = (tokens(a), tokens(b));
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    if ta == tb {
        return 1.0;
    }
    let common = ta.iter().filter(|t| tb.contains(t)).count() as f64;
    common / (ta.len().max(tb.len()) as f64)
}

pub fn reconcile(kind: &str, books: &[BookParty], portal: &[PortalParty]) -> Recon {
    let mut used = vec![false; portal.len()];
    let mut rows = Vec::new();
    for b in books {
        let best = portal
            .iter()
            .enumerate()
            .filter(|(i, _)| !used[*i])
            .map(|(i, p)| (i, name_score(&b.ledger, &p.name)))
            .filter(|(_, s)| *s >= 0.6)
            .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        match best {
            Some((i, s)) => {
                used[i] = true;
                let p = &portal[i];
                rows.push(ReconRow {
                    ledger: Some(b.ledger.clone()),
                    portal_id: Some(p.id.clone()),
                    portal_name: Some(p.name.clone()),
                    books_base: b.base,
                    books_tax: b.tax,
                    portal_base: p.base,
                    portal_tax: p.tax,
                    difference: p.tax - b.tax,
                    matched_by: if s >= 1.0 { "exact" } else { "close" }.into(),
                });
            }
            None if !b.tax.is_zero() => rows.push(ReconRow {
                ledger: Some(b.ledger.clone()),
                books_base: b.base,
                books_tax: b.tax,
                difference: -b.tax,
                ..Default::default()
            }),
            None => {}
        }
    }
    for (i, p) in portal.iter().enumerate().filter(|(i, _)| !used[*i]) {
        let _ = i;
        rows.push(ReconRow {
            portal_id: Some(p.id.clone()),
            portal_name: Some(p.name.clone()),
            portal_base: p.base,
            portal_tax: p.tax,
            difference: p.tax,
            ..Default::default()
        });
    }
    Recon {
        kind: kind.into(),
        books_total: books.iter().map(|b| b.tax).sum(),
        portal_total: portal.iter().map(|p| p.tax).sum(),
        rows,
    }
}

fn has(n: &str, words: &[&str]) -> bool {
    let padded = format!(" {n} ");
    words
        .iter()
        .any(|w| padded.contains(&format!(" {w} ")) || padded.contains(&format!(" {w}")))
}

/// Input tax credit booked per supplier: in each voucher, the input GST lines
/// (Dr) are attributed to the supplier credited in the same voucher.
pub fn books_itc(eng: &Engagement) -> Vec<BookParty> {
    party_tax(
        eng,
        Class::SundryCreditors,
        |n, cl| {
            has(n, &["input"]) && has(n, &["gst", "cgst", "sgst", "igst", "utgst", "cess"])
                || (cl == Some(Class::DutiesTaxes) && has(n, &["itc"]))
        },
        true,
    )
}

/// TDS booked per customer: in each voucher, TDS receivable lines (Dr) are
/// attributed to the customer credited in the same voucher.
pub fn books_tds(eng: &Engagement) -> Vec<BookParty> {
    party_tax(
        eng,
        Class::SundryDebtors,
        |n, cl| {
            has(n, &["tds", "tax deducted"])
                && !matches!(cl, Some(Class::DutiesTaxes) if has(n, &["payable"]))
                && !has(n, &["payable"])
        },
        false,
    )
}

fn party_tax(
    eng: &Engagement,
    party_class: Class,
    is_tax: impl Fn(&str, Option<Class>) -> bool,
    supplier: bool,
) -> Vec<BookParty> {
    let res = GroupResolver::new(&eng.cy);
    let class_of: BTreeMap<String, Option<Class>> = eng
        .cy
        .ledgers
        .iter()
        .map(|l| (norm_name(&l.name), res.resolve(&l.group).ok()))
        .collect();
    let mut by: BTreeMap<String, BookParty> = BTreeMap::new();
    for v in &eng.vouchers {
        let cls = |l: &str| class_of.get(&norm_name(l)).copied().flatten();
        // The party: the line of the party class on the expected side
        // (suppliers credited on purchases, customers debited on sales and
        // credited on receipts).
        let party = v
            .lines
            .iter()
            .filter(|l| cls(&l.ledger) == Some(party_class))
            .max_by_key(|l| l.amount.0.abs());
        let Some(party) = party else { continue };
        let tax: Money = v
            .lines
            .iter()
            .filter(|l| l.amount.0 > 0 && is_tax(&norm_name(&l.ledger), cls(&l.ledger)))
            .map(|l| l.amount)
            .sum();
        let e = by
            .entry(norm_name(&party.ledger))
            .or_insert_with(|| BookParty {
                ledger: party.ledger.clone(),
                ..Default::default()
            });
        e.tax += tax;
        // Base: purchases credited to the supplier / sales debited to the customer.
        if supplier && party.amount.is_cr() {
            e.base += -party.amount - tax;
        }
        if !supplier && party.amount.0 > 0 {
            e.base += party.amount;
        }
    }
    by.into_values()
        .filter(|b| !b.tax.is_zero() || !b.base.is_zero())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_without_noise_words() {
        assert_eq!(
            name_score("M/s Shree Ganesh Hardware", "SHREE GANESH HARDWARE"),
            1.0
        );
        assert_eq!(
            name_score(
                "Om Sai Enterprises Pvt Ltd",
                "Om Sai Enterprises Private Limited"
            ),
            1.0
        );
        assert!(name_score("Shree Ganesh Traders", "Shree Ganesh Hardware") < 0.7);
        assert!(name_score("Patil Agro Industries", "PATIL AGRO") >= 0.6);
    }

    #[test]
    fn rows_for_matched_missing_and_extra() {
        let books = vec![
            BookParty {
                ledger: "Shree Ganesh Hardware".into(),
                base: Money(100000),
                tax: Money(18000),
            },
            BookParty {
                ledger: "Non Filer Traders".into(),
                base: Money(50000),
                tax: Money(9000),
            },
        ];
        let portal = vec![
            PortalParty {
                id: "27AAAAA0000A1Z5".into(),
                name: "SHREE GANESH HARDWARE".into(),
                documents: 2,
                base: Money(100000),
                tax: Money(18000),
            },
            PortalParty {
                id: "27BBBBB0000B1Z5".into(),
                name: "Unbooked Supplier".into(),
                documents: 1,
                base: Money(10000),
                tax: Money(1800),
            },
        ];
        let r = reconcile("gstr2b", &books, &portal);
        assert_eq!(r.rows.len(), 3);
        assert_eq!(r.rows[0].difference, Money::ZERO);
        assert_eq!(r.rows[0].matched_by, "exact");
        assert_eq!(
            r.rows[1].portal_name, None,
            "in books, not in 2B: supplier may not have filed"
        );
        assert_eq!(r.rows[2].ledger, None, "in 2B, not in books");
    }

    #[test]
    fn itc_and_tds_are_attributed_to_the_party_of_the_voucher() {
        use crate::model::{EntityType, Ledger, TrialBalance, Voucher, VoucherLine};
        use chrono::NaiveDate;
        let l = |n: &str, g: &str| Ledger {
            name: n.into(),
            group: g.into(),
            opening: Money::ZERO,
            closing: Money::ZERO,
            closing_stock: None,
            tags: vec![],
        };
        let vl = |n: &str, a: i64| VoucherLine {
            ledger: n.into(),
            amount: Money(a),
        };
        let d = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let v = |lines: Vec<VoucherLine>| Voucher {
            date: d,
            number: "1".into(),
            vtype: "Journal".into(),
            narration: String::new(),
            lines,
        };
        let eng = Engagement {
            entity_name: "T".into(),
            entity_type: EntityType::Firm,
            fy_start: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            fy_end: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            cy: TrialBalance {
                ledgers: vec![
                    l("Shree Ganesh Hardware", "Sundry Creditors"),
                    l("Purchases", "Purchase Accounts"),
                    l("Input CGST", "Duties & Taxes"),
                    l("Input SGST", "Duties & Taxes"),
                    l("Om Sai Enterprises", "Sundry Debtors"),
                    l("Sales", "Sales Accounts"),
                    l("Bank", "Bank Accounts"),
                    l("TDS Receivable", "Loans & Advances (Asset)"),
                ],
                groups: vec![],
            },
            py: None,
            vouchers: vec![
                v(vec![
                    vl("Purchases", 1000000),
                    vl("Input CGST", 90000),
                    vl("Input SGST", 90000),
                    vl("Shree Ganesh Hardware", -1180000),
                ]),
                v(vec![
                    vl("Om Sai Enterprises", 5000000),
                    vl("Sales", -5000000),
                ]),
                v(vec![
                    vl("Bank", 4900000),
                    vl("TDS Receivable", 100000),
                    vl("Om Sai Enterprises", -5000000),
                ]),
            ],
            mapping_memory: Default::default(),
            mapping_context: Default::default(),
            far: None,
            profit_sharing: vec![],
            format_pack: None,
            consolidation: None,
            comparative_end: None,
        };
        let itc = books_itc(&eng);
        assert_eq!(itc.len(), 1);
        assert_eq!(itc[0].tax, Money(180000));
        assert_eq!(itc[0].base, Money(1000000));
        let tds = books_tds(&eng);
        let om = tds
            .iter()
            .find(|b| b.ledger == "Om Sai Enterprises")
            .unwrap();
        assert_eq!(om.tax, Money(100000));
        assert_eq!(om.base, Money(5000000));
    }
}
