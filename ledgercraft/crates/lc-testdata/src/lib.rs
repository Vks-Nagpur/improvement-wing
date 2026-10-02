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
