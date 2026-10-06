//! Loan register (Form 3CD clause 31 helper) and cash-loan limit tests.
//!
//! Interpretation built in:
//! * Interest credited to a lender's account is NOT a loan accepted; TDS on that
//!   interest is NOT a repayment. Only principal counts as "loan accepted".
//! * Acceptance test (s.269SS): the loan accepted, or that loan plus earlier
//!   unpaid principal from the same person, is ₹20,000 or more.
//! * Repayment test (s.269T): the repayment, or the balance outstanding
//!   including interest, is ₹20,000 or more.
//! * Loans from banks / ledgers tagged `exempt` are listed but not tested.

use super::{rs, Ctx, Detail, Findings};
use crate::groups::{Class, Nature};
use crate::mapping::has_stem;
use crate::model::norm_name;
use crate::money::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LoanRow {
    pub ledger: String,
    pub group: String,
    pub exempt: bool,
    /// Balances as positive = amount owed by the entity.
    pub opening: Money,
    pub accepted_bank: Money,
    pub accepted_cash: Money,
    pub accepted_journal: Money,
    pub interest_credited: Money,
    pub tds_on_interest: Money,
    pub repaid_bank: Money,
    pub repaid_cash: Money,
    pub repaid_journal: Money,
    pub closing: Money,
    pub max_outstanding: Money,
    pub squared_up: bool,
}

impl LoanRow {
    pub fn principal_accepted(&self) -> Money {
        self.accepted_bank + self.accepted_cash + self.accepted_journal
    }
    pub fn repaid(&self) -> Money {
        self.repaid_bank + self.repaid_cash + self.repaid_journal
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    Bank,
    Cash,
    Interest,
    Tds,
    Journal,
}

pub fn run(ctx: &Ctx, f: &mut Findings) -> Vec<LoanRow> {
    let eng = ctx.eng;
    let loans: Vec<usize> = (0..eng.cy.ledgers.len())
        .filter(|&i| ctx.is_loan_taken(i))
        .collect();
    let mut rows = Vec::new();
    let limit = Money(ctx.threshold().cash_loan_paise);

    for &li in &loans {
        let l = ctx.ledger(li);
        let exempt = ctx.loan_exempt(li);
        let mut row = LoanRow {
            ledger: l.name.clone(),
            group: l.group.clone(),
            exempt,
            opening: -l.opening,
            ..Default::default()
        };
        let mut total = -l.opening; // owed incl. interest
        let mut principal = total.max(Money::ZERO);
        let mut max_out = total;

        for &k in &ctx.postings[li] {
            let v = &eng.vouchers[k];
            let idxs = &ctx.line_idx[k];
            let amt: Money = v
                .lines
                .iter()
                .zip(idxs)
                .filter(|(_, x)| **x == Some(li))
                .map(|(ln, _)| ln.amount)
                .sum();
            if amt.is_zero() {
                continue;
            }
            let mode = counterpart_mode(ctx, k, li);
            if amt.is_cr() {
                let a = -amt;
                match mode {
                    Mode::Interest => row.interest_credited += a,
                    m => {
                        match m {
                            Mode::Bank => row.accepted_bank += a,
                            Mode::Cash => row.accepted_cash += a,
                            _ => row.accepted_journal += a,
                        }
                        let aggregate = principal + a;
                        if !exempt
                            && matches!(m, Mode::Cash | Mode::Journal)
                            && (a >= limit || aggregate >= limit)
                        {
                            let code = if m == Mode::Cash {
                                "LOAN_ACCEPTED_CASH"
                            } else {
                                "LOAN_ACCEPTED_JOURNAL"
                            };
                            f.add(
                                code,
                                &format!("{} {}", v.key(), l.name),
                                &format!(
                                    "{}: {} accepted from '{}' ({}); unpaid principal before this {}, aggregate {}.",
                                    v.key(),
                                    rs(a),
                                    l.name,
                                    if m == Mode::Cash { "cash" } else { "journal entry" },
                                    rs(principal),
                                    rs(aggregate)
                                ),
                                Detail { ledger: Some(l.name.clone()), voucher: Some(v.key()), date: Some(v.date), amount: Some(a), suggestion: None },
                            );
                        }
                        principal = aggregate;
                    }
                }
                total += a;
            } else {
                let a = amt;
                match mode {
                    Mode::Tds => row.tds_on_interest += a,
                    m => {
                        match m {
                            Mode::Bank => row.repaid_bank += a,
                            Mode::Cash => row.repaid_cash += a,
                            _ => row.repaid_journal += a,
                        }
                        if !exempt && m == Mode::Cash && (a >= limit || total >= limit) {
                            f.add(
                                "LOAN_REPAID_CASH",
                                &format!("{} {}", v.key(), l.name),
                                &format!("{}: {} repaid in cash to '{}'; balance (with interest) before repayment {}.", v.key(), rs(a), l.name, rs(total)),
                                Detail { ledger: Some(l.name.clone()), voucher: Some(v.key()), date: Some(v.date), amount: Some(a), suggestion: None },
                            );
                        }
                        principal = (principal - a).max(Money::ZERO);
                    }
                }
                total -= a;
            }
            if total > max_out {
                max_out = total;
            }
        }
        row.closing = -l.closing;
        row.max_outstanding = max_out.max(Money::ZERO);
        let active = !row.opening.is_zero()
            || !row.principal_accepted().is_zero()
            || !row.interest_credited.is_zero();
        row.squared_up = active && row.closing.is_zero();
        rows.push(row);
    }
    rows
}

/// What is on the other side of the loan line in voucher `k`.
fn counterpart_mode(ctx: &Ctx, k: usize, loan: usize) -> Mode {
    let v = &ctx.eng.vouchers[k];
    let has = |pred: &dyn Fn(usize, Class, &str) -> bool| {
        v.lines.iter().zip(&ctx.line_idx[k]).any(|(_, x)| match x {
            Some(i) if *i != loan => ctx.classes[*i]
                .map(|c| pred(*i, c, &ctx.ledger(*i).name))
                .unwrap_or(false),
            _ => false,
        })
    };
    if has(&|_, c, n| c.nature() == Nature::Expense && has_stem(&norm_name(n), &["interest"])) {
        return Mode::Interest;
    }
    if has(&|_, c, n| c == Class::DutiesTaxes || has_stem(&norm_name(n), &["tds"])) {
        return Mode::Tds;
    }
    if has(&|_, c, _| c.is_cash()) {
        return Mode::Cash;
    }
    if has(&|_, c, _| c.is_bank()) {
        return Mode::Bank;
    }
    Mode::Journal
}
