//! Disclosures that are not in the books and must be entered by the user:
//! share capital particulars, shareholders, promoters, contingent
//! liabilities and commitments, related parties, MSME interest, accounting
//! policy wording and free notes. All amounts are exact paise.

use crate::money::Money;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Disclosures {
    /// Company only: one entry per class of shares.
    pub share_classes: Vec<ShareClass>,
    /// Shareholders holding more than 5% (company).
    pub holders_5pct: Vec<Holder>,
    /// Promoters' shareholding (company).
    pub promoters: Vec<Holder>,
    pub contingent_liabilities: Vec<AmountLine>,
    pub commitments: Vec<AmountLine>,
    pub related_parties: Vec<RelatedParty>,
    pub related_transactions: Vec<RelatedTxn>,
    pub msme: MsmeInterest,
    /// Replacement wording for a standard accounting policy, by its title.
    pub policy_text: BTreeMap<String, String>,
    /// Additional accounting policies, printed after the standard ones.
    pub extra_policies: Vec<FreeNote>,
    /// Additional notes (events after the balance sheet date, going concern …).
    pub notes: Vec<FreeNote>,
    /// Sections the user explicitly answered: section id → "nil" or "provided".
    /// A section with no answer and no particulars is *not answered*; it is
    /// never treated as nil (TRUTH-MODEL.md §2).
    pub answers: BTreeMap<String, String>,
    /// Sections carried from last year that the user has not looked at yet;
    /// they count as not answered until confirmed.
    pub pending_review: Vec<String>,
}

/// The three states of a required disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    NotAnswered,
    Nil,
    Provided,
}

/// Required disclosure sections: (id, label, companies only).
pub const SECTIONS: [(&str, &str, bool); 4] = [
    ("share_capital", "Share capital particulars", true),
    (
        "contingent",
        "Contingent liabilities and commitments",
        false,
    ),
    ("related_parties", "Related parties", false),
    (
        "msme",
        "Dues to micro and small enterprises (MSME status of suppliers)",
        false,
    ),
];

impl Disclosures {
    pub fn answer(&self, section: &str) -> Answer {
        if self.pending_review.iter().any(|x| x == section) {
            return Answer::NotAnswered;
        }
        let has = match section {
            "share_capital" => !self.share_classes.is_empty(),
            "contingent" => {
                self.contingent_liabilities
                    .iter()
                    .any(|l| !l.nature.trim().is_empty())
                    || self.commitments.iter().any(|l| !l.nature.trim().is_empty())
            }
            "related_parties" => self
                .related_parties
                .iter()
                .any(|p| !p.name.trim().is_empty()),
            "msme" => !self.msme.is_empty(),
            _ => false,
        };
        match self.answers.get(section).map(|s| s.as_str()) {
            _ if has => Answer::Provided,
            Some("nil") => Answer::Nil,
            // MSME: "provided" means the status of suppliers was checked and the
            // MSME creditors are tagged, even when no interest is due.
            Some("provided") if section == "msme" => Answer::Provided,
            _ => Answer::NotAnswered,
        }
    }

    /// Particulars for the next year: this year's figures become last year's,
    /// lists are kept to be updated, and every section must be answered again.
    pub fn roll_forward(&self) -> Disclosures {
        let mut d = self.clone();
        for c in d.share_classes.iter_mut() {
            c.py_authorised = c.authorised;
            c.py_issued = c.issued;
            c.py_subscribed = c.subscribed;
            c.added = 0;
            c.reduced = 0;
        }
        for h in d.holders_5pct.iter_mut().chain(d.promoters.iter_mut()) {
            h.py_shares = h.shares;
        }
        for l in d
            .contingent_liabilities
            .iter_mut()
            .chain(d.commitments.iter_mut())
        {
            l.py = l.cy;
            l.cy = Money::ZERO;
        }
        for t in d.related_transactions.iter_mut() {
            t.py = t.cy;
            t.cy = Money::ZERO;
        }
        let roll = |v: &mut (Money, Money)| *v = (Money::ZERO, v.0);
        let m = &mut d.msme;
        for v in [
            &mut m.interest_due_unpaid,
            &mut m.paid_beyond_appointed_day,
            &mut m.interest_paid_s16,
            &mut m.interest_due_for_delay,
            &mut m.interest_accrued_unpaid,
            &mut m.further_interest,
        ] {
            roll(v);
        }
        // Year-specific notes (events after the balance sheet date …) are not carried.
        d.notes.clear();
        d.answers.clear();
        d.pending_review = SECTIONS.iter().map(|(id, _, _)| id.to_string()).collect();
        d
    }

    /// Required sections still not answered for this kind of entity.
    pub fn missing(&self, is_company: bool) -> Vec<&'static str> {
        SECTIONS
            .iter()
            .filter(|(_, _, co)| is_company || !co)
            .filter(|(id, _, _)| self.answer(id) == Answer::NotAnswered)
            .map(|(_, label, _)| *label)
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShareClass {
    /// e.g. "Equity shares of ₹10 each".
    pub name: String,
    pub face_value: Money,
    /// Amount called up per share (equals face value when fully paid).
    pub paid_per_share: Money,
    pub authorised: i64,
    pub issued: i64,
    pub subscribed: i64,
    pub py_authorised: i64,
    pub py_issued: i64,
    pub py_subscribed: i64,
    /// Movement in shares outstanding during the year.
    pub added: i64,
    pub reduced: i64,
    /// Rights, preferences and restrictions (free text).
    pub rights: String,
}

impl ShareClass {
    pub fn paid(&self) -> Money {
        if self.paid_per_share.is_zero() {
            self.face_value
        } else {
            self.paid_per_share
        }
    }
    pub fn amount(&self, count: i64) -> Money {
        Money(self.face_value.0 * count)
    }
    pub fn paid_amount(&self, count: i64) -> Money {
        Money(self.paid().0 * count)
    }
    /// Shares outstanding at the beginning of the year.
    pub fn opening(&self) -> i64 {
        self.subscribed - self.added + self.reduced
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Holder {
    pub name: String,
    pub shares: i64,
    pub py_shares: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AmountLine {
    pub nature: String,
    pub cy: Money,
    pub py: Money,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelatedParty {
    pub name: String,
    /// e.g. "Key managerial personnel", "Relative of partner", "Entity controlled by a director".
    pub relationship: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelatedTxn {
    pub party: String,
    /// e.g. "Remuneration", "Purchase of goods", "Loan taken", "Balance payable at year end".
    pub nature: String,
    pub cy: Money,
    pub py: Money,
}

/// Disclosures under section 22 of the MSMED Act, 2006 (principal comes
/// from the creditors tagged MSME; the interest figures are entered).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MsmeInterest {
    pub interest_due_unpaid: (Money, Money),
    pub paid_beyond_appointed_day: (Money, Money),
    pub interest_paid_s16: (Money, Money),
    pub interest_due_for_delay: (Money, Money),
    pub interest_accrued_unpaid: (Money, Money),
    pub further_interest: (Money, Money),
}

impl MsmeInterest {
    pub fn is_empty(&self) -> bool {
        [
            self.interest_due_unpaid,
            self.paid_beyond_appointed_day,
            self.interest_paid_s16,
            self.interest_due_for_delay,
            self.interest_accrued_unpaid,
            self.further_interest,
        ]
        .iter()
        .all(|(a, b)| a.is_zero() && b.is_zero())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FreeNote {
    pub title: String,
    pub text: String,
}
