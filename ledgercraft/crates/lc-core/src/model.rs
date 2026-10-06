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
    /// Date of the comparative figures (default: the day before `fy_start`).
    #[serde(default)]
    pub comparative_end: Option<NaiveDate>,
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

/// Largest total of all amounts in one engagement (absolute values, paise).
/// Far above any real book; keeps every sum the engine makes inside i64.
pub const MAX_BOOK_PAISE: i128 = (i64::MAX / 8) as i128;

impl Engagement {
    /// Refuse books whose amounts are too large to add up safely (corrupt
    /// or hostile input) instead of failing part-way through.
    pub fn amounts_fit(&self) -> Result<(), String> {
        let mut total: i128 = 0;
        let mut add = |m: Money| total += (m.0 as i128).abs();
        for tb in std::iter::once(&self.cy).chain(self.py.iter()) {
            for l in &tb.ledgers {
                add(l.opening);
                add(l.closing);
                add(l.closing_stock.unwrap_or_default());
            }
        }
        for v in &self.vouchers {
            for l in &v.lines {
                add(l.amount);
            }
        }
        if total > MAX_BOOK_PAISE {
            Err(format!(
                "The amounts in these books add up to more than {} rupees, which is not a real book: the file is probably damaged or read in the wrong unit.",
                MAX_BOOK_PAISE / 100
            ))
        } else {
            Ok(())
        }
    }

    /// True when the statements cover a full year (12 months).
    pub fn is_full_year(&self) -> bool {
        use chrono::Datelike;
        let next = self
            .fy_start
            .with_year(self.fy_start.year() + 1)
            .unwrap_or(self.fy_start);
        next.pred_opt() == Some(self.fy_end)
    }

    /// "year ended 31 March 2026" or "period from 1 April 2025 to 30 June 2025".
    pub fn period_phrase(&self) -> String {
        use chrono::Datelike;
        let d = |x: NaiveDate| format!("{} {}", x.day(), x.format("%B %Y"));
        if self.is_full_year() {
            format!("year ended {}", d(self.fy_end))
        } else {
            format!("period from {} to {}", d(self.fy_start), d(self.fy_end))
        }
    }

    /// Date of the comparative column.
    pub fn comparative_date(&self) -> NaiveDate {
        self.comparative_end
            .unwrap_or_else(|| self.fy_start.pred_opt().unwrap_or(self.fy_start))
    }
}
