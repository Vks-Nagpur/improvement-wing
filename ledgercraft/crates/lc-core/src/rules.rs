//! Rules pack: thresholds, severities and legal references, kept as data.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Objective contradiction or integrity failure: blocks a final copy.
    Blocker,
    /// Needs a professional decision (statutory scope, exceptions, facts).
    Review,
    Warning,
    Info,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Blocker => "Must fix",
            Severity::Review => "Review",
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
    /// Provisions (ids in sections.json) behind the rule, for years under
    /// the Income-tax Act, 1961 and the Income-tax Act, 2025.
    #[serde(default)]
    pub refs_old: Vec<String>,
    #[serde(default)]
    pub refs_new: Vec<String>,
    /// "internal" (accounting logic), "statements" (presentation law) or
    /// "tax_audit" (only material when tax audit output is prepared).
    #[serde(default)]
    pub scope: String,
    /// How the program detected the event (aggregation, threshold, ledgers).
    #[serde(default)]
    pub detection_basis: String,
    /// Facts the program cannot know; a person must establish them.
    #[serde(default)]
    pub unknown_facts: Vec<String>,
    #[serde(default)]
    pub possible_exceptions: Vec<String>,
    /// Suggested next step (review wording, never a legal conclusion).
    #[serde(default)]
    pub next_step: String,
}

/// Text printed instead of a section number that is not settled.
pub const PENDING_REF: &str = "Statutory reference pending primary-source verification";
/// Tag added to a section number taken from secondary sources.
pub const UNVERIFIED_TAG: &str = "[not yet verified against the official text]";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Provision {
    pub act: String,
    pub citation: String,
    pub title: String,
    pub effective_from: String,
    pub effective_until: String,
    /// "secondary", "conflicting", "unknown" (as shipped); verification by a
    /// person is recorded separately and never written here by the program.
    pub status: String,
}

impl Provision {
    /// Whether a section number can be shown at all.
    pub fn resolved(&self) -> bool {
        !self.citation.is_empty() && !matches!(self.status.as_str(), "conflicting" | "unknown")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Sections {
    pub version: String,
    #[serde(default)]
    pub note: String,
    pub provisions: BTreeMap<String, Provision>,
}

const SECTIONS: &str = include_str!("../packs/sections.json");

impl Sections {
    pub fn builtin() -> Sections {
        serde_json::from_str(SECTIONS).expect("built-in sections pack is valid JSON")
    }
    pub fn get(&self, id: &str) -> Option<&Provision> {
        self.provisions.get(id)
    }
    /// Printable reference for a list of provisions. Unresolved provisions
    /// (unknown or conflicting) never show a number. `verified` holds the ids
    /// a person has verified (from the Legal verification register).
    pub fn cite(&self, ids: &[String], verified: &std::collections::BTreeSet<String>) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut pending = false;
        for id in ids {
            match self.get(id) {
                Some(p) if p.resolved() => {
                    // "s.185, Income-tax Act, 2025 [not yet verified ...]"
                    let mut t = p.citation.clone();
                    if p.act.starts_with("Income-tax Act") {
                        t = format!("{t}, {}", p.act);
                    }
                    if !verified.contains(&format!("provision:{id}")) {
                        t = format!("{t} {UNVERIFIED_TAG}");
                    }
                    if !parts.contains(&t) {
                        parts.push(t);
                    }
                }
                _ => pending = true,
            }
        }
        if pending {
            parts.push(PENDING_REF.to_string());
        }
        parts.join("; ")
    }
}

/// The sections pack shipped with this build, as text.
pub fn sections_text() -> &'static str {
    SECTIONS
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
    /// Statutory provisions referred to by the rules (not part of the pack file).
    #[serde(skip, default = "Sections::builtin")]
    pub sections: Sections,
    /// Legal items a person has verified (filled in by the app; never by the engine).
    #[serde(skip)]
    pub verified: std::collections::BTreeSet<String>,
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
        // Packs with provision references: built from sections data.
        let ids = if fy_start >= self.new_act_from {
            &r.refs_new
        } else {
            &r.refs_old
        };
        if !ids.is_empty() {
            return self.sections.cite(ids, &self.verified);
        }
        // Older pinned packs: their own reference text.
        let base = if fy_start >= self.new_act_from {
            r.new_ref.clone()
        } else {
            r.old_ref.clone()
        };
        if !base.is_empty() && r.verification == "unverified" {
            format!("{base} {UNVERIFIED_TAG}")
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
