//! Wrong-grouping checks based on ledger names (suggestions only).

use super::{looks_like_loan, Ctx, Detail, Findings};
use crate::groups::Class;
use crate::mapping::{has_stem, has_word};
use crate::model::norm_name;

pub fn run(ctx: &Ctx, f: &mut Findings) {
    for (i, l) in ctx.eng.cy.ledgers.iter().enumerate() {
        let Some(class) = ctx.classes[i] else {
            continue;
        };
        let n = norm_name(&l.name);
        let d = |s: &str| Detail {
            ledger: Some(l.name.clone()),
            amount: Some(l.closing),
            suggestion: Some(s.to_string()),
            ..Default::default()
        };

        if matches!(
            class,
            Class::SundryCreditors
                | Class::SundryDebtors
                | Class::CurrentLiabilities
                | Class::CurrentAssets
        ) && looks_like_loan(&l.name)
        {
            let taken = l.closing.is_cr()
                || has_word(&n, &["from"])
                || matches!(class, Class::SundryCreditors | Class::CurrentLiabilities);
            let (what, sug) = if taken {
                ("loan taken", "Move to Unsecured Loans / Secured Loans (Borrowings). Cash-loan limits are already being tested on it.")
            } else {
                ("loan given", "Move to Loans & Advances (Asset).")
            };
            f.add(
                "MISGROUP_LOAN",
                &l.name,
                &format!(
                    "'{}' looks like a {} but is under '{}'.",
                    l.name,
                    what,
                    class.label()
                ),
                d(sug),
            );
        }

        if class == Class::BankAccounts
            && (has_stem(&n, &["overdraft", "cash credit"]) || has_word(&n, &["od", "cc", "occ"]))
        {
            f.add(
                "MISGROUP_OD",
                &l.name,
                &format!("'{}' is under Bank Accounts.", l.name),
                d("Move to Bank OD A/c (shown under Short-term Borrowings)."),
            );
        }

        if matches!(class, Class::SundryCreditors | Class::SundryDebtors)
            && (has_word(
                &n,
                &["gst", "cgst", "sgst", "igst", "utgst", "tds", "tcs", "cess"],
            ) || has_stem(&n, &["professional tax", "goods and services tax"]))
        {
            f.add(
                "MISGROUP_TAX",
                &l.name,
                &format!("'{}' is under '{}'.", l.name, class.label()),
                d("Move to Duties & Taxes."),
            );
        }

        if matches!(class, Class::DirectExpenses | Class::IndirectExpenses)
            && l.closing.abs().paise() >= ctx.threshold().capital_item_in_expense_paise
            && has_stem(
                &n,
                &[
                    "laptop",
                    "computer",
                    "printer",
                    "furniture",
                    "air condition",
                    "machinery",
                    "vehicle",
                    "motor car",
                    "building",
                    "mobile phone",
                    "generator",
                    "inverter",
                    "equipment",
                ],
            )
            && !has_stem(
                &n,
                &[
                    "repair",
                    "maint",
                    "amc",
                    "rent",
                    "hire",
                    "insurance",
                    "fuel",
                    "petrol",
                    "diesel",
                    "running",
                    "consumable",
                    "stationery",
                    "subscription",
                    "upkeep",
                    "service",
                ],
            )
        {
            f.add(
                "CAPITAL_IN_EXPENSE",
                &l.name,
                &format!("'{}' ({}) shows {}.", l.name, class.label(), l.closing.fmt_drcr()),
                d("If it gives benefit for more than a year, move to Fixed Assets and charge depreciation."),
            );
        }
    }
}
