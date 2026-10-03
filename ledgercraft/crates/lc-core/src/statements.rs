//! Builds the Balance Sheet, Statement of Profit and Loss and notes from
//! mapped ledgers using a format pack (layout as data).

use crate::groups::{Class, Nature};
use crate::mapping::Head;
use crate::model::EntityType;
use crate::money::Money;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct PackRow {
    pub t: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub head: Option<Head>,
    #[serde(default)]
    pub heads: Vec<Head>,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub calc: Option<String>,
    /// Show even when nil in both years (keeps fixed numbering intact).
    #[serde(default)]
    pub keep: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormatPack {
    pub id: String,
    pub name: String,
    pub status: String,
    pub bs_title: String,
    pub pl_title: String,
    pub profit_to: Head,
    pub profit_note_label: String,
    pub balance_sheet: Vec<PackRow>,
    pub profit_loss: Vec<PackRow>,
}

const COMPANY_DIV1: &str = include_str!("../packs/company_div1.json");
const NON_CORPORATE: &str = include_str!("../packs/non_corporate.json");
const LLP: &str = include_str!("../packs/llp.json");

impl FormatPack {
    /// Format pack for the entity: Schedule III Div I, ICAI LLP or non-corporate Guidance Note.
    pub fn for_entity(e: EntityType) -> FormatPack {
        let src = match e {
            EntityType::Company => COMPANY_DIV1,
            EntityType::Llp => LLP,
            _ => NON_CORPORATE,
        };
        serde_json::from_str(src).expect("built-in format pack is valid")
    }
}

pub fn capital_label(e: EntityType) -> &'static str {
    match e {
        EntityType::Company => "Share capital",
        EntityType::Llp => "Partners' contribution",
        EntityType::Firm => "Partners' capital accounts",
        EntityType::Proprietor => "Proprietor's capital account",
        EntityType::Huf => "Capital account (HUF)",
        EntityType::Aop | EntityType::Boi => "Members' capital accounts",
    }
}

pub fn remuneration_label(e: EntityType) -> &'static str {
    match e {
        EntityType::Firm | EntityType::Llp => "Partners' remuneration and interest",
        EntityType::Aop | EntityType::Boi => "Members' remuneration and interest",
        _ => "Remuneration to owners",
    }
}

/// One ledger after mapping, for one year.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappedLedger {
    pub name: String,
    pub group: String,
    pub class: Option<Class>,
    pub head: Option<Head>,
    pub source: Option<crate::mapping::MapSource>,
    pub reclassified: bool,
    /// Signed balance used in statements (stock ledgers: closing stock value).
    pub amount: Money,
    /// Signed trial-balance closing.
    pub tb_closing: Money,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Heading,
    SubHeading,
    Item,
    Subtotal,
    Total,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub kind: RowKind,
    pub label: String,
    pub note: Option<u32>,
    pub cy: Option<Money>,
    pub py: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteLine {
    pub label: String,
    pub cy: Money,
    pub py: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub no: u32,
    pub title: String,
    pub lines: Vec<NoteLine>,
    pub total_cy: Money,
    pub total_py: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statements {
    pub pack_id: String,
    pub pack_status: String,
    pub bs_title: String,
    pub pl_title: String,
    pub balance_sheet: Vec<Row>,
    pub profit_loss: Vec<Row>,
    pub notes: Vec<Note>,
    pub total_assets: (Money, Option<Money>),
    pub total_liabilities: (Money, Option<Money>),
    pub profit: (Money, Option<Money>),
}

/// Per-year figures by head, in *display* sign (assets/expenses Dr positive,
/// liabilities/capital/income Cr positive).
struct Year {
    heads: BTreeMap<Head, Money>,
    ledgers: BTreeMap<Head, Vec<(String, Money)>>,
    opening_stock: Money,
    closing_stock: Money,
    profit: Money,
    pbt: Money,
}

fn display(head: Head, signed: Money) -> Money {
    match head.nature() {
        Nature::Asset | Nature::Expense => signed,
        _ => -signed,
    }
}

fn year(mapped: &[MappedLedger], pack: &FormatPack) -> Year {
    let mut heads: BTreeMap<Head, Money> = BTreeMap::new();
    let mut ledgers: BTreeMap<Head, Vec<(String, Money)>> = BTreeMap::new();
    let mut opening_stock = Money::ZERO;
    let mut closing_stock = Money::ZERO;
    for m in mapped {
        let Some(h) = m.head else { continue };
        if m.class == Some(Class::StockInHand) {
            opening_stock += m.tb_closing;
            closing_stock += m.amount;
        }
        let d = display(h, m.amount);
        *heads.entry(h).or_default() += d;
        if !m.amount.is_zero() {
            ledgers.entry(h).or_default().push((m.name.clone(), d));
        }
    }
    let change = opening_stock - closing_stock;
    *heads.entry(Head::ChangeInInventories).or_default() += change;
    let get = |h: Head| heads.get(&h).copied().unwrap_or_default();
    let income: Money = Head::ALL
        .iter()
        .filter(|h| h.nature() == Nature::Income)
        .map(|h| get(*h))
        .sum();
    let expense_ex_tax: Money = Head::ALL
        .iter()
        .filter(|h| h.nature() == Nature::Expense && **h != Head::TaxExpense)
        .map(|h| get(*h))
        .sum();
    let pbt = income - expense_ex_tax;
    let profit = pbt - get(Head::TaxExpense);
    *heads.entry(pack.profit_to).or_default() += profit;
    Year {
        heads,
        ledgers,
        opening_stock,
        closing_stock,
        profit,
        pbt,
    }
}

pub fn build(
    entity: EntityType,
    fy_end: NaiveDate,
    cy: &[MappedLedger],
    py: Option<&[MappedLedger]>,
    pack: &FormatPack,
) -> Statements {
    let c = year(cy, pack);
    let p = py.map(|m| year(m, pack));
    let fill = |s: &str| {
        s.replace("{end}", &fy_end.format("%d %B %Y").to_string())
            .replace("{capital_label}", capital_label(entity))
            .replace("{remuneration_label}", remuneration_label(entity))
    };
    let get = |y: &Year, h: Head| y.heads.get(&h).copied().unwrap_or_default();

    let mut notes: Vec<Note> = Vec::new();
    let mut note_no: BTreeMap<Head, u32> = BTreeMap::new();

    let mut render = |rows: &[PackRow], notes: &mut Vec<Note>| -> Vec<Row> {
        let mut out = Vec::new();
        for r in rows {
            let label = fill(&r.label);
            match r.t.as_str() {
                "h" => out.push(Row {
                    kind: RowKind::Heading,
                    label,
                    note: None,
                    cy: None,
                    py: None,
                }),
                "s" | "s2" => out.push(Row {
                    kind: RowKind::SubHeading,
                    label,
                    note: None,
                    cy: None,
                    py: None,
                }),
                "i" => {
                    let h = r.head.expect("item row has a head");
                    let vc = get(&c, h);
                    let vp = p.as_ref().map(|y| get(y, h));
                    let nil = vc.is_zero() && vp.map(|x| x.is_zero()).unwrap_or(true);
                    if nil && r.keep {
                        out.push(Row {
                            kind: RowKind::Item,
                            label,
                            note: None,
                            cy: Some(vc),
                            py: vp,
                        });
                        continue;
                    }
                    if nil {
                        continue;
                    }
                    let no = *note_no.entry(h).or_insert_with(|| {
                        let no = notes.len() as u32 + 1;
                        notes.push(make_note(no, h, &label, &c, p.as_ref(), pack, entity));
                        no
                    });
                    out.push(Row {
                        kind: RowKind::Item,
                        label,
                        note: Some(no),
                        cy: Some(vc),
                        py: vp,
                    });
                }
                "sum" => {
                    let vc: Money = r.heads.iter().map(|h| get(&c, *h)).sum();
                    let vp = p.as_ref().map(|y| r.heads.iter().map(|h| get(y, *h)).sum());
                    out.push(Row {
                        kind: RowKind::Subtotal,
                        label,
                        note: None,
                        cy: Some(vc),
                        py: vp,
                    });
                }
                "calc" => {
                    let f = |y: &Year| match r.calc.as_deref() {
                        Some("pbt") => y.pbt,
                        Some("pbr") => y.pbt + get(y, Head::PartnersRemuneration),
                        _ => y.profit,
                    };
                    out.push(Row {
                        kind: RowKind::Subtotal,
                        label,
                        note: None,
                        cy: Some(f(&c)),
                        py: p.as_ref().map(f),
                    });
                }
                "total" => {
                    let side = r.side.as_deref().unwrap_or("asset");
                    let tot = |y: &Year| -> Money {
                        y.heads
                            .iter()
                            .filter(|(h, _)| match side {
                                "liab" => matches!(h.nature(), Nature::Liability | Nature::Capital),
                                _ => h.nature() == Nature::Asset,
                            })
                            .map(|(_, v)| *v)
                            .sum()
                    };
                    out.push(Row {
                        kind: RowKind::Total,
                        label,
                        note: None,
                        cy: Some(tot(&c)),
                        py: p.as_ref().map(tot),
                    });
                }
                other => panic!("unknown pack row type {other}"),
            }
        }
        prune_empty_headings(out)
    };

    let balance_sheet = render(&pack.balance_sheet, &mut notes);
    let profit_loss = render(&pack.profit_loss, &mut notes);
    let side_total = |rows: &[Row], n: usize| {
        rows.iter()
            .filter(|r| r.kind == RowKind::Total)
            .nth(n)
            .map(|r| (r.cy.unwrap_or_default(), r.py))
    };
    let total_liabilities = side_total(&balance_sheet, 0).unwrap_or_default();
    let total_assets = side_total(&balance_sheet, 1).unwrap_or_default();

    Statements {
        pack_id: pack.id.clone(),
        pack_status: pack.status.clone(),
        bs_title: fill(&pack.bs_title),
        pl_title: fill(&pack.pl_title),
        balance_sheet,
        profit_loss,
        notes,
        total_assets,
        total_liabilities,
        profit: (c.profit, p.as_ref().map(|y| y.profit)),
    }
}

fn make_note(
    no: u32,
    h: Head,
    label: &str,
    c: &Year,
    p: Option<&Year>,
    pack: &FormatPack,
    entity: EntityType,
) -> Note {
    let title = strip_numbering(label);
    let mut lines: Vec<NoteLine> = Vec::new();
    if h == Head::ChangeInInventories {
        lines.push(NoteLine {
            label: "Inventories at the beginning of the year".into(),
            cy: c.opening_stock,
            py: p.map(|y| y.opening_stock),
        });
        lines.push(NoteLine {
            label: "Less: Inventories at the end of the year".into(),
            cy: -c.closing_stock,
            py: p.map(|y| -y.closing_stock),
        });
    } else {
        // Ledger-wise schedule; union of CY and PY ledger names.
        let mut names: Vec<String> = Vec::new();
        for y in std::iter::once(c).chain(p) {
            for (n, _) in y.ledgers.get(&h).into_iter().flatten() {
                if !names.contains(n) {
                    names.push(n.clone());
                }
            }
        }
        let find = |y: &Year, n: &str| {
            y.ledgers
                .get(&h)
                .and_then(|v| v.iter().find(|(x, _)| x == n))
                .map(|(_, a)| *a)
                .unwrap_or_default()
        };
        for n in names {
            lines.push(NoteLine {
                cy: find(c, &n),
                py: p.map(|y| find(y, &n)),
                label: n,
            });
        }
        if h == pack.profit_to {
            lines.push(NoteLine {
                label: pack.profit_note_label.clone(),
                cy: c.profit,
                py: p.map(|y| y.profit),
            });
        }
    }
    let _ = entity;
    let total_cy = c.heads.get(&h).copied().unwrap_or_default();
    let total_py = p.map(|y| y.heads.get(&h).copied().unwrap_or_default());
    Note {
        no,
        title,
        lines,
        total_cy,
        total_py,
    }
}

fn strip_numbering(label: &str) -> String {
    let t = label.trim();
    if let Some(rest) = t.strip_prefix('(') {
        if let Some(pos) = rest.find(')') {
            return rest[pos + 1..].trim().to_string();
        }
    }
    if let Some(pos) = t.find(". ") {
        if t[..pos].chars().all(|c| c.is_ascii_uppercase()) {
            return t[pos + 2..].to_string();
        }
    }
    t.to_string()
}

/// Remove headings that have no item below them before the next heading/total.
fn prune_empty_headings(rows: Vec<Row>) -> Vec<Row> {
    let level = |k: RowKind| match k {
        RowKind::Heading => 0,
        RowKind::SubHeading => 1,
        _ => 9,
    };
    let mut keep = vec![true; rows.len()];
    for i in 0..rows.len() {
        let k = rows[i].kind;
        if !matches!(k, RowKind::Heading | RowKind::SubHeading) {
            continue;
        }
        let mut has_content = false;
        for r in &rows[i + 1..] {
            match r.kind {
                RowKind::Item | RowKind::Subtotal => {
                    has_content = true;
                    break;
                }
                RowKind::Total => break,
                kk if level(kk) <= level(k) => break,
                _ => {}
            }
        }
        // Sub-sub headings (s2) are SubHeading too; they share the same rule.
        keep[i] = has_content;
    }
    rows.into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(r, _)| r)
        .collect()
}
