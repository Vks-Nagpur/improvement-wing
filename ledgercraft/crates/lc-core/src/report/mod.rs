//! Document model of the printed financial statements.
//!
//! Architecture: engine figures → `build` (rounding, wording, structure) →
//! `Report` (this neutral model) → renderers (PDF, Excel, HTML) that only lay
//! it out. Content decisions live here; visual decisions live in the renderers'
//! design system. Every amount in a `Report` is already rounded so that every
//! total equals the sum of the figures printed above it.

pub mod build;
pub mod cashflow;
pub mod policies;

pub use build::build;

use crate::units::Unit;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Full grid – every column and the outer frame ruled (classic printed accounts).
    #[default]
    Boxed,
    /// Open layout – horizontal rules only under headings and at totals.
    Ruled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Toggle {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportOptions {
    pub unit: Unit,
    pub decimals: u8,
    pub layout: Layout,
    pub show_previous_year: bool,
    pub hide_nil_lines: bool,
    /// Re-letter (a), (b), (c) … after nil lines are hidden.
    pub reletter: bool,
    pub cover_page: bool,
    pub accounting_policies: bool,
    pub cash_flow: Toggle,
    pub ratios: Toggle,
    pub ageing: bool,
    pub ppe_schedule: bool,
    pub tax_depreciation_annexure: bool,
    /// List every customer / supplier inside the trade receivable / payable notes.
    pub party_wise_details: bool,
    pub draft: bool,
    /// Address, CIN / LLPIN / PAN lines printed under the entity name.
    pub entity_details: Vec<String>,
    pub inventory_cost_formula: String,
    /// Reasons for ratio changes above 25% (ratio name → text).
    pub ratio_explanations: BTreeMap<String, String>,
}

impl Default for ReportOptions {
    fn default() -> Self {
        ReportOptions {
            unit: Unit::Rupees,
            decimals: 2,
            layout: Layout::Boxed,
            show_previous_year: true,
            hide_nil_lines: true,
            reletter: true,
            cover_page: true,
            accounting_policies: true,
            cash_flow: Toggle::Auto,
            ratios: Toggle::Auto,
            ageing: true,
            ppe_schedule: true,
            tax_depreciation_annexure: true,
            party_wise_details: false,
            draft: true,
            entity_details: Vec::new(),
            inventory_cost_formula: "first-in, first-out (FIFO)".into(),
            ratio_explanations: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Signatory {
    pub name: String,
    pub designation: String,
    /// e.g. "DIN", "DPIN", "PAN".
    pub id_label: String,
    pub id: String,
}

/// Sign-off details. UDIN is pasted by the user (no portal connection).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SignOff {
    pub signatories: Vec<Signatory>,
    pub auditor_firm: String,
    pub frn: String,
    pub auditor_partner: String,
    pub membership_no: String,
    pub udin: String,
    pub place: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Report {
    pub meta: Meta,
    pub sections: Vec<Section>,
    /// Things the preparer must still do (shown in the app, not printed).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Meta {
    pub entity: String,
    pub entity_type: String,
    pub details: Vec<String>,
    /// e.g. "Financial Statements for the year ended 31 March 2026".
    pub title: String,
    pub period_end: String,
    pub unit_note: String,
    pub unit: Unit,
    pub decimals: u8,
    pub draft: bool,
    pub layout: Layout,
    pub cover: bool,
    pub format_name: String,
    pub format_status: String,
    pub generator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Section {
    pub id: String,
    pub title: String,
    /// Listed in the contents page.
    pub contents: bool,
    pub blocks: Vec<Block>,
    pub closing_note: Option<String>,
    pub signature: Option<Signature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Signature {
    pub left: Vec<String>,
    pub right: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// Note title, e.g. "3  Long-term borrowings".
    Heading {
        text: String,
        level: u8,
    },
    Para {
        text: String,
    },
    Table(Table),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Table {
    pub title: Option<String>,
    pub columns: Vec<Column>,
    pub header: Vec<Vec<HeaderCell>>,
    pub rows: Vec<Row>,
    /// Do not split across pages (small notes).
    pub keep_together: bool,
    /// Print on a landscape page (wide schedules).
    pub landscape: bool,
    /// Smaller type for dense schedules.
    pub dense: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Column {
    /// Typst-style width: "1fr" or "30mm".
    pub width: String,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeaderCell {
    pub text: String,
    pub colspan: u16,
    pub rowspan: u16,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RowStyle {
    /// Bold capitals, e.g. "I. EQUITY AND LIABILITIES".
    Heading,
    /// Bold, e.g. "(1) Shareholders' funds".
    Subheading,
    Item,
    /// Bold with a rule above.
    Subtotal,
    /// Bold with a rule above and double rule below.
    Total,
    /// Small italic explanatory line.
    Remark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Row {
    pub style: RowStyle,
    pub indent: u8,
    /// Display text per column (amounts already formatted).
    pub cells: Vec<String>,
    /// Numeric value per column in the chosen unit (for spreadsheets).
    pub values: Vec<Option<f64>>,
}
