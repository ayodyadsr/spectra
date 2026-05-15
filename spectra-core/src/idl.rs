//! Anchor legacy-schema IDL parser.
//!
//! At M0 this module only supports the Anchor legacy IDL JSON shape. Anchor
//! 2026 (Codama) and Shank native IDL adapters are M1 deliverables and will
//! both flow through the same [`Idl::from_path`] entry point.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level Anchor IDL document. The `from_path` constructor parses JSON
/// from disk; all other fields are deserialised verbatim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idl {
    /// Anchor IDL schema version. Optional in legacy schemas.
    #[serde(default)]
    pub version: Option<String>,
    /// Program name as declared in the IDL.
    pub name: String,
    /// Instruction handlers exposed by the program.
    #[serde(default)]
    pub instructions: Vec<Instruction>,
    /// Account types defined by the program.
    #[serde(default)]
    pub accounts: Vec<Account>,
    /// Reusable type definitions (structs / enums) referenced by accounts and instructions.
    #[serde(default)]
    pub types: Vec<TypeDef>,
}

/// A single instruction handler entry from the IDL `instructions` array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    /// Instruction name. Used as the preimage for the Anchor discriminator
    /// `sha256("global:<name>")[..8]`.
    pub name: String,
    /// Accounts the instruction handler expects, in order.
    #[serde(default)]
    pub accounts: Vec<InstructionAccount>,
    /// Borsh-serialised argument fields, in order.
    #[serde(default)]
    pub args: Vec<Field>,
}

/// One account input slot on an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionAccount {
    /// Account binding name as declared in the IDL.
    pub name: String,
    /// True if the runtime must allow mutation of this account.
    #[serde(default, alias = "isMut")]
    pub is_mut: bool,
    /// True if the account must sign the transaction.
    #[serde(default, alias = "isSigner")]
    pub is_signer: bool,
}

/// An account type the program owns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account-type name. Used as the preimage for the Anchor discriminator
    /// `sha256("account:<name>")[..8]`.
    pub name: String,
    /// Storage layout for the account.
    #[serde(rename = "type")]
    pub ty: TypeKind,
}

/// Top-level reusable type definition from the IDL `types` array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    /// Type name.
    pub name: String,
    /// Type body (struct or enum).
    #[serde(rename = "type")]
    pub ty: TypeKind,
}

/// Variant body of an Anchor IDL named type: either a struct with ordered
/// fields or an enum with ordered variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TypeKind {
    /// Struct body with ordered fields. Order is load-bearing for Borsh.
    Struct {
        /// Ordered field list. Reordering or width-changing any entry is a
        /// breaking change against existing on-chain data.
        #[serde(default)]
        fields: Vec<Field>,
    },
    /// Enum body with ordered variants. Variant order is load-bearing because
    /// it defines the Borsh discriminant byte.
    Enum {
        /// Ordered variant list.
        #[serde(default)]
        variants: Vec<EnumVariant>,
    },
}

/// One field of a struct or one argument of an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Field type as serialised in the IDL JSON. Kept as raw `Value` so the
    /// diff engine can compare arbitrary nested types without a separate
    /// type-tree parser at M0.
    #[serde(rename = "type")]
    pub ty: serde_json::Value,
}

/// One variant of an enum body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    /// Variant name.
    pub name: String,
}

impl Idl {
    /// Parse an Anchor legacy IDL JSON file from disk.
    ///
    /// Returns a context-wrapped error if the file cannot be read or the JSON
    /// is not a valid Anchor IDL document.
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read IDL: {}", path.display()))?;
        let idl: Idl = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse IDL JSON: {}", path.display()))?;
        Ok(idl)
    }
}
