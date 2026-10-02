//! Orchestrates checks, mapping and statement building.

use crate::checks::{self, loans::LoanRow, Ctx, Detail, Finding, Findings};
use crate::groups::{Class, GroupResolver};
use crate::mapping::{group_default, name_rule, reclass_by_side, Head, MapSource};
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

    let pack = FormatPack::for_entity(eng.entity_type);
    let mapping = map_tb(
        &eng.cy,
        &ctx.classes,
        &eng.mapping_memory,
        pack.profit_to,
        Some(&mut f),
    );
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
        map_tb(py, &classes, &eng.mapping_memory, pack.profit_to, None)
    });

    let st = statements::build(
        eng.entity_type,
        eng.fy_end,
        &mapping,
        py_mapping.as_deref(),
        &pack,
    );

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
        printable,
        rules_version: rules.version.clone(),
    }
}

fn map_tb(
    tb: &TrialBalance,
    classes: &[Option<Class>],
    memory: &HashMap<String, String>,
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
        out.push(MappedLedger {
            name: l.name.clone(),
            group: l.group.clone(),
            class,
            head,
            source,
            reclassified,
            amount,
            tb_closing: l.closing,
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
