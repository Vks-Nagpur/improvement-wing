//! Builds the printable `Report` from an analysis.
//!
//! All figures come from `YearFacts::rounded`, and every total printed is the
//! sum of the rounded figures above it, so statements and notes cast exactly
//! in whatever unit is chosen.

use super::cashflow::{self, CfLine};
use super::policies;
use super::*;
use crate::engine::Analysis;
use crate::facts::YearFacts;
use crate::groups::Class;
use crate::mapping::Head;
use crate::model::{norm_name, Engagement, EntityType};
use crate::money::Money;
use crate::rounding::round_to_target;
use crate::statements::{capital_label, remuneration_label, FormatPack, PackRow};
use crate::units::{fmt_amount, granularity, round_to};
use chrono::{Datelike, NaiveDate};
use std::collections::BTreeMap;

type FaceRow = (String, String, Option<u32>, Option<Money>, Option<Money>);
type LineFilter<'a> = &'a dyn Fn(Option<Class>, &[String]) -> bool;
type CapCol = (
    &'static str,
    Box<dyn Fn(&crate::capital::CapitalRow) -> Money>,
);

fn long_date(d: NaiveDate) -> String {
    format!("{} {}", d.day(), d.format("%B %Y"))
}

struct Fmt {
    unit: crate::units::Unit,
    decimals: u8,
    g: i64,
}

impl Fmt {
    fn s(&self, m: Money) -> String {
        fmt_amount(m, self.unit, self.decimals)
    }
    fn v(&self, m: Money) -> Option<f64> {
        let scale = 10f64.powi(self.decimals as i32);
        Some((m.paise() as f64 / self.unit.paise() as f64 * scale).round() / scale)
    }
}

/// One line of a two-column note (current year / previous year).
#[derive(Clone)]
struct NRow {
    style: RowStyle,
    indent: u8,
    label: String,
    cy: Option<Money>,
    py: Option<Money>,
}

impl NRow {
    fn item(indent: u8, label: impl Into<String>, cy: Money, py: Option<Money>) -> NRow {
        NRow {
            style: RowStyle::Item,
            indent,
            label: label.into(),
            cy: Some(cy),
            py,
        }
    }
    fn head(label: impl Into<String>) -> NRow {
        NRow {
            style: RowStyle::Subheading,
            indent: 0,
            label: label.into(),
            cy: None,
            py: None,
        }
    }
    fn total(label: impl Into<String>, cy: Money, py: Option<Money>) -> NRow {
        NRow {
            style: RowStyle::Total,
            indent: 0,
            label: label.into(),
            cy: Some(cy),
            py,
        }
    }
    fn sub(label: impl Into<String>, cy: Money, py: Option<Money>) -> NRow {
        NRow {
            style: RowStyle::Subtotal,
            indent: 0,
            label: label.into(),
            cy: Some(cy),
            py,
        }
    }
    fn remark(label: impl Into<String>) -> NRow {
        NRow {
            style: RowStyle::Remark,
            indent: 0,
            label: label.into(),
            cy: None,
            py: None,
        }
    }
}

fn hc(text: impl Into<String>, colspan: u16, rowspan: u16, align: Align) -> HeaderCell {
    HeaderCell {
        text: text.into(),
        colspan,
        rowspan,
        align,
    }
}

fn col(width: &str, align: Align) -> Column {
    Column {
        width: width.into(),
        align,
    }
}

struct Ctx<'a> {
    eng: &'a Engagement,
    a: &'a Analysis,
    opt: &'a ReportOptions,
    f: Fmt,
    cy: YearFacts,
    py: Option<YearFacts>,
    end_cy: NaiveDate,
    end_py: NaiveDate,
    warnings: Vec<String>,
    blockers: Vec<String>,
}

impl Ctx<'_> {
    fn period_heads(&self, as_at: bool) -> (String, String) {
        if as_at {
            (
                format!("As at {}", long_date(self.end_cy)),
                format!("As at {}", long_date(self.end_py)),
            )
        } else if self.eng.is_full_year() {
            (
                format!("For the year ended {}", long_date(self.end_cy)),
                format!("For the year ended {}", long_date(self.end_py)),
            )
        } else {
            (
                format!("For the period ended {}", long_date(self.end_cy)),
                format!("Previous period ({})", long_date(self.end_py)),
            )
        }
    }

    fn has_py(&self) -> bool {
        self.py.is_some()
    }

    fn two_col(&self, rows: Vec<NRow>, as_at: bool) -> Table {
        let (h1, h2) = self.period_heads(as_at);
        let mut columns = vec![col("1fr", Align::Left), col("34mm", Align::Right)];
        let mut header = vec![
            hc("Particulars", 1, 1, Align::Left),
            hc(h1, 1, 1, Align::Right),
        ];
        if self.has_py() {
            columns.push(col("34mm", Align::Right));
            header.push(hc(h2, 1, 1, Align::Right));
        }
        let n = rows.len();
        let rows = rows
            .into_iter()
            .map(|r| {
                let mut cells = vec![
                    r.label.clone(),
                    r.cy.map(|m| self.f.s(m)).unwrap_or_default(),
                ];
                let mut values = vec![None, r.cy.and_then(|m| self.f.v(m))];
                if self.has_py() {
                    cells.push(r.py.map(|m| self.f.s(m)).unwrap_or_default());
                    values.push(r.py.and_then(|m| self.f.v(m)));
                }
                Row {
                    style: r.style,
                    indent: r.indent,
                    cells,
                    values,
                }
            })
            .collect();
        Table {
            title: None,
            columns,
            header: vec![header],
            rows,
            keep_together: n <= 28,
            landscape: false,
            dense: false,
        }
    }

    /// Ledger-wise rows of a head (union of both years), hiding lines nil in both.
    fn ledger_rows(
        &self,
        h: Head,
        filter: &dyn Fn(Option<Class>, &[String]) -> bool,
        indent: u8,
    ) -> Vec<NRow> {
        let cy_lines: Vec<(String, Option<Class>, Vec<String>, Money)> = self
            .cy
            .lines
            .get(&h)
            .map(|v| {
                v.iter()
                    .map(|l| (l.name.clone(), l.class, l.tags.clone(), l.amount))
                    .collect()
            })
            .unwrap_or_default();
        let py_full: Vec<(String, Option<Class>, Vec<String>, Money)> = self
            .py
            .as_ref()
            .and_then(|p| p.lines.get(&h))
            .map(|v| {
                v.iter()
                    .map(|l| (l.name.clone(), l.class, l.tags.clone(), l.amount))
                    .collect()
            })
            .unwrap_or_default();
        let mut names: Vec<(String, Option<Class>, Vec<String>)> = Vec::new();
        for (n, c, t, _) in cy_lines.iter().chain(py_full.iter()) {
            if !names.iter().any(|(x, _, _)| norm_name(x) == norm_name(n)) {
                names.push((n.clone(), *c, t.clone()));
            }
        }
        let find = |v: &Vec<(String, Option<Class>, Vec<String>, Money)>, n: &str| {
            v.iter()
                .filter(|x| norm_name(&x.0) == norm_name(n))
                .map(|x| x.3)
                .sum::<Money>()
        };
        names
            .into_iter()
            .filter(|(_, c, t)| filter(*c, t))
            .map(|(n, _, _)| {
                let cyv = find(&cy_lines, &n);
                let pyv = self.py.as_ref().map(|_| find(&py_full, &n));
                NRow::item(indent, n, cyv, pyv)
            })
            .filter(|r| {
                !(self.opt.hide_nil_lines
                    && r.cy.unwrap_or_default().is_zero()
                    && r.py.unwrap_or_default().is_zero())
            })
            .collect()
    }

    fn sum_where(
        &self,
        f: &YearFacts,
        h: Head,
        filter: &dyn Fn(Option<Class>, &[String]) -> bool,
    ) -> Money {
        f.lines
            .get(&h)
            .map(|v| {
                v.iter()
                    .filter(|l| filter(l.class, &l.tags))
                    .map(|l| l.amount)
                    .sum()
            })
            .unwrap_or_default()
    }

    fn head_cy_py(&self, h: Head) -> (Money, Option<Money>) {
        (self.cy.head(h), self.py.as_ref().map(|p| p.head(h)))
    }
}

/// Note numbering and content.
struct Notes {
    list: Vec<(u32, String, Vec<Block>)>,
    by_head: BTreeMap<Head, u32>,
    next: u32,
}

pub fn build(eng: &Engagement, a: &Analysis, opt: &ReportOptions, signoff: &SignOff) -> Report {
    let pack = FormatPack::of(eng);
    let g = granularity(opt.unit, opt.decimals);
    let cy = a.facts_cy.rounded(g);
    let py = if opt.show_previous_year {
        a.facts_py.as_ref().map(|p| p.rounded(g))
    } else {
        None
    };
    let mut c = Ctx {
        eng,
        a,
        opt,
        f: Fmt {
            unit: opt.unit,
            decimals: opt.decimals,
            g,
        },
        cy,
        py,
        end_cy: eng.fy_end,
        end_py: eng.comparative_date(),
        warnings: Vec::new(),
        blockers: Vec::new(),
    };
    if eng.entity_type.is_company()
        && !opt
            .unit
            .allowed_for_company(a.facts_cy.head(Head::RevenueOps))
    {
        c.warnings.push(format!(
            "Schedule III rounding: '{}' is not a permitted unit for a company with this turnover (below ₹100 crore: hundreds, thousands, lakhs or millions; ₹100 crore or more: lakhs, millions or crores).",
            opt.unit.long()
        ));
    }
    for m in opt.disclosures.missing(eng.entity_type.is_company()) {
        c.blockers.push(format!("Not answered yet: {m} (Notes and disclosures). Enter the particulars or mark it as nil."));
    }
    if pack.status.starts_with("draft") {
        c.warnings.push(format!(
            "Format pack '{}' is marked draft: {}",
            pack.name, pack.status
        ));
    }

    let mut notes = Notes {
        list: Vec::new(),
        by_head: BTreeMap::new(),
        next: 1,
    };
    if opt.accounting_policies {
        let info = policies::entity_information(eng, opt)
            .into_iter()
            .map(|t| Block::Para { text: t })
            .collect();
        notes.list.push((1, "Entity information".into(), info));
        let has_inv = !c.cy.head(Head::Inventories).is_zero();
        let has_emp = !c.cy.head(Head::EmployeeBenefits).is_zero();
        let mut blocks = Vec::new();
        let d = &opt.disclosures;
        let mut list = policies::accounting_policies(eng, opt, has_inv, has_emp);
        for (t, body) in list.iter_mut() {
            if let Some(own) = d
                .policy_text
                .get(t.as_str())
                .filter(|x| !x.trim().is_empty())
            {
                *body = own.trim().to_string();
            }
        }
        list.extend(
            d.extra_policies
                .iter()
                .filter(|n| !n.title.trim().is_empty())
                .map(|n| (n.title.trim().to_string(), n.text.trim().to_string())),
        );
        for (i, (t, body)) in list.into_iter().enumerate() {
            blocks.push(Block::Heading {
                text: format!("2.{} {t}", i + 1),
                level: 3,
            });
            blocks.push(Block::Para { text: body });
        }
        notes
            .list
            .push((2, "Significant accounting policies".into(), blocks));
        notes.next = 3;
        if d.policy_text.values().all(|x| x.trim().is_empty()) {
            c.warnings.push(
                "Accounting policies are standard wording: review and edit them for this entity (Notes and disclosures)."
                    .into(),
            );
        }
    }

    let bs_rows = face(&mut c, &pack, &pack.balance_sheet, &mut notes, true);
    let pl_rows = face(&mut c, &pack, &pack.profit_loss, &mut notes, false);

    let sig = signature(eng, signoff);
    let mut sections = Vec::new();
    let face_table = |c: &Ctx, rows: Vec<Row>, as_at: bool| -> Table {
        let (h1, h2) = c.period_heads(as_at);
        let mut columns = vec![
            col("1fr", Align::Left),
            col("16mm", Align::Center),
            col("34mm", Align::Right),
        ];
        let mut header = vec![
            hc("Particulars", 1, 1, Align::Left),
            hc("Note No.", 1, 1, Align::Center),
            hc(h1, 1, 1, Align::Right),
        ];
        if c.has_py() {
            columns.push(col("34mm", Align::Right));
            header.push(hc(h2, 1, 1, Align::Right));
        }
        Table {
            title: None,
            columns,
            header: vec![header],
            rows,
            keep_together: false,
            landscape: false,
            dense: false,
        }
    };
    let closing = Some(
        "The accompanying notes form an integral part of the financial statements.".to_string(),
    );
    sections.push(Section {
        id: "balance_sheet".into(),
        title: format!("Balance Sheet as at {}", long_date(eng.fy_end)),
        contents: true,
        blocks: vec![Block::Table(face_table(&c, bs_rows, true))],
        closing_note: closing.clone(),
        signature: Some(sig.clone()),
    });
    sections.push(Section {
        id: "profit_and_loss".into(),
        title: format!(
            "Statement of Profit and Loss for the {}",
            eng.period_phrase()
        ),
        contents: true,
        blocks: vec![Block::Table(face_table(&c, pl_rows, false))],
        closing_note: closing.clone(),
        signature: Some(sig.clone()),
    });

    // Cash flow statement.
    let cf_on = match opt.cash_flow {
        Toggle::On => true,
        Toggle::Off => false,
        Toggle::Auto => eng.entity_type.is_company(),
    };
    if cf_on {
        match a.facts_py.as_ref().map(|p| p.rounded(g)) {
            Some(pyr) => match cashflow::build(&c.cy, &pyr, eng.entity_type.is_company()) {
                Ok(lines) => {
                    let (h1, _) = c.period_heads(false);
                    let rows: Vec<Row> = lines
                        .into_iter()
                        .map(|l| {
                            let (style, indent, label, v) = match l {
                                CfLine::Heading(t) => (RowStyle::Subheading, 0, t, None),
                                CfLine::Item(t, m) => (RowStyle::Item, 1, t, Some(m)),
                                CfLine::Subtotal(t, m) => (RowStyle::Subtotal, 0, t, Some(m)),
                                CfLine::Total(t, m) => (RowStyle::Total, 0, t, Some(m)),
                            };
                            Row { style, indent, cells: vec![label, v.map(|m| c.f.s(m)).unwrap_or_default()], values: vec![None, v.and_then(|m| c.f.v(m))] }
                        })
                        .collect();
                    sections.push(Section {
                        id: "cash_flow".into(),
                        title: format!("Cash Flow Statement for the {}", eng.period_phrase()),
                        contents: true,
                        blocks: vec![
                            Block::Table(Table {
                                title: None,
                                columns: vec![col("1fr", Align::Left), col("38mm", Align::Right)],
                                header: vec![vec![hc("Particulars", 1, 1, Align::Left), hc(h1, 1, 1, Align::Right)]],
                                rows,
                                keep_together: false,
                                landscape: false,
                                dense: false,
                            }),
                            Block::Para { text: "The Cash Flow Statement has been prepared under the indirect method set out in Accounting Standard 3 'Cash Flow Statements'. Previous year figures are not presented as balances at the beginning of the previous year are not available.".into() },
                        ],
                        closing_note: closing.clone(),
                        signature: Some(sig.clone()),
                    });
                }
                Err(diff) => c.warnings.push(format!("Cash Flow Statement not produced: it does not reconcile to cash and bank balances by {diff}.")),
            },
            None => c.warnings.push("Cash Flow Statement needs last year's trial balance.".into()),
        }
    }

    // Ratios (Schedule III additional regulatory information).
    let ratios_on = match opt.ratios {
        Toggle::On => true,
        Toggle::Off => false,
        Toggle::Auto => eng.entity_type.is_company(),
    };
    if ratios_on && !a.ratios.is_empty() {
        let no = notes.next;
        notes.next += 1;
        notes.list.push((
            no,
            "Additional regulatory information: analytical ratios".into(),
            vec![Block::Table(ratio_table(&mut c))],
        ));
    }

    for (title, blocks) in extra_notes(&mut c) {
        let no = notes.next;
        notes.next += 1;
        notes.list.push((no, title, blocks));
    }

    let mut note_blocks = Vec::new();
    for (no, title, blocks) in &notes.list {
        note_blocks.push(Block::Heading {
            text: format!("{no}  {title}"),
            level: 2,
        });
        note_blocks.extend(blocks.iter().cloned());
    }
    sections.push(Section {
        id: "notes".into(),
        title: "Notes to the financial statements".into(),
        contents: true,
        blocks: note_blocks,
        closing_note: None,
        signature: Some(sig.clone()),
    });

    if !eng.is_full_year() {
        c.warnings.push("These statements cover less than a year: ratios are for the period and not annualised; the Income-tax depreciation annexure is left out (tax depreciation is worked out for a full year).".into());
    }
    if opt.tax_depreciation_annexure && eng.is_full_year() {
        if let Some(far) = &a.far {
            if !far.it.is_empty() {
                sections.push(Section {
                    id: "tax_depreciation".into(),
                    title: "Annexure: Depreciation as per the Income-tax Act (not part of the financial statements)".into(),
                    contents: true,
                    blocks: vec![Block::Table(it_table(&c, far))],
                    closing_note: None,
                    signature: None,
                });
            }
        }
    }

    if !opt.hidden_sections.is_empty() {
        sections.retain(|s| {
            let hide = opt.hidden_sections.iter().any(|h| h == &s.id);
            if hide && matches!(s.id.as_str(), "balance_sheet" | "profit_and_loss" | "notes") {
                c.warnings.push(format!(
                    "'{}' is left out of this print. A complete set of financial statements must include it.",
                    s.title
                ));
            }
            !hide
        });
    }

    let meta = Meta {
        entity: eng.entity_name.clone(),
        entity_type: eng.entity_type.label().into(),
        details: opt.entity_details.clone(),
        title: format!("Financial Statements for the {}", eng.period_phrase()),
        period_end: long_date(eng.fy_end),
        unit_note: format!(
            "(All amounts in {}, unless otherwise stated)",
            opt.unit.long()
        ),
        unit: opt.unit,
        decimals: opt.decimals.min(2),
        draft: opt.draft,
        layout: opt.layout,
        cover: opt.cover_page,
        format_name: pack.name.clone(),
        format_status: pack.status.clone(),
        generator: format!("LedgerCraft {}", env!("CARGO_PKG_VERSION")),
        draft_note: String::new(),
    };
    let unresolved = a.count(crate::rules::Severity::Blocker) + c.blockers.len();
    let mut meta = meta;
    if opt.draft && unresolved > 0 {
        meta.draft_note = format!(
            "{unresolved} problem{} unresolved",
            if unresolved == 1 { "" } else { "s" }
        );
    }
    let mut warnings = c.blockers.clone();
    warnings.extend(c.warnings);
    Report {
        meta,
        sections,
        warnings,
        blockers: c.blockers,
    }
}

/// Face statement rows from the format pack.
fn face(
    c: &mut Ctx,
    pack: &FormatPack,
    rows: &[PackRow],
    notes: &mut Notes,
    as_at: bool,
) -> Vec<Row> {
    let fill = |s: &str| {
        s.replace("{capital_label}", capital_label(c.eng.entity_type))
            .replace(
                "{remuneration_label}",
                remuneration_label(c.eng.entity_type),
            )
    };
    // (kind, label, note, cy, py)
    let mut out: Vec<FaceRow> = Vec::new();
    for r in rows {
        let label = fill(&r.label);
        match r.t.as_str() {
            "h" | "s" | "s2" => out.push((r.t.clone(), label, None, None, None)),
            "i" => {
                let h = r.head.expect("item has a head");
                let (vc, vp) = c.head_cy_py(h);
                let nil = vc.is_zero() && vp.map(|x| x.is_zero()).unwrap_or(true);
                if nil && !r.keep && c.opt.hide_nil_lines {
                    continue;
                }
                let note = if nil {
                    None
                } else {
                    Some(note_for(c, pack, notes, h, &label, as_at))
                };
                out.push((
                    "i".into(),
                    label,
                    note,
                    Some(vc),
                    c.py.as_ref().map(|_| vp.unwrap_or_default()),
                ));
            }
            "sum" => {
                let vc: Money = r.heads.iter().map(|h| c.cy.head(*h)).sum();
                let vp =
                    c.py.as_ref()
                        .map(|p| r.heads.iter().map(|h| p.head(*h)).sum());
                out.push(("sum".into(), label, None, Some(vc), vp));
            }
            "calc" => {
                let f = |y: &YearFacts| match r.calc.as_deref() {
                    Some("pbt") => y.pbt(),
                    Some("pbr") => y.pbt() + y.head(Head::PartnersRemuneration),
                    _ => y.profit(),
                };
                out.push((
                    "calc".into(),
                    label,
                    None,
                    Some(f(&c.cy)),
                    c.py.as_ref().map(f),
                ));
            }
            "total" => {
                let liab = r.side.as_deref() == Some("liab");
                let f = |y: &YearFacts| {
                    if liab {
                        y.total_equity_liabilities()
                    } else {
                        y.total_assets()
                    }
                };
                out.push((
                    "total".into(),
                    label,
                    None,
                    Some(f(&c.cy)),
                    c.py.as_ref().map(f),
                ));
            }
            other => panic!("unknown pack row type {other}"),
        }
    }
    // Drop headings with nothing under them.
    let level = |k: &str| match k {
        "h" => 0,
        "s" => 1,
        "s2" => 2,
        _ => 9,
    };
    let mut keep = vec![true; out.len()];
    for i in 0..out.len() {
        let k = out[i].0.as_str();
        if level(k) > 2 {
            continue;
        }
        let mut has = false;
        for r in &out[i + 1..] {
            let kk = r.0.as_str();
            if matches!(kk, "i" | "sum" | "calc") {
                has = true;
                break;
            }
            if kk == "total" || level(kk) <= level(k) {
                break;
            }
        }
        keep[i] = has;
    }
    let mut out: Vec<_> = out
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(r, _)| r)
        .collect();
    if c.opt.reletter {
        reletter(&mut out);
    }
    out.into_iter()
        .map(|(k, label, note, vc, vp)| {
            let style = match k.as_str() {
                "h" => RowStyle::Heading,
                "s" | "s2" => RowStyle::Subheading,
                "i" => RowStyle::Item,
                "total" => RowStyle::Total,
                _ => RowStyle::Subtotal,
            };
            let indent = match k.as_str() {
                "s2" => 1,
                "i" if label.starts_with("(i") || label.starts_with("(v") => 2,
                "i" if label.starts_with('(') => 1,
                _ => 0,
            };
            let mut cells = vec![
                label,
                note.map(|n| n.to_string()).unwrap_or_default(),
                vc.map(|m| c.f.s(m)).unwrap_or_default(),
            ];
            let mut values = vec![None, None, vc.and_then(|m| c.f.v(m))];
            if c.has_py() {
                cells.push(vp.map(|m| c.f.s(m)).unwrap_or_default());
                values.push(vp.and_then(|m| c.f.v(m)));
            }
            Row {
                style,
                indent,
                cells,
                values,
            }
        })
        .collect()
}

fn is_roman_label(l: &str) -> bool {
    let Some(rest) = l.strip_prefix('(') else {
        return false;
    };
    let Some(end) = rest.find(')') else {
        return false;
    };
    let tag = &rest[..end];
    !tag.is_empty() && tag.chars().all(|c| matches!(c, 'i' | 'v' | 'x'))
}

fn roman(n: usize) -> &'static str {
    ["i", "ii", "iii", "iv", "v", "vi", "vii", "viii", "ix", "x"]
        .get(n)
        .copied()
        .unwrap_or("x")
}

/// Re-letter (a), (b), (c) within each sub-heading, and (i), (ii) within each
/// sub-sub-heading, after nil lines are removed.
fn reletter(rows: &mut [FaceRow]) {
    let mut letter = 0usize;
    let mut rom = 0usize;
    let mut in_s2 = false;
    for r in rows.iter_mut() {
        let replace = |label: &str, tag: &str| -> String {
            match label.find(") ") {
                Some(p) if label.starts_with('(') => format!("({tag}) {}", &label[p + 2..]),
                _ => label.to_string(),
            }
        };
        match r.0.as_str() {
            "h" | "s" => {
                letter = 0;
                in_s2 = false;
            }
            "s2" => {
                let t = ((b'a' + letter as u8) as char).to_string();
                r.1 = replace(&r.1, &t);
                letter += 1;
                rom = 0;
                in_s2 = true;
            }
            "i" if r.1.starts_with('(') => {
                if in_s2 && is_roman_label(&r.1) {
                    r.1 = replace(&r.1, roman(rom));
                    rom += 1;
                } else {
                    in_s2 = false;
                    let t = ((b'a' + letter as u8) as char).to_string();
                    r.1 = replace(&r.1, &t);
                    letter += 1;
                }
            }
            _ => {}
        }
    }
}

fn strip_numbering(label: &str) -> String {
    let t = label.trim();
    if let Some(rest) = t.strip_prefix('(') {
        if let Some(p) = rest.find(')') {
            return rest[p + 1..].trim().to_string();
        }
    }
    if let Some(p) = t.find(". ") {
        if t[..p].chars().all(|c| c.is_ascii_uppercase()) {
            return t[p + 2..].to_string();
        }
    }
    t.to_string()
}

fn note_for(
    c: &mut Ctx,
    pack: &FormatPack,
    notes: &mut Notes,
    h: Head,
    label: &str,
    as_at: bool,
) -> u32 {
    if let Some(n) = notes.by_head.get(&h) {
        return *n;
    }
    let no = notes.next;
    notes.next += 1;
    notes.by_head.insert(h, no);
    let title = strip_numbering(label);
    let blocks = note_content(c, pack, h, as_at);
    notes.list.push((no, title, blocks));
    no
}

fn any(_: Option<Class>, _: &[String]) -> bool {
    true
}

fn note_content(c: &mut Ctx, pack: &FormatPack, h: Head, as_at: bool) -> Vec<Block> {
    let (tc, tp) = c.head_cy_py(h);
    let total = |rows: &mut Vec<NRow>| rows.push(NRow::total("Total", tc, tp));
    match h {
        Head::Capital if !c.eng.entity_type.is_company() => capital_note(c),
        Head::Capital if !c.opt.disclosures.share_classes.is_empty() => {
            share_capital_note(c, tc, tp)
        }
        Head::Capital if c.eng.entity_type.is_company() => {
            let mut rows = c.ledger_rows(h, &any, 0);
            total(&mut rows);
            vec![
                Block::Table(c.two_col(rows, true)),
                Block::Para {
                    text: NOT_PROVIDED.into(),
                },
            ]
        }
        Head::ReservesSurplus if pack.profit_to == Head::ReservesSurplus => {
            let mut rows = c.ledger_rows(h, &|cl, _| cl != Some(Class::ProfitLossAc), 0);
            let open_c = c.sum_where(&c.cy, h, &|cl, _| cl == Some(Class::ProfitLossAc));
            let open_p =
                c.py.as_ref()
                    .map(|p| c.sum_where(p, h, &|cl, _| cl == Some(Class::ProfitLossAc)));
            let pc = c.cy.profit();
            let pp = c.py.as_ref().map(|p| p.profit());
            rows.push(NRow::head("Surplus in the Statement of Profit and Loss"));
            rows.push(NRow::item(
                1,
                "Balance at the beginning of the year",
                open_c,
                open_p,
            ));
            rows.push(NRow::item(1, "Add: Profit / (loss) for the year", pc, pp));
            rows.push(NRow::sub(
                "Balance at the end of the year",
                open_c + pc,
                open_p.zip(pp).map(|(a, b)| a + b),
            ));
            total(&mut rows);
            vec![Block::Table(c.two_col(rows, true))]
        }
        Head::LtBorrowings | Head::StBorrowings => {
            let groups: [(&str, LineFilter); 3] = [
                ("Secured", &|cl, _| cl == Some(Class::SecuredLoans)),
                ("Loans repayable on demand from banks", &|cl, _| {
                    matches!(cl, Some(Class::BankOdAc) | Some(Class::BankAccounts))
                }),
                ("Unsecured", &|cl, _| {
                    !matches!(
                        cl,
                        Some(Class::SecuredLoans)
                            | Some(Class::BankOdAc)
                            | Some(Class::BankAccounts)
                    )
                }),
            ];
            let mut rows = Vec::new();
            for (title, f) in groups {
                let items = c.ledger_rows(h, f, 1);
                if !items.is_empty() {
                    rows.push(NRow::head(title));
                    rows.extend(items);
                }
            }
            total(&mut rows);
            vec![Block::Table(c.two_col(rows, true))]
        }
        Head::TradePayables => {
            let msme =
                |_: Option<Class>, t: &[String]| t.iter().any(|x| x.eq_ignore_ascii_case("msme"));
            let other = |cl: Option<Class>, t: &[String]| !msme(cl, t);
            let mut rows = Vec::new();
            for (title, f) in [
                ("Total outstanding dues of micro enterprises and small enterprises", &msme as &dyn Fn(Option<Class>, &[String]) -> bool),
                ("Total outstanding dues of creditors other than micro enterprises and small enterprises", &other),
            ] {
                let vc = c.sum_where(&c.cy, h, f);
                let vp = c.py.as_ref().map(|p| c.sum_where(p, h, f));
                rows.push(NRow::item(0, title, vc, vp));
                if c.opt.party_wise_details {
                    rows.extend(c.ledger_rows(h, f, 1));
                }
            }
            total(&mut rows);
            let msme_cy = rows.first().and_then(|r| r.cy).unwrap_or_default();
            let msme_py = rows.first().and_then(|r| r.py);
            let mut blocks = vec![Block::Table(c.two_col(rows, true))];
            blocks.extend(msme_block(c, msme_cy, msme_py));
            if c.opt.ageing {
                blocks.extend(ageing_block(c, h));
            }
            blocks
        }
        Head::TradeReceivables => {
            let doubtful = |_: Option<Class>, t: &[String]| {
                t.iter().any(|x| x.eq_ignore_ascii_case("doubtful"))
            };
            let good = |cl: Option<Class>, t: &[String]| !doubtful(cl, t);
            let mut rows = vec![NRow::head("Unsecured")];
            for (title, f) in [
                (
                    "Considered good",
                    &good as &dyn Fn(Option<Class>, &[String]) -> bool,
                ),
                ("Considered doubtful", &doubtful),
            ] {
                let vc = c.sum_where(&c.cy, h, f);
                let vp = c.py.as_ref().map(|p| c.sum_where(p, h, f));
                if !(vc.is_zero() && vp.unwrap_or_default().is_zero()) {
                    rows.push(NRow::item(1, title, vc, vp));
                    if c.opt.party_wise_details {
                        rows.extend(c.ledger_rows(h, f, 2));
                    }
                }
            }
            total(&mut rows);
            let mut blocks = vec![Block::Table(c.two_col(rows, true))];
            if c.opt.ageing {
                blocks.extend(ageing_block(c, h));
            }
            blocks
        }
        Head::CashBank => {
            let cash = |cl: Option<Class>, _: &[String]| cl == Some(Class::CashInHand);
            let bank = |cl: Option<Class>, _: &[String]| cl != Some(Class::CashInHand);
            let mut rows = Vec::new();
            for (title, f) in [
                (
                    "Cash on hand",
                    &cash as &dyn Fn(Option<Class>, &[String]) -> bool,
                ),
                ("Balances with banks", &bank),
            ] {
                let items = c.ledger_rows(h, f, 1);
                if !items.is_empty() {
                    rows.push(NRow::head(title));
                    rows.extend(items);
                }
            }
            total(&mut rows);
            vec![Block::Table(c.two_col(rows, true))]
        }
        Head::Inventories => {
            let mut rows = c.ledger_rows(h, &any, 0);
            total(&mut rows);
            rows.push(NRow::remark(
                "Inventories are valued at the lower of cost and net realisable value.",
            ));
            vec![Block::Table(c.two_col(rows, true))]
        }
        Head::Ppe | Head::Intangibles | Head::Cwip => {
            if let Some(t) = ppe_schedule(c, h) {
                return vec![Block::Table(t)];
            }
            let mut rows = c.ledger_rows(h, &any, 0);
            total(&mut rows);
            rows.push(NRow::remark("Net book values as per the books of account (fixed asset register not provided or not matching)."));
            vec![Block::Table(c.two_col(rows, true))]
        }
        Head::ChangeInInventories => {
            let pc = (c.cy.opening_stock, c.cy.closing_stock);
            let pp = c.py.as_ref().map(|p| (p.opening_stock, p.closing_stock));
            let rows = vec![
                NRow::item(
                    0,
                    "Inventories at the beginning of the year",
                    pc.0,
                    pp.map(|x| x.0),
                ),
                NRow::item(
                    0,
                    "Less: Inventories at the end of the year",
                    -pc.1,
                    pp.map(|x| -x.1),
                ),
                NRow::total("Net (increase) / decrease in inventories", tc, tp),
            ];
            vec![Block::Table(c.two_col(rows, false))]
        }
        _ => {
            let mut rows = c.ledger_rows(h, &any, 0);
            if h == pack.profit_to {
                rows.push(NRow::item(
                    0,
                    pack.profit_note_label.clone(),
                    c.cy.profit(),
                    c.py.as_ref().map(|p| p.profit()),
                ));
            }
            total(&mut rows);
            vec![Block::Table(c.two_col(rows, as_at))]
        }
    }
}

fn capital_note(c: &mut Ctx) -> Vec<Block> {
    let g = c.f.g;
    let rows = c.a.capital.clone();
    if rows.is_empty() {
        let mut r = c.ledger_rows(Head::Capital, &any, 0);
        let (tc, tp) = c.head_cy_py(Head::Capital);
        r.push(NRow::total("Total", tc, tp));
        return vec![Block::Table(c.two_col(r, true))];
    }
    let detailed = rows.iter().any(|r| r.detailed);
    let (tc, tp) = c.head_cy_py(Head::Capital);
    let exact_close: Vec<Money> = rows.iter().map(|r| r.closing).collect();
    let close_r = round_to_target(&exact_close, g, tc);
    // Columns (signed contribution to closing).
    let cols: Vec<CapCol> = if detailed {
        let mut v: Vec<CapCol> = vec![
            (
                "Balance at the beginning of the year",
                Box::new(|r| r.opening),
            ),
            ("Capital introduced", Box::new(|r| r.introduced)),
            (
                "Remuneration and interest",
                Box::new(|r| r.remuneration_interest),
            ),
            ("Share of profit / (loss)", Box::new(|r| r.share_of_profit)),
            ("Withdrawals", Box::new(|r| -r.withdrawn)),
        ];
        if rows.iter().any(|r| !r.other.is_zero()) {
            v.push(("Other adjustments", Box::new(|r| r.other)));
        }
        v
    } else {
        vec![
            (
                "Balance at the beginning of the year",
                Box::new(|r| r.opening),
            ),
            (
                "Net movement during the year",
                Box::new(|r| r.closing - r.opening - r.share_of_profit),
            ),
            ("Share of profit / (loss)", Box::new(|r| r.share_of_profit)),
        ]
    };
    let mut table_rows = Vec::new();
    let mut col_tot = vec![Money::ZERO; cols.len()];
    for (r, target) in rows.iter().zip(&close_r) {
        let parts: Vec<Money> = cols.iter().map(|(_, f)| f(r)).collect();
        let parts_r = round_to_target(&parts, g, *target);
        let mut cells = vec![r.owner.clone()];
        let mut values = vec![None];
        for (i, p) in parts_r.iter().enumerate() {
            col_tot[i] += *p;
            // Withdrawals are printed as positive figures (they reduce the balance).
            let shown = if cols[i].0 == "Withdrawals" { -*p } else { *p };
            cells.push(c.f.s(shown));
            values.push(c.f.v(shown));
        }
        cells.push(c.f.s(*target));
        values.push(c.f.v(*target));
        table_rows.push(Row {
            style: RowStyle::Item,
            indent: 0,
            cells,
            values,
        });
    }
    let mut cells = vec!["Total".to_string()];
    let mut values = vec![None];
    for (i, t) in col_tot.iter().enumerate() {
        let shown = if cols[i].0 == "Withdrawals" { -*t } else { *t };
        cells.push(c.f.s(shown));
        values.push(c.f.v(shown));
    }
    cells.push(c.f.s(tc));
    values.push(c.f.v(tc));
    table_rows.push(Row {
        style: RowStyle::Total,
        indent: 0,
        cells,
        values,
    });
    let mut columns = vec![col("1fr", Align::Left)];
    let mut header = vec![hc("Name", 1, 1, Align::Left)];
    for (t, _) in &cols {
        columns.push(col("22mm", Align::Right));
        header.push(hc(*t, 1, 1, Align::Right));
    }
    columns.push(col("24mm", Align::Right));
    header.push(hc("Balance at the end of the year", 1, 1, Align::Right));
    let mut blocks = vec![Block::Table(Table {
        title: Some(format!("Movement during the {}", c.eng.period_phrase())),
        columns,
        header: vec![header],
        rows: table_rows,
        keep_together: true,
        landscape: false,
        dense: true,
    })];
    if !detailed {
        blocks.push(Block::Para {
            text:
                "Introductions and withdrawals are shown net because the day book was not imported."
                    .into(),
        });
    }
    // Comparative balances.
    if let Some(py) = c.py.clone() {
        let weights: Vec<u64> = rows.iter().map(|r| r.weight).collect();
        let shares =
            crate::capital::allocate(py.profit() + crate::engine::plac_balance(&py), &weights);
        let mut exact = Vec::new();
        for (r, s) in rows.iter().zip(&shares) {
            let lines: Money = py
                .lines
                .get(&Head::Capital)
                .map(|v| {
                    v.iter()
                        .filter(|l| r.ledgers.iter().any(|x| norm_name(x) == norm_name(&l.name)))
                        .map(|l| l.amount)
                        .sum()
                })
                .unwrap_or_default();
            exact.push(
                lines
                    + if py.profit_to == Head::Capital {
                        *s
                    } else {
                        Money::ZERO
                    },
            );
        }
        let py_exact_head =
            c.a.facts_py
                .as_ref()
                .map(|p| p.head(Head::Capital))
                .unwrap_or_default();
        let other = py_exact_head - exact.iter().copied().sum::<Money>();
        let mut all = exact.clone();
        all.push(other);
        let pyr = round_to_target(&all, g, tp.unwrap_or_default());
        let mut cmp = Vec::new();
        for ((r, cy), p) in rows.iter().zip(&close_r).zip(&pyr) {
            cmp.push(NRow::item(0, r.owner.clone(), *cy, Some(*p)));
        }
        if !pyr.last().copied().unwrap_or_default().is_zero() {
            cmp.push(NRow::item(
                0,
                "Other balances",
                Money::ZERO,
                pyr.last().copied(),
            ));
        }
        cmp.push(NRow::total("Total", tc, tp));
        blocks.push(Block::Table(c.two_col(cmp, true)));
    }
    blocks
}

fn ageing_block(c: &mut Ctx, h: Head) -> Vec<Block> {
    let ag = match h {
        Head::TradeReceivables => c.a.ageing_receivables.as_ref(),
        _ => c.a.ageing_payables.as_ref(),
    };
    let Some(ag) = ag else {
        c.warnings
            .push("Ageing schedules need the day book (vouchers) to be imported.".into());
        return vec![];
    };
    let g = c.f.g;
    let target = c.cy.head(h);
    let flat: Vec<Money> = ag
        .rows
        .iter()
        .flat_map(|r| r.buckets.iter().copied())
        .collect();
    if flat.iter().copied().sum::<Money>() != c.a.facts_cy.head(h) {
        c.warnings.push(
            "Ageing does not add up to the balance; check vouchers against the trial balance."
                .into(),
        );
    }
    let r = round_to_target(&flat, g, target);
    let nb = ag.bucket_labels.len();
    let mut columns = vec![col("1fr", Align::Left)];
    let header1 = vec![
        hc("Particulars", 1, 2, Align::Left),
        hc(
            "Outstanding for following periods from the date of the transaction",
            nb as u16,
            1,
            Align::Center,
        ),
        hc("Total", 1, 2, Align::Right),
    ];
    let mut header2 = Vec::new();
    for l in &ag.bucket_labels {
        columns.push(col("21mm", Align::Right));
        header2.push(hc(l.clone(), 1, 1, Align::Right));
    }
    columns.push(col("23mm", Align::Right));
    let mut rows = Vec::new();
    let mut col_tot = vec![Money::ZERO; nb];
    for (i, ar) in ag.rows.iter().enumerate() {
        let vals = &r[i * nb..(i + 1) * nb];
        let t: Money = vals.iter().copied().sum();
        let mut cells = vec![format!("({}) {}", roman(i), ar.category)];
        let mut values = vec![None];
        for (j, v) in vals.iter().enumerate() {
            col_tot[j] += *v;
            cells.push(c.f.s(*v));
            values.push(c.f.v(*v));
        }
        cells.push(c.f.s(t));
        values.push(c.f.v(t));
        rows.push(Row {
            style: RowStyle::Item,
            indent: 0,
            cells,
            values,
        });
    }
    let mut cells = vec!["Total".to_string()];
    let mut values = vec![None];
    for v in &col_tot {
        cells.push(c.f.s(*v));
        values.push(c.f.v(*v));
    }
    cells.push(c.f.s(target));
    values.push(c.f.v(target));
    rows.push(Row {
        style: RowStyle::Total,
        indent: 0,
        cells,
        values,
    });
    vec![
        Block::Table(Table {
            title: Some(format!("Ageing schedule as at {}", long_date(c.end_cy))),
            columns,
            header: vec![header1, header2],
            rows,
            keep_together: true,
            landscape: false,
            dense: true,
        }),
        Block::Para { text: "Ageing is computed from the date of the transaction on a first-in, first-out basis; opening balances are aged from the beginning of the year.".into() },
    ]
}

fn ppe_schedule(c: &mut Ctx, h: Head) -> Option<Table> {
    let far = c.a.far.as_ref()?;
    if !c.opt.ppe_schedule {
        return None;
    }
    let lines = c.cy.lines.get(&h)?.clone();
    // Use the register only when every ledger of this head ties to it exactly.
    let mut picked = Vec::new();
    for l in &lines {
        let row = far
            .ppe
            .iter()
            .find(|r| norm_name(&r.ledger) == norm_name(&l.name))?;
        let exact =
            c.a.facts_cy
                .lines
                .get(&h)?
                .iter()
                .find(|x| x.name == l.name)?
                .amount;
        if row.net_closing != exact {
            return None;
        }
        picked.push((row.clone(), l.amount));
    }
    if picked.is_empty() {
        return None;
    }
    let g = c.f.g;
    let start = long_date(c.eng.fy_start);
    let end = long_date(c.end_cy);
    let end_py = long_date(c.end_py);
    let mut columns = vec![col("1fr", Align::Left)];
    for _ in 0..10 {
        columns.push(col("21mm", Align::Right));
    }
    let header = vec![
        vec![
            hc("Particulars", 1, 2, Align::Left),
            hc("Gross block", 4, 1, Align::Center),
            hc("Depreciation / amortisation", 4, 1, Align::Center),
            hc("Net block", 2, 1, Align::Center),
        ],
        vec![
            hc(format!("As at {start}"), 1, 1, Align::Right),
            hc("Additions", 1, 1, Align::Right),
            hc("Deductions", 1, 1, Align::Right),
            hc(format!("As at {end}"), 1, 1, Align::Right),
            hc(format!("Up to {end_py}"), 1, 1, Align::Right),
            hc("For the year", 1, 1, Align::Right),
            hc("On deductions", 1, 1, Align::Right),
            hc(format!("Up to {end}"), 1, 1, Align::Right),
            hc(format!("As at {end}"), 1, 1, Align::Right),
            hc(format!("As at {end_py}"), 1, 1, Align::Right),
        ],
    ];
    let mut rows = Vec::new();
    let mut tot = [Money::ZERO; 10];
    for (r, face) in &picked {
        let parts = [
            r.gross_opening,
            r.additions,
            -r.deletions,
            -r.dep_opening,
            -r.dep_for_year,
            r.dep_on_deletions,
        ];
        let p = round_to_target(&parts, g, *face);
        let (go, ad, de, dop, dfy, dod) = (p[0], p[1], -p[2], -p[3], -p[4], p[5]);
        let gc = go + ad - de;
        let dc = dop + dfy - dod;
        let vals = [go, ad, de, gc, dop, dfy, dod, dc, gc - dc, go - dop];
        let mut cells = vec![r.ledger.clone()];
        let mut values = vec![None];
        for (i, v) in vals.iter().enumerate() {
            tot[i] += *v;
            cells.push(c.f.s(*v));
            values.push(c.f.v(*v));
        }
        rows.push(Row {
            style: RowStyle::Item,
            indent: 0,
            cells,
            values,
        });
    }
    let mut cells = vec!["Total".to_string()];
    let mut values = vec![None];
    for v in tot {
        cells.push(c.f.s(v));
        values.push(c.f.v(v));
    }
    rows.push(Row {
        style: RowStyle::Total,
        indent: 0,
        cells,
        values,
    });
    Some(Table {
        title: None,
        columns,
        header,
        rows,
        keep_together: true,
        landscape: true,
        dense: true,
    })
}

fn ratio_table(c: &mut Ctx) -> Table {
    let fmt = |v: Option<f64>, unit: &str| match v {
        Some(x) if unit == "%" => format!("{x:.2}%"),
        Some(x) => format!("{x:.2}"),
        None => "N.A.".into(),
    };
    let mut rows = Vec::new();
    for r in &c.a.ratios {
        let reason = if r.needs_explanation {
            match c.opt.ratio_explanations.get(&r.name) {
                Some(t) if !t.trim().is_empty() => t.clone(),
                _ => {
                    c.warnings.push(format!(
                        "Explain the change of more than 25% in '{}'.",
                        r.name
                    ));
                    String::new()
                }
            }
        } else {
            String::new()
        };
        rows.push(Row {
            style: RowStyle::Item,
            indent: 0,
            cells: vec![
                r.name.clone(),
                r.numerator.clone(),
                r.denominator.clone(),
                fmt(r.cy, &r.unit),
                fmt(r.py, &r.unit),
                r.variance_pct
                    .map(|v| format!("{v:.2}%"))
                    .unwrap_or_else(|| "N.A.".into()),
                reason,
            ],
            values: vec![None, None, None, r.cy, r.py, r.variance_pct, None],
        });
    }
    let (h1, h2) = (
        format!("{}", c.end_cy.year()),
        format!("{}", c.end_py.year()),
    );
    Table {
        title: None,
        columns: vec![
            col("42mm", Align::Left),
            col("52mm", Align::Left),
            col("52mm", Align::Left),
            col("20mm", Align::Right),
            col("20mm", Align::Right),
            col("20mm", Align::Right),
            col("1fr", Align::Left),
        ],
        header: vec![vec![
            hc("Ratio", 1, 1, Align::Left),
            hc("Numerator", 1, 1, Align::Left),
            hc("Denominator", 1, 1, Align::Left),
            hc(format!("March {h1}"), 1, 1, Align::Right),
            hc(format!("March {h2}"), 1, 1, Align::Right),
            hc("Variance", 1, 1, Align::Right),
            hc("Reason for variance above 25%", 1, 1, Align::Left),
        ]],
        rows,
        keep_together: true,
        landscape: true,
        dense: true,
    }
}

fn it_table(c: &Ctx, far: &crate::far::FarResult) -> Table {
    let g = granularity(crate::units::Unit::Rupees, 0);
    let fr = Fmt {
        unit: crate::units::Unit::Rupees,
        decimals: 0,
        g,
    };
    let _ = c;
    let mut rows = Vec::new();
    let mut tot = [Money::ZERO; 6];
    for r in &far.it {
        let parts = [
            r.opening_wdv,
            r.additions_180_or_more,
            r.additions_less_180,
            -r.sale_proceeds,
            -r.depreciation,
        ];
        let target = round_to(r.closing_wdv, g);
        let p = round_to_target(&parts, g, target);
        let vals = [p[0], p[1], p[2], -p[3], -p[4], target];
        let mut cells = vec![r.label.clone(), format!("{}%", r.rate)];
        let mut values = vec![None, Some(r.rate)];
        for (i, v) in vals.iter().enumerate() {
            tot[i] += *v;
            cells.push(fr.s(*v));
            values.push(fr.v(*v));
        }
        rows.push(Row {
            style: RowStyle::Item,
            indent: 0,
            cells,
            values,
        });
    }
    let mut cells = vec!["Total".to_string(), String::new()];
    let mut values = vec![None, None];
    for v in tot {
        cells.push(fr.s(v));
        values.push(fr.v(v));
    }
    rows.push(Row {
        style: RowStyle::Total,
        indent: 0,
        cells,
        values,
    });
    let mut columns = vec![col("1fr", Align::Left), col("13mm", Align::Right)];
    for _ in 0..6 {
        columns.push(col("26mm", Align::Right));
    }
    Table {
        title: Some("Amounts in ₹".into()),
        columns,
        header: vec![vec![
            hc("Block of assets", 1, 1, Align::Left),
            hc("Rate", 1, 1, Align::Right),
            hc("Opening written down value", 1, 1, Align::Right),
            hc(
                "Additions – put to use 180 days or more",
                1,
                1,
                Align::Right,
            ),
            hc(
                "Additions – put to use less than 180 days",
                1,
                1,
                Align::Right,
            ),
            hc("Sale proceeds / deductions", 1, 1, Align::Right),
            hc("Depreciation for the year", 1, 1, Align::Right),
            hc("Closing written down value", 1, 1, Align::Right),
        ]],
        rows,
        keep_together: true,
        landscape: true,
        dense: true,
    }
}

fn signature(eng: &Engagement, s: &SignOff) -> Signature {
    let blank = |v: &str| {
        if v.trim().is_empty() {
            "________________".to_string()
        } else {
            v.to_string()
        }
    };
    let left = vec![
        "As per our report of even date attached".to_string(),
        format!("For {}", blank(&s.auditor_firm)),
        "Chartered Accountants".to_string(),
        format!("Firm Registration No. {}", blank(&s.frn)),
        String::new(),
        String::new(),
        blank(&s.auditor_partner),
        "Partner".to_string(),
        format!("Membership No. {}", blank(&s.membership_no)),
        format!("UDIN: {}", blank(&s.udin)),
    ];
    let default_title = match eng.entity_type {
        EntityType::Company => ("Director", "DIN"),
        EntityType::Llp => ("Designated Partner", "DPIN"),
        EntityType::Firm => ("Partner", ""),
        EntityType::Proprietor => ("Proprietor", ""),
        EntityType::Huf => ("Karta", ""),
        EntityType::Aop | EntityType::Boi => ("Authorised Member", ""),
    };
    let mut right = vec![
        format!("For and on behalf of {}", eng.entity_name),
        String::new(),
        String::new(),
    ];
    let sigs: Vec<Signatory> = if s.signatories.is_empty() {
        let n = if matches!(eng.entity_type, EntityType::Proprietor | EntityType::Huf) {
            1
        } else {
            2
        };
        (0..n)
            .map(|_| Signatory {
                designation: default_title.0.into(),
                id_label: default_title.1.into(),
                ..Default::default()
            })
            .collect()
    } else {
        s.signatories.clone()
    };
    for (i, sg) in sigs.iter().enumerate() {
        if i > 0 {
            right.push(String::new());
            right.push(String::new());
        }
        right.push(blank(&sg.name));
        let des = if sg.designation.is_empty() {
            default_title.0.to_string()
        } else {
            sg.designation.clone()
        };
        if sg.id_label.is_empty() {
            right.push(des);
        } else {
            right.push(format!("{des} ({}: {})", sg.id_label, blank(&sg.id)));
        }
    }
    right.push(String::new());
    right.push(format!("Place: {}", blank(&s.place)));
    right.push(format!("Date: {}", blank(&s.date)));
    Signature { left, right }
}

// ---- disclosures entered by the user ---------------------------------------------

fn text_table(cols: &[(&str, &str, Align)], rows: Vec<Row>) -> Table {
    Table {
        title: None,
        columns: cols.iter().map(|(_, w, a)| col(w, *a)).collect(),
        header: vec![cols.iter().map(|(t, _, a)| hc(*t, 1, 1, *a)).collect()],
        keep_together: rows.len() <= 28,
        rows,
        landscape: false,
        dense: false,
    }
}

fn trow(style: RowStyle, cells: Vec<String>) -> Row {
    let n = cells.len();
    Row {
        style,
        indent: 0,
        cells,
        values: vec![None; n],
    }
}

fn count(n: i64) -> String {
    fmt_amount(Money(n * 100), crate::units::Unit::Rupees, 0)
}

fn pct(part: i64, whole: i64) -> String {
    if whole == 0 {
        "-".into()
    } else {
        format!("{:.2}%", part as f64 * 100.0 / whole as f64)
    }
}

/// Share capital (Schedule III, Division I, Part I, note on share capital).
fn share_capital_note(c: &mut Ctx, tc: Money, tp: Option<Money>) -> Vec<Block> {
    let d = &c.opt.disclosures;
    let g = c.f.g;
    let r = |m: Money| round_to(m, g);
    let mut rows = Vec::new();
    let (mut auth, mut auth_p, mut paid, mut paid_p) =
        (Money::ZERO, Money::ZERO, Money::ZERO, Money::ZERO);
    rows.push(NRow::head("Authorised"));
    for s in &d.share_classes {
        let (a, b) = (r(s.amount(s.authorised)), r(s.amount(s.py_authorised)));
        auth += a;
        auth_p += b;
        rows.push(NRow::item(
            1,
            format!("{} {}", count(s.authorised), s.name),
            a,
            Some(b),
        ));
    }
    rows.push(NRow::sub("", auth, Some(auth_p)));
    rows.push(NRow::head("Issued"));
    for s in &d.share_classes {
        rows.push(NRow::item(
            1,
            format!("{} {}", count(s.issued), s.name),
            r(s.amount(s.issued)),
            Some(r(s.amount(s.py_issued))),
        ));
    }
    rows.push(NRow::head("Subscribed and paid up"));
    for s in &d.share_classes {
        let (a, b) = (
            r(s.paid_amount(s.subscribed)),
            r(s.paid_amount(s.py_subscribed)),
        );
        paid += a;
        paid_p += b;
        let tail = if s.paid() == s.face_value {
            "fully paid up".to_string()
        } else {
            format!("{} paid up per share", c.f.s(s.paid()))
        };
        rows.push(NRow::item(
            1,
            format!("{} {}, {}", count(s.subscribed), s.name, tail),
            a,
            Some(b),
        ));
    }
    rows.push(NRow::total("Total", paid, Some(paid_p)));
    let rows: Vec<NRow> = rows
        .into_iter()
        .map(|mut x| {
            if !c.has_py() {
                x.py = None;
            }
            x
        })
        .collect();
    if paid != tc || (c.has_py() && tp.is_some() && Some(paid_p) != tp) {
        c.blockers.push(format!(
            "Share capital entered ({}) does not agree with the books ({}). Check the share particulars in Notes and disclosures.",
            c.f.s(paid),
            c.f.s(tc)
        ));
    }
    let mut blocks = vec![Block::Table(c.two_col(rows, true))];

    // Reconciliation of shares outstanding.
    let mut rec = Vec::new();
    for s in &d.share_classes {
        rec.push(trow(
            RowStyle::Subheading,
            vec![s.name.clone(), String::new()],
        ));
        rec.push(trow(
            RowStyle::Item,
            vec![
                "Shares outstanding at the beginning of the year".into(),
                count(s.opening()),
            ],
        ));
        rec.push(trow(
            RowStyle::Item,
            vec!["Add: Shares issued during the year".into(), count(s.added)],
        ));
        rec.push(trow(
            RowStyle::Item,
            vec![
                "Less: Shares bought back / reduced during the year".into(),
                count(s.reduced),
            ],
        ));
        rec.push(trow(
            RowStyle::Subtotal,
            vec![
                "Shares outstanding at the end of the year".into(),
                count(s.subscribed),
            ],
        ));
    }
    blocks.push(Block::Para {
        text: "Reconciliation of the number of shares outstanding".into(),
    });
    blocks.push(Block::Table(text_table(
        &[
            ("Particulars", "1fr", Align::Left),
            ("Number of shares", "40mm", Align::Right),
        ],
        rec,
    )));
    for s in d
        .share_classes
        .iter()
        .filter(|s| !s.rights.trim().is_empty())
    {
        blocks.push(Block::Para {
            text: format!(
                "Rights, preferences and restrictions ({}): {}",
                s.name,
                s.rights.trim()
            ),
        });
    }

    let total_cy: i64 = d.share_classes.iter().map(|s| s.subscribed).sum();
    let total_py: i64 = d.share_classes.iter().map(|s| s.py_subscribed).sum();
    if !d.holders_5pct.is_empty() {
        blocks.push(Block::Para {
            text: "Shares held by shareholders holding more than 5% of the shares".into(),
        });
        let rows = d
            .holders_5pct
            .iter()
            .map(|h| {
                trow(
                    RowStyle::Item,
                    vec![
                        h.name.clone(),
                        count(h.shares),
                        pct(h.shares, total_cy),
                        count(h.py_shares),
                        pct(h.py_shares, total_py),
                    ],
                )
            })
            .collect();
        blocks.push(Block::Table(text_table(
            &[
                ("Name of shareholder", "1fr", Align::Left),
                ("No. of shares (this year)", "30mm", Align::Right),
                ("% held", "20mm", Align::Right),
                ("No. of shares (last year)", "30mm", Align::Right),
                ("% held", "20mm", Align::Right),
            ],
            rows,
        )));
    }
    if !d.promoters.is_empty() {
        blocks.push(Block::Para {
            text: "Shares held by promoters at the end of the year".into(),
        });
        let rows = d
            .promoters
            .iter()
            .map(|h| {
                let change = if h.py_shares == 0 {
                    "-".to_string()
                } else {
                    format!(
                        "{:.2}%",
                        (h.shares - h.py_shares) as f64 * 100.0 / h.py_shares as f64
                    )
                };
                trow(
                    RowStyle::Item,
                    vec![
                        h.name.clone(),
                        count(h.shares),
                        pct(h.shares, total_cy),
                        change,
                    ],
                )
            })
            .collect();
        blocks.push(Block::Table(text_table(
            &[
                ("Promoter name", "1fr", Align::Left),
                ("No. of shares", "32mm", Align::Right),
                ("% of total shares", "30mm", Align::Right),
                ("% change during the year", "34mm", Align::Right),
            ],
            rows,
        )));
    }
    blocks
}

/// MSMED Act, 2006, section 22 particulars, under trade payables.
fn msme_block(c: &mut Ctx, principal_cy: Money, principal_py: Option<Money>) -> Vec<Block> {
    use crate::report::disclosures::Answer;
    let m = c.opt.disclosures.msme.clone();
    match c.opt.disclosures.answer("msme") {
        Answer::NotAnswered => {
            return vec![Block::Para {
                text: format!("Dues to micro and small enterprises: {NOT_PROVIDED}"),
            }];
        }
        Answer::Nil => {
            if !principal_cy.is_zero() {
                c.blockers.push("MSME marked as nil, but creditors tagged MSME have a balance. Correct the tags or the MSME answer.".into());
            }
            return vec![Block::Para { text: "There are no amounts due to micro and small enterprises as defined in the Micro, Small and Medium Enterprises Development Act, 2006, on the basis of information available with the entity.".into() }];
        }
        Answer::Provided => {}
    }
    let g = c.f.g;
    let r = |x: Money| round_to(x, g);
    let py = |v: (Money, Money)| if c.has_py() { Some(r(v.1)) } else { None };
    let rows = vec![
        NRow::head("Dues to micro and small enterprises (Micro, Small and Medium Enterprises Development Act, 2006)"),
        NRow::item(1, "Principal amount remaining unpaid at the end of the year", principal_cy, principal_py),
        NRow::item(1, "Interest due on the above and remaining unpaid", r(m.interest_due_unpaid.0), py(m.interest_due_unpaid)),
        NRow::item(1, "Interest paid in terms of section 16, along with payments made beyond the appointed day", r(m.interest_paid_s16.0), py(m.interest_paid_s16)),
        NRow::item(1, "Payments made to suppliers beyond the appointed day during the year", r(m.paid_beyond_appointed_day.0), py(m.paid_beyond_appointed_day)),
        NRow::item(1, "Interest due and payable for the period of delay (paid beyond the appointed day) without adding the interest under the Act", r(m.interest_due_for_delay.0), py(m.interest_due_for_delay)),
        NRow::item(1, "Interest accrued and remaining unpaid at the end of the year", r(m.interest_accrued_unpaid.0), py(m.interest_accrued_unpaid)),
        NRow::item(1, "Further interest remaining due and payable in succeeding years", r(m.further_interest.0), py(m.further_interest)),
        NRow::remark("Micro and small enterprises have been identified on the basis of information available with the entity."),
    ];
    vec![Block::Table(c.two_col(rows, true))]
}

/// Contingent liabilities and commitments, related parties and free notes.
fn extra_notes(c: &mut Ctx) -> Vec<(String, Vec<Block>)> {
    let d = c.opt.disclosures.clone();
    let g = c.f.g;
    let r = |x: Money| round_to(x, g);
    let mut out = Vec::new();
    let lines = |c: &Ctx,
                 title: &str,
                 v: &[crate::report::disclosures::AmountLine],
                 rows: &mut Vec<NRow>| {
        let v: Vec<_> = v.iter().filter(|l| !l.nature.trim().is_empty()).collect();
        if v.is_empty() {
            return;
        }
        rows.push(NRow::head(title));
        let (mut a, mut b) = (Money::ZERO, Money::ZERO);
        for l in v {
            a += r(l.cy);
            b += r(l.py);
            rows.push(NRow::item(
                1,
                l.nature.trim(),
                r(l.cy),
                c.has_py().then_some(r(l.py)),
            ));
        }
        rows.push(NRow::sub("", a, c.has_py().then_some(b)));
    };
    use crate::report::disclosures::Answer;
    let mut rows = Vec::new();
    lines(
        c,
        "Contingent liabilities (not provided for)",
        &d.contingent_liabilities,
        &mut rows,
    );
    lines(c, "Commitments", &d.commitments, &mut rows);
    let title = "Contingent liabilities and commitments".to_string();
    match d.answer("contingent") {
        Answer::Provided => out.push((title, vec![Block::Table(c.two_col(rows, true))])),
        Answer::Nil => out.push((
            title,
            vec![Block::Para {
                text: "There are no contingent liabilities or commitments.".into(),
            }],
        )),
        Answer::NotAnswered => out.push((
            title,
            vec![Block::Para {
                text: NOT_PROVIDED.into(),
            }],
        )),
    }

    let parties: Vec<_> = d
        .related_parties
        .iter()
        .filter(|p| !p.name.trim().is_empty())
        .collect();
    if !parties.is_empty() {
        let mut blocks = vec![Block::Para {
            text: "Names of related parties and nature of relationship (Accounting Standard 18)"
                .into(),
        }];
        let rows = parties
            .iter()
            .map(|p| {
                trow(
                    RowStyle::Item,
                    vec![p.name.trim().into(), p.relationship.trim().into()],
                )
            })
            .collect();
        blocks.push(Block::Table(text_table(
            &[
                ("Name", "1fr", Align::Left),
                ("Relationship", "1fr", Align::Left),
            ],
            rows,
        )));
        let txns: Vec<_> = d
            .related_transactions
            .iter()
            .filter(|t| !t.party.trim().is_empty())
            .collect();
        if !txns.is_empty() {
            blocks.push(Block::Para {
                text: "Transactions and balances with related parties".into(),
            });
            let mut cols = vec![
                ("Related party", "1fr", Align::Left),
                ("Nature", "1fr", Align::Left),
                ("This year", "32mm", Align::Right),
            ];
            if c.has_py() {
                cols.push(("Last year", "32mm", Align::Right));
            }
            let rows = txns
                .iter()
                .map(|t| {
                    let mut cells = vec![
                        t.party.trim().to_string(),
                        t.nature.trim().to_string(),
                        c.f.s(r(t.cy)),
                    ];
                    if c.has_py() {
                        cells.push(c.f.s(r(t.py)));
                    }
                    trow(RowStyle::Item, cells)
                })
                .collect();
            blocks.push(Block::Table(text_table(&cols, rows)));
        }
        out.push(("Related party disclosures".to_string(), blocks));
    } else {
        let text = match d.answer("related_parties") {
            Answer::Nil => "There are no related parties with whom transactions have taken place during the year, or with whom the entity has a relationship of control.",
            _ => NOT_PROVIDED,
        };
        out.push((
            "Related party disclosures".to_string(),
            vec![Block::Para { text: text.into() }],
        ));
    }

    for n in d.notes.iter().filter(|n| !n.title.trim().is_empty()) {
        let blocks = n
            .text
            .split("\n\n")
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| Block::Para {
                text: p.to_string(),
            })
            .collect();
        out.push((n.title.trim().to_string(), blocks));
    }
    out
}

/// Printed (on drafts only; a final copy is refused) where a required
/// disclosure has not been answered. Never printed as nil.
const NOT_PROVIDED: &str = "[Information not yet provided]";
