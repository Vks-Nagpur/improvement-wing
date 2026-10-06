//! Checks on the shipped packs (run by the test suite and by CI through
//! `ledgercraft packs`): well-formed, internally consistent, and never
//! claiming that legal content was verified. Verification is recorded only by
//! a person in the Legal verification register (TRUTH-MODEL.md §7).

use crate::far::DepPack;
use crate::legal::{self, Scope};
use crate::model::{Engagement, EntityType};
use crate::rules::{RulesPack, PENDING_REF};
use crate::statements::FormatPack;
use crate::taxaudit::TaxAuditSchema;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ENTITIES: [EntityType; 7] = [
    EntityType::Company,
    EntityType::Llp,
    EntityType::Firm,
    EntityType::Proprietor,
    EntityType::Huf,
    EntityType::Aop,
    EntityType::Boi,
];

/// Statuses a pack may ship with. "verified" is not among them.
const SHIPPED: [&str; 6] = [
    "draft",
    "secondary",
    "conflicting",
    "unknown",
    "internal",
    "unverified",
];

const FORMS: [(&str, &str); 2] = [
    (
        "taxaudit_form26.json",
        include_str!("../packs/taxaudit_form26.json"),
    ),
    (
        "taxaudit_form3cd.json",
        include_str!("../packs/taxaudit_form3cd.json"),
    ),
];

fn shipped_ok(what: &str, status: &str, errs: &mut Vec<String>) {
    if !SHIPPED.contains(&status.trim().to_ascii_lowercase().as_str()) {
        errs.push(format!(
            "{what}: status {status:?} is not allowed in a shipped pack (allowed: {SHIPPED:?}); only a person records verification"
        ));
    }
}

/// Pack-level statuses are sentences; they must not claim verification.
fn note_ok(what: &str, status: &str, errs: &mut Vec<String>) {
    let s = status.trim().to_ascii_lowercase();
    if s.is_empty() || s.starts_with("verified") || s.starts_with("final") {
        errs.push(format!(
            "{what}: status {status:?} must say the content is not yet verified; only a person records verification"
        ));
    }
}

/// Every problem found in the built-in packs; empty means all good.
pub fn validate() -> Vec<String> {
    let mut errs = Vec::new();
    let rules = RulesPack::builtin();
    let provisions = &rules.sections.provisions;

    for (id, p) in provisions {
        shipped_ok(&format!("sections.json {id}"), &p.status, &mut errs);
        if p.act.trim().is_empty() || p.title.trim().is_empty() {
            errs.push(format!("sections.json {id}: act and title are required"));
        }
        if p.status == "secondary" && p.citation.trim().is_empty() {
            errs.push(format!(
                "sections.json {id}: a 'secondary' provision needs a citation"
            ));
        }
    }

    note_ok("rules.json pack", &rules.verification, &mut errs);
    for (code, r) in &rules.rules {
        let w = format!("rules.json {code}");
        shipped_ok(&w, &r.verification, &mut errs);
        for p in r.refs_old.iter().chain(&r.refs_new) {
            if !provisions.contains_key(p) {
                errs.push(format!(
                    "{w}: refers to provision {p:?} not in sections.json"
                ));
            }
        }
        if r.title.trim().is_empty() || r.simple.trim().is_empty() {
            errs.push(format!("{w}: title and plain-language text are required"));
        }
        // A statutory rule must cite through sections.json, not free text,
        // and must say what the user should do next.
        let statutory = !r.refs_old.is_empty() || !r.refs_new.is_empty();
        if statutory && r.next_step.trim().is_empty() {
            errs.push(format!(
                "{w}: a rule with statutory content needs next_step"
            ));
        }
        if !statutory
            && r.verification != "internal"
            && !r.old_ref.contains(PENDING_REF)
            && !r.new_ref.contains(PENDING_REF)
            && (!r.old_ref.trim().is_empty() || !r.new_ref.trim().is_empty())
        {
            errs.push(format!(
                "{w}: has a free-text statutory reference but no provision ids"
            ));
        }
    }

    for e in ENTITIES {
        let f = FormatPack::for_entity(e);
        let w = format!("format {}", f.id);
        note_ok(&w, &f.status, &mut errs);
        if f.authority.trim().is_empty() || f.document.trim().is_empty() {
            errs.push(format!("{w}: authority and document are required"));
        }
        if f.balance_sheet.is_empty() || f.profit_loss.is_empty() {
            errs.push(format!(
                "{w}: balance sheet and profit and loss lines are required"
            ));
        }
    }

    let dep = DepPack::builtin();
    note_ok("depreciation.json", &dep.status, &mut errs);
    if dep.book_classes.is_empty() || dep.it_blocks.is_empty() {
        errs.push("depreciation.json: book classes and income-tax blocks are required".into());
    }

    for (name, text) in FORMS {
        match serde_json::from_str::<TaxAuditSchema>(text) {
            Err(e) => errs.push(format!("{name}: {e}")),
            Ok(s) => {
                shipped_ok(name, &s.status, &mut errs);
                let mut keys = BTreeSet::new();
                for c in s.clauses.iter().chain(&s.supporting) {
                    if !keys.insert(c.key.clone()) {
                        errs.push(format!("{name}: clause key {:?} repeats", c.key));
                    }
                    for p in &c.statutory {
                        if !provisions.contains_key(p) {
                            errs.push(format!(
                                "{name} {}: refers to provision {p:?} not in sections.json",
                                c.key
                            ));
                        }
                    }
                }
            }
        }
    }
    errs
}

pub fn sample_engagement(e: EntityType, fy: i32) -> Engagement {
    Engagement {
        entity_name: "Sample".into(),
        entity_type: e,
        fy_start: chrono::NaiveDate::from_ymd_opt(fy, 4, 1).unwrap(),
        fy_end: chrono::NaiveDate::from_ymd_opt(fy + 1, 3, 31).unwrap(),
        cy: Default::default(),
        py: None,
        vouchers: vec![],
        mapping_memory: Default::default(),
        mapping_context: Default::default(),
        format_pack: None,
        consolidation: None,
        comparative_end: None,
        far: None,
        profit_sharing: vec![],
    }
}

#[derive(Debug, Serialize)]
pub struct ReadinessRow {
    pub entity: String,
    pub year: String,
    pub applicable: usize,
    /// Items by status as shipped (secondary, unknown, ...).
    pub shipped: BTreeMap<String, usize>,
}

/// How much legal content applies to each entity type and year, by shipped
/// status, with nothing verified (as the program ships).
pub fn readiness_matrix(years: &[i32]) -> Vec<ReadinessRow> {
    let rules = RulesPack::builtin();
    let dep = DepPack::builtin();
    let mut out = Vec::new();
    for &fy in years {
        for e in ENTITIES {
            let eng = sample_engagement(e, fy);
            let fmt = FormatPack::for_entity(e);
            let items = legal::applicable(
                &eng,
                &rules,
                &fmt,
                &dep,
                Scope {
                    tax_audit: true,
                    depreciation: true,
                },
            );
            let mut shipped = BTreeMap::new();
            for it in &items {
                // Pack statuses are sentences ("draft – line items to be ..."): keep the first word.
                let s = it
                    .shipped_status
                    .split(|c: char| !c.is_alphanumeric())
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                *shipped.entry(s).or_insert(0) += 1;
            }
            out.push(ReadinessRow {
                entity: format!("{e:?}"),
                year: format!("{}-{:02}", fy, (fy + 1) % 100),
                applicable: items.len(),
                shipped,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_packs_are_valid_and_claim_nothing_verified() {
        let errs = validate();
        assert!(errs.is_empty(), "{errs:#?}");
        for row in readiness_matrix(&[2025, 2026]) {
            assert!(row.applicable > 0, "{row:?}");
            assert!(
                !row.shipped.contains_key("verified"),
                "{} {} ships verified items",
                row.entity,
                row.year
            );
        }
    }

    #[test]
    fn a_pack_claiming_verified_is_refused() {
        let mut errs = Vec::new();
        shipped_ok("x", "Verified", &mut errs);
        shipped_ok("y", "secondary", &mut errs);
        note_ok("z", "Verified against the Gazette", &mut errs);
        note_ok("w", "draft – to be verified", &mut errs);
        assert_eq!(errs.len(), 2);
    }
}
