//! LedgerCraft core engine.
//!
//! Pure functions only: no files, no UI. Input is an [`Engagement`]
//! (trial balances + vouchers), output is an [`Analysis`] (findings,
//! mapping, statements, loan register). Every money value is an exact
//! integer number of paise.

pub mod adjust;
pub mod ageing;
pub mod bankrec;
pub mod capital;
pub mod checks;
pub mod consolidate;
pub mod date;
pub mod engine;
pub mod facts;
pub mod far;
pub mod groups;
pub mod mapping;
pub mod model;
pub mod money;
pub mod ratios;
pub mod recon;
pub mod report;
pub mod rounding;
pub mod rules;
pub mod statements;
pub mod units;

pub use engine::{analyse, Analysis};
pub use model::*;
pub use money::Money;
