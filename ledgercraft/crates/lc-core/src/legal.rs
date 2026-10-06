//! Legal content readiness (TRUTH-MODEL.md §7 and §10).
//!
//! Every piece of legal content the program relies on for a client year is a
//! *legal item*: a format line, a rule with statutory content, a statutory
//! provision, a depreciation row, the tax audit form. Each item has a content
//! hash. A person records a verification against the official text; the
//! record is bound to that hash, so any change to the item voids it.
//! The program never marks anything verified by itself.
//!
//! A final (signing) copy needs every applicable item verified.

use crate::far::DepPack;
use crate::model::Engagement;
use crate::rules::RulesPack;
use crate::statements::FormatPack;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegalItem {
    /// Stable id, e.g. `format:company_div1:bs:07`, `rule:CASH_RECEIPT_LIMIT`,
    /// `provision:it2025.s185`, `dep:book:furniture`, `taxaudit:form26`.
    pub id: String,
    /// format_line, rule, provision, depreciation, tax_audit_form
    pub kind: String,
    /// Heading to group items under in the register.
    pub group: String,
    pub title: String,
    /// The exact content a person compares with the official text.
    pub content: String,
    pub hash: String,
    /// Status as shipped (draft, secondary, conflicting, unknown, ...).
    pub shipped_status: String,
}

fn item(
    id: String,
    kind: &str,
    group: &str,
    title: String,
    content: String,
    shipped: &str,
) -> LegalItem {
    let hash = format!(
        "{:x}",
        Sha256::digest(format!("{id}\n{content}").as_bytes())
    );
    LegalItem {
        id,
        kind: kind.into(),
        group: group.into(),
        title,
        content,
        hash,
        shipped_status: shipped.into(),
    }
}

/// What output is being prepared (decides which items apply).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    pub tax_audit: bool,
    pub depreciation: bool,
}

/// Legal items that apply to this engagement for this output.
pub fn applicable(
    eng: &Engagement,
    rules: &RulesPack,
    fmt: &FormatPack,
    dep: &DepPack,
    scope: Scope,
) -> Vec<LegalItem> {
    let mut out = Vec::new();
    let pk = &fmt.id;
    let grp = format!("Format: {}", fmt.name);
    out.push(item(
        format!("format:{pk}:titles"),
        "format_line",
        &grp,
        format!("Source document and statement titles: {}", fmt.document),
        format!(
            "{} | {} | {} | {} | profit to {:?}: {}",
            fmt.authority,
            fmt.document,
            fmt.bs_title,
            fmt.pl_title,
            fmt.profit_to,
            fmt.profit_note_label
        ),
        &fmt.status,
    ));
    for (side, rows) in [("bs", &fmt.balance_sheet), ("pl", &fmt.profit_loss)] {
        for (i, r) in rows.iter().enumerate() {
            let content = serde_json::to_string(r).unwrap_or_default();
            let what = if side == "bs" {
                "Balance Sheet"
            } else {
                "Statement of Profit and Loss"
            };
            out.push(item(
                format!("format:{pk}:{side}:{i:02}"),
                "format_line",
                &grp,
                format!("{what}: {}", r.label),
                content,
                &fmt.status,
            ));
        }
    }
    let new_act = eng.fy_start >= rules.new_act_from;
    let mut provisions: BTreeSet<String> = BTreeSet::new();
    for (code, r) in &rules.rules {
        // Packs pinned before scopes existed: unverified rules still count.
        let sc = match r.scope.as_str() {
            "" if r.verification == "unverified"
                && r.source.authority.starts_with("Income-tax") =>
            {
                "tax_audit"
            }
            "" if r.verification == "unverified" => "statements",
            x => x,
        };
        let applies = match sc {
            "statements" => true,
            "tax_audit" => scope.tax_audit,
            _ => false,
        };
        if !applies {
            continue;
        }
        let refs = if new_act { &r.refs_new } else { &r.refs_old };
        provisions.extend(refs.iter().cloned());
        let content = serde_json::json!({
            "title": r.title, "simple": r.simple, "detection_basis": r.detection_basis,
            "unknown_facts": r.unknown_facts, "possible_exceptions": r.possible_exceptions,
            "next_step": r.next_step, "refs": refs, "thresholds": rules.thresholds,
        })
        .to_string();
        let group = if sc == "tax_audit" {
            "Checks: tax audit items"
        } else {
            "Checks: presentation"
        };
        out.push(item(
            format!("rule:{code}"),
            "rule",
            group,
            r.title.clone(),
            content,
            if r.verification.is_empty() {
                "unverified"
            } else {
                &r.verification
            },
        ));
    }
    if scope.depreciation {
        provisions.insert(if new_act {
            "it2025.depreciation".into()
        } else {
            "it1961.s32".into()
        });
    }
    for id in provisions {
        let (title, content, st) = match rules.sections.get(&id) {
            Some(p) => (
                format!("{} {}", p.citation, p.act).trim().to_string() + &format!(": {}", p.title),
                serde_json::to_string(p).unwrap_or_default(),
                p.status.clone(),
            ),
            None => (id.clone(), String::new(), "unknown".into()),
        };
        out.push(item(
            format!("provision:{id}"),
            "provision",
            "Statutory references",
            title,
            content,
            &st,
        ));
    }
    if scope.depreciation {
        if let Some(far) = &eng.far {
            let books: BTreeSet<&str> = far
                .assets
                .iter()
                .map(|a| a.book_class.as_str())
                .filter(|k| !k.is_empty())
                .collect();
            let blocks: BTreeSet<&str> = far
                .assets
                .iter()
                .map(|a| a.it_block.as_str())
                .filter(|k| !k.is_empty())
                .collect();
            for k in books {
                let (title, content) = match dep.class(k) {
                    Some(c) => (
                        format!("{}: useful life {} years", c.label, c.life),
                        format!(
                            "{} | {} | {} | residual {}%",
                            c.key, c.label, c.life, dep.residual_pct
                        ),
                    ),
                    None => (format!("{k}: not in the depreciation pack"), String::new()),
                };
                out.push(item(
                    format!("dep:book:{k}"),
                    "depreciation",
                    "Depreciation: useful lives (Companies Act, 2013, Schedule II)",
                    title,
                    content,
                    &dep.status,
                ));
            }
            for k in blocks {
                let (title, content) = match dep.block(k) {
                    Some(b) => (
                        format!("{}: {}%", b.label, b.rate),
                        format!("{} | {} | {}", b.key, b.label, b.rate),
                    ),
                    None => (format!("{k}: not in the depreciation pack"), String::new()),
                };
                out.push(item(
                    format!("dep:it:{k}"),
                    "depreciation",
                    "Depreciation: income-tax blocks and rates",
                    title,
                    content,
                    &dep.status,
                ));
            }
        }
    }
    if scope.tax_audit {
        let schema = crate::taxaudit::schema_for(eng);
        let title = if schema.clauses.is_empty() {
            format!(
                "{} ({}): no clause mapped yet",
                schema.form, schema.act_section
            )
        } else {
            format!(
                "{} ({}): clauses {}",
                schema.form,
                schema.act_section,
                schema
                    .clauses
                    .iter()
                    .map(|c| c.number.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        out.push(item(
            format!("taxaudit:{}", schema.id),
            "tax_audit_form",
            "Tax audit form",
            title,
            crate::taxaudit::schema_text(eng).to_string(),
            &schema.status,
        ));
    }
    out
}

/// One verification recorded by a person. Never created by the program.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Verification {
    pub item_id: String,
    /// Hash of the item content when it was verified.
    pub content_hash: String,
    pub authority: String,
    pub document_title: String,
    pub provision: String,
    /// Official source identifier or URL (Gazette notification, Act, ICAI document).
    pub official_source: String,
    pub effective_from: String,
    pub effective_until: String,
    pub verified_on: String,
    pub verified_by: String,
    pub note: String,
    /// When the record was entered (set by the app).
    pub recorded_at: String,
    /// A later record withdrawing an earlier verification.
    pub revoked: bool,
}

impl Verification {
    /// Fields a record must have before it is accepted.
    pub fn check(&self) -> Result<(), String> {
        let need = [
            ("authority", &self.authority),
            ("document title", &self.document_title),
            ("provision / paragraph / table", &self.provision),
            ("official source", &self.official_source),
            ("verified on", &self.verified_on),
            ("verified by", &self.verified_by),
        ];
        if self.revoked {
            return if self.verified_by.trim().is_empty() {
                Err("Say who is withdrawing the verification.".into())
            } else {
                Ok(())
            };
        }
        let missing: Vec<&str> = need
            .iter()
            .filter(|(_, v)| v.trim().is_empty())
            .map(|(k, _)| *k)
            .collect();
        if !missing.is_empty() {
            return Err(format!("Missing: {}.", missing.join(", ")));
        }
        if chrono::NaiveDate::parse_from_str(self.verified_on.trim(), "%Y-%m-%d").is_err() {
            return Err("'Verified on' must be a date (YYYY-MM-DD).".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStatus {
    #[serde(flatten)]
    pub item: LegalItem,
    /// verified, stale (item changed since it was verified), pending
    pub status: String,
    pub verification: Option<Verification>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Readiness {
    pub ready: bool,
    pub applicable: usize,
    pub verified: usize,
    pub stale: usize,
    pub pending: usize,
    pub items: Vec<ItemStatus>,
}

/// Latest record per item (records are append-only; a revocation wins if later).
pub fn latest(records: &[Verification]) -> BTreeMap<&str, &Verification> {
    let mut m = BTreeMap::new();
    for r in records {
        m.insert(r.item_id.as_str(), r);
    }
    m
}

pub fn readiness(items: Vec<LegalItem>, records: &[Verification]) -> Readiness {
    let last = latest(records);
    let mut out = Readiness {
        applicable: items.len(),
        ..Default::default()
    };
    for it in items {
        let v = last.get(it.id.as_str()).copied().filter(|v| !v.revoked);
        let status = match v {
            Some(v) if v.content_hash == it.hash => "verified",
            Some(_) => "stale",
            None => "pending",
        };
        match status {
            "verified" => out.verified += 1,
            "stale" => out.stale += 1,
            _ => out.pending += 1,
        }
        out.items.push(ItemStatus {
            item: it,
            status: status.into(),
            verification: v.cloned(),
        });
    }
    out.ready = out.applicable > 0 && out.verified == out.applicable;
    out
}

/// Ids of items verified and still current (given to the rules pack so that
/// references drop the "not verified" tag).
pub fn verified_ids(r: &Readiness) -> BTreeSet<String> {
    r.items
        .iter()
        .filter(|s| s.status == "verified")
        .map(|s| s.item.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntityType;

    fn eng(fy: i32, e: EntityType) -> Engagement {
        Engagement {
            entity_name: "T".into(),
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

    fn rec(it: &LegalItem) -> Verification {
        Verification {
            item_id: it.id.clone(),
            content_hash: it.hash.clone(),
            authority: "A".into(),
            document_title: "D".into(),
            provision: "P".into(),
            official_source: "S".into(),
            verified_on: "2026-10-07".into(),
            verified_by: "CA X".into(),
            ..Default::default()
        }
    }

    #[test]
    fn every_entity_and_year_has_items_and_nothing_is_verified_by_default() {
        let rules = RulesPack::builtin();
        let dep = DepPack::builtin();
        for e in [
            EntityType::Company,
            EntityType::Llp,
            EntityType::Firm,
            EntityType::Proprietor,
            EntityType::Huf,
            EntityType::Aop,
            EntityType::Boi,
        ] {
            for fy in [2025, 2026] {
                let g = eng(fy, e);
                let items = applicable(
                    &g,
                    &rules,
                    &FormatPack::for_entity(e),
                    &dep,
                    Scope {
                        tax_audit: true,
                        depreciation: false,
                    },
                );
                assert!(
                    items.iter().filter(|i| i.kind == "format_line").count() > 20,
                    "{e:?}"
                );
                assert!(
                    items.iter().any(|i| i.kind == "rule")
                        && items.iter().any(|i| i.kind == "provision")
                );
                let r = readiness(items, &[]);
                assert!(!r.ready && r.verified == 0, "{e:?} {fy}");
            }
        }
    }

    #[test]
    fn verification_binds_to_content_and_can_be_withdrawn() {
        let rules = RulesPack::builtin();
        let g = eng(2026, EntityType::Firm);
        let items = applicable(
            &g,
            &rules,
            &FormatPack::for_entity(EntityType::Firm),
            &DepPack::builtin(),
            Scope::default(),
        );
        let mut recs: Vec<Verification> = items.iter().map(rec).collect();
        assert!(readiness(items.clone(), &recs).ready);
        // The item text changes: the old verification no longer counts.
        let mut changed = items.clone();
        changed[3].content.push('x');
        changed[3].hash = "different".into();
        let r = readiness(changed, &recs);
        assert!(!r.ready && r.stale == 1);
        // A later withdrawal wins.
        recs.push(Verification {
            item_id: items[0].id.clone(),
            revoked: true,
            verified_by: "CA X".into(),
            ..Default::default()
        });
        let r = readiness(items, &recs);
        assert!(!r.ready && r.pending == 1);
    }

    #[test]
    fn tax_audit_items_only_when_tax_audit_is_prepared_and_form_follows_the_year() {
        let rules = RulesPack::builtin();
        let dep = DepPack::builtin();
        let f = FormatPack::for_entity(EntityType::Firm);
        let without = applicable(
            &eng(2026, EntityType::Firm),
            &rules,
            &f,
            &dep,
            Scope::default(),
        );
        assert!(!without
            .iter()
            .any(|i| i.id == "rule:CASH_RECEIPT_LIMIT" || i.kind == "tax_audit_form"));
        let new = applicable(
            &eng(2026, EntityType::Firm),
            &rules,
            &f,
            &dep,
            Scope {
                tax_audit: true,
                depreciation: false,
            },
        );
        assert!(
            new.iter().any(|i| i.id == "taxaudit:form26")
                && new.iter().any(|i| i.id == "provision:it2025.s186")
        );
        assert!(!new.iter().any(|i| i.id.starts_with("provision:it1961")));
        let old = applicable(
            &eng(2025, EntityType::Firm),
            &rules,
            &f,
            &dep,
            Scope {
                tax_audit: true,
                depreciation: false,
            },
        );
        assert!(
            old.iter().any(|i| i.id == "taxaudit:form3cd")
                && old.iter().any(|i| i.id == "provision:it1961.s269ST")
        );
    }

    #[test]
    fn a_record_needs_source_date_and_person() {
        let mut v = Verification {
            item_id: "x".into(),
            ..Default::default()
        };
        assert!(v.check().is_err());
        v = Verification {
            authority: "CBDT".into(),
            document_title: "Act".into(),
            provision: "s.185".into(),
            official_source: "Gazette".into(),
            verified_on: "07-10-2026".into(),
            verified_by: "CA".into(),
            ..v
        };
        assert!(v.check().is_err(), "date format");
        v.verified_on = "2026-10-07".into();
        assert!(v.check().is_ok());
    }
}
