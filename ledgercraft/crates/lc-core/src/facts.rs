//! Figures for one year, grouped by statement head and ledger, in display
//! sign (assets/expenses Dr positive; liabilities/capital/income Cr positive).
//! `rounded()` produces the printed figures: every ledger line is rounded so
//! that the trial-balance identity still holds, hence the Balance Sheet tallies
//! and every note adds up to its face figure after rounding.

use crate::groups::{Class, Nature};
use crate::mapping::Head;
use crate::money::Money;
use crate::rounding::round_to_target;
use crate::statements::MappedLedger;
use crate::units::round_to;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactLine {
    pub name: String,
    pub class: Option<Class>,
    pub tags: Vec<String>,
    /// Display sign.
    pub amount: Money,
}

impl FactLine {
    pub fn has_tag(&self, t: &str) -> bool {
        self.tags.iter().any(|x| x.eq_ignore_ascii_case(t))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct YearFacts {
    pub lines: BTreeMap<Head, Vec<FactLine>>,
    pub opening_stock: Money,
    pub closing_stock: Money,
    pub change_in_inventories: Money,
    pub profit_to: Head,
}

pub fn display_sign(head: Head, signed: Money) -> Money {
    match head.nature() {
        Nature::Asset | Nature::Expense => signed,
        _ => -signed,
    }
}

impl YearFacts {
    pub fn build(mapped: &[MappedLedger], profit_to: Head) -> YearFacts {
        let mut lines: BTreeMap<Head, Vec<FactLine>> = BTreeMap::new();
        let mut opening_stock = Money::ZERO;
        let mut closing_stock = Money::ZERO;
        for m in mapped {
            let Some(h) = m.head else { continue };
            if m.class == Some(Class::StockInHand) {
                opening_stock += m.tb_closing;
                closing_stock += m.amount;
            }
            lines.entry(h).or_default().push(FactLine {
                name: m.name.clone(),
                class: m.class,
                tags: m.tags.clone(),
                amount: display_sign(h, m.amount),
            });
        }
        YearFacts {
            lines,
            opening_stock,
            closing_stock,
            change_in_inventories: opening_stock - closing_stock,
            profit_to,
        }
    }

    /// Sum of ledger lines of a head (without derived items).
    pub fn lines_total(&self, h: Head) -> Money {
        self.lines
            .get(&h)
            .map(|v| v.iter().map(|l| l.amount).sum())
            .unwrap_or_default()
    }

    pub fn income(&self) -> Money {
        Head::ALL
            .iter()
            .filter(|h| h.nature() == Nature::Income)
            .map(|h| self.lines_total(*h))
            .sum()
    }

    pub fn expenses_ex_tax(&self) -> Money {
        Head::ALL
            .iter()
            .filter(|h| h.nature() == Nature::Expense && **h != Head::TaxExpense)
            .map(|h| self.lines_total(*h))
            .sum::<Money>()
            + self.change_in_inventories
    }

    pub fn pbt(&self) -> Money {
        self.income() - self.expenses_ex_tax()
    }

    pub fn profit(&self) -> Money {
        self.pbt() - self.lines_total(Head::TaxExpense)
    }

    /// Face figure of a head, including derived items.
    pub fn head(&self, h: Head) -> Money {
        let mut v = self.lines_total(h);
        if h == Head::ChangeInInventories {
            v += self.change_in_inventories;
        }
        if h == self.profit_to {
            v += self.profit();
        }
        v
    }

    pub fn total_assets(&self) -> Money {
        Head::ALL
            .iter()
            .filter(|h| h.nature() == Nature::Asset)
            .map(|h| self.head(*h))
            .sum()
    }

    pub fn total_equity_liabilities(&self) -> Money {
        Head::ALL
            .iter()
            .filter(|h| matches!(h.nature(), Nature::Liability | Nature::Capital))
            .map(|h| self.head(*h))
            .sum()
    }

    /// Printed figures rounded to step `g` paise, keeping every identity exact.
    pub fn rounded(&self, g: i64) -> YearFacts {
        if g <= 1 {
            return self.clone();
        }
        // Leaves in trial-balance sign (Dr +): they sum to zero for balanced books.
        let mut keys: Vec<(Head, usize)> = Vec::new();
        let mut signed: Vec<Money> = Vec::new();
        for (h, v) in &self.lines {
            for (i, l) in v.iter().enumerate() {
                keys.push((*h, i));
                signed.push(display_sign(*h, l.amount));
            }
        }
        signed.push(self.change_in_inventories); // expense: Dr +
        let total: Money = signed.iter().copied().sum();
        let r = round_to_target(&signed, g, round_to(total, g));
        let mut out = self.clone();
        for (k, (h, i)) in keys.iter().enumerate() {
            out.lines.get_mut(h).unwrap()[*i].amount = display_sign(*h, r[k]);
        }
        out.change_in_inventories = *r.last().unwrap();
        out.closing_stock = out
            .lines
            .values()
            .flatten()
            .filter(|l| l.class == Some(Class::StockInHand))
            .map(|l| l.amount)
            .sum();
        out.opening_stock = out.change_in_inventories + out.closing_stock;
        out
    }
}
