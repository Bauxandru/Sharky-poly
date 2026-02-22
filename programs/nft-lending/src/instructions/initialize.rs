use anchor_lang::prelude::*;
use crate::state::GlobalState;

/// One-time initialization of the global protocol state.
/// Must be called by the admin before any other instruction.
pub fn initialize_global(
    ctx: Context<InitializeGlobal>,
    protocol_fee_bps: u16,
    reward_share_bps: u16,
) -> Result<()> {
    require!(protocol_fee_bps <= 1_000, crate::error::LendingError::FeeTooHigh); // max 10%
    require!(reward_share_bps <= 10_000, crate::error::LendingError::InvalidParam);

    let state = &mut ctx.accounts.global_state;
    state.admin = ctx.accounts.admin.key();
    state.protocol_fee_bps = protocol_fee_bps;
    state.reward_share_bps = reward_share_bps;
    state.treasury = ctx.accounts.treasury.key();
    state.reward_pool_vault = ctx.accounts.reward_pool_vault.key();
    state.total_orderbooks = 0;
    state.total_volume_lamports = 0;
    state.bump = ctx.bumps.global_state;

    emit!(crate::events::GlobalInitialized {
        admin: state.admin,
        protocol_fee_bps,
        reward_share_bps,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGlobal<'info> {
    #[account(
        init,
        payer = admin,
        space = GlobalState::SIZE,
        seeds = [GlobalState::SEED],
        bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: just stored as a pubkey, not validated further
    pub treasury: UncheckedAccount<'info>,

    /// CHECK: just stored as a pubkey; actual vault account managed separately
    pub reward_pool_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

/// Update protocol parameters. Admin only.
pub fn update_global(
    ctx: Context<UpdateGlobal>,
    protocol_fee_bps: Option<u16>,
    reward_share_bps: Option<u16>,
) -> Result<()> {
    let state = &mut ctx.accounts.global_state;
    require!(ctx.accounts.admin.key() == state.admin, crate::error::LendingError::Unauthorized);

    if let Some(fee) = protocol_fee_bps {
        require!(fee <= 1_000, crate::error::LendingError::FeeTooHigh);
        state.protocol_fee_bps = fee;
    }
    if let Some(share) = reward_share_bps {
        require!(share <= 10_000, crate::error::LendingError::InvalidParam);
        state.reward_share_bps = share;
    }
    Ok(())
}

#[derive(Accounts)]
pub struct UpdateGlobal<'info> {
    #[account(
        mut,
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    pub admin: Signer<'info>,
}
