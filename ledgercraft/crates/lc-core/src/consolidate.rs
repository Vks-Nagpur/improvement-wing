//! Branch consolidation: head office and branch books (each its own trial
//! balance and day book) are added ledger by ledger. Inter-branch accounts
//! (group "Branch / Divisions") must cancel out; whatever does not cancel is
//! kept and reported (TRUTH-MODEL.md §7: it blocks a final copy).

use crate::groups::{Class, GroupResolver};
use crate::model::{norm_name, Ledger, TrialBalance, Voucher};
use crate::money::Money;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Ledger that replaces inter-branch accounts once they cancel out.
pub const ELIMINATED: &str = "Inter-branch accounts (eliminated)";

/// One set of books: head office or a branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub name: String,
    pub tb: TrialBalance,
    #[serde(default)]
    pub vouchers: Vec<Voucher>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MergeNotes {
    /// Units combined, in order.
    pub units: Vec<String>,
    /// Ledgers grouped differently in different units: (ledger, [(unit, group)]).
    pub group_conflicts: Vec<(String, Vec<(String, String)>)>,
    /// Inter-branch ledgers that cancelled out and were removed.
    pub eliminated: Vec<String>,
    /// Net of inter-branch accounts that did not cancel (zero when consolidated cleanly).
    pub inter_branch_difference: Money,
}

/// Combine the units. The first unit is the head office; its group names win
/// when the same ledger is grouped differently elsewhere.
pub fn merge(units: &[Unit]) -> (TrialBalance, Vec<Voucher>, MergeNotes) {
    let mut notes = MergeNotes {
        units: units.iter().map(|u| u.name.clone()).collect(),
        ..Default::default()
    };
    let mut order: Vec<String> = Vec::new();
    let mut by_key: BTreeMap<String, Ledger> = BTreeMap::new();
    let mut seen_groups: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut groups: Vec<(String, String)> = Vec::new();
    for u in units {
        for g in &u.tb.groups {
            if !groups.iter().any(|x| norm_name(&x.0) == norm_name(&g.0)) {
                groups.push(g.clone());
            }
        }
        for l in &u.tb.ledgers {
            let key = norm_name(&l.name);
            seen_groups
                .entry(key.clone())
                .or_default()
                .push((u.name.clone(), l.group.clone()));
            match by_key.get_mut(&key) {
                Some(acc) => {
                    acc.opening += l.opening;
                    acc.closing += l.closing;
                    acc.closing_stock = match (acc.closing_stock, l.closing_stock) {
                        (None, None) => None,
                        (a, b) => Some(a.unwrap_or_default() + b.unwrap_or_default()),
                    };
                    for t in &l.tags {
                        if !acc.tags.contains(t) {
                            acc.tags.push(t.clone());
                        }
                    }
                }
                None => {
                    order.push(key.clone());
                    by_key.insert(key, l.clone());
                }
            }
        }
    }
    for (key, list) in &seen_groups {
        let mut distinct: Vec<&str> = list.iter().map(|(_, g)| g.as_str()).collect();
        distinct.sort_by_key(|g| norm_name(g));
        distinct.dedup_by_key(|g| norm_name(g));
        if distinct.len() > 1 {
            notes
                .group_conflicts
                .push((by_key[key].name.clone(), list.clone()));
        }
    }
    let mut tb = TrialBalance {
        ledgers: order.iter().map(|k| by_key[k].clone()).collect(),
        groups,
    };
    // Inter-branch accounts: drop them when they cancel in total.
    let res = GroupResolver::new(&tb);
    let branch: Vec<usize> = tb
        .ledgers
        .iter()
        .enumerate()
        .filter(|(_, l)| res.resolve(&l.group).ok() == Some(Class::BranchDivisions))
        .map(|(i, _)| i)
        .collect();
    let net: Money = branch.iter().map(|&i| tb.ledgers[i].closing).sum();
    let open_net: Money = branch.iter().map(|&i| tb.ledgers[i].opening).sum();
    notes.inter_branch_difference = net;
    if net.is_zero() && open_net.is_zero() && units.len() > 1 {
        notes.eliminated = branch.iter().map(|&i| tb.ledgers[i].name.clone()).collect();
        let drop: Vec<String> = notes.eliminated.iter().map(|n| norm_name(n)).collect();
        let group = tb.ledgers[branch[0]].group.clone();
        tb.ledgers.retain(|l| !drop.contains(&norm_name(&l.name)));
        // One ledger stands in for them (nil balance) so that the vouchers
        // which moved money between units still balance and tie to the books.
        tb.ledgers.push(Ledger {
            name: ELIMINATED.into(),
            group,
            opening: Money::ZERO,
            closing: Money::ZERO,
            closing_stock: None,
            tags: vec![],
        });
    }
    // Day books: kept unit by unit; voucher numbers carry the unit name so
    // duplicates are not reported across branches.
    let mut vouchers = Vec::new();
    for (k, u) in units.iter().enumerate() {
        for v in &u.vouchers {
            let mut v = v.clone();
            if k > 0 {
                v.number = format!("{}/{}", u.name, v.number);
            }
            for l in v.lines.iter_mut() {
                if notes
                    .eliminated
                    .iter()
                    .any(|e| norm_name(e) == norm_name(&l.ledger))
                {
                    l.ledger = ELIMINATED.into();
                }
            }
            vouchers.push(v);
        }
    }
    (tb, vouchers, notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(name: &str, group: &str, open: i64, close: i64) -> Ledger {
        Ledger {
            name: name.into(),
            group: group.into(),
            opening: Money(open),
            closing: Money(close),
            closing_stock: None,
            tags: vec![],
        }
    }

    fn unit(name: &str, ledgers: Vec<Ledger>) -> Unit {
        Unit {
            name: name.into(),
            tb: TrialBalance {
                ledgers,
                groups: vec![],
            },
            vouchers: vec![],
        }
    }

    #[test]
    fn adds_ledgers_and_eliminates_branch_accounts() {
        let ho = unit(
            "Head office",
            vec![
                l("Cash", "Cash-in-Hand", 0, 1000),
                l("Pune Branch", "Branch / Divisions", 0, 5000),
                l("Capital", "Capital Account", 0, -6000),
            ],
        );
        let br = unit(
            "Pune",
            vec![
                l("Cash", "Cash-in-Hand", 0, 2000),
                l("Head Office A/c", "Branch / Divisions", 0, -5000),
                l("Sales", "Sales Accounts", 0, -3000),
                l("Rent", "Indirect Expenses", 0, 6000),
            ],
        );
        let (tb, _, notes) = merge(&[ho, br]);
        assert!(notes.inter_branch_difference.is_zero());
        assert_eq!(notes.eliminated.len(), 2);
        assert!(!tb
            .ledgers
            .iter()
            .any(|l| l.group == "Branch / Divisions" && l.name != ELIMINATED));
        let cash = tb.ledgers.iter().find(|l| l.name == "Cash").unwrap();
        assert_eq!(cash.closing, Money(3000));
        let total: Money = tb.ledgers.iter().map(|l| l.closing).sum();
        assert!(total.is_zero());
    }

    #[test]
    fn reports_differences_and_group_conflicts() {
        let ho = unit(
            "Head office",
            vec![
                l("Pune Branch", "Branch / Divisions", 0, 5000),
                l("Rent", "Indirect Expenses", 0, 100),
            ],
        );
        let br = unit(
            "Pune",
            vec![
                l("Head Office A/c", "Branch / Divisions", 0, -4500),
                l("Rent", "Direct Expenses", 0, 50),
            ],
        );
        let (tb, _, notes) = merge(&[ho, br]);
        assert_eq!(notes.inter_branch_difference, Money(500));
        assert!(notes.eliminated.is_empty());
        assert_eq!(
            tb.ledgers
                .iter()
                .filter(|l| l.group == "Branch / Divisions")
                .count(),
            2
        );
        assert_eq!(notes.group_conflicts.len(), 1);
        assert_eq!(notes.group_conflicts[0].0, "Rent");
    }
}
