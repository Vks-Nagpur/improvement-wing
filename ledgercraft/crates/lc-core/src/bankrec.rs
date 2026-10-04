//! Bank reconciliation: the bank ledger in the books against the bank's
//! statement. Entries are matched one to one on amount (and reference when
//! both sides have one), nearest date first, within a window of days. What
//! is left becomes the reconciliation statement; it must close to the
//! statement's balance, otherwise the difference is shown.

use crate::model::{norm_name, Engagement};
use crate::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// One line of a bank statement. `amount` is money into the account
/// (deposit) positive, money out (withdrawal) negative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankLine {
    pub date: NaiveDate,
    pub narration: String,
    #[serde(default)]
    pub reference: String,
    pub amount: Money,
    #[serde(default)]
    pub balance: Option<Money>,
}

/// One entry of the bank ledger in the books, in the same sign (Dr = into bank).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookLine {
    pub date: NaiveDate,
    pub voucher: String,
    pub narration: String,
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Matched {
    pub book: BookLine,
    pub bank: BankLine,
    pub days_apart: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reconciliation {
    pub ledger: String,
    pub as_at: Option<NaiveDate>,
    pub book_balance: Money,
    /// Closing balance shown by the statement (last line with a balance), if any.
    pub statement_balance: Option<Money>,
    pub matched: Vec<Matched>,
    /// Payments in the books not yet in the bank (cheques issued, not presented).
    pub issued_not_presented: Vec<BookLine>,
    /// Receipts in the books not yet in the bank (deposited, not cleared).
    pub deposited_not_cleared: Vec<BookLine>,
    /// Bank credits not in the books (interest, direct receipts).
    pub credited_by_bank_only: Vec<BankLine>,
    /// Bank debits not in the books (charges, direct debits).
    pub debited_by_bank_only: Vec<BankLine>,
    /// Balance per books adjusted for the open items: should equal the statement.
    pub computed_statement_balance: Money,
    /// Statement balance minus computed balance (zero when it reconciles).
    pub difference: Option<Money>,
}

/// Entries of a ledger in the day book, up to and including `as_at`.
pub fn book_lines(eng: &Engagement, ledger: &str, as_at: NaiveDate) -> Vec<BookLine> {
    let key = norm_name(ledger);
    let mut out = Vec::new();
    for v in eng.vouchers.iter().filter(|v| v.date <= as_at) {
        for l in v.lines.iter().filter(|l| norm_name(&l.ledger) == key) {
            out.push(BookLine {
                date: v.date,
                voucher: v.key(),
                narration: v.narration.clone(),
                amount: l.amount,
            });
        }
    }
    out
}

fn digits(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Match books and statement and prepare the reconciliation as at `as_at`.
pub fn reconcile(
    ledger: &str,
    book_balance: Money,
    book: &[BookLine],
    bank: &[BankLine],
    as_at: NaiveDate,
    window_days: i64,
) -> Reconciliation {
    let bank: Vec<&BankLine> = bank.iter().filter(|b| b.date <= as_at).collect();
    let mut used_bank = vec![false; bank.len()];
    let mut matched = Vec::new();
    let mut open_book = Vec::new();
    // Earlier book entries first; each takes the nearest-dated unused bank line
    // of the same amount (a matching cheque number wins over date).
    let mut order: Vec<&BookLine> = book.iter().collect();
    order.sort_by_key(|b| b.date);
    for b in order {
        let bref = digits(&b.narration) + &digits(&b.voucher);
        let best = bank
            .iter()
            .enumerate()
            .filter(|(i, l)| !used_bank[*i] && l.amount == b.amount)
            .filter(|(_, l)| {
                (l.date - b.date).num_days() >= -window_days.min(3)
                    && (l.date - b.date).num_days() <= window_days
            })
            .min_by_key(|(_, l)| {
                let r = digits(&l.reference);
                let same_ref = !r.is_empty() && r.len() >= 4 && bref.contains(&r);
                (!same_ref, (l.date - b.date).num_days().abs())
            })
            .map(|(i, _)| i);
        match best {
            Some(i) => {
                used_bank[i] = true;
                matched.push(Matched {
                    book: b.clone(),
                    bank: bank[i].clone(),
                    days_apart: (bank[i].date - b.date).num_days(),
                });
            }
            None => open_book.push(b.clone()),
        }
    }
    let open_bank: Vec<BankLine> = bank
        .iter()
        .enumerate()
        .filter(|(i, _)| !used_bank[*i])
        .map(|(_, l)| (*l).clone())
        .collect();
    let (issued, deposited): (Vec<BookLine>, Vec<BookLine>) =
        open_book.into_iter().partition(|b| b.amount.is_cr());
    let (credited, debited): (Vec<BankLine>, Vec<BankLine>) =
        open_bank.into_iter().partition(|l| l.amount.0 > 0);
    let sum_b = |v: &[BookLine]| v.iter().map(|x| x.amount).sum::<Money>();
    let sum_s = |v: &[BankLine]| v.iter().map(|x| x.amount).sum::<Money>();
    // Bank balance = books − book items not in bank + bank items not in books.
    let computed =
        book_balance - sum_b(&issued) - sum_b(&deposited) + sum_s(&credited) + sum_s(&debited);
    let statement_balance = bank.iter().rev().find_map(|l| l.balance);
    Reconciliation {
        ledger: ledger.to_string(),
        as_at: Some(as_at),
        book_balance,
        statement_balance,
        matched,
        issued_not_presented: issued,
        deposited_not_cleared: deposited,
        credited_by_bank_only: credited,
        debited_by_bank_only: debited,
        computed_statement_balance: computed,
        difference: statement_balance.map(|s| s - computed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, day).unwrap()
    }
    fn bk(date: NaiveDate, v: &str, a: i64) -> BookLine {
        BookLine {
            date,
            voucher: v.into(),
            narration: String::new(),
            amount: Money(a),
        }
    }
    fn st(date: NaiveDate, r: &str, a: i64, bal: Option<i64>) -> BankLine {
        BankLine {
            date,
            narration: "x".into(),
            reference: r.into(),
            amount: Money(a),
            balance: bal.map(Money),
        }
    }

    #[test]
    fn open_items_explain_the_difference() {
        // Opening 10,000 in both. Books: receipt 5,000 (cleared), payment 2,000
        // (cheque not presented), receipt 1,000 on 31 March (not cleared).
        // Bank: also charges 100 and interest 50 not in the books.
        let book = vec![
            bk(d(3, 2), "Receipt 1", 5000),
            bk(d(3, 10), "Payment 7 chq 123456", -2000),
            bk(d(3, 31), "Receipt 2", 1000),
        ];
        let bank = vec![
            st(d(3, 3), "", 5000, Some(15000)),
            st(d(3, 25), "", -100, Some(14900)),
            st(d(3, 31), "", 50, Some(14950)),
        ];
        let book_balance = Money(10000 + 5000 - 2000 + 1000);
        let r = reconcile("HDFC", book_balance, &book, &bank, d(3, 31), 7);
        assert_eq!(r.matched.len(), 1);
        assert_eq!(r.issued_not_presented.len(), 1);
        assert_eq!(r.deposited_not_cleared.len(), 1);
        assert_eq!(r.debited_by_bank_only.len(), 1);
        assert_eq!(r.credited_by_bank_only.len(), 1);
        assert_eq!(r.computed_statement_balance, Money(14950));
        assert_eq!(r.difference, Some(Money::ZERO));
    }

    #[test]
    fn same_amount_prefers_cheque_number_then_nearest_date() {
        let book = vec![bk(d(3, 10), "Payment chq 654321", -500)];
        let bank = vec![
            st(d(3, 11), "000111", -500, None),
            st(d(3, 14), "654321", -500, None),
        ];
        let r = reconcile("Bank", Money(-500), &book, &bank, d(3, 31), 7);
        assert_eq!(r.matched[0].bank.reference, "654321");
        assert_eq!(r.debited_by_bank_only.len(), 1);
    }
}
