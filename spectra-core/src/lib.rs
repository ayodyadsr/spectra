//! Spectra: behavioural-regression diff engine for Solana program upgrades.
//!
//! Spectra parses two Anchor IDL JSON files (a baseline `v_n` and a candidate
//! `v_{n+1}`), compares them, and emits findings keyed by 11 rule kinds covering
//! the interface-shape and on-chain-layout hazards that the Solana BPF Loader
//! does not check.
//!
//! See the project README and `docs/SEVERITY.md` for the canonical rule list,
//! severity contract, and exit-code mapping.

#![warn(missing_docs)]

pub mod diff;
pub mod discriminator;
pub mod idl;
pub mod report;

pub use diff::{diff_idls, DiffReport, Finding, Severity};
pub use idl::Idl;
