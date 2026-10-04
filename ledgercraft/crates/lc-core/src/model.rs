//! Canonical input data model. Every importer (Tally, Zoho, BUSY, Excel)
//! produces exactly these structures.

use crate::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// One ledger row of a trial balance. Balances are signed: Dr positive, Cr negative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ledger {
    pub name: String,
    /// Immediate parent group as in the accounting software.
    pub group: String,
    pub opening: Money,
    pub closing: Money,
    /// Only for stock ledgers in books where closing stock is entered separately
    /// (the ledger balance then still shows opening stock).
    #[serde(default)]
    pub closing_stock: Option<Money>,
    /// Free tags, e.g. `transporter`, `msme`, `related`.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Ledger {
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TrialBalance {
    pub ledgers: Vec<Ledger>,
    /// User-defined group tree: (group, parent group).
    #[serde(default)]
    pub groups: Vec<(String, String)>,
}

impl TrialBalance {
    pub fn index(&self) -> HashMap<String, usize> {
        self.ledgers
            .iter()
            .enumerate()
            .map(|(i, l)| (norm_name(&l.name), i))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoucherLine {
    pub ledger: String,
    /// Dr positive, Cr negative.
    pub amount: Money,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Voucher {
    pub date: NaiveDate,
    pub number: String,
    pub vtype: String,
    #[serde(default)]
    pub narration: String,
    pub lines: Vec<VoucherLine>,
}

impl Voucher {
    pub fn key(&self) -> String {
        format!(
            "{} {} dt {}",
            self.vtype,
            self.number,
            self.date.format("%d-%m-%Y")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Company,
    Llp,
    Firm,
    Proprietor,
    Huf,
    Aop,
    Boi,
}

impl EntityType {
    pub fn parse(s: &str) -> Option<EntityType> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "company" | "pvt ltd" | "private limited" | "ltd" => EntityType::Company,
            "llp" => EntityType::Llp,
            "firm" | "partnership" | "partnership firm" => EntityType::Firm,
            "proprietor" | "proprietorship" | "individual" => EntityType::Proprietor,
            "huf" => EntityType::Huf,
            "aop" => EntityType::Aop,
            "boi" => EntityType::Boi,
            _ => return None,
        })
    }
    pub fn is_company(self) -> bool {
        matches!(self, EntityType::Company)
    }
    pub fn label(self) -> &'static str {
        match self {
            EntityType::Company => "Company",
            EntityType::Llp => "Limited Liability Partnership",
            EntityType::Firm => "Partnership Firm",
            EntityType::Proprietor => "Proprietorship / Individual",
            EntityType::Huf => "Hindu Undivided Family",
            EntityType::Aop => "Association of Persons",
            EntityType::Boi => "Body of Individuals",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Engagement {
    pub entity_name: String,
    pub entity_type: EntityType,
    pub fy_start: NaiveDate,
    pub fy_end: NaiveDate,
    pub cy: TrialBalance,
    #[serde(default)]
    pub py: Option<TrialBalance>,
    #[serde(default)]
    pub vouchers: Vec<Voucher>,
    /// Remembered mapping: normalised ledger name -> head id.
    #[serde(default)]
    pub mapping_memory: HashMap<String, String>,
    /// For each remembered mapping: the group and balance side when it was
    /// confirmed (`memory_context`). A change re-opens the mapping for review.
    #[serde(default)]
    pub mapping_context: HashMap<String, String>,
    /// Format pack pinned to this client year (None: the built-in one).
    #[serde(default)]
    pub format_pack: Option<crate::statements::FormatPack>,
    /// Set when the books are a consolidation of head office and branches.
    #[serde(default)]
    pub consolidation: Option<crate::consolidate::MergeNotes>,
    /// Fixed asset register (optional).
    #[serde(default)]
    pub far: Option<crate::far::Register>,
    /// Profit-sharing ratio: (owner or capital ledger name, share). Empty = equal.
    #[serde(default)]
    pub profit_sharing: Vec<(String, u32)>,
}

/// Normalise a ledger or group name for matching: lowercase, collapse spaces,
/// drop punctuation that varies between exports.
pub fn norm_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for c in s.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_alphanumeric() || c == '&' {
            out.push(c);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}
