//! All checks. Each check reads the [`Ctx`] and appends [`Finding`]s.
//! Checks only flag; they never change figures.

pub mod cash;
pub mod classify;
pub mod loans;
pub mod opening;
pub mod tb;
pub mod vouchers;

use crate::groups::{Class, GroupResolver};
use crate::mapping::{has_stem, has_word};
use crate::model::{norm_name, Engagement, Ledger};
use crate::money::Money;
use crate::rules::{RulesPack, Severity};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ALL_CODES: &[&str] = &[
    "TB_UNBALANCED",
    "OPENING_TOTAL_DIFF",
    "OPENING_DIFF",
    "OPENING_MISSING_LEDGER",
    "OPENING_NEW_LEDGER",
    "PL_OPENING",
    "UNKNOWN_GROUP",
    "UNMAPPED",
    "MAPPING_UNCONFIRMED",
    "MAPPING_REVIEW",
    "VOUCHER_UNBALANCED",
    "VOUCHER_UNKNOWN_LEDGER",
    "VOUCHER_TB_MISMATCH",
    "VOUCHER_OUTSIDE_PERIOD",
    "DUPLICATE_VOUCHER",
    "NEGATIVE_CASH",
    "CASH_CR_BALANCE",
    "SUSPENSE_BALANCE",
    "BRANCH_NOT_ELIMINATED",
    "BRANCH_GROUP_CONFLICT",
    "ABNORMAL_BALANCE",
    "RECLASS_DEBTOR_CR",
    "RECLASS_CREDITOR_DR",
    "RECLASS_BANK_CR",
    "MISGROUP_LOAN",
    "MISGROUP_OD",
    "MISGROUP_TAX",
    "CAPITAL_IN_EXPENSE",
    "MISC_EXP_ASSET",
    "CASH_PAYMENT_LIMIT",
    "CASH_ASSET_PURCHASE",
    "CASH_RECEIPT_LIMIT",
    "LOAN_ACCEPTED_CASH",
    "LOAN_ACCEPTED_JOURNAL",
    "LOAN_REPAID_CASH",
    "STATEMENT_NOT_BALANCED",
    "FAR_TB_MISMATCH",
    "FAR_DEP_MISMATCH",
    "FAR_MISSING_LEDGER",
    "FAR_ERROR",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub title: String,
    /// Simple-language explanation with the specific figures.
    pub message: String,
    /// Legal reference for Expert mode (may be empty).
    pub legal_ref: String,
    pub ledger: Option<String>,
    pub voucher: Option<String>,
    pub date: Option<NaiveDate>,
    pub amount: Option<Money>,
    pub suggestion: Option<String>,
    /// Stable identity `CODE:subject`, used for de-duplication and tests.
    pub key: String,
    /// Why the rule fired: aggregation, threshold, ledgers looked at.
    #[serde(default)]
    pub detection_basis: String,
    #[serde(default)]
    pub unknown_facts: Vec<String>,
    #[serde(default)]
    pub possible_exceptions: Vec<String>,
    /// "accounting logic", "not verified" or "verified" (legal content).
    #[serde(default)]
    pub verification_status: String,
    /// A person must decide (statutory scope, exceptions, facts).
    #[serde(default)]
    pub professional_review_required: bool,
    /// Blocks a final copy while open.
    #[serde(default)]
    pub blocks_final: bool,
}

pub struct Findings<'a> {
    rules: &'a RulesPack,
    fy_start: NaiveDate,
    pub list: Vec<Finding>,
}

#[derive(Default)]
pub struct Detail {
    pub ledger: Option<String>,
    pub voucher: Option<String>,
    pub date: Option<NaiveDate>,
    pub amount: Option<Money>,
    pub suggestion: Option<String>,
}

impl<'a> Findings<'a> {
    pub fn new(rules: &'a RulesPack, fy_start: NaiveDate) -> Self {
        Findings {
            rules,
            fy_start,
            list: Vec::new(),
        }
    }

    pub fn add(&mut self, code: &str, subject: &str, specific: &str, d: Detail) {
        let r = self.rules.rule(code);
        let message = if specific.is_empty() {
            r.simple.clone()
        } else {
            format!("{} {}", r.simple, specific)
        };
        self.list.push(Finding {
            code: code.to_string(),
            severity: r.severity,
            title: r.title.clone(),
            message,
            legal_ref: self.rules.legal_ref(code, self.fy_start),
            ledger: d.ledger,
            voucher: d.voucher,
            date: d.date,
            amount: d.amount,
            suggestion: d
                .suggestion
                .or_else(|| (!r.next_step.is_empty()).then(|| r.next_step.clone())),
            key: format!("{code}:{subject}"),
            detection_basis: r.detection_basis.clone(),
            unknown_facts: r.unknown_facts.clone(),
            possible_exceptions: r.possible_exceptions.clone(),
            verification_status: if r.scope == "internal"
                || r.scope.is_empty() && r.verification != "unverified"
            {
                "accounting logic".into()
            } else if self.rules.verified.contains(&format!("rule:{code}")) {
                "verified".into()
            } else {
                "not verified".into()
            },
            professional_review_required: r.severity == crate::rules::Severity::Review,
            blocks_final: r.severity == crate::rules::Severity::Blocker,
        });
    }
}

/// Resolved view of the current-year trial balance shared by all checks.
pub struct Ctx<'a> {
    pub eng: &'a Engagement,
    pub rules: &'a RulesPack,
    /// Class of each CY ledger (None = group not recognised).
    pub classes: Vec<Option<Class>>,
    /// Normalised name -> index into `eng.cy.ledgers`.
    pub idx: HashMap<String, usize>,
    /// For every voucher line, the CY ledger index it refers to.
    pub line_idx: Vec<Vec<Option<usize>>>,
    /// Voucher indices sorted by date (stable: same-date vouchers keep input order).
    pub by_date: Vec<usize>,
    /// For each CY ledger, the vouchers touching it, in date order.
    pub postings: Vec<Vec<usize>>,
}

impl<'a> Ctx<'a> {
    pub fn new(eng: &'a Engagement, rules: &'a RulesPack) -> Self {
        let resolver = GroupResolver::new(&eng.cy);
        let classes = eng
            .cy
            .ledgers
            .iter()
            .map(|l| resolver.resolve(&l.group).ok())
            .collect();
        let idx = eng.cy.index();
        // Resolve each distinct line name once (large books repeat names millions of times).
        let mut cache: HashMap<&str, Option<usize>> = HashMap::new();
        let line_idx: Vec<Vec<Option<usize>>> = eng
            .vouchers
            .iter()
            .map(|v| {
                v.lines
                    .iter()
                    .map(|ln| {
                        *cache
                            .entry(ln.ledger.as_str())
                            .or_insert_with(|| idx.get(&norm_name(&ln.ledger)).copied())
                    })
                    .collect()
            })
            .collect();
        let mut by_date: Vec<usize> = (0..eng.vouchers.len()).collect();
        by_date.sort_by_key(|&k| eng.vouchers[k].date);
        let mut postings: Vec<Vec<usize>> = vec![Vec::new(); eng.cy.ledgers.len()];
        for &k in &by_date {
            let lines: &Vec<Option<usize>> = &line_idx[k];
            for (pos, i) in lines.iter().enumerate() {
                if let Some(i) = *i {
                    // Record each voucher once per ledger.
                    if !lines[..pos].contains(&Some(i)) {
                        postings[i].push(k);
                    }
                }
            }
        }
        Ctx {
            eng,
            rules,
            classes,
            idx,
            line_idx,
            by_date,
            postings,
        }
    }

    pub fn ledger(&self, i: usize) -> &'a Ledger {
        &self.eng.cy.ledgers[i]
    }

    pub fn lookup(&self, name: &str) -> Option<usize> {
        self.idx.get(&norm_name(name)).copied()
    }

    pub fn class_of(&self, name: &str) -> Option<Class> {
        self.lookup(name).and_then(|i| self.classes[i])
    }

    pub fn threshold(&self) -> &crate::rules::Thresholds {
        &self.rules.thresholds
    }

    /// A ledger that represents a loan *taken*: in a loan group, or named like a
    /// loan but grouped under creditors / current liabilities (mis-grouped).
    pub fn is_loan_taken(&self, i: usize) -> bool {
        match self.classes[i] {
            Some(c) if c.is_loan() => true,
            Some(Class::SundryCreditors) | Some(Class::CurrentLiabilities) => {
                looks_like_loan(&self.ledger(i).name)
            }
            _ => false,
        }
    }

    /// Loans from banks / tagged exempt are outside the cash-loan limits.
    pub fn loan_exempt(&self, i: usize) -> bool {
        let l = self.ledger(i);
        l.has_tag("exempt") || l.has_tag("bank") || has_word(&norm_name(&l.name), &["bank"])
    }
}

pub fn looks_like_loan(name: &str) -> bool {
    let n = norm_name(name);
    has_word(&n, &["loan", "loans"])
        || has_stem(
            &n,
            &["unsecured", "borrowing", "deposit from", "deposits from"],
        )
}

pub fn rs(m: Money) -> String {
    format!("₹{}", m.abs().fmt_indian())
}
