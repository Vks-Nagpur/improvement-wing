//! Synthetic accounting data with known answers.
//!
//! Every scenario returns the books *and* the exact set of findings the
//! engine must report (ground truth), plus the expected profit, so tests can
//! prove both that every planted glitch is caught and that clean books raise
//! no false alarms.

pub mod builder;
pub mod rng;
pub mod scenarios;

pub use builder::{Builder, Expected};
pub use scenarios::Scenario;

use lc_core::{Analysis, Engagement};
use std::collections::BTreeSet;

/// Keys of the findings about the books themselves. Workflow reminders
/// (placements waiting for the user's confirmation) are left out.
pub fn problem_keys(a: &Analysis) -> BTreeSet<String> {
    a.findings
        .iter()
        .filter(|f| !f.code.starts_with("MAPPING_"))
        .map(|f| f.key.clone())
        .collect()
}

/// Confirm every current placement, as a user who reviewed the Map ledgers
/// screen would. Returns the engagement with mapping memory and context set.
pub fn confirm_all(eng: &Engagement, a: &Analysis) -> Engagement {
    let mut e = eng.clone();
    for m in &a.mapping {
        if let Some(h) = m.head {
            let key = lc_core::model::norm_name(&m.name);
            e.mapping_memory.insert(key.clone(), h.id());
            e.mapping_context.insert(
                key,
                lc_core::mapping::memory_context(&m.group, m.tb_closing),
            );
        }
    }
    e
}
