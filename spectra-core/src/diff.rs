//! IDL pairwise comparison engine.
//!
//! Produces an ordered list of [`Finding`]s for every detectable regression
//! between an old and new [`Idl`]. See `docs/SEVERITY.md` for the full rule
//! catalogue, severity tiers, and the exit-code mapping.

use crate::discriminator::{account_discriminator, hex, instruction_discriminator};
use crate::idl::{Field, Idl, Instruction, TypeKind};
use serde::Serialize;
use std::collections::HashMap;

/// A single diff finding. Each variant maps to one canonical rule ID in
/// `docs/SEVERITY.md`. Variant names are also used as the `kind` field in
/// JSON and SARIF output via `#[serde(rename_all = "snake_case")]`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    /// A new instruction is present in the new IDL only. Informational —
    /// old clients simply never call it.
    InstructionAdded {
        /// Instruction name.
        name: String,
        /// Lowercase hex of the 8-byte Anchor instruction discriminator.
        discriminator: String,
    },
    /// An instruction present in the old IDL has been removed. Breaking —
    /// old clients calling this name will get a runtime dispatch error.
    InstructionRemoved {
        /// Instruction name.
        name: String,
        /// Lowercase hex of the 8-byte Anchor instruction discriminator.
        discriminator: String,
    },
    /// The Borsh-serialised argument tuple of an instruction has changed.
    /// Breaking — old clients send the wrong byte layout.
    InstructionArgsChanged {
        /// Instruction name.
        name: String,
        /// Ordered `name: type` strings as they appeared in the old IDL.
        old_args: Vec<String>,
        /// Ordered `name: type` strings as they appear in the new IDL.
        new_args: Vec<String>,
    },
    /// A new account type is present in the new IDL only. Informational.
    AccountAdded {
        /// Account-type name.
        name: String,
        /// Lowercase hex of the 8-byte Anchor account discriminator.
        discriminator: String,
    },
    /// An account type present in the old IDL has been removed. Breaking —
    /// old on-chain accounts of this type can no longer be deserialised by
    /// the new program.
    AccountRemoved {
        /// Account-type name.
        name: String,
        /// Lowercase hex of the 8-byte Anchor account discriminator.
        discriminator: String,
    },
    /// Field order changed within an account. Breaking — old on-chain data
    /// will now be read into different fields.
    AccountFieldReordered {
        /// Account-type name.
        account: String,
        /// Field names in their old IDL order.
        old_order: Vec<String>,
        /// Field names in their new IDL order.
        new_order: Vec<String>,
    },
    /// A new field was added to an account. Warning — caller must confirm
    /// that storage resize / migration is handled.
    AccountFieldAdded {
        /// Account-type name.
        account: String,
        /// Newly-added field name.
        field: String,
        /// Field type string.
        ty: String,
    },
    /// A field was removed from an account. Breaking — old data is now
    /// misaligned by exactly that field's width.
    AccountFieldRemoved {
        /// Account-type name.
        account: String,
        /// Removed field name.
        field: String,
        /// Removed field's type string.
        ty: String,
    },
    /// A field's type changed (e.g. `u64 → u128`). Breaking — old data is
    /// the wrong width or shape.
    AccountFieldTypeChanged {
        /// Account-type name.
        account: String,
        /// Field name whose type changed.
        field: String,
        /// Old field type string.
        old_ty: String,
        /// New field type string.
        new_ty: String,
    },
    /// Silent-corruption case: account name (and therefore Anchor discriminator)
    /// unchanged between versions, but at least one field-level breaking change
    /// is present. The runtime accepts the old account data and deserialises
    /// it into the new layout, producing wrong-field reads.
    AccountLayoutChangedSameDiscriminator {
        /// Account-type name.
        account: String,
        /// Discriminator hex that is identical between old and new versions.
        discriminator: String,
    },
    /// Two distinct IDL names produce the same truncated 8-byte SHA-256
    /// discriminator. Either an accidental collision (vanishingly rare for
    /// human names but possible) or an attacker-introduced one. Either way
    /// the dispatcher will mis-route.
    DiscriminatorCollision {
        /// Kind of the first colliding entry (`"instruction"` or `"account"`).
        kind_a: String,
        /// Name of the first colliding entry.
        name_a: String,
        /// Kind of the second colliding entry.
        kind_b: String,
        /// Name of the second colliding entry.
        name_b: String,
        /// The 8-byte discriminator hex that both entries share.
        discriminator: String,
    },
}

/// Severity tier for a finding. Maps to the exit-code contract:
/// any `Breaking` finding causes the CLI to exit `1`.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Block the merge: the upgrade can corrupt on-chain state or
    /// misroute existing calls.
    Breaking,
    /// Informational: caller may still want to review, but no automatic
    /// CI failure is produced.
    Warning,
}

impl Finding {
    /// Return the severity tier of this finding.
    pub fn severity(&self) -> Severity {
        match self {
            Finding::AccountLayoutChangedSameDiscriminator { .. }
            | Finding::DiscriminatorCollision { .. } => Severity::Breaking,
            Finding::InstructionRemoved { .. }
            | Finding::InstructionArgsChanged { .. }
            | Finding::AccountRemoved { .. }
            | Finding::AccountFieldReordered { .. }
            | Finding::AccountFieldRemoved { .. }
            | Finding::AccountFieldTypeChanged { .. } => Severity::Breaking,
            Finding::InstructionAdded { .. }
            | Finding::AccountAdded { .. }
            | Finding::AccountFieldAdded { .. } => Severity::Warning,
        }
    }
}

/// Aggregated output of one diff run: ordered findings plus convenience counts.
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    /// Program name as declared in the old IDL.
    pub old_program: String,
    /// Program name as declared in the new IDL.
    pub new_program: String,
    /// Findings in deterministic discovery order.
    pub findings: Vec<Finding>,
    /// Number of `Breaking`-severity findings. Drives the exit-code contract.
    pub breaking_count: usize,
    /// Number of `Warning`-severity findings.
    pub warning_count: usize,
}

impl DiffReport {
    /// `true` iff the report contains zero breaking findings. Warnings do
    /// not affect cleanliness.
    pub fn is_clean(&self) -> bool {
        self.breaking_count == 0
    }
}

/// Diff two parsed Anchor IDLs and return the full [`DiffReport`].
///
/// Output is deterministic over the same input pair: identical inputs always
/// yield zero findings; the same input pair always yields the same ordered
/// list of findings.
pub fn diff_idls(old: &Idl, new: &Idl) -> DiffReport {
    let mut findings = Vec::new();

    diff_instructions(&old.instructions, &new.instructions, &mut findings);
    diff_accounts(old, new, &mut findings);
    detect_silent_corruption(&mut findings);
    detect_discriminator_collisions(new, &mut findings);

    let breaking_count = findings
        .iter()
        .filter(|f| f.severity() == Severity::Breaking)
        .count();
    let warning_count = findings
        .iter()
        .filter(|f| f.severity() == Severity::Warning)
        .count();

    DiffReport {
        old_program: old.name.clone(),
        new_program: new.name.clone(),
        findings,
        breaking_count,
        warning_count,
    }
}

fn detect_silent_corruption(findings: &mut Vec<Finding>) {
    let mut accounts_with_breaking_field_change: Vec<String> = findings
        .iter()
        .filter_map(|f| match f {
            Finding::AccountFieldReordered { account, .. }
            | Finding::AccountFieldRemoved { account, .. }
            | Finding::AccountFieldTypeChanged { account, .. } => Some(account.clone()),
            _ => None,
        })
        .collect();
    accounts_with_breaking_field_change.sort();
    accounts_with_breaking_field_change.dedup();

    for account in accounts_with_breaking_field_change {
        findings.push(Finding::AccountLayoutChangedSameDiscriminator {
            discriminator: hex(&account_discriminator(&account)),
            account,
        });
    }
}

fn detect_discriminator_collisions(new: &Idl, findings: &mut Vec<Finding>) {
    let mut seen: HashMap<[u8; 8], (String, String)> = HashMap::new();

    for ix in &new.instructions {
        let d = instruction_discriminator(&ix.name);
        if let Some(prior) = seen.get(&d) {
            findings.push(Finding::DiscriminatorCollision {
                kind_a: prior.0.clone(),
                name_a: prior.1.clone(),
                kind_b: "instruction".into(),
                name_b: ix.name.clone(),
                discriminator: hex(&d),
            });
        } else {
            seen.insert(d, ("instruction".into(), ix.name.clone()));
        }
    }

    for acc in &new.accounts {
        let d = account_discriminator(&acc.name);
        if let Some(prior) = seen.get(&d) {
            findings.push(Finding::DiscriminatorCollision {
                kind_a: prior.0.clone(),
                name_a: prior.1.clone(),
                kind_b: "account".into(),
                name_b: acc.name.clone(),
                discriminator: hex(&d),
            });
        } else {
            seen.insert(d, ("account".into(), acc.name.clone()));
        }
    }
}

fn diff_instructions(old: &[Instruction], new: &[Instruction], findings: &mut Vec<Finding>) {
    let old_map: HashMap<&str, &Instruction> = old.iter().map(|i| (i.name.as_str(), i)).collect();
    let new_map: HashMap<&str, &Instruction> = new.iter().map(|i| (i.name.as_str(), i)).collect();

    for (name, old_ix) in &old_map {
        match new_map.get(name) {
            None => findings.push(Finding::InstructionRemoved {
                name: (*name).to_string(),
                discriminator: hex(&instruction_discriminator(name)),
            }),
            Some(new_ix) => {
                let old_args: Vec<String> = old_ix.args.iter().map(field_signature).collect();
                let new_args: Vec<String> = new_ix.args.iter().map(field_signature).collect();
                if old_args != new_args {
                    findings.push(Finding::InstructionArgsChanged {
                        name: (*name).to_string(),
                        old_args,
                        new_args,
                    });
                }
            }
        }
    }

    for name in new_map.keys() {
        if !old_map.contains_key(name) {
            findings.push(Finding::InstructionAdded {
                name: (*name).to_string(),
                discriminator: hex(&instruction_discriminator(name)),
            });
        }
    }
}

fn diff_accounts(old: &Idl, new: &Idl, findings: &mut Vec<Finding>) {
    let old_map: HashMap<&str, &crate::idl::Account> =
        old.accounts.iter().map(|a| (a.name.as_str(), a)).collect();
    let new_map: HashMap<&str, &crate::idl::Account> =
        new.accounts.iter().map(|a| (a.name.as_str(), a)).collect();

    for (name, old_acc) in &old_map {
        match new_map.get(name) {
            None => findings.push(Finding::AccountRemoved {
                name: (*name).to_string(),
                discriminator: hex(&account_discriminator(name)),
            }),
            Some(new_acc) => {
                diff_struct_fields(name, &old_acc.ty, &new_acc.ty, findings);
            }
        }
    }

    for name in new_map.keys() {
        if !old_map.contains_key(name) {
            findings.push(Finding::AccountAdded {
                name: (*name).to_string(),
                discriminator: hex(&account_discriminator(name)),
            });
        }
    }
}

fn diff_struct_fields(
    account: &str,
    old_ty: &TypeKind,
    new_ty: &TypeKind,
    findings: &mut Vec<Finding>,
) {
    let (old_fields, new_fields) = match (old_ty, new_ty) {
        (TypeKind::Struct { fields: o }, TypeKind::Struct { fields: n }) => (o, n),
        _ => return,
    };

    let old_names: Vec<String> = old_fields.iter().map(|f| f.name.clone()).collect();
    let new_names: Vec<String> = new_fields.iter().map(|f| f.name.clone()).collect();

    let old_by_name: HashMap<&str, &Field> =
        old_fields.iter().map(|f| (f.name.as_str(), f)).collect();
    let new_by_name: HashMap<&str, &Field> =
        new_fields.iter().map(|f| (f.name.as_str(), f)).collect();

    for (name, old_f) in &old_by_name {
        match new_by_name.get(name) {
            None => findings.push(Finding::AccountFieldRemoved {
                account: account.to_string(),
                field: (*name).to_string(),
                ty: type_string(&old_f.ty),
            }),
            Some(new_f) => {
                let old_ty_str = type_string(&old_f.ty);
                let new_ty_str = type_string(&new_f.ty);
                if old_ty_str != new_ty_str {
                    findings.push(Finding::AccountFieldTypeChanged {
                        account: account.to_string(),
                        field: (*name).to_string(),
                        old_ty: old_ty_str,
                        new_ty: new_ty_str,
                    });
                }
            }
        }
    }

    for (name, new_f) in &new_by_name {
        if !old_by_name.contains_key(name) {
            findings.push(Finding::AccountFieldAdded {
                account: account.to_string(),
                field: (*name).to_string(),
                ty: type_string(&new_f.ty),
            });
        }
    }

    let old_common: Vec<&String> = old_names.iter().filter(|n| new_names.contains(n)).collect();
    let new_common: Vec<&String> = new_names.iter().filter(|n| old_names.contains(n)).collect();
    if old_common != new_common {
        findings.push(Finding::AccountFieldReordered {
            account: account.to_string(),
            old_order: old_names,
            new_order: new_names,
        });
    }
}

fn field_signature(f: &Field) -> String {
    format!("{}: {}", f.name, type_string(&f.ty))
}

fn type_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
