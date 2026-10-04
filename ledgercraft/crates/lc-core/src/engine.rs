//! Orchestrates checks, mapping and statement building.

use crate::checks::{self, loans::LoanRow, Ctx, Detail, Finding, Findings};
use crate::groups::{Class, GroupResolver};
use crate::mapping::{
    group_default, is_unambiguous, memory_context, name_rule, reclass_by_side, Head, MapSource,
    MapStatus,
};
use crate::model::{norm_name, Engagement, TrialBalance};
use crate::rules::{RulesPack, Severity};
use crate::statements::{self, FormatPack, MappedLedger, Statements};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub findings: Vec<Finding>,
    pub mapping: Vec<MappedLedger>,
    pub py_mapping: Option<Vec<MappedLedger>>,
    pub statements: Statements,
    pub loans: Vec<LoanRow>,
    /// Exact figures by head (current / previous year).
    pub facts_cy: crate::facts::YearFacts,
    pub facts_py: Option<crate::facts::YearFacts>,
    /// Owner-wise capital movement (non-corporate entities).
    pub capital: Vec<crate::capital::CapitalRow>,
    pub ageing_receivables: Option<crate::ageing::Ageing>,
    pub ageing_payables: Option<crate::ageing::Ageing>,
    pub ratios: Vec<crate::ratios::Ratio>,
    pub far: Option<crate::far::FarResult>,
    /// True when there is no blocker: statements may be exported as a signing copy.
    pub printable: bool,
    pub rules_version: String,
}

impl Analysis {
    pub fn count(&self, s: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == s).count()
    }
}

pub fn analyse(eng: &Engagement, rules: &RulesPack) -> Analysis {
    let ctx = Ctx::new(eng, rules);
    let mut f = Findings::new(rules, eng.fy_start);

    checks::tb::run(&ctx, &mut f);
    checks::opening::run(&ctx, &mut f);
    checks::classify::run(&ctx, &mut f);
    checks::vouchers::run(&ctx, &mut f);
    checks::cash::run(&ctx, &mut f);
    let loans = checks::loans::run(&ctx, &mut f);

    // A remembered mapping resolves an unknown group.
    f.list.retain(|x| {
        !(x.code == "UNKNOWN_GROUP"
            && x.ledger
                .as_ref()
                .map(|l| eng.mapping_memory.contains_key(&norm_name(l)))
                .unwrap_or(false))
    });

    let pack = FormatPack::of(eng);
    let mapping = map_tb(
        &eng.cy,
        &ctx.classes,
        &eng.mapping_memory,
        &eng.mapping_context,
        pack.profit_to,
        Some(&mut f),
    );
    // Placements that need the user (TRUTH-MODEL.md §4–5): one finding each kind.
    for (status, code) in [
        (MapStatus::Suggested, "MAPPING_UNCONFIRMED"),
        (MapStatus::Review, "MAPPING_REVIEW"),
    ] {
        let names: Vec<&str> = mapping
            .iter()
            .filter(|m| m.status == status)
            .map(|m| m.name.as_str())
            .collect();
        if !names.is_empty() {
            let shown: Vec<&str> = names.iter().take(8).copied().collect();
            f.add(
                code,
                "mapping",
                &format!(
                    "{} ledger(s): {}{}.",
                    names.len(),
                    shown.join(", "),
                    if names.len() > shown.len() {
                        ", …"
                    } else {
                        ""
                    }
                ),
                Detail::default(),
            );
        }
    }
    let py_mapping = eng.py.as_ref().map(|py| {
        let res = GroupResolver::new(py);
        let cy_res = GroupResolver::new(&eng.cy);
        let classes: Vec<Option<Class>> = py
            .ledgers
            .iter()
            .map(|l| {
                res.resolve(&l.group)
                    .ok()
                    .or_else(|| cy_res.resolve(&l.group).ok())
            })
            .collect();
        map_tb(
            py,
            &classes,
            &eng.mapping_memory,
            &HashMap::new(),
            pack.profit_to,
            None,
        )
    });

    let st = statements::build(
        eng.entity_type,
        eng.fy_end,
        &mapping,
        py_mapping.as_deref(),
        &pack,
    );

    let facts_cy = crate::facts::YearFacts::build(&mapping, pack.profit_to);
    let facts_py = py_mapping
        .as_ref()
        .map(|m| crate::facts::YearFacts::build(m, pack.profit_to));
    let capital = if eng.entity_type.is_company() {
        Vec::new()
    } else {
        crate::capital::movements(
            &ctx,
            facts_cy.profit() + plac_balance(&facts_cy),
            &eng.profit_sharing,
        )
    };
    let ageing_receivables =
        crate::ageing::compute(&ctx, &mapping, crate::ageing::AgeingKind::Receivables);
    let ageing_payables =
        crate::ageing::compute(&ctx, &mapping, crate::ageing::AgeingKind::Payables);
    let repaid: crate::money::Money = loans.iter().map(|l| l.repaid()).sum();
    let ratios = crate::ratios::compute(&facts_cy, facts_py.as_ref(), repaid);
    let far = eng
        .far
        .as_ref()
        .map(|reg| far_checks(&ctx, reg, &mapping, &facts_cy, &mut f));

    let structural = f.list.iter().any(|x| {
        matches!(
            x.code.as_str(),
            "TB_UNBALANCED" | "UNKNOWN_GROUP" | "UNMAPPED"
        )
    });
    if !structural {
        let (a, l) = (st.total_assets, st.total_liabilities);
        if a.0 != l.0 || a.1 != l.1 {
            f.add(
                "STATEMENT_NOT_BALANCED",
                "balance_sheet",
                &format!("Assets {} vs Equity & Liabilities {}.", a.0, l.0),
                Detail::default(),
            );
        }
    }

    let mut findings = f.list;
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then(a.code.cmp(&b.code))
            .then(a.key.cmp(&b.key))
    });
    let printable = !findings.iter().any(|x| x.severity == Severity::Blocker);
    Analysis {
        findings,
        mapping,
        py_mapping,
        statements: st,
        loans,
        facts_cy,
        facts_py,
        capital,
        ageing_receivables,
        ageing_payables,
        ratios,
        far,
        printable,
        rules_version: rules.version.clone(),
    }
}

fn map_tb(
    tb: &TrialBalance,
    classes: &[Option<Class>],
    memory: &HashMap<String, String>,
    context: &HashMap<String, String>,
    profit_to: Head,
    mut findings: Option<&mut Findings>,
) -> Vec<MappedLedger> {
    let mut out = Vec::with_capacity(tb.ledgers.len());
    for (i, l) in tb.ledgers.iter().enumerate() {
        let class = classes[i];
        let amount = if class == Some(Class::StockInHand) {
            l.closing_stock.unwrap_or(l.closing)
        } else {
            l.closing
        };
        let remembered = memory.get(&norm_name(&l.name));
        let (mut head, source) = match (remembered, class) {
            (Some(id), _) => match Head::from_id(id) {
                Some(h) => (Some(h), Some(MapSource::Memory)),
                None => {
                    if let Some(f) = findings.as_deref_mut() {
                        f.add(
                            "UNMAPPED",
                            &l.name,
                            &format!("Saved mapping '{id}' for '{}' is not a valid head.", l.name),
                            Detail {
                                ledger: Some(l.name.clone()),
                                ..Default::default()
                            },
                        );
                    }
                    (None, None)
                }
            },
            // The Profit & Loss A/c ledger sits where the year's profit goes, so a
            // profit already transferred (e.g. to partners' capital) nets to nil.
            (None, Some(Class::ProfitLossAc)) => (Some(profit_to), Some(MapSource::GroupDefault)),
            (None, Some(c)) => match name_rule(c, &l.name) {
                Some(h) => (Some(h), Some(MapSource::NameRule)),
                None => (Some(group_default(c)), Some(MapSource::GroupDefault)),
            },
            (None, None) => (None, None),
        };
        let mut reclassified = false;
        if let (Some(h), Some(c), None) = (head, class, remembered) {
            if let Some((nh, code)) = reclass_by_side(h, c, amount) {
                head = Some(nh);
                reclassified = true;
                if let Some(f) = findings.as_deref_mut() {
                    f.add(
                        code,
                        &l.name,
                        &format!("'{}' shows {}.", l.name, amount.fmt_drcr()),
                        Detail {
                            ledger: Some(l.name.clone()),
                            amount: Some(amount),
                            ..Default::default()
                        },
                    );
                }
            }
        }
        let key = norm_name(&l.name);
        let (status, status_reason) = match (head, source) {
            (None, _) | (_, None) => (MapStatus::Unmapped, "No line chosen yet".to_string()),
            (Some(_), Some(MapSource::Memory)) => match context.get(&key) {
                Some(ctx) if *ctx != memory_context(&l.group, l.closing) => {
                    let (g0, s0) = ctx.split_once('|').unwrap_or((ctx.as_str(), ""));
                    let now = memory_context(&l.group, l.closing);
                    let (g1, s1) = now.split_once('|').unwrap_or((now.as_str(), ""));
                    let why = if g0 != g1 {
                        format!("Group changed since you confirmed it (was '{g0}')")
                    } else {
                        format!(
                            "Balance changed from {} to {}",
                            s0.to_uppercase(),
                            s1.to_uppercase()
                        )
                    };
                    (MapStatus::Review, why)
                }
                _ => (MapStatus::Confirmed, "Your choice".to_string()),
            },
            (Some(_), Some(MapSource::NameRule)) => (
                MapStatus::Suggested,
                "Placed by the ledger name".to_string(),
            ),
            (Some(_), Some(MapSource::GroupDefault)) => {
                if reclassified {
                    (
                        MapStatus::Suggested,
                        "Moved to the other side because of its balance".to_string(),
                    )
                } else if class.map(is_unambiguous).unwrap_or(false) {
                    (
                        MapStatus::Rule,
                        "Only one line possible for this group".to_string(),
                    )
                } else {
                    (MapStatus::Suggested, "This group can go under more than one line (for example long-term or short-term)".to_string())
                }
            }
        };
        out.push(MappedLedger {
            name: l.name.clone(),
            group: l.group.clone(),
            class,
            head,
            source,
            reclassified,
            amount,
            tb_closing: l.closing,
            tags: l.tags.clone(),
            status,
            status_reason,
        });
    }
    out
}

/// Convenience: findings of at least this severity.
pub fn at_least(findings: &[Finding], s: Severity) -> impl Iterator<Item = &Finding> {
    findings.iter().filter(move |f| f.severity <= s)
}

#[allow(dead_code)]
fn _assert_severity_order() {
    debug_assert!(Severity::Blocker < Severity::Warning && Severity::Warning < Severity::Info);
}

fn far_checks(
    ctx: &Ctx,
    reg: &crate::far::Register,
    mapping: &[MappedLedger],
    facts: &crate::facts::YearFacts,
    f: &mut Findings,
) -> crate::far::FarResult {
    let eng = ctx.eng;
    let res = crate::far::compute(
        reg,
        &crate::far::DepPack::builtin(),
        eng.fy_start,
        eng.fy_end,
    );
    for e in &res.errors {
        f.add("FAR_ERROR", e, e, Detail::default());
    }
    for row in &res.ppe {
        let tb = ctx.lookup(&row.ledger).map(|i| ctx.ledger(i).closing);
        match tb {
            Some(c) if c != row.net_closing => f.add(
                "FAR_TB_MISMATCH",
                &row.ledger,
                &format!("'{}': register net block {} vs trial balance {}.", row.ledger, row.net_closing, c.fmt_drcr()),
                Detail { ledger: Some(row.ledger.clone()), amount: Some(c - row.net_closing), suggestion: Some("If accumulated depreciation is kept in a separate ledger, map it to this asset; otherwise correct the register or the books.".into()), ..Default::default() },
            ),
            None => f.add(
                "FAR_TB_MISMATCH",
                &row.ledger,
                &format!("'{}' is in the register but not in the trial balance.", row.ledger),
                Detail { ledger: Some(row.ledger.clone()), ..Default::default() },
            ),
            _ => {}
        }
    }
    for m in mapping {
        if matches!(m.head, Some(Head::Ppe) | Some(Head::Intangibles))
            && !m.amount.is_zero()
            && !res
                .ppe
                .iter()
                .any(|r| norm_name(&r.ledger) == norm_name(&m.name))
        {
            f.add(
                "FAR_MISSING_LEDGER",
                &m.name,
                &format!("'{}' shows {}.", m.name, m.amount.fmt_drcr()),
                Detail {
                    ledger: Some(m.name.clone()),
                    amount: Some(m.amount),
                    ..Default::default()
                },
            );
        }
    }
    let booked = facts.lines_total(Head::Depreciation);
    if booked != res.book_dep_total {
        f.add(
            "FAR_DEP_MISMATCH",
            "depreciation",
            &format!("Books {} vs register {}.", booked, res.book_dep_total),
            Detail {
                amount: Some(booked - res.book_dep_total),
                suggestion: Some(
                    "Pass an adjustment entry for the difference or correct the register.".into(),
                ),
                ..Default::default()
            },
        );
    }
    res
}

/// Balance of the Profit & Loss A/c ledger(s) in display sign (Cr positive).
pub fn plac_balance(f: &crate::facts::YearFacts) -> crate::money::Money {
    f.lines
        .values()
        .flatten()
        .filter(|l| l.class == Some(Class::ProfitLossAc))
        .map(|l| l.amount)
        .sum()
}
