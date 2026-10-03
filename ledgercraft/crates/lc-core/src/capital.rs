//! Owner-wise capital account movement (partners, proprietor, Karta, members).

use crate::checks::Ctx;
use crate::groups::{Class, Nature};
use crate::mapping::{has_stem, Head};
use crate::model::norm_name;
use crate::money::Money;
use serde::{Deserialize, Serialize};

/// All amounts Cr-positive (owed to the owner).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CapitalRow {
    pub owner: String,
    pub ledgers: Vec<String>,
    pub opening: Money,
    pub introduced: Money,
    pub remuneration_interest: Money,
    pub withdrawn: Money,
    pub other: Money,
    pub share_of_profit: Money,
    pub closing: Money,
    /// True when movement could be split using the day book.
    pub detailed: bool,
    /// Profit-sharing weight used for this owner.
    pub weight: u64,
}

fn owner_key(name: &str) -> String {
    let n = norm_name(name);
    let base = n
        .split(" capital")
        .next()
        .unwrap_or(&n)
        .split(" drawing")
        .next()
        .unwrap_or(&n)
        .split(" current")
        .next()
        .unwrap_or(&n);
    base.trim_end_matches(" a c").trim().to_string()
}

/// `distributable`: profit of the year plus the balance left in the Profit &
/// Loss A/c (Cr positive). Profit already credited to owners by a journal
/// through the Profit & Loss A/c is therefore not shared out twice.
/// `ratios`: (ledger or owner name, share) – missing or empty means equal shares.
pub fn movements(ctx: &Ctx, distributable: Money, ratios: &[(String, u32)]) -> Vec<CapitalRow> {
    let profit = distributable;
    let eng = ctx.eng;
    let mut rows: Vec<CapitalRow> = Vec::new();
    for (i, l) in eng.cy.ledgers.iter().enumerate() {
        if ctx.classes[i] != Some(Class::CapitalAccount) {
            continue;
        }
        let key = owner_key(&l.name);
        let pos = match rows.iter().position(|r| owner_key(&r.owner) == key) {
            Some(p) => p,
            None => {
                let label = l.name.clone();
                rows.push(CapitalRow {
                    owner: label,
                    ..Default::default()
                });
                rows.len() - 1
            }
        };
        let r = &mut rows[pos];
        r.ledgers.push(l.name.clone());
        // Prefer the capital ledger's name as the row label.
        if !has_stem(&norm_name(&l.name), &["drawing"]) {
            r.owner = l.name.clone();
        }
        r.opening += -l.opening;
        r.closing += -l.closing;
        if eng.vouchers.is_empty() {
            continue;
        }
        r.detailed = true;
        for &k in &ctx.postings[i] {
            let v = &eng.vouchers[k];
            let amt: Money = v
                .lines
                .iter()
                .zip(&ctx.line_idx[k])
                .filter(|(_, x)| **x == Some(i))
                .map(|(ln, _)| ln.amount)
                .sum();
            let other_classes: Vec<(Class, String)> = v
                .lines
                .iter()
                .zip(&ctx.line_idx[k])
                .filter_map(|(_, x)| {
                    x.filter(|j| *j != i)
                        .and_then(|j| ctx.classes[j].map(|c| (c, ctx.ledger(j).name.clone())))
                })
                .collect();
            let via_cash_bank = other_classes
                .iter()
                .any(|(c, _)| c.is_cash() || c.is_bank());
            let via_remun = other_classes.iter().any(|(c, n)| {
                c.nature() == Nature::Expense
                    && crate::mapping::name_rule(*c, n) == Some(Head::PartnersRemuneration)
            });
            let via_plac = other_classes.iter().any(|(c, _)| *c == Class::ProfitLossAc);
            if amt.is_cr() {
                let a = -amt;
                if via_plac {
                    r.share_of_profit += a;
                } else if via_remun {
                    r.remuneration_interest += a;
                } else if via_cash_bank {
                    r.introduced += a;
                } else {
                    r.other += a;
                }
            } else if !amt.is_zero() {
                if via_plac {
                    r.share_of_profit -= amt;
                } else {
                    r.withdrawn += amt;
                }
            }
        }
    }
    if rows.is_empty() {
        return rows;
    }
    // Share of profit by ratio (exact paise, remainder to the largest shares first).
    let shares: Vec<u64> = rows
        .iter()
        .map(|r| {
            ratios
                .iter()
                .find(|(n, _)| {
                    owner_key(n) == owner_key(&r.owner)
                        || r.ledgers.iter().any(|l| norm_name(l) == norm_name(n))
                })
                .map(|(_, s)| *s as u64)
                .unwrap_or(if ratios.is_empty() { 1 } else { 0 })
        })
        .collect();
    let total: u64 = shares.iter().sum::<u64>().max(1);
    let mut alloc: Vec<Money> = shares
        .iter()
        .map(|s| Money(profit.paise() * *s as i64 / total as i64))
        .collect();
    let mut rest = profit - alloc.iter().copied().sum::<Money>();
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|a, b| shares[*b].cmp(&shares[*a]).then(a.cmp(b)));
    let step = Money(rest.paise().signum());
    let mut k = 0;
    while !rest.is_zero() && shares.iter().any(|s| *s > 0) {
        let i = order[k % order.len()];
        if shares[i] > 0 {
            alloc[i] += step;
            rest -= step;
        }
        k += 1;
    }
    for ((r, a), w) in rows.iter_mut().zip(alloc).zip(&shares) {
        r.weight = *w;
        r.share_of_profit += a;
        r.closing += a;
        if r.detailed {
            // Whatever the day book could not classify stays in "other" so each row adds up.
            let explained = r.opening + r.introduced + r.remuneration_interest - r.withdrawn
                + r.other
                + r.share_of_profit;
            r.other += r.closing - explained;
        }
    }
    rows
}

/// Split `total` in proportion to `weights`, exactly (remainder paise go to the
/// largest weights first).
pub fn allocate(total: Money, weights: &[u64]) -> Vec<Money> {
    let sum: u64 = weights.iter().sum::<u64>();
    if sum == 0 || weights.is_empty() {
        return vec![Money::ZERO; weights.len()];
    }
    let mut out: Vec<Money> = weights
        .iter()
        .map(|w| Money(total.paise() * *w as i64 / sum as i64))
        .collect();
    let mut rest = total - out.iter().copied().sum::<Money>();
    let mut order: Vec<usize> = (0..weights.len()).filter(|i| weights[*i] > 0).collect();
    order.sort_by(|a, b| weights[*b].cmp(&weights[*a]).then(a.cmp(b)));
    let step = Money(rest.paise().signum());
    let mut k = 0;
    while !rest.is_zero() {
        out[order[k % order.len()]] += step;
        rest -= step;
        k += 1;
    }
    out
}
