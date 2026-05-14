use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idl {
    #[serde(default)]
    pub version: Option<String>,
    pub name: String,
    #[serde(default)]
    pub instructions: Vec<Instruction>,
    #[serde(default)]
    pub accounts: Vec<Account>,
    #[serde(default)]
    pub types: Vec<TypeDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub name: String,
    #[serde(default)]
    pub accounts: Vec<InstructionAccount>,
    #[serde(default)]
    pub args: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionAccount {
    pub name: String,
    #[serde(default, alias = "isMut")]
    pub is_mut: bool,
    #[serde(default, alias = "isSigner")]
    pub is_signer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TypeKind {
    Struct {
        #[serde(default)]
        fields: Vec<Field>,
    },
    Enum {
        #[serde(default)]
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
}

impl Idl {
    pub fn from_path(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read IDL: {}", path.display()))?;
        let idl: Idl = serde_json::from_str(&content)
            .with_context(|| format!("failed to parse IDL JSON: {}", path.display()))?;
        Ok(idl)
    }
}
