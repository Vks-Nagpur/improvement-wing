//! Opening balance vs previous-year closing.

use super::{rs, Ctx, Detail, Findings};
use crate::groups::{Class, GroupResolver};
use crate::model::norm_name;
use crate::money::Money;
use std::collections::HashMap;

pub fn run(ctx: &Ctx, f: &mut Findings) {
    let eng = ctx.eng;
    let Some(py) = &eng.py else { return };
    let py_res = GroupResolver::new(py);
    let cy_res = GroupResolver::new(&eng.cy);
    let py_class = |g: &str| py_res.resolve(g).ok().or_else(|| cy_res.resolve(g).ok());

    // Net result of last year's income/expense ledgers (Dr positive = loss).
    let mut py_pl_net = Money::ZERO;
    let mut py_by_name: HashMap<String, usize> = HashMap::new();
    for (j, l) in py.ledgers.iter().enumerate() {
        py_by_name.insert(norm_name(&l.name), j);
        if let Some(c) = py_class(&l.group) {
            if c.nature().is_pl() {
                py_pl_net += l.closing;
            } else if c == Class::StockInHand {
                // Stock change of last year is part of last year's result.
                py_pl_net -= l.closing_stock.unwrap_or(l.closing) - l.closing;
            }
        }
    }

    let mut seen_py = vec![false; py.ledgers.len()];
    for (i, l) in eng.cy.ledgers.iter().enumerate() {
        let Some(class) = ctx.classes[i] else {
            continue;
        };
        let pj = py_by_name.get(&norm_name(&l.name)).copied();
        if let Some(j) = pj {
            seen_py[j] = true;
        }
        if class.nature().is_pl() {
            if !l.opening.is_zero() {
                f.add(
                    "PL_OPENING",
                    &l.name,
                    &format!(
                        "'{}' starts the year with {}.",
                        l.name,
                        l.opening.fmt_drcr()
                    ),
                    Detail {
                        ledger: Some(l.name.clone()),
                        amount: Some(l.opening),
                        ..Default::default()
                    },
                );
            }
            continue;
        }
        let expected = match pj {
            Some(j) => {
                let p = &py.ledgers[j];
                let base = if class == Class::StockInHand {
                    p.closing_stock.unwrap_or(p.closing)
                } else {
                    p.closing
                };
                if class == Class::ProfitLossAc {
                    base + py_pl_net
                } else {
                    base
                }
            }
            None if class == Class::ProfitLossAc => py_pl_net,
            None => {
                if !l.opening.is_zero() {
                    f.add(
                        "OPENING_NEW_LEDGER",
                        &l.name,
                        &format!("'{}' has opening {} but no such ledger existed last year.", l.name, l.opening.fmt_drcr()),
                        Detail { ledger: Some(l.name.clone()), amount: Some(l.opening), suggestion: Some("Check if the ledger was renamed; if so, link it to last year's name.".into()), ..Default::default() },
                    );
                }
                continue;
            }
        };
        if l.opening != expected {
            let diff = l.opening - expected;
            f.add(
                "OPENING_DIFF",
                &l.name,
                &format!(
                    "'{}': opening this year {}, expected {} ({}). Difference {}.",
                    l.name,
                    l.opening.fmt_drcr(),
                    expected.fmt_drcr(),
                    if class == Class::ProfitLossAc { "last year's closing plus last year's profit/loss" } else { "last year's closing" },
                    rs(diff)
                ),
                Detail { ledger: Some(l.name.clone()), amount: Some(diff), suggestion: Some("Correct the opening balance, or record the reason (e.g. prior-period adjustment).".into()), ..Default::default() },
            );
        }
    }

    for (j, p) in py.ledgers.iter().enumerate() {
        if seen_py[j] || p.closing.is_zero() {
            continue;
        }
        let Some(c) = py_class(&p.group) else {
            continue;
        };
        if c.nature().is_pl() {
            continue;
        }
        f.add(
            "OPENING_MISSING_LEDGER",
            &p.name,
            &format!(
                "'{}' closed last year at {} but is not in this year's books.",
                p.name,
                p.closing.fmt_drcr()
            ),
            Detail {
                ledger: Some(p.name.clone()),
                amount: Some(p.closing),
                suggestion: Some("Check if it was renamed or merged.".into()),
                ..Default::default()
            },
        );
    }
}
