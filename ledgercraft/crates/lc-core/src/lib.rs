//! LedgerCraft core engine.
//!
//! Pure functions only: no files, no UI. Input is an [`Engagement`]
//! (trial balances + vouchers), output is an [`Analysis`] (findings,
//! mapping, statements, loan register). Every money value is an exact
//! integer number of paise.

pub mod checks;
pub mod date;
pub mod engine;
pub mod groups;
pub mod mapping;
pub mod model;
pub mod money;
pub mod rules;
pub mod statements;

pub use engine::{analyse, Analysis};
pub use model::*;
pub use money::Money;
