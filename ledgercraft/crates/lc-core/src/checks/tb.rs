//! Trial-balance level checks.

use super::{rs, Ctx, Detail, Findings};
use crate::groups::{Class, Nature};
use crate::money::Money;

pub fn run(ctx: &Ctx, f: &mut Findings) {
    if let Some(c) = &ctx.eng.consolidation {
        if !c.inter_branch_difference.is_zero() {
            f.add(
                "BRANCH_NOT_ELIMINATED",
                "branches",
                &format!(
                    "Units: {}. Net difference {}.",
                    c.units.join(", "),
                    c.inter_branch_difference.fmt_drcr()
                ),
                Detail {
                    amount: Some(c.inter_branch_difference),
                    suggestion: Some("Reconcile the branch accounts in each set of books (transit items, missing entries) and import again.".into()),
                    ..Default::default()
                },
            );
        }
        for (ledger, list) in &c.group_conflicts {
            f.add(
                "BRANCH_GROUP_CONFLICT",
                ledger,
                &format!(
                    "'{ledger}': {}.",
                    list.iter()
                        .map(|(u, g)| format!("{u}: {g}"))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                Detail {
                    ledger: Some(ledger.clone()),
                    ..Default::default()
                },
            );
        }
    }

    // Ledger names must be unique within a year's books: two ledgers with
    // one name cannot be told apart in vouchers, mapping or last year.
    for (year, tb) in std::iter::once(("this year", &ctx.eng.cy))
        .chain(ctx.eng.py.iter().map(|p| ("last year", p)))
    {
        let mut seen: std::collections::BTreeMap<String, Vec<&str>> = Default::default();
        for l in &tb.ledgers {
            seen.entry(crate::model::norm_name(&l.name))
                .or_default()
                .push(&l.group);
        }
        for l in &tb.ledgers {
            let Some(groups) = seen.get(&crate::model::norm_name(&l.name)) else {
                continue;
            };
            if groups.len() > 1 {
                f.add(
                    "DUPLICATE_LEDGER",
                    &format!("{}:{year}", l.name),
                    &format!(
                        "'{}' appears {} times in {year}'s trial balance (groups: {}).",
                        l.name,
                        groups.len(),
                        groups.join(", ")
                    ),
                    Detail {
                        ledger: Some(l.name.clone()),
                        ..Default::default()
                    },
                );
                seen.remove(&crate::model::norm_name(&l.name));
            }
        }
    }

    let tb = &ctx.eng.cy;
    let total_closing: Money = tb.ledgers.iter().map(|l| l.closing).sum();
    if !total_closing.is_zero() {
        f.add(
            "TB_UNBALANCED",
            "closing",
            &format!(
                "Difference: {} {}.",
                rs(total_closing),
                if total_closing.is_dr() { "Dr" } else { "Cr" }
            ),
            Detail {
                amount: Some(total_closing),
                ..Default::default()
            },
        );
    }
    let total_opening: Money = tb.ledgers.iter().map(|l| l.opening).sum();
    if !total_opening.is_zero() {
        f.add(
            "OPENING_TOTAL_DIFF",
            "opening",
            &format!(
                "Difference: {} {}.",
                rs(total_opening),
                if total_opening.is_dr() { "Dr" } else { "Cr" }
            ),
            Detail {
                amount: Some(total_opening),
                ..Default::default()
            },
        );
    }

    for (i, l) in tb.ledgers.iter().enumerate() {
        let Some(class) = ctx.classes[i] else {
            f.add(
                "UNKNOWN_GROUP",
                &l.name,
                &format!("Ledger '{}' is in group '{}'.", l.name, l.group),
                Detail { ledger: Some(l.name.clone()), suggestion: Some("Add the group with its parent in the Groups sheet, or map the ledger manually.".into()), ..Default::default() },
            );
            continue;
        };
        let c = l.closing;
        if c.is_zero() {
            continue;
        }
        let d = || Detail {
            ledger: Some(l.name.clone()),
            amount: Some(c),
            ..Default::default()
        };
        match class {
            Class::SuspenseAc => f.add(
                "SUSPENSE_BALANCE",
                &l.name,
                &format!("'{}' shows {}.", l.name, c.fmt_drcr()),
                d(),
            ),
            Class::CashInHand if c.is_cr() => f.add(
                "CASH_CR_BALANCE",
                &l.name,
                &format!("'{}' shows {}.", l.name, c.fmt_drcr()),
                d(),
            ),
            Class::MiscExpensesAsset => f.add(
                "MISC_EXP_ASSET",
                &l.name,
                &format!("'{}' shows {}.", l.name, c.fmt_drcr()),
                d(),
            ),
            _ => {}
        }
        let abnormal = match class {
            Class::FixedAssets
            | Class::Investments
            | Class::DepositsAsset
            | Class::LoansAdvancesAsset
            | Class::StockInHand => c.is_cr(),
            Class::SecuredLoans
            | Class::UnsecuredLoans
            | Class::LoansLiability
            | Class::Provisions => c.is_dr(),
            _ => match class.nature() {
                Nature::Income => c.is_dr(),
                Nature::Expense => c.is_cr(),
                _ => false,
            },
        };
        if abnormal {
            f.add(
                "ABNORMAL_BALANCE",
                &l.name,
                &format!("'{}' ({}) shows {}.", l.name, class.label(), c.fmt_drcr()),
                Detail {
                    suggestion: Some("Check for wrong entries or wrong grouping.".into()),
                    ..d()
                },
            );
        }
    }
}
