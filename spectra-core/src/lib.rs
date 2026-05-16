//! Spectra: a strictly-differential account-validation security-regression
//! gate for Solana program upgrades.
//!
//! Spectra parses two versions of a Solana program's Rust source — a
//! *baseline* (the last released / on-chain-deployed version) and a
//! *candidate* (the upgrade under review) — extracts the Anchor
//! account-validation guard set of every instruction context, and fails CI
//! **only** when the candidate removes or weakens a guard the baseline
//! enforced (missing signer / owner / type-cosplay / `has_one` / PDA / CPI).
//!
//! It is deliberately *not* an absolute scanner: a check that was already
//! missing in the baseline is not a regression and Spectra stays silent on
//! it. This is what makes it complementary to Sec3 X-Ray / Auditware Radar
//! and gives it a near-zero false-positive rate by construction.
//!
//! See the project README and `docs/SEVERITY.md` for the canonical rule
//! catalogue, severity contract, and exit-code mapping.

#![warn(missing_docs)]

pub mod accounts;
pub mod regression;
pub mod report;

pub use accounts::ProgramModel;
pub use regression::{diff_programs, Finding, RegressionReport, Severity};
