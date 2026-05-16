//! Anchor `#[derive(Accounts)]` account-validation constraint extractor.
//!
//! Spectra's regression engine works on a *constraint model*, not on raw
//! source text. This module walks a Solana program's Rust source tree, finds
//! every `#[derive(Accounts)]` context struct, and reduces each account slot
//! to the set of security [`Guard`]s Anchor will enforce for it at runtime:
//! signer checks, owner / discriminator (type-cosplay) checks, `has_one`
//! relational checks, `address` pins, PDA `seeds` derivation, custom
//! `constraint` predicates, and CPI program-id pins.
//!
//! It deliberately does **not** try to prove a program is *absolutely* safe —
//! that is the job of absolute scanners (Sec3 X-Ray, Auditware Radar). Spectra
//! only needs a faithful, deterministic snapshot of the guard set so the
//! [`crate::regression`] differ can detect when a later version *removes or
//! weakens* a guard that an earlier (already-deployed) version enforced.
//!
//! Scope at this milestone: Anchor `#[derive(Accounts)]` structs. Native
//! (non-Anchor) manual `is_signer` / `owner ==` checks are a documented
//! non-goal for M0 and a roadmap item, not silently mis-handled.

use anyhow::{Context, Result};
use proc_macro2::TokenTree;
use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use syn::{Fields, Item, Type};
use walkdir::WalkDir;

/// One security guarantee Anchor enforces for an account slot.
///
/// Ordering is derived so a slot's guard set is a deterministic [`BTreeSet`];
/// equality is structural so the differ can compute exact set differences.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Guard {
    /// The account must have signed the transaction (`Signer<'info>` or
    /// `#[account(signer)]`). Dropping this is the canonical Solana
    /// missing-signer-check bug.
    Signer,
    /// Anchor owner + 8-byte discriminator check via a typed wrapper
    /// (`Account<'info, T>`, `AccountLoader<'info, T>`, `SystemAccount`, …).
    /// Carries the inner type name. Downgrading to `UncheckedAccount` /
    /// `AccountInfo` removes the type-cosplay protection.
    Typed(String),
    /// Explicit `#[account(owner = expr)]` owner pin.
    Owner(String),
    /// Hard `#[account(address = expr)]` pubkey pin (the strongest pin; used
    /// for CPI program-id and well-known accounts).
    Address(String),
    /// `#[account(has_one = field)]` relational-integrity check.
    HasOne(String),
    /// `#[account(seeds = [...], bump)]` PDA-derivation check.
    Seeds,
    /// Custom `#[account(constraint = expr)]` predicate.
    Constraint(String),
    /// `Program<'info, T>` CPI-target pin (program id checked against `T`).
    ProgramId(String),
}

/// One account slot inside a `#[derive(Accounts)]` context struct.
#[derive(Debug, Clone)]
pub struct Slot {
    /// Field name as written in the context struct.
    pub name: String,
    /// Rendered field type (e.g. `Account < 'info , Vault >`). Kept for
    /// human-readable findings; the differ keys off [`Slot::guards`].
    pub ty: String,
    /// `true` when the slot is `UncheckedAccount` / `AccountInfo` and Anchor
    /// performs no automatic validation on it.
    pub unchecked: bool,
    /// The set of security guards Anchor enforces for this slot.
    pub guards: BTreeSet<Guard>,
}

/// One `#[derive(Accounts)]` context struct (the account list of one
/// instruction handler).
#[derive(Debug, Clone)]
pub struct AccountsContext {
    /// Struct name (e.g. `Withdraw`).
    pub name: String,
    /// Account slots in source order.
    pub slots: Vec<Slot>,
}

/// The constraint model of a whole program version: every Anchor account
/// context, keyed by struct name for deterministic pairing across versions.
#[derive(Debug, Clone, Default)]
pub struct ProgramModel {
    /// Account contexts by struct name.
    pub contexts: BTreeMap<String, AccountsContext>,
}

impl ProgramModel {
    /// Build the constraint model for a program version by recursively
    /// parsing every `.rs` file under `dir`.
    ///
    /// Files that fail to parse as Rust are skipped rather than aborting the
    /// run: a regression gate must still report on the files it *can* model.
    /// `target/` and hidden directories are ignored.
    pub fn from_source_dir(dir: &Path) -> Result<Self> {
        let mut contexts = BTreeMap::new();

        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_entry(|e| {
                let n = e.file_name().to_string_lossy();
                n != "target" && !(n.starts_with('.') && n != ".")
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read source: {}", path.display()))?;
            let Ok(file) = syn::parse_file(&src) else {
                continue;
            };
            collect_contexts(&file.items, &mut contexts);
        }

        Ok(Self { contexts })
    }
}

fn collect_contexts(items: &[Item], out: &mut BTreeMap<String, AccountsContext>) {
    for item in items {
        match item {
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    collect_contexts(inner, out);
                }
            }
            Item::Struct(s) if has_accounts_derive(&s.attrs) => {
                if let Fields::Named(named) = &s.fields {
                    let slots = named.named.iter().filter_map(parse_slot).collect();
                    let name = s.ident.to_string();
                    out.insert(name.clone(), AccountsContext { name, slots });
                }
            }
            _ => {}
        }
    }
}

fn has_accounts_derive(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("derive") {
            return false;
        }
        let mut found = false;
        let _ = a.parse_nested_meta(|m| {
            if m.path.is_ident("Accounts") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

fn parse_slot(field: &syn::Field) -> Option<Slot> {
    let name = field.ident.as_ref()?.to_string();
    let ty = field.ty.to_token_stream().to_string();
    let (base_guard, unchecked) = classify_type(&field.ty);

    let mut guards: BTreeSet<Guard> = BTreeSet::new();
    if let Some(g) = base_guard {
        guards.insert(g);
    }
    for attr in &field.attrs {
        if attr.path().is_ident("account") {
            parse_account_attr(attr, &mut guards);
        }
    }

    Some(Slot {
        name,
        ty,
        unchecked,
        guards,
    })
}

/// Returns `(base guard from the wrapper type, is_unchecked)`.
fn classify_type(ty: &Type) -> (Option<Guard>, bool) {
    let head = type_head_ident(ty);
    match head.as_deref() {
        Some("Signer") => (Some(Guard::Signer), false),
        Some("UncheckedAccount") | Some("AccountInfo") => (None, true),
        Some("Program") => (
            Some(Guard::ProgramId(first_type_arg(ty).unwrap_or_default())),
            false,
        ),
        Some("Account") | Some("AccountLoader") | Some("InterfaceAccount") => (
            Some(Guard::Typed(first_type_arg(ty).unwrap_or_default())),
            false,
        ),
        Some("SystemAccount") => (Some(Guard::Typed("System".into())), false),
        Some("Sysvar") => (
            Some(Guard::Typed(
                first_type_arg(ty).unwrap_or_else(|| "Sysvar".into()),
            )),
            false,
        ),
        Some("Box") => {
            // `Box<Account<'info, T>>` etc. — recurse into the inner type.
            if let Type::Path(p) = ty {
                if let Some(seg) = p.path.segments.last() {
                    if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                            return classify_type(inner);
                        }
                    }
                }
            }
            (None, false)
        }
        // Interface, Other Anchor wrappers: treated as typed (owner-checked).
        Some(_) => (Some(Guard::Typed(head.unwrap())), false),
        None => (None, false),
    }
}

fn type_head_ident(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        return p.path.segments.last().map(|s| s.ident.to_string());
    }
    None
}

/// First *type* generic argument as a flattened string (lifetimes skipped).
fn first_type_arg(ty: &Type) -> Option<String> {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                for a in &args.args {
                    if let syn::GenericArgument::Type(t) = a {
                        return Some(t.to_token_stream().to_string().replace(' ', ""));
                    }
                }
            }
        }
    }
    None
}

/// Parse an `#[account(...)]` attribute into the guard set.
///
/// We deliberately walk the raw `proc_macro2` token stream rather than
/// `syn::Meta`: Anchor account attributes mix bare Rust keywords (`mut`) with
/// `key = expr` pairs, and `mut` is a reserved keyword that is *not* a valid
/// `syn::Meta::Path`. Parsing the attribute as `Punctuated<Meta, ,>` therefore
/// fails for the whole attribute, silently dropping every constraint in it
/// (e.g. `#[account(mut, has_one = authority)]` would lose `has_one`). The
/// token walk below splits on top-level commas and tolerates bare keywords.
fn parse_account_attr(attr: &syn::Attribute, guards: &mut BTreeSet<Guard>) {
    let syn::Meta::List(list) = &attr.meta else {
        return;
    };

    let mut items: Vec<Vec<TokenTree>> = vec![Vec::new()];
    for tt in list.tokens.clone() {
        match &tt {
            TokenTree::Punct(p) if p.as_char() == ',' => items.push(Vec::new()),
            _ => items.last_mut().unwrap().push(tt),
        }
    }

    for item in items {
        if item.is_empty() {
            continue;
        }
        let key = match &item[0] {
            TokenTree::Ident(i) => i.to_string(),
            _ => continue,
        };
        let has_eq = item
            .get(1)
            .map(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == '='))
            .unwrap_or(false);
        let value: String = if has_eq {
            item[2..]
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("")
        } else {
            String::new()
        };
        match key.as_str() {
            "signer" => {
                guards.insert(Guard::Signer);
            }
            "has_one" => {
                guards.insert(Guard::HasOne(value));
            }
            "owner" => {
                guards.insert(Guard::Owner(value));
            }
            "address" => {
                guards.insert(Guard::Address(value));
            }
            "constraint" => {
                guards.insert(Guard::Constraint(value));
            }
            "seeds" => {
                guards.insert(Guard::Seeds);
            }
            _ => {}
        }
    }
}
