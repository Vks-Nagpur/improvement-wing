//! Typed facts about a ledger beyond its statement line (LC-P1-023 groundwork).
//!
//! A statement line (`Head`) says where a balance is printed. Disclosures need
//! more: is it current or non-current, secured, owed to an MSME, disputed, a
//! related party, cash, a statutory due, which party, which branch. Each fact
//! records where it came from. When the books do not say, the fact is
//! `Unknown`; it is never guessed into a value (TRUTH-MODEL.md: three-state
//! disclosures).

use crate::groups::Class;
use crate::mapping::{Head, MapStatus};
use crate::statements::MappedLedger;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// The books do not say.
    #[default]
    Unknown,
    /// From the group the ledger sits in.
    Group,
    /// From the ledger name.
    Name,
    /// From a tag on the ledger (imported or set by the user).
    Tag,
    /// Chosen or confirmed by the user.
    User,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Fact<T> {
    pub value: Option<T>,
    pub basis: Basis,
    /// Plain words: where the value comes from, or what is missing.
    pub evidence: String,
}

impl<T> Fact<T> {
    fn known(value: T, basis: Basis, evidence: impl Into<String>) -> Self {
        Fact {
            value: Some(value),
            basis,
            evidence: evidence.into(),
        }
    }
    fn unknown(evidence: impl Into<String>) -> Self {
        Fact {
            value: None,
            basis: Basis::Unknown,
            evidence: evidence.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Maturity {
    Current,
    NonCurrent,
}

/// Facts about one ledger. `None` means the fact does not apply to this
/// kind of ledger; `Some(Fact { value: None, .. })` means it applies but the
/// books do not say.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Dims {
    pub maturity: Option<Fact<Maturity>>,
    pub secured: Option<Fact<bool>>,
    pub msme: Option<Fact<bool>>,
    pub disputed: Option<Fact<bool>>,
    pub related_party: Option<Fact<bool>>,
    pub cash: Option<Fact<bool>>,
    pub statutory_due: Option<Fact<bool>>,
    pub counterparty: Option<Fact<String>>,
    pub branch: Option<Fact<String>>,
}

/// The current / non-current pair a head belongs to, if any.
pub fn maturity_of(h: Head) -> Option<Maturity> {
    use Head::*;
    match h {
        LtBorrowings | OtherLtLiabilities | LtProvisions | NcInvestments | LtLoansAdvances
        | OtherNcAssets => Some(Maturity::NonCurrent),
        StBorrowings | StProvisions | StLoansAdvances | CurrentInvestments => {
            Some(Maturity::Current)
        }
        _ => None,
    }
}

/// Plain-words evidence for a current / non-current placement.
pub fn maturity_evidence(m: &MappedLedger, which: Maturity) -> String {
    let w = match which {
        Maturity::Current => "current",
        Maturity::NonCurrent => "non-current",
    };
    match m.status {
        MapStatus::Confirmed => format!(
            "You confirmed it as {w} (group '{}'). The books do not record due dates: re-check it each year, and split any part due within 12 months.",
            m.group
        ),
        _ => format!(
            "Shown as {w} because of the group '{}'. The books do not say when it falls due: confirm it, and split any part due within 12 months.",
            m.group
        ),
    }
}

fn tag(m: &MappedLedger, t: &str) -> bool {
    m.tags.iter().any(|x| x.eq_ignore_ascii_case(t))
}

/// Facts for one mapped ledger. `branch` is the unit the ledger came from in
/// combined books.
pub fn dims(m: &MappedLedger, branch: Option<&str>) -> Dims {
    let mut d = Dims::default();
    let head = m.head;
    if let Some(h) = head {
        if let Some(which) = maturity_of(h) {
            let basis = if m.status == MapStatus::Confirmed {
                Basis::User
            } else {
                Basis::Group
            };
            d.maturity = Some(Fact::known(which, basis, maturity_evidence(m, which)));
        }
    }
    let borrowing = matches!(head, Some(Head::LtBorrowings | Head::StBorrowings));
    if borrowing {
        d.secured = Some(match m.class {
            Some(Class::SecuredLoans) => {
                Fact::known(true, Basis::Group, "In the group Secured Loans.")
            }
            Some(Class::UnsecuredLoans) => {
                Fact::known(false, Basis::Group, "In the group Unsecured Loans.")
            }
            Some(Class::BankOdAc) => {
                Fact::unknown("Overdraft / cash credit: whether it is secured is not in the books.")
            }
            _ => Fact::unknown("Whether it is secured is not in the books."),
        });
    }
    let payable = matches!(head, Some(Head::TradePayables));
    let receivable = matches!(head, Some(Head::TradeReceivables));
    if payable {
        d.msme = Some(if tag(m, "msme") {
            Fact::known(true, Basis::Tag, "Marked as a micro or small enterprise.")
        } else {
            Fact::unknown("MSME status of this supplier is not recorded.")
        });
    }
    if payable || receivable {
        d.disputed = Some(if tag(m, "disputed") {
            Fact::known(true, Basis::Tag, "Marked as disputed.")
        } else {
            Fact::unknown("Not marked disputed; the books do not say it is undisputed.")
        });
        d.counterparty = Some(Fact::known(
            m.name.clone(),
            Basis::Name,
            "The party ledger.",
        ));
    }
    if payable
        || receivable
        || borrowing
        || matches!(head, Some(Head::LtLoansAdvances | Head::StLoansAdvances))
    {
        d.related_party = Some(if tag(m, "related_party") {
            Fact::known(true, Basis::Tag, "Marked as a related party.")
        } else {
            Fact::unknown("Related-party status is not recorded for this ledger.")
        });
    }
    if matches!(head, Some(Head::CashBank)) {
        d.cash = Some(Fact::known(true, Basis::Group, "Cash or bank balance."));
    }
    if matches!(m.class, Some(Class::DutiesTaxes)) {
        d.statutory_due = Some(Fact::known(
            true,
            Basis::Group,
            "In the group Duties & Taxes.",
        ));
    }
    if let Some(b) = branch {
        d.branch = Some(Fact::known(
            b.to_string(),
            Basis::Group,
            "From the unit's books.",
        ));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

    fn ml(name: &str, group: &str, class: Class, head: Head, status: MapStatus) -> MappedLedger {
        MappedLedger {
            name: name.into(),
            group: group.into(),
            class: Some(class),
            head: Some(head),
            source: None,
            reclassified: false,
            amount: Money::rupees(100),
            tb_closing: Money::rupees(-100),
            tags: vec![],
            status,
            status_reason: String::new(),
        }
    }

    #[test]
    fn current_or_non_current_carries_its_evidence() {
        let s = ml(
            "HDFC Term Loan",
            "Secured Loans",
            Class::SecuredLoans,
            Head::LtBorrowings,
            MapStatus::Suggested,
        );
        let d = dims(&s, None);
        let m = d.maturity.unwrap();
        assert_eq!(m.value, Some(Maturity::NonCurrent));
        assert_eq!(m.basis, Basis::Group);
        assert!(m.evidence.contains("Secured Loans") && m.evidence.contains("12 months"));
        assert_eq!(d.secured.unwrap().value, Some(true));
        assert_eq!(d.related_party.unwrap().value, None, "never assumed");

        let c = ml(
            "HDFC Term Loan",
            "Secured Loans",
            Class::SecuredLoans,
            Head::LtBorrowings,
            MapStatus::Confirmed,
        );
        assert_eq!(dims(&c, None).maturity.unwrap().basis, Basis::User);
    }

    #[test]
    fn unknown_stays_unknown() {
        let mut p = ml(
            "Balaji Traders",
            "Sundry Creditors",
            Class::SundryCreditors,
            Head::TradePayables,
            MapStatus::Rule,
        );
        let d = dims(&p, Some("Pune"));
        assert_eq!(d.msme.as_ref().unwrap().value, None);
        assert_eq!(d.disputed.as_ref().unwrap().value, None);
        assert_eq!(d.branch.unwrap().value.as_deref(), Some("Pune"));
        assert!(d.maturity.is_none());
        p.tags = vec!["msme".into()];
        let d = dims(&p, None);
        assert_eq!(d.msme.unwrap().value, Some(true));
        let cash = ml(
            "Cash",
            "Cash-in-Hand",
            Class::CashInHand,
            Head::CashBank,
            MapStatus::Rule,
        );
        assert_eq!(dims(&cash, None).cash.unwrap().value, Some(true));
    }
}
