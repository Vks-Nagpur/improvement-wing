//! Voucher-level integrity checks.

use super::{rs, Ctx, Detail, Findings};
use crate::groups::Class;
use crate::model::norm_name;
use crate::money::Money;
use chrono::NaiveDate;
use std::collections::{BTreeMap, HashMap};

pub fn run(ctx: &Ctx, f: &mut Findings) {
    let eng = ctx.eng;
    if eng.vouchers.is_empty() {
        return;
    }
    let n = eng.cy.ledgers.len();
    let mut movement = vec![Money::ZERO; n];
    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    let mut signatures: HashMap<(NaiveDate, Vec<(String, i64)>), usize> = HashMap::new();

    for (k, v) in eng.vouchers.iter().enumerate() {
        let total: Money = v.lines.iter().map(|l| l.amount).sum();
        if !total.is_zero() {
            f.add(
                "VOUCHER_UNBALANCED",
                &v.key(),
                &format!("{}: difference {}.", v.key(), rs(total)),
                Detail {
                    voucher: Some(v.key()),
                    date: Some(v.date),
                    amount: Some(total),
                    ..Default::default()
                },
            );
        }
        if v.date < eng.fy_start || v.date > eng.fy_end {
            f.add(
                "VOUCHER_OUTSIDE_PERIOD",
                &v.key(),
                &format!(
                    "{} is dated outside {} to {}.",
                    v.key(),
                    eng.fy_start.format("%d-%m-%Y"),
                    eng.fy_end.format("%d-%m-%Y")
                ),
                Detail {
                    voucher: Some(v.key()),
                    date: Some(v.date),
                    ..Default::default()
                },
            );
        }
        for (ln, li) in v.lines.iter().zip(&ctx.line_idx[k]) {
            match li {
                Some(i) => movement[*i] += ln.amount,
                None => *unknown.entry(ln.ledger.clone()).or_default() += 1,
            }
        }
        let mut sig: Vec<(String, i64)> = v
            .lines
            .iter()
            .map(|l| (norm_name(&l.ledger), l.amount.paise()))
            .collect();
        sig.sort();
        if let Some(&first) = signatures.get(&(v.date, sig.clone())) {
            let fv = &eng.vouchers[first];
            f.add(
                "DUPLICATE_VOUCHER",
                &v.key(),
                &format!("{} looks the same as {}.", v.key(), fv.key()),
                Detail {
                    voucher: Some(v.key()),
                    date: Some(v.date),
                    amount: Some(
                        v.lines
                            .iter()
                            .filter(|l| l.amount.is_dr())
                            .map(|l| l.amount)
                            .sum(),
                    ),
                    suggestion: Some("Delete one if it was entered twice.".into()),
                    ..Default::default()
                },
            );
        } else {
            signatures.insert((v.date, sig), k);
        }
    }

    for (name, count) in unknown {
        f.add(
            "VOUCHER_UNKNOWN_LEDGER",
            &name,
            &format!("'{name}' is used in {count} voucher line(s)."),
            Detail {
                ledger: Some(name.clone()),
                ..Default::default()
            },
        );
    }

    for (i, l) in eng.cy.ledgers.iter().enumerate() {
        let expected = l.opening + movement[i];
        if expected != l.closing {
            f.add(
                "VOUCHER_TB_MISMATCH",
                &l.name,
                &format!(
                    "'{}': opening {} + vouchers {} = {}, but trial balance shows {}.",
                    l.name,
                    l.opening.fmt_drcr(),
                    movement[i].fmt_drcr(),
                    expected.fmt_drcr(),
                    l.closing.fmt_drcr()
                ),
                Detail {
                    ledger: Some(l.name.clone()),
                    amount: Some(l.closing - expected),
                    ..Default::default()
                },
            );
        }
    }

    negative_cash(ctx, f);
}

/// End-of-day cash balance must never go below zero.
fn negative_cash(ctx: &Ctx, f: &mut Findings) {
    let eng = ctx.eng;
    let cash: Vec<usize> = (0..eng.cy.ledgers.len())
        .filter(|&i| ctx.classes[i] == Some(Class::CashInHand))
        .collect();
    for ci in cash {
        let l = ctx.ledger(ci);
        let mut bal = l.opening;
        let mut day: Option<NaiveDate> = None;
        let report = |d: NaiveDate, b: Money, f: &mut Findings| {
            if b.is_cr() {
                f.add(
                    "NEGATIVE_CASH",
                    &format!("{} {}", l.name, d.format("%d-%m-%Y")),
                    &format!(
                        "'{}' closed {} at {}.",
                        l.name,
                        d.format("%d-%m-%Y"),
                        b.fmt_drcr()
                    ),
                    Detail {
                        ledger: Some(l.name.clone()),
                        date: Some(d),
                        amount: Some(b),
                        suggestion: Some(
                            "Check missing cash receipts / withdrawals, or wrong dates.".into(),
                        ),
                        ..Default::default()
                    },
                );
            }
        };
        for &k in &ctx.postings[ci] {
            let v = &eng.vouchers[k];
            let mv: Money = v
                .lines
                .iter()
                .zip(&ctx.line_idx[k])
                .filter(|(_, li)| **li == Some(ci))
                .map(|(ln, _)| ln.amount)
                .sum();
            if mv.is_zero() {
                continue;
            }
            if let Some(d) = day {
                if d != v.date {
                    report(d, bal, f);
                }
            }
            day = Some(v.date);
            bal += mv;
        }
        if let Some(d) = day {
            report(d, bal, f);
        }
    }
}
