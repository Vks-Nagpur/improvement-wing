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
    /// "amount and date", "reference" (same cheque / UTR number) or "manual".
    #[serde(default)]
    pub method: String,
}

/// A possible split: one entry on one side equals two or three on the other.
/// Never matched automatically; the preparer confirms it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub book: Vec<BookLine>,
    pub bank: Vec<BankLine>,
    pub kind: String,
}

/// A match the preparer made: the book voucher and the statement line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManualMatch {
    pub voucher: String,
    pub bank_date: NaiveDate,
    pub amount: Money,
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
    /// Splits to look at (one entry against two or three on the other side).
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
    /// Cheques issued more than 90 days before the date and still not presented.
    #[serde(default)]
    pub stale: Vec<BookLine>,
    /// Statement lines that appear twice (same date, amount, reference, narration).
    #[serde(default)]
    pub duplicates: Vec<BankLine>,
    /// Places where a statement balance does not follow from the one before.
    #[serde(default)]
    pub balance_breaks: Vec<String>,
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
    manual: &[ManualMatch],
) -> Reconciliation {
    let bank: Vec<&BankLine> = bank.iter().filter(|b| b.date <= as_at).collect();
    let mut used_bank = vec![false; bank.len()];
    let mut matched = Vec::new();
    let mut open_book = Vec::new();
    // Tier 4 first: matches the preparer made.
    let mut book_rest: Vec<&BookLine> = Vec::new();
    for b in book {
        let m = manual
            .iter()
            .find(|m| m.voucher == b.voucher && m.amount == b.amount)
            .and_then(|m| {
                (0..bank.len()).find(|&i| {
                    !used_bank[i] && bank[i].date == m.bank_date && bank[i].amount == m.amount
                })
            });
        match m {
            Some(i) => {
                used_bank[i] = true;
                matched.push(Matched {
                    book: b.clone(),
                    bank: bank[i].clone(),
                    days_apart: (bank[i].date - b.date).num_days(),
                    method: "manual".into(),
                });
            }
            None => book_rest.push(b),
        }
    }
    let book: Vec<BookLine> = book_rest.into_iter().cloned().collect();
    let book = &book[..];
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
                    method: "amount and date".into(),
                });
            }
            None => open_book.push(b.clone()),
        }
    }
    // Tier 2: the same cheque / UTR number and amount, even outside the window.
    let mut still = Vec::new();
    for b in open_book {
        let bref = digits(&b.narration) + &digits(&b.voucher);
        let hit = (0..bank.len()).find(|&i| {
            let r = digits(&bank[i].reference);
            !used_bank[i]
                && bank[i].amount == b.amount
                && r.len() >= 6
                && bref.contains(&r)
                && (bank[i].date - b.date).num_days().abs() <= 60
        });
        match hit {
            Some(i) => {
                used_bank[i] = true;
                matched.push(Matched {
                    book: b.clone(),
                    bank: bank[i].clone(),
                    days_apart: (bank[i].date - b.date).num_days(),
                    method: "reference".into(),
                });
            }
            None => still.push(b),
        }
    }
    let open_book = still;
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
    // Tier 3 (suggestions only): one entry against two or three on the other side.
    let mut suggestions = Vec::new();
    let near = |a: NaiveDate, b: NaiveDate| (a - b).num_days().abs() <= window_days.max(1) * 2;
    for b in issued.iter().chain(&deposited) {
        let c: Vec<&BankLine> = credited
            .iter()
            .chain(&debited)
            .filter(|l| l.amount.0.signum() == b.amount.0.signum() && near(l.date, b.date))
            .take(30)
            .collect();
        if let Some(set) = subset_sum(&c.iter().map(|l| l.amount).collect::<Vec<_>>(), b.amount) {
            suggestions.push(Suggestion {
                book: vec![b.clone()],
                bank: set.iter().map(|&i| c[i].clone()).collect(),
                kind: "one book entry = several bank lines".into(),
            });
        }
    }
    for l in credited.iter().chain(&debited) {
        let c: Vec<&BookLine> = issued
            .iter()
            .chain(&deposited)
            .filter(|b| b.amount.0.signum() == l.amount.0.signum() && near(l.date, b.date))
            .take(30)
            .collect();
        if let Some(set) = subset_sum(&c.iter().map(|b| b.amount).collect::<Vec<_>>(), l.amount) {
            suggestions.push(Suggestion {
                book: set.iter().map(|&i| c[i].clone()).collect(),
                bank: vec![l.clone()],
                kind: "several book entries = one bank line".into(),
            });
        }
    }
    let stale: Vec<BookLine> = issued
        .iter()
        .filter(|b| (as_at - b.date).num_days() > 90)
        .cloned()
        .collect();
    let mut duplicates = Vec::new();
    for (i, a) in bank.iter().enumerate() {
        if bank[..i].iter().any(|b| {
            b.date == a.date
                && b.amount == a.amount
                && b.reference == a.reference
                && b.narration == a.narration
        }) {
            duplicates.push((*a).clone());
        }
    }
    let mut balance_breaks = Vec::new();
    for w in bank.windows(2) {
        if let (Some(p), Some(q)) = (w[0].balance, w[1].balance) {
            if p + w[1].amount != q {
                balance_breaks.push(format!(
                    "{}: balance {} does not follow from {} on {} plus {}",
                    w[1].date.format("%d-%m-%Y"),
                    q,
                    p,
                    w[0].date.format("%d-%m-%Y"),
                    w[1].amount
                ));
            }
        }
    }
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
        suggestions,
        stale,
        duplicates,
        balance_breaks,
    }
}

/// Two or three of `xs` (by index) that add up to `target`, if exactly one
/// such set exists (an ambiguous split is not suggested).
fn subset_sum(xs: &[Money], target: Money) -> Option<Vec<usize>> {
    let mut found: Vec<Vec<usize>> = Vec::new();
    let n = xs.len();
    for i in 0..n {
        for j in i + 1..n {
            if xs[i] + xs[j] == target {
                found.push(vec![i, j]);
            }
            for k in j + 1..n {
                if xs[i] + xs[j] + xs[k] == target {
                    found.push(vec![i, j, k]);
                }
            }
            if found.len() > 1 {
                return None;
            }
        }
    }
    if found.len() == 1 {
        found.pop()
    } else {
        None
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
        let r = reconcile("HDFC", book_balance, &book, &bank, d(3, 31), 7, &[]);
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
        let r = reconcile("Bank", Money(-500), &book, &bank, d(3, 31), 7, &[]);
        assert_eq!(r.matched[0].bank.reference, "654321");
        assert_eq!(r.debited_by_bank_only.len(), 1);
    }

    #[test]
    fn reference_split_manual_stale_duplicate_and_balance_checks() {
        // A cheque presented 40 days later (outside the window) matches by number.
        let book = vec![
            bk(d(1, 5), "Payment 9 chq 778899", -700),
            bk(d(3, 1), "Receipt 3", 3000),
            bk(d(3, 20), "Payment 12", -450),
        ];
        let bank = vec![
            st(d(2, 14), "778899", -700, Some(9300)),
            st(d(3, 2), "", 1000, Some(10300)),
            st(d(3, 3), "", 2000, Some(12300)),
            st(d(3, 25), "", -100, Some(12300)),
            st(d(3, 25), "", -100, Some(12100)),
        ];
        let r = reconcile("B", Money(0), &book, &bank, d(3, 31), 7, &[]);
        assert!(r.matched.iter().any(|m| m.method == "reference"));
        // 3,000 in the books = 1,000 + 2,000 in the bank: suggested, not matched.
        assert!(r
            .suggestions
            .iter()
            .any(|s| s.book.len() == 1 && s.bank.len() == 2));
        assert_eq!(
            r.deposited_not_cleared.len(),
            1,
            "a suggestion is not a match"
        );
        assert_eq!(r.duplicates.len(), 1);
        assert!(
            !r.balance_breaks.is_empty(),
            "12,300 then -100 cannot be 12,300"
        );
        // The preparer matches the 450 payment with one of the 100 lines? No: amounts differ, so
        // a manual match only applies when voucher, date and amount agree.
        let man = vec![ManualMatch {
            voucher: "Receipt 3".into(),
            bank_date: d(3, 2),
            amount: Money(3000),
        }];
        let r2 = reconcile("B", Money(0), &book, &bank, d(3, 31), 7, &man);
        assert!(
            r2.matched.iter().all(|m| m.method != "manual"),
            "no bank line of 3,000 on that date"
        );
        // Stale cheque: issued more than 90 days before.
        let r3 = reconcile(
            "B",
            Money(0),
            &[bk(d(1, 1), "Payment 1", -50)],
            &[],
            d(4, 30),
            7,
            &[],
        );
        assert_eq!(r3.stale.len(), 1);
    }
}
