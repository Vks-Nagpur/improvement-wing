//! Ageing of trade receivables and trade payables from the day book (FIFO).
//!
//! Payments are applied to the oldest bills first; what remains outstanding is
//! aged from its bill date to the year end. Opening balances are dated at the
//! start of the year (their original dates need last year's day book).

use crate::checks::Ctx;
use crate::mapping::Head;
use crate::money::Money;
use crate::statements::MappedLedger;
use chrono::{Months, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeingKind {
    Receivables,
    Payables,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgeingRow {
    pub category: String,
    pub buckets: Vec<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ageing {
    pub kind: AgeingKind,
    pub bucket_labels: Vec<String>,
    pub rows: Vec<AgeingRow>,
}

impl Ageing {
    pub fn total(&self) -> Money {
        self.rows
            .iter()
            .flat_map(|r| r.buckets.iter().copied())
            .sum()
    }
}

fn bucket(kind: AgeingKind, fy_end: NaiveDate, date: NaiveDate) -> usize {
    let back = |m: u32| fy_end.checked_sub_months(Months::new(m)).unwrap_or(fy_end);
    match kind {
        AgeingKind::Receivables => {
            if date > back(6) {
                0
            } else if date > back(12) {
                1
            } else if date > back(24) {
                2
            } else if date > back(36) {
                3
            } else {
                4
            }
        }
        AgeingKind::Payables => {
            if date > back(12) {
                0
            } else if date > back(24) {
                1
            } else if date > back(36) {
                2
            } else {
                3
            }
        }
    }
}

/// Outstanding bills of one party ledger (display sign: amount owed), FIFO.
fn outstanding(ctx: &Ctx, i: usize, kind: AgeingKind) -> Vec<(NaiveDate, Money)> {
    let eng = ctx.eng;
    let l = ctx.ledger(i);
    // Receivables: bills are debits. Payables: bills are credits.
    let sign = if kind == AgeingKind::Receivables {
        1
    } else {
        -1
    };
    let mut bills: Vec<(NaiveDate, Money)> = Vec::new();
    let mut settled = Money::ZERO;
    let open = Money(l.opening.paise() * sign);
    if open.is_dr() {
        bills.push((eng.fy_start, open));
    } else {
        settled += -open;
    }
    for &k in &ctx.postings[i] {
        let v = &eng.vouchers[k];
        let amt: Money = v
            .lines
            .iter()
            .zip(&ctx.line_idx[k])
            .filter(|(_, x)| **x == Some(i))
            .map(|(ln, _)| ln.amount)
            .sum();
        let a = Money(amt.paise() * sign);
        if a.is_dr() {
            bills.push((v.date, a));
        } else {
            settled += -a;
        }
    }
    let mut out = Vec::new();
    for (d, a) in bills {
        if settled >= a {
            settled -= a;
        } else {
            out.push((d, a - settled));
            settled = Money::ZERO;
        }
    }
    out
}

pub fn compute(ctx: &Ctx, mapping: &[MappedLedger], kind: AgeingKind) -> Option<Ageing> {
    let eng = ctx.eng;
    if eng.vouchers.is_empty() {
        return None;
    }
    let head = match kind {
        AgeingKind::Receivables => Head::TradeReceivables,
        AgeingKind::Payables => Head::TradePayables,
    };
    let (labels, cats): (Vec<&str>, Vec<&str>) = match kind {
        AgeingKind::Receivables => (
            vec![
                "Less than 6 months",
                "6 months - 1 year",
                "1-2 years",
                "2-3 years",
                "More than 3 years",
            ],
            vec![
                "Undisputed trade receivables - considered good",
                "Undisputed trade receivables - considered doubtful",
                "Disputed trade receivables - considered good",
                "Disputed trade receivables - considered doubtful",
            ],
        ),
        AgeingKind::Payables => (
            vec![
                "Less than 1 year",
                "1-2 years",
                "2-3 years",
                "More than 3 years",
            ],
            vec![
                "MSME",
                "Others",
                "Disputed dues - MSME",
                "Disputed dues - Others",
            ],
        ),
    };
    let mut rows: Vec<AgeingRow> = cats
        .iter()
        .map(|c| AgeingRow {
            category: c.to_string(),
            buckets: vec![Money::ZERO; labels.len()],
        })
        .collect();
    for m in mapping.iter().filter(|m| m.head == Some(head)) {
        let Some(i) = ctx.lookup(&m.name) else {
            continue;
        };
        let l = ctx.ledger(i);
        let disputed = l.has_tag("disputed");
        let cat = match kind {
            AgeingKind::Receivables => {
                (if disputed { 2 } else { 0 }) + usize::from(l.has_tag("doubtful"))
            }
            AgeingKind::Payables => {
                (if disputed { 2 } else { 0 }) + usize::from(!l.has_tag("msme"))
            }
        };
        let bills = outstanding(ctx, i, kind);
        let aged: Money = bills.iter().map(|(_, a)| *a).sum();
        for (d, a) in bills {
            rows[cat].buckets[bucket(kind, eng.fy_end, d)] += a;
        }
        // Any difference to the ledger balance (books not fully reconciled) sits in the newest bucket.
        let target = if kind == AgeingKind::Receivables {
            l.closing
        } else {
            -l.closing
        };
        rows[cat].buckets[0] += target - aged;
    }
    Some(Ageing {
        kind,
        bucket_labels: labels.iter().map(|s| s.to_string()).collect(),
        rows,
    })
}
