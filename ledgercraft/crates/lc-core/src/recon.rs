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
    /// "identifier" (GSTIN/TAN in the ledger name, or your confirmation),
    /// "exact" (same name: suggested, confirm it), or "" when not matched.
    pub matched_by: String,
    /// matched (by identifier), suggested (same name), review (similar
    /// names, not matched until you choose), unmatched.
    #[serde(default)]
    pub status: String,
    /// Possible portal parties for a book party under review (best first).
    #[serde(default)]
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub score: f64,
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

/// A GSTIN (15 characters) or TAN (10) written inside a ledger name.
pub fn id_in_name(name: &str) -> Option<String> {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(|t| t.to_ascii_uppercase())
        .find(|t| {
            let b = t.as_bytes();
            (b.len() == 15
                && b[..2].iter().all(|c| c.is_ascii_digit())
                && b[2..7].iter().all(|c| c.is_ascii_uppercase()))
                || (b.len() == 10
                    && b[..4].iter().all(|c| c.is_ascii_uppercase())
                    && b[4..9].iter().all(|c| c.is_ascii_digit())
                    && b[9].is_ascii_uppercase())
        })
}

/// Match book parties with portal parties.
/// 1. Identifier: GSTIN/TAN in the ledger name, or a choice you confirmed
///    (`confirmed`: normalised ledger name → portal id). Deterministic.
/// 2. Same name (after removing noise words), unique on both sides:
///    suggested; shown for confirmation.
/// 3. Similar names: review only. The candidates are listed with scores and
///    the portal party is NOT consumed, so both sides stay unmatched until
///    you choose. Ties are never broken by order.
pub fn reconcile(
    kind: &str,
    books: &[BookParty],
    portal: &[PortalParty],
    confirmed: &BTreeMap<String, String>,
) -> Recon {
    let mut used = vec![false; portal.len()];
    let mut done = vec![false; books.len()];
    let mut rows = Vec::new();
    let pair = |b: &BookParty, p: &PortalParty, by: &str, status: &str| ReconRow {
        ledger: Some(b.ledger.clone()),
        portal_id: Some(p.id.clone()),
        portal_name: Some(p.name.clone()),
        books_base: b.base,
        books_tax: b.tax,
        portal_base: p.base,
        portal_tax: p.tax,
        difference: p.tax - b.tax,
        matched_by: by.into(),
        status: status.into(),
        candidates: vec![],
    };
    // 1. Identifiers.
    for (bi, b) in books.iter().enumerate() {
        let id = confirmed
            .get(&norm_name(&b.ledger))
            .cloned()
            .or_else(|| id_in_name(&b.ledger));
        if let Some(id) = id {
            if let Some(pi) = portal
                .iter()
                .enumerate()
                .position(|(i, p)| !used[i] && p.id.eq_ignore_ascii_case(&id))
            {
                used[pi] = true;
                done[bi] = true;
                rows.push(pair(b, &portal[pi], "identifier", "matched"));
            }
        }
    }
    // 2. Same name, unique on both sides.
    for (bi, b) in books.iter().enumerate() {
        if done[bi] {
            continue;
        }
        let same: Vec<usize> = (0..portal.len())
            .filter(|&i| !used[i] && name_score(&b.ledger, &portal[i].name) >= 1.0)
            .collect();
        if same.len() != 1 {
            continue;
        }
        let pi = same[0];
        let rivals = books
            .iter()
            .enumerate()
            .filter(|(j, o)| {
                *j != bi && !done[*j] && name_score(&o.ledger, &portal[pi].name) >= 1.0
            })
            .count();
        if rivals == 0 {
            used[pi] = true;
            done[bi] = true;
            rows.push(pair(b, &portal[pi], "exact", "suggested"));
        }
    }
    // 3. Similar names: candidates only.
    for (bi, b) in books.iter().enumerate().filter(|(i, _)| !done[*i]) {
        let mut cands: Vec<Candidate> = portal
            .iter()
            .enumerate()
            .filter(|(i, _)| !used[*i])
            .map(|(_, p)| Candidate {
                id: p.id.clone(),
                name: p.name.clone(),
                score: name_score(&b.ledger, &p.name),
            })
            .filter(|c| c.score >= 0.5)
            .collect();
        cands.sort_by(|x, y| {
            y.score
                .partial_cmp(&x.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        cands.truncate(3);
        if cands.is_empty() && b.tax.is_zero() {
            continue;
        }
        let _ = bi;
        rows.push(ReconRow {
            ledger: Some(b.ledger.clone()),
            books_base: b.base,
            books_tax: b.tax,
            difference: -b.tax,
            status: if cands.is_empty() {
                "unmatched"
            } else {
                "review"
            }
            .into(),
            candidates: cands,
            ..Default::default()
        });
    }
    for (_, p) in portal.iter().enumerate().filter(|(i, _)| !used[*i]) {
        rows.push(ReconRow {
            portal_id: Some(p.id.clone()),
            portal_name: Some(p.name.clone()),
            portal_base: p.base,
            portal_tax: p.tax,
            difference: p.tax,
            status: "unmatched".into(),
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
        let r = reconcile("gstr2b", &books, &portal, &BTreeMap::new());
        assert_eq!(r.rows.len(), 3);
        assert_eq!(r.rows[0].difference, Money::ZERO);
        assert_eq!(r.rows[0].matched_by, "exact");
        assert_eq!(r.rows[0].status, "suggested", "a name alone is never final");
        assert_eq!(
            r.rows[1].portal_name, None,
            "in books, not in 2B: supplier may not have filed"
        );
        assert_eq!(r.rows[2].ledger, None, "in 2B, not in books");
    }

    fn bp(n: &str, t: i64) -> BookParty {
        BookParty {
            ledger: n.into(),
            base: Money(t * 5),
            tax: Money(t),
        }
    }
    fn pp(id: &str, n: &str, t: i64) -> PortalParty {
        PortalParty {
            id: id.into(),
            name: n.into(),
            documents: 1,
            base: Money(t * 5),
            tax: Money(t),
        }
    }

    #[test]
    fn similar_names_are_review_only_and_ties_are_not_broken_by_order() {
        let books = vec![bp("Shree Ganesh Traders", 100)];
        let portal = vec![
            pp("27AAAAA0000A1Z5", "Shree Ganesh Hardware", 100),
            pp("27CCCCC0000C1Z5", "Shree Ganesh Steel", 100),
        ];
        let r = reconcile("gstr2b", &books, &portal, &BTreeMap::new());
        let b = r.rows.iter().find(|x| x.ledger.is_some()).unwrap();
        assert_eq!(b.status, "review");
        assert_eq!(
            b.portal_id, None,
            "no portal party consumed by a similar name"
        );
        assert_eq!(b.candidates.len(), 2);
        assert_eq!(
            r.rows.iter().filter(|x| x.ledger.is_none()).count(),
            2,
            "both portal parties still listed"
        );
    }

    #[test]
    fn same_name_twice_is_not_matched_and_identifiers_win() {
        let books = vec![bp("Om Traders", 50), bp("Om Traders Pvt Ltd", 60)];
        let portal = vec![pp("27DDDDD0000D1Z5", "OM TRADERS", 60)];
        let r = reconcile("gstr2b", &books, &portal, &BTreeMap::new());
        assert!(
            r.rows
                .iter()
                .all(|x| x.status != "suggested" && x.status != "matched"),
            "two books claim the same name"
        );
        // The user confirms which ledger is that GSTIN.
        let conf: BTreeMap<String, String> = [(
            norm_name("Om Traders Pvt Ltd"),
            "27DDDDD0000D1Z5".to_string(),
        )]
        .into();
        let r = reconcile("gstr2b", &books, &portal, &conf);
        let m = r.rows.iter().find(|x| x.status == "matched").unwrap();
        assert_eq!(m.ledger.as_deref(), Some("Om Traders Pvt Ltd"));
        assert_eq!(m.matched_by, "identifier");
        // GSTIN written in the ledger name.
        let r = reconcile(
            "gstr2b",
            &[bp("Om Traders (27DDDDD0000D1Z5)", 60)],
            &portal,
            &BTreeMap::new(),
        );
        assert_eq!(r.rows[0].matched_by, "identifier");
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
