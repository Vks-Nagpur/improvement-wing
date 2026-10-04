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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    pub authority: String,
    pub document: String,
    pub section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleInfo {
    pub severity: Severity,
    pub title: String,
    pub simple: String,
    pub old_ref: String,
    pub new_ref: String,
    /// Where the rule comes from (law, standard, or LedgerCraft's own logic).
    #[serde(default)]
    pub source: Source,
    /// "verified" against the official text, "unverified", or "internal".
    #[serde(default)]
    pub verification: String,
    #[serde(default)]
    pub effective_from: String,
    #[serde(default)]
    pub effective_until: String,
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
    #[serde(default)]
    pub pack_id: String,
    #[serde(default)]
    pub verification: String,
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
        let base = if fy_start >= self.new_act_from {
            r.new_ref.clone()
        } else {
            r.old_ref.clone()
        };
        if !base.is_empty() && r.verification == "unverified" {
            format!("{base} [reference not yet verified against the official text]")
        } else {
            base
        }
    }
}

/// The rules pack shipped with this build, as text (to pin a client year to it).
pub fn builtin_text() -> &'static str {
    BUILTIN
}

/// Codes whose rule differs between two packs (added, removed or changed).
pub fn changed_rules(old: &RulesPack, new: &RulesPack) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in &new.rules {
        match old.rules.get(k) {
            None => out.push(format!("{k} (new)")),
            Some(o) if serde_json::to_string(o).ok() != serde_json::to_string(v).ok() => {
                out.push(k.clone())
            }
            _ => {}
        }
    }
    for k in old.rules.keys().filter(|k| !new.rules.contains_key(*k)) {
        out.push(format!("{k} (removed)"));
    }
    if serde_json::to_string(&old.thresholds).ok() != serde_json::to_string(&new.thresholds).ok() {
        out.push("thresholds".into());
    }
    out
}
