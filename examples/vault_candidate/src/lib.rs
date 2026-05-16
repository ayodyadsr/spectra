//! Synthetic Anchor vault program — CANDIDATE (the upgrade under review).
//!
//! Versus `vault_baseline`, this version silently removes five distinct
//! account-validation guards on `Withdraw` and adds one brand-new
//! unvalidated account slot. `Initialize` is byte-identical to the baseline
//! and must produce zero findings — the strictly-differential property.
//!
//! This file only needs to parse as Rust; it is a fixture, not a deployable
//! program.

use anchor_lang::prelude::*;

declare_id!("Vau1t1111111111111111111111111111111111111");

#[program]
pub mod vault {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn withdraw(_ctx: Context<Withdraw>, _amount: u64) -> Result<()> {
        Ok(())
    }

    pub fn emergency_drain(_ctx: Context<EmergencyDrain>) -> Result<()> {
        Ok(())
    }
}

// Identical to baseline — must NOT produce any finding.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + 64)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    // REGRESSION: `has_one = authority` dropped.
    #[account(mut)]
    pub vault: Account<'info, Vault>,

    // REGRESSION: was `Signer<'info>` — signer check removed.
    /// CHECK: intentionally unchecked in this (vulnerable) candidate.
    pub authority: UncheckedAccount<'info>,

    // REGRESSION: was `Account<'info, TokenAccount>` + ownership constraint.
    /// CHECK: intentionally unchecked in this (vulnerable) candidate.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    // REGRESSION: PDA `seeds`/`bump` derivation dropped.
    pub config: Account<'info, Config>,

    // REGRESSION: was `Program<'info, Token>` — CPI target no longer pinned.
    /// CHECK: intentionally unchecked in this (vulnerable) candidate.
    pub token_program: UncheckedAccount<'info>,
}

// NEW context introducing a brand-new unvalidated slot (warning, not a
// regression of an existing guarantee).
#[derive(Accounts)]
pub struct EmergencyDrain<'info> {
    #[account(mut)]
    pub vault: Account<'info, Vault>,
    /// CHECK: new unvalidated account.
    pub anyone: UncheckedAccount<'info>,
}

#[account]
pub struct Vault {
    pub authority: Pubkey,
    pub amount: u64,
}

#[account]
pub struct Config {
    pub fee_bps: u16,
}
