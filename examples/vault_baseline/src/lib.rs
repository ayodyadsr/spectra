//! Synthetic Anchor vault program — BASELINE (the last released / on-chain
//! version). Spectra snapshots the account-validation guard set of every
//! `#[derive(Accounts)]` context here and fails CI if a later candidate
//! version removes or weakens any of these guards.
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
}

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
    #[account(mut, has_one = authority)]
    pub vault: Account<'info, Vault>,

    pub authority: Signer<'info>,

    #[account(mut, constraint = destination.owner == authority.key())]
    pub destination: Account<'info, TokenAccount>,

    #[account(seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,

    pub token_program: Program<'info, Token>,
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
