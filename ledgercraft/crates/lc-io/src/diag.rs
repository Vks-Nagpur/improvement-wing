//! Import diagnostics: what a reader accepted and what it skipped, and why.
//! Readers note rows as they go; `collect` runs a read and returns the report.
//! Nothing is dropped silently: every row is accepted, skipped with a reason,
//! or the whole file is refused with an error.

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Skipped {
    /// Row number in the file (1-based).
    pub row: usize,
    pub reason: String,
    /// The first cells of the row, for recognising it.
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub rows_in_file: usize,
    pub accepted: usize,
    pub skipped: Vec<Skipped>,
    pub warnings: Vec<String>,
}

thread_local! {
    static CURRENT: RefCell<Option<ImportReport>> = const { RefCell::new(None) };
}

/// Run a read with a fresh report.
pub fn collect<T>(f: impl FnOnce() -> Result<T, String>) -> (Result<T, String>, ImportReport) {
    CURRENT.with(|c| *c.borrow_mut() = Some(ImportReport::default()));
    let r = f();
    let rep = CURRENT.with(|c| c.borrow_mut().take()).unwrap_or_default();
    (r, rep)
}

fn with(f: impl FnOnce(&mut ImportReport)) {
    CURRENT.with(|c| {
        if let Some(r) = c.borrow_mut().as_mut() {
            f(r)
        }
    });
}

pub fn rows(n: usize) {
    with(|r| r.rows_in_file = n);
}

pub fn accept() {
    with(|r| r.accepted += 1);
}

pub fn skip(row: usize, reason: &str, cells: &[String]) {
    let text: String = cells
        .iter()
        .filter(|c| !c.is_empty())
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(" | ");
    with(|r| {
        r.skipped.push(Skipped {
            row,
            reason: reason.into(),
            text: text.chars().take(120).collect(),
        })
    });
}

pub fn warn(w: String) {
    with(|r| r.warnings.push(w));
}
