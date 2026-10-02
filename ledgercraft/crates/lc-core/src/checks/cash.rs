//! Cash payment / receipt limit tests (auditor reference; flags only).
//!
//! * Payments for expenses / purchases: per voucher for expense ledgers (the
//!   payee is not known), per party per day for supplier ledgers.
//! * Assets bought in cash: per asset ledger per day.
//! * Receipts: per party per day; cash sales per voucher.
//!
//! Loans are excluded here and tested in `loans`.

use super::{rs, Ctx, Detail, Findings};
use crate::groups::{Class, Nature};
use crate::money::Money;
use chrono::NaiveDate;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Payment,
    Receipt,
}

pub fn run(ctx: &Ctx, f: &mut Findings) {
    let eng = ctx.eng;
    let t = ctx.threshold();
    // (date, ledger) -> (total, voucher keys)
    let mut party_pay: BTreeMap<(NaiveDate, usize), (Money, Vec<String>)> = BTreeMap::new();
    let mut asset_pay: BTreeMap<(NaiveDate, usize), (Money, Vec<String>)> = BTreeMap::new();
    let mut party_rcpt: BTreeMap<(NaiveDate, usize), (Money, Vec<String>)> = BTreeMap::new();

    for &k in &ctx.by_date {
        let v = &eng.vouchers[k];
        let li = &ctx.line_idx[k];
        let is_cash = |j: usize| {
            li[j]
                .and_then(|i| ctx.classes[i])
                .map(Class::is_cash)
                .unwrap_or(false)
        };
        let cash_total: Money = (0..v.lines.len())
            .filter(|&j| is_cash(j))
            .map(|j| v.lines[j].amount)
            .sum();
        if cash_total.is_zero() {
            continue;
        }
        let side = if cash_total.is_cr() {
            Side::Payment
        } else {
            Side::Receipt
        };
        // Only clean cash vouchers: every line on the cash side must be cash.
        let mixed = (0..v.lines.len()).any(|j| {
            !is_cash(j)
                && match side {
                    Side::Payment => v.lines[j].amount.is_cr(),
                    Side::Receipt => v.lines[j].amount.is_dr(),
                }
        });
        if mixed {
            continue;
        }
        for (j, ln) in v.lines.iter().enumerate() {
            if is_cash(j) || ln.amount.is_zero() {
                continue;
            }
            let Some(i) = li[j] else { continue };
            let Some(class) = ctx.classes[i] else {
                continue;
            };
            if class.is_bank() || ctx.is_loan_taken(i) {
                continue;
            }
            let amt = ln.amount.abs();
            match side {
                Side::Payment => {
                    if class == Class::FixedAssets {
                        let e = asset_pay.entry((v.date, i)).or_default();
                        e.0 += amt;
                        e.1.push(v.key());
                    } else if class == Class::SundryCreditors {
                        let e = party_pay.entry((v.date, i)).or_default();
                        e.0 += amt;
                        e.1.push(v.key());
                    } else if class.nature() == Nature::Expense {
                        let l = ctx.ledger(i);
                        let limit = pay_limit(ctx, i);
                        if amt.paise() > limit {
                            f.add(
                                "CASH_PAYMENT_LIMIT",
                                &format!("{} {}", v.key(), l.name),
                                &format!("{}: {} paid in cash for '{}' (limit {}).", v.key(), rs(amt), l.name, rs(Money(limit))),
                                Detail { ledger: Some(l.name.clone()), voucher: Some(v.key()), date: Some(v.date), amount: Some(amt), suggestion: Some("Check Rule 6DD exceptions; otherwise consider disallowance in tax computation.".into()) },
                            );
                        }
                    }
                }
                Side::Receipt => {
                    if class.nature() == Nature::Income {
                        if amt.paise() >= t.cash_receipt_paise {
                            let l = ctx.ledger(i);
                            f.add(
                                "CASH_RECEIPT_LIMIT",
                                &format!("{} {}", v.key(), l.name),
                                &format!("{}: {} received in cash ('{}').", v.key(), rs(amt), l.name),
                                Detail { ledger: Some(l.name.clone()), voucher: Some(v.key()), date: Some(v.date), amount: Some(amt), suggestion: Some("Identify the customer; receipt of ₹2 lakh or more in cash attracts penalty.".into()) },
                            );
                        }
                    } else {
                        let e = party_rcpt.entry((v.date, i)).or_default();
                        e.0 += amt;
                        e.1.push(v.key());
                    }
                }
            }
        }
    }

    for ((d, i), (amt, keys)) in party_pay {
        let limit = pay_limit(ctx, i);
        if amt.paise() > limit {
            let l = ctx.ledger(i);
            f.add(
                "CASH_PAYMENT_LIMIT",
                &format!("{} {}", d.format("%d-%m-%Y"), l.name),
                &format!("{} paid in cash to '{}' on {} (limit {}). Vouchers: {}.", rs(amt), l.name, d.format("%d-%m-%Y"), rs(Money(limit)), keys.join("; ")),
                Detail { ledger: Some(l.name.clone()), voucher: Some(keys.join("; ")), date: Some(d), amount: Some(amt), suggestion: Some("Check Rule 6DD exceptions; otherwise consider disallowance in tax computation.".into()) },
            );
        }
    }
    for ((d, i), (amt, keys)) in asset_pay {
        if amt.paise() > ctx.threshold().cash_expense_payment_paise {
            let l = ctx.ledger(i);
            f.add(
                "CASH_ASSET_PURCHASE",
                &format!("{} {}", d.format("%d-%m-%Y"), l.name),
                &format!(
                    "{} paid in cash for '{}' on {}. Vouchers: {}.",
                    rs(amt),
                    l.name,
                    d.format("%d-%m-%Y"),
                    keys.join("; ")
                ),
                Detail {
                    ledger: Some(l.name.clone()),
                    voucher: Some(keys.join("; ")),
                    date: Some(d),
                    amount: Some(amt),
                    suggestion: Some(
                        "Exclude this amount from the asset's cost for tax depreciation.".into(),
                    ),
                },
            );
        }
    }
    for ((d, i), (amt, keys)) in party_rcpt {
        if amt.paise() >= t.cash_receipt_paise {
            let l = ctx.ledger(i);
            f.add(
                "CASH_RECEIPT_LIMIT",
                &format!("{} {}", d.format("%d-%m-%Y"), l.name),
                &format!("{} received in cash from '{}' on {}. Vouchers: {}.", rs(amt), l.name, d.format("%d-%m-%Y"), keys.join("; ")),
                Detail { ledger: Some(l.name.clone()), voucher: Some(keys.join("; ")), date: Some(d), amount: Some(amt), suggestion: Some("Receipt of ₹2 lakh or more in cash from a person in a day attracts penalty.".into()) },
            );
        }
    }
}

fn pay_limit(ctx: &Ctx, i: usize) -> i64 {
    if ctx.ledger(i).has_tag("transporter") {
        ctx.threshold().cash_payment_transporter_paise
    } else {
        ctx.threshold().cash_expense_payment_paise
    }
}
