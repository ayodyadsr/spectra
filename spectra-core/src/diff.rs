use crate::discriminator::{account_discriminator, hex, instruction_discriminator};
use crate::idl::{Field, Idl, Instruction, TypeKind};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    InstructionAdded {
        name: String,
        discriminator: String,
    },
    InstructionRemoved {
        name: String,
        discriminator: String,
    },
    InstructionArgsChanged {
        name: String,
        old_args: Vec<String>,
        new_args: Vec<String>,
    },
    AccountAdded {
        name: String,
        discriminator: String,
    },
    AccountRemoved {
        name: String,
        discriminator: String,
    },
    AccountFieldReordered {
        account: String,
        old_order: Vec<String>,
        new_order: Vec<String>,
    },
    AccountFieldAdded {
        account: String,
        field: String,
        ty: String,
    },
    AccountFieldRemoved {
        account: String,
        field: String,
        ty: String,
    },
    AccountFieldTypeChanged {
        account: String,
        field: String,
        old_ty: String,
        new_ty: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Breaking,
    Warning,
}

impl Finding {
    pub fn severity(&self) -> Severity {
        match self {
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

#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    pub old_program: String,
    pub new_program: String,
    pub findings: Vec<Finding>,
    pub breaking_count: usize,
    pub warning_count: usize,
}

impl DiffReport {
    pub fn is_clean(&self) -> bool {
        self.breaking_count == 0
    }
}

pub fn diff_idls(old: &Idl, new: &Idl) -> DiffReport {
    let mut findings = Vec::new();

    diff_instructions(&old.instructions, &new.instructions, &mut findings);
    diff_accounts(old, new, &mut findings);

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
