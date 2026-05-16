//! Strictly-differential account-validation regression engine.
//!
//! Given the [`crate::accounts::ProgramModel`] of a *baseline* version (the
//! last released / on-chain-deployed program) and a *candidate* version (the
//! upgrade under review), this module emits a [`Finding`] **only** when the
//! candidate removes, weakens, or bypasses an account-validation guard that
//! the baseline enforced.
//!
//! This is the property that makes Spectra complementary to — not a
//! re-implementation of — absolute scanners such as Sec3 X-Ray and Auditware
//! Radar. An absolute scanner asks *"is there a missing owner check anywhere
//! in this code?"* and must tune heuristics to keep false positives down. A
//! missing check that was **already missing in the baseline** is, by
//! construction, *not* a regression and Spectra stays silent on it. Spectra
//! only ever asks *"did this upgrade take away a guarantee the deployed
//! version already gave its users?"* — a question stateless scanners
//! structurally cannot answer, and one whose false-positive rate is near-zero
//! by construction rather than by tuning.

use crate::accounts::{AccountsContext, Guard, ProgramModel, Slot};
use serde::Serialize;

/// A single regression finding. Each variant maps to one canonical rule ID
/// (see `docs/SEVERITY.md`); variant names become the `kind` field in JSON /
/// SARIF via `#[serde(rename_all = "snake_case")]`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    /// Baseline required this account to sign; the candidate no longer does.
    /// The canonical Solana missing-signer-check bug, introduced on upgrade.
    SignerCheckRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot whose signer guard was dropped.
        account: String,
    },
    /// Baseline used a typed wrapper (`Account<T>` / `AccountLoader<T>` …)
    /// that enforces Anchor's owner + discriminator check; the candidate
    /// downgraded the slot to `UncheckedAccount` / `AccountInfo`, removing
    /// type-cosplay protection.
    TypeCosplayProtectionRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot that lost its typed wrapper.
        account: String,
        /// Baseline type that was enforced.
        baseline_type: String,
        /// Candidate type it was downgraded to.
        candidate_type: String,
    },
    /// Baseline pinned the account owner (`#[account(owner = …)]` or
    /// `address = …`); the candidate dropped that pin.
    OwnerCheckRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot whose owner/address pin was dropped.
        account: String,
        /// The baseline owner/address expression that is no longer enforced.
        baseline_pin: String,
    },
    /// Baseline enforced a `has_one` relational-integrity check that the
    /// candidate dropped.
    HasOneConstraintRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot whose `has_one` guard was dropped.
        account: String,
        /// The `has_one` target field that is no longer checked.
        target: String,
    },
    /// Baseline enforced a custom `#[account(constraint = …)]` predicate that
    /// the candidate dropped.
    CustomConstraintRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot whose custom constraint was dropped.
        account: String,
        /// The constraint expression that is no longer enforced.
        expr: String,
    },
    /// Baseline derived this account as a PDA (`seeds = [...], bump`); the
    /// candidate dropped the derivation check, allowing an arbitrary account.
    PdaDerivationRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot whose PDA derivation was dropped.
        account: String,
    },
    /// Baseline pinned a CPI target program id (`Program<'info, T>` or an
    /// `address` pin); the candidate downgraded it to an unvalidated account,
    /// enabling arbitrary-program invocation.
    CpiTargetUnpinned {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot that lost its program-id pin.
        account: String,
    },
    /// A validated account slot present in the baseline context was removed
    /// from the candidate while the context itself still exists — the
    /// instruction no longer takes (and therefore no longer checks) it.
    ValidatedAccountSlotRemoved {
        /// Instruction context (struct) name.
        context: String,
        /// Account slot that was removed.
        account: String,
    },
    /// The candidate introduces a brand-new `UncheckedAccount` / `AccountInfo`
    /// slot that did not exist in the baseline. Not a regression of an
    /// existing guarantee — flagged as new attack surface to review.
    UnvalidatedAccountIntroduced {
        /// Instruction context (struct) name.
        context: String,
        /// New unvalidated account slot.
        account: String,
    },
}

/// Severity tier. Any `Breaking` finding drives the CLI to exit `1`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Block the merge: the upgrade removes a security guarantee the
    /// deployed version gave its users.
    Breaking,
    /// Review recommended, but no automatic CI failure.
    Warning,
}

impl Finding {
    /// Severity tier of this finding.
    pub fn severity(&self) -> Severity {
        match self {
            Finding::UnvalidatedAccountIntroduced { .. } => Severity::Warning,
            _ => Severity::Breaking,
        }
    }
}

/// Aggregated output of one regression run.
#[derive(Debug, Clone, Serialize)]
pub struct RegressionReport {
    /// Findings in deterministic discovery order (context name, then slot
    /// source order).
    pub findings: Vec<Finding>,
    /// Number of `Breaking` findings. Drives the exit-code contract.
    pub breaking_count: usize,
    /// Number of `Warning` findings.
    pub warning_count: usize,
}

impl RegressionReport {
    /// `true` iff there are zero breaking findings. Warnings do not affect
    /// cleanliness.
    pub fn is_clean(&self) -> bool {
        self.breaking_count == 0
    }
}

/// Diff a baseline against a candidate program model and return every
/// account-validation regression.
///
/// Deterministic: identical input trees yield zero findings; the same pair
/// always yields the same ordered finding list.
pub fn diff_programs(baseline: &ProgramModel, candidate: &ProgramModel) -> RegressionReport {
    let mut findings = Vec::new();

    for (name, base_ctx) in &baseline.contexts {
        if let Some(cand_ctx) = candidate.contexts.get(name) {
            diff_context(base_ctx, cand_ctx, &mut findings);
        }
        // A context removed wholesale is an interface change, not a silent
        // weakening of a still-callable instruction — out of scope here.
    }

    for (name, cand_ctx) in &candidate.contexts {
        if !baseline.contexts.contains_key(name) {
            for slot in &cand_ctx.slots {
                if slot.unchecked && slot.guards.is_empty() {
                    findings.push(Finding::UnvalidatedAccountIntroduced {
                        context: name.clone(),
                        account: slot.name.clone(),
                    });
                }
            }
        }
    }

    let breaking_count = findings
        .iter()
        .filter(|f| f.severity() == Severity::Breaking)
        .count();
    let warning_count = findings.len() - breaking_count;

    RegressionReport {
        findings,
        breaking_count,
        warning_count,
    }
}

fn diff_context(base: &AccountsContext, cand: &AccountsContext, findings: &mut Vec<Finding>) {
    let ctx = base.name.clone();

    for base_slot in &base.slots {
        match cand.slots.iter().find(|s| s.name == base_slot.name) {
            None => {
                if !base_slot.guards.is_empty() {
                    findings.push(Finding::ValidatedAccountSlotRemoved {
                        context: ctx.clone(),
                        account: base_slot.name.clone(),
                    });
                }
            }
            Some(cand_slot) => diff_slot(&ctx, base_slot, cand_slot, findings),
        }
    }

    for cand_slot in &cand.slots {
        let is_new = !base.slots.iter().any(|s| s.name == cand_slot.name);
        if is_new && cand_slot.unchecked && cand_slot.guards.is_empty() {
            findings.push(Finding::UnvalidatedAccountIntroduced {
                context: ctx.clone(),
                account: cand_slot.name.clone(),
            });
        }
    }
}

fn diff_slot(ctx: &str, base: &Slot, cand: &Slot, findings: &mut Vec<Finding>) {
    for g in &base.guards {
        if cand.guards.contains(g) {
            continue;
        }
        match g {
            Guard::Signer => findings.push(Finding::SignerCheckRemoved {
                context: ctx.to_string(),
                account: base.name.clone(),
            }),
            Guard::Typed(t) => {
                // Only a *real* downgrade: the candidate no longer has any
                // typed/owner pin for this slot (e.g. became Unchecked).
                let still_owner_pinned = cand.guards.iter().any(|cg| {
                    matches!(
                        cg,
                        Guard::Typed(_) | Guard::Owner(_) | Guard::Address(_) | Guard::ProgramId(_)
                    )
                });
                if !still_owner_pinned {
                    findings.push(Finding::TypeCosplayProtectionRemoved {
                        context: ctx.to_string(),
                        account: base.name.clone(),
                        baseline_type: t.clone(),
                        candidate_type: cand.ty.replace(' ', ""),
                    });
                }
            }
            Guard::Owner(p) | Guard::Address(p) => {
                let still_owner_pinned = cand
                    .guards
                    .iter()
                    .any(|cg| matches!(cg, Guard::Owner(_) | Guard::Address(_) | Guard::Typed(_)));
                if !still_owner_pinned {
                    findings.push(Finding::OwnerCheckRemoved {
                        context: ctx.to_string(),
                        account: base.name.clone(),
                        baseline_pin: p.clone(),
                    });
                }
            }
            Guard::HasOne(target) => findings.push(Finding::HasOneConstraintRemoved {
                context: ctx.to_string(),
                account: base.name.clone(),
                target: target.clone(),
            }),
            Guard::Constraint(expr) => findings.push(Finding::CustomConstraintRemoved {
                context: ctx.to_string(),
                account: base.name.clone(),
                expr: expr.clone(),
            }),
            Guard::Seeds => findings.push(Finding::PdaDerivationRemoved {
                context: ctx.to_string(),
                account: base.name.clone(),
            }),
            Guard::ProgramId(_) => {
                let still_pinned = cand
                    .guards
                    .iter()
                    .any(|cg| matches!(cg, Guard::ProgramId(_) | Guard::Address(_)));
                if !still_pinned {
                    findings.push(Finding::CpiTargetUnpinned {
                        context: ctx.to_string(),
                        account: base.name.clone(),
                    });
                }
            }
        }
    }
}
