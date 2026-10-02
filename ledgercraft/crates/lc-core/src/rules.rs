//! Rules pack: thresholds, severities and legal references, kept as data.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Blocker,
    Warning,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Blocker => "Must fix",
            Severity::Warning => "Check",
            Severity::Info => "Note",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    pub severity: Severity,
    pub title: String,
    pub simple: String,
    pub old_ref: String,
    pub new_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub cash_expense_payment_paise: i64,
    pub cash_payment_transporter_paise: i64,
    pub cash_receipt_paise: i64,
    pub cash_loan_paise: i64,
    pub capital_item_in_expense_paise: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesPack {
    pub version: String,
    pub new_act_from: NaiveDate,
    pub thresholds: Thresholds,
    pub rules: BTreeMap<String, RuleInfo>,
}

const BUILTIN: &str = include_str!("../packs/rules.json");

impl RulesPack {
    pub fn builtin() -> RulesPack {
        serde_json::from_str(BUILTIN).expect("built-in rules pack is valid JSON")
    }

    pub fn from_json(s: &str) -> Result<RulesPack, String> {
        let p: RulesPack = serde_json::from_str(s).map_err(|e| e.to_string())?;
        for code in crate::checks::ALL_CODES {
            if !p.rules.contains_key(*code) {
                return Err(format!("rules pack is missing rule {code}"));
            }
        }
        Ok(p)
    }

    pub fn rule(&self, code: &str) -> &RuleInfo {
        self.rules
            .get(code)
            .unwrap_or_else(|| panic!("rule {code} missing from rules pack"))
    }

    /// Legal reference for the financial year starting on `fy_start`.
    pub fn legal_ref(&self, code: &str, fy_start: NaiveDate) -> String {
        let r = self.rule(code);
        if fy_start >= self.new_act_from {
            r.new_ref.clone()
        } else {
            r.old_ref.clone()
        }
    }
}
