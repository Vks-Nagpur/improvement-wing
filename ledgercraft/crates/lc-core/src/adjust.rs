//! Manual adjustments: journal entries the CA passes on top of the imported
//! books (rectification, provisions, audit adjustments, reclassification).
//!
//! The imported books are never edited. Adjustments are applied to a copy of
//! the trial balance; when the books have a day book, each adjustment is also
//! added as a voucher dated the last day of the year, so every check, the
//! capital movement and the ageing see the same figures as the statements.

use crate::model::{norm_name, Engagement, Ledger, Voucher, VoucherLine};
use crate::money::Money;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjKind {
    /// Correcting a wrong entry in the books.
    Rectification,
    /// Provision or accrual not booked (audit fee, expenses payable …).
    Provision,
    /// Adjustment proposed by the auditor.
    #[default]
    Audit,
    /// Moving a balance from one ledger to another.
    Reclassification,
    /// Closing stock value changed.
    ClosingStock,
}

impl AdjKind {
    pub fn label(self) -> &'static str {
        match self {
            AdjKind::Rectification => "Rectification",
            AdjKind::Provision => "Provision / accrual",
            AdjKind::Audit => "Audit adjustment",
            AdjKind::Reclassification => "Reclassification",
            AdjKind::ClosingStock => "Closing stock",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AdjLine {
    pub ledger: String,
    /// Dr positive, Cr negative.
    pub amount: Money,
    /// Group for a ledger that is not in the books yet (it is created).
    pub new_group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Adjustment {
    pub id: u32,
    pub kind: AdjKind,
    pub narration: String,
    pub lines: Vec<AdjLine>,
    /// Switched-off entries are kept (and listed) but not applied.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Applied {
    pub id: u32,
    pub ledger: String,
    pub before: Money,
    pub after: Money,
    pub created: bool,
}

/// Check one entry without applying it. `tb` is the trial balance as imported.
pub fn validate(a: &Adjustment, eng: &Engagement) -> Result<(), String> {
    if a.narration.trim().is_empty() {
        return Err("Write a narration (why this entry is passed).".into());
    }
    let lines: Vec<&AdjLine> = a.lines.iter().filter(|l| !l.amount.is_zero()).collect();
    if lines.is_empty() {
        return Err("Enter at least one amount.".into());
    }
    let idx = eng.cy.index();
    let mut total = Money::ZERO;
    for l in &lines {
        if l.ledger.trim().is_empty() {
            return Err("Every line needs a ledger.".into());
        }
        match idx.get(&norm_name(&l.ledger)) {
            Some(&i) if eng.cy.ledgers[i].closing_stock.is_some() => {} // single-sided
            Some(_) => total += l.amount,
            None => {
                if l.new_group.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(format!(
                        "'{}' is not in the books. Choose the group for this new ledger.",
                        l.ledger.trim()
                    ));
                }
                total += l.amount;
            }
        }
    }
    if !total.is_zero() {
        return Err(format!(
            "Debit and credit do not agree (difference {}).",
            total.fmt_drcr()
        ));
    }
    Ok(())
}

/// Apply all active adjustments to the engagement. Returns what changed.
pub fn apply(eng: &mut Engagement, adjs: &[Adjustment]) -> Result<Vec<Applied>, String> {
    let mut out = Vec::new();
    let with_vouchers = !eng.vouchers.is_empty();
    for a in adjs.iter().filter(|a| a.active) {
        validate(a, eng).map_err(|e| format!("Adjustment {}: {e}", a.id))?;
        let mut idx: HashMap<String, usize> = eng.cy.index();
        let mut vlines = Vec::new();
        for l in a.lines.iter().filter(|l| !l.amount.is_zero()) {
            let key = norm_name(&l.ledger);
            let (i, created) = match idx.get(&key) {
                Some(&i) => (i, false),
                None => {
                    eng.cy.ledgers.push(Ledger {
                        name: l.ledger.trim().to_string(),
                        group: l.new_group.clone().unwrap_or_default().trim().to_string(),
                        opening: Money::ZERO,
                        closing: Money::ZERO,
                        closing_stock: None,
                        tags: Vec::new(),
                    });
                    let i = eng.cy.ledgers.len() - 1;
                    idx.insert(key, i);
                    (i, true)
                }
            };
            let led = &mut eng.cy.ledgers[i];
            let (before, after) = match led.closing_stock.as_mut() {
                // Stock entered separately: only the closing value moves.
                Some(cs) => {
                    let b = *cs;
                    *cs += l.amount;
                    (b, *cs)
                }
                None => {
                    let b = led.closing;
                    led.closing += l.amount;
                    vlines.push(VoucherLine {
                        ledger: led.name.clone(),
                        amount: l.amount,
                    });
                    (b, led.closing)
                }
            };
            out.push(Applied {
                id: a.id,
                ledger: led.name.clone(),
                before,
                after,
                created,
            });
        }
        if with_vouchers && !vlines.is_empty() {
            eng.vouchers.push(Voucher {
                date: eng.fy_end,
                number: format!("ADJ-{}", a.id),
                vtype: "Adjustment".into(),
                narration: a.narration.clone(),
                lines: vlines,
            });
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EntityType, TrialBalance};
    use chrono::NaiveDate;

    fn eng() -> Engagement {
        let l = |n: &str, g: &str, c: i64| Ledger {
            name: n.into(),
            group: g.into(),
            opening: Money::ZERO,
            closing: Money(c),
            closing_stock: None,
            tags: vec![],
        };
        Engagement {
            entity_name: "T".into(),
            entity_type: EntityType::Firm,
            fy_start: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
            fy_end: NaiveDate::from_ymd_opt(2026, 3, 31).unwrap(),
            cy: TrialBalance {
                ledgers: vec![
                    l("Cash", "Cash-in-Hand", 10000),
                    l("Capital", "Capital Account", -10000),
                ],
                groups: vec![],
            },
            py: None,
            vouchers: vec![],
            mapping_memory: Default::default(),
            mapping_context: Default::default(),
            format_pack: None,
            consolidation: None,
            far: None,
            profit_sharing: vec![],
        }
    }

    fn adj(lines: Vec<(&str, i64, Option<&str>)>) -> Adjustment {
        Adjustment {
            id: 1,
            kind: AdjKind::Provision,
            narration: "Audit fee payable".into(),
            active: true,
            lines: lines
                .into_iter()
                .map(|(l, a, g)| AdjLine {
                    ledger: l.into(),
                    amount: Money(a),
                    new_group: g.map(|x| x.into()),
                })
                .collect(),
        }
    }

    #[test]
    fn creates_ledger_and_keeps_tb_balanced() {
        let mut e = eng();
        let a = adj(vec![
            ("Audit Fee", 5000, Some("Indirect Expenses")),
            ("Audit Fee Payable", -5000, Some("Provisions")),
        ]);
        let r = apply(&mut e, &[a]).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|x| x.created));
        let sum: Money = e.cy.ledgers.iter().map(|l| l.closing).sum();
        assert!(sum.is_zero());
    }

    #[test]
    fn rejects_unbalanced_unknown_and_blank() {
        let e = eng();
        assert!(validate(&adj(vec![("Cash", 100, None), ("Capital", -90, None)]), &e).is_err());
        assert!(validate(&adj(vec![("Cash", 100, None), ("Nowhere", -100, None)]), &e).is_err());
        let mut a = adj(vec![("Cash", 100, None), ("Capital", -100, None)]);
        a.narration = " ".into();
        assert!(validate(&a, &e).is_err());
    }

    #[test]
    fn inactive_is_ignored_and_vouchers_added_only_with_day_book() {
        let mut e = eng();
        let mut a = adj(vec![("Cash", 100, None), ("Capital", -100, None)]);
        a.active = false;
        assert!(apply(&mut e, &[a.clone()]).unwrap().is_empty());
        a.active = true;
        apply(&mut e, &[a.clone()]).unwrap();
        assert!(e.vouchers.is_empty());
        let mut e2 = eng();
        e2.vouchers.push(Voucher {
            date: e2.fy_start,
            number: "1".into(),
            vtype: "Journal".into(),
            narration: String::new(),
            lines: vec![],
        });
        apply(&mut e2, &[a]).unwrap();
        assert_eq!(e2.vouchers.len(), 2);
        assert_eq!(e2.vouchers[1].number, "ADJ-1");
    }
}
