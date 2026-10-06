//! Tax audit form schemas (data in packs/taxaudit_*.json). Form 3CD applies
//! up to FY 2025-26; Form 26 (Income-tax Act, 2025, s.63) from Tax Year
//! 2026-27. They are separate schemas: a Form 3CD clause number is never
//! presented as a Form 26 clause. Form 26 has no clause mapped yet.

use crate::model::Engagement;
use serde::{Deserialize, Serialize};

/// One clause of a tax audit form, or a supporting schedule (no number).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Clause {
    pub key: String,
    #[serde(default)]
    pub number: String,
    pub short: String,
    pub title: String,
    /// Provision ids (sections.json).
    #[serde(default)]
    pub statutory: Vec<String>,
    #[serde(default)]
    pub derivable: String,
    #[serde(default)]
    pub user_input: String,
    #[serde(default)]
    pub judgement: String,
    #[serde(default)]
    pub exceptions: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TaxAuditSchema {
    pub id: String,
    pub form: String,
    pub act_section: String,
    #[serde(default)]
    pub effective_from: Option<String>,
    #[serde(default)]
    pub effective_until: Option<String>,
    pub status: String,
    pub note: String,
    pub clauses: Vec<Clause>,
    #[serde(default)]
    pub supporting: Vec<Clause>,
}

impl TaxAuditSchema {
    /// Sheet name, title and reference text for a schedule, if the schema has it.
    pub fn slot(
        &self,
        key: &str,
        sections: &crate::rules::Sections,
    ) -> Option<(String, String, String)> {
        let none = std::collections::BTreeSet::new();
        if let Some(c) = self.clauses.iter().find(|c| c.key == key) {
            let name: String = format!("{} {}", c.number, c.short)
                .chars()
                .filter(|ch| !"/\\?*[]:".contains(*ch))
                .take(31)
                .collect();
            return Some((
                name,
                format!("Clause {}: {}", c.number, c.title),
                sections.cite(&c.statutory, &none),
            ));
        }
        self.supporting.iter().find(|c| c.key == key).map(|c| {
            let refs = if c.statutory.is_empty() {
                String::new()
            } else {
                sections.cite(&c.statutory, &none)
            };
            (
                c.short.clone(),
                format!(
                    "{} (supporting schedule; Form 26 clause not mapped)",
                    c.title
                ),
                refs,
            )
        })
    }
}

/// The schema for the year: Form 26 from Tax Year 2026-27, Form 3CD before.
pub fn schema_for(eng: &Engagement) -> TaxAuditSchema {
    serde_json::from_str(schema_text(eng)).expect("built-in tax audit schema is valid")
}

/// The schema file for the year, as text (fingerprinted for legal verification).
pub fn schema_text(eng: &Engagement) -> &'static str {
    if eng.fy_start >= chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap() {
        include_str!("../packs/taxaudit_form26.json")
    } else {
        include_str!("../packs/taxaudit_form3cd.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form26_has_no_clause_and_never_borrows_3cd_numbers() {
        let s26: TaxAuditSchema =
            serde_json::from_str(include_str!("../packs/taxaudit_form26.json")).unwrap();
        let s3: TaxAuditSchema =
            serde_json::from_str(include_str!("../packs/taxaudit_form3cd.json")).unwrap();
        assert!(s26.clauses.is_empty() && s26.note.contains("not fully mapped"));
        assert!(s26.supporting.iter().all(|c| c.number.is_empty()));
        let secs = crate::rules::Sections::builtin();
        for c in &s26.supporting {
            let (name, title, refs) = s26.slot(&c.key, &secs).unwrap();
            for n in s3.clauses.iter().map(|c| c.number.clone()) {
                assert!(
                    !name.starts_with(&n) && !title.contains(&format!("Clause {n}")),
                    "{name} / {title}"
                );
            }
            assert!(!refs.contains("1961") && !refs.contains("3CD"), "{refs}");
        }
        assert!(s3.clauses.iter().all(|c| !c.number.is_empty()));
    }
}
