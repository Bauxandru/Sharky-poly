use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::state::{GlobalState, OrderBook, RewardEpoch, MakerScore};

/// Initialize a new reward epoch for an orderbook.
/// Admin only — sets the time window and funds the reward pool.
pub fn initialize_reward_epoch(
    ctx: Context<InitializeRewardEpoch>,
    epoch_id: u64,
    start_ts: i64,
    end_ts: i64,
    total_reward_lamports: u64,
) -> Result<()> {
    let global = &ctx.accounts.global_state;
    require!(
        ctx.accounts.admin.key() == global.admin,
        crate::error::LendingError::Unauthorized
    );
    require!(end_ts > start_ts, crate::error::LendingError::InvalidParam);
    require!(total_reward_lamports > 0, crate::error::LendingError::InvalidParam);

    // Fund the reward pool (transfer from admin to vault)
    let transfer_ctx = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: ctx.accounts.admin.to_account_info(),
            to: ctx.accounts.reward_pool_vault.to_account_info(),
        },
    );
    system_program::transfer(transfer_ctx, total_reward_lamports)?;

    let epoch = &mut ctx.accounts.reward_epoch;
    epoch.orderbook = ctx.accounts.orderbook.key();
    epoch.epoch_id = epoch_id;
    epoch.start_ts = start_ts;
    epoch.end_ts = end_ts;
    epoch.total_reward_lamports = total_reward_lamports;
    epoch.total_score = 0;
    epoch.finalized = false;
    epoch.bump = ctx.bumps.reward_epoch;

    emit!(crate::events::RewardEpochCreated {
        orderbook: ctx.accounts.orderbook.key(),
        epoch_id,
        start_ts,
        end_ts,
        total_reward_lamports,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct InitializeRewardEpoch<'info> {
    #[account(
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    pub orderbook: Account<'info, OrderBook>,

    #[account(
        init,
        payer = admin,
        space = RewardEpoch::SIZE,
        seeds = [
            RewardEpoch::SEED,
            orderbook.key().as_ref(),
            &epoch_id.to_le_bytes(),
        ],
        bump
    )]
    pub reward_epoch: Account<'info, RewardEpoch>,

    /// CHECK: Reward pool vault, address verified against global state.
    #[account(mut, address = global_state.reward_pool_vault)]
    pub reward_pool_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Crank instruction: record a maker's score contribution for a sampling window.
///
/// This is called by the off-chain crank bot once per minute for each eligible
/// maker order. The crank computes the Polymarket-style quadratic score off-chain
/// and submits it here. The crank authority is trusted to compute scores honestly;
/// in production, this can be made verifiable with ZK proofs or oracle attestation.
pub fn record_maker_score(
    ctx: Context<RecordMakerScore>,
    score_delta: u64,
) -> Result<()> {
    let epoch = &mut ctx.accounts.reward_epoch;
    let now = Clock::get()?.unix_timestamp;
    require!(epoch.is_active(now), crate::error::LendingError::EpochNotActive);

    let maker_score = &mut ctx.accounts.maker_score;
    // Initialize if first time
    if maker_score.maker == Pubkey::default() {
        maker_score.maker = ctx.accounts.maker.key();
        maker_score.orderbook = ctx.accounts.orderbook.key();
        maker_score.epoch_id = epoch.epoch_id;
        maker_score.score = 0;
        maker_score.claimed = false;
        maker_score.bump = ctx.bumps.maker_score;
    }

    maker_score.score = maker_score.score.saturating_add(score_delta);
    epoch.total_score = epoch.total_score.saturating_add(score_delta);

    Ok(())
}

#[derive(Accounts)]
pub struct RecordMakerScore<'info> {
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        mut,
        seeds = [
            RewardEpoch::SEED,
            orderbook.key().as_ref(),
            &reward_epoch.epoch_id.to_le_bytes(),
        ],
        bump = reward_epoch.bump,
        has_one = orderbook,
    )]
    pub reward_epoch: Account<'info, RewardEpoch>,

    #[account(
        init_if_needed,
        payer = crank,
        space = MakerScore::SIZE,
        seeds = [
            MakerScore::SEED,
            maker.key().as_ref(),
            orderbook.key().as_ref(),
            &reward_epoch.epoch_id.to_le_bytes(),
        ],
        bump
    )]
    pub maker_score: Account<'info, MakerScore>,

    /// CHECK: The maker wallet being scored — just stored.
    pub maker: UncheckedAccount<'info>,

    /// The trusted crank authority that submits scores.
    #[account(mut)]
    pub crank: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Finalize a reward epoch (admin). Once finalized, no more scores can be added.
pub fn finalize_reward_epoch(ctx: Context<FinalizeRewardEpoch>) -> Result<()> {
    let global = &ctx.accounts.global_state;
    require!(
        ctx.accounts.admin.key() == global.admin,
        crate::error::LendingError::Unauthorized
    );
    let epoch = &mut ctx.accounts.reward_epoch;
    let now = Clock::get()?.unix_timestamp;
    require!(now >= epoch.end_ts, crate::error::LendingError::EpochNotEnded);
    epoch.finalized = true;
    Ok(())
}

#[derive(Accounts)]
pub struct FinalizeRewardEpoch<'info> {
    #[account(seeds = [GlobalState::SEED], bump = global_state.bump)]
    pub global_state: Account<'info, GlobalState>,

    pub orderbook: Account<'info, OrderBook>,

    #[account(
        mut,
        seeds = [
            RewardEpoch::SEED,
            orderbook.key().as_ref(),
            &reward_epoch.epoch_id.to_le_bytes(),
        ],
        bump = reward_epoch.bump,
        has_one = orderbook,
    )]
    pub reward_epoch: Account<'info, RewardEpoch>,

    pub admin: Signer<'info>,
}

/// Claim rewards for a finalized epoch.
///
/// Reward = (maker_score / total_score) * total_reward_lamports
pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
    let epoch = &ctx.accounts.reward_epoch;
    let maker_score_acc = &mut ctx.accounts.maker_score;

    require!(epoch.finalized, crate::error::LendingError::EpochNotFinalized);
    require!(!maker_score_acc.claimed, crate::error::LendingError::AlreadyClaimed);
    require!(epoch.total_score > 0, crate::error::LendingError::NoScore);

    let reward = (maker_score_acc.score as u128)
        .saturating_mul(epoch.total_reward_lamports as u128)
        / epoch.total_score as u128;

    let reward = reward as u64;
    require!(reward >= 1_000_000, crate::error::LendingError::RewardTooSmall); // min 0.001 SOL

    maker_score_acc.claimed = true;

    // Transfer reward from vault to maker
    // Vault is a regular account; we use a PDA signer (reward_vault_authority)
    let seeds = &[b"reward_vault_authority".as_ref(), &[ctx.bumps.reward_vault_authority]];
    let signer_seeds = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: ctx.accounts.reward_pool_vault.to_account_info(),
            to: ctx.accounts.maker.to_account_info(),
        },
        signer_seeds,
    );
    system_program::transfer(transfer_ctx, reward)?;

    emit!(crate::events::RewardClaimed {
        maker: ctx.accounts.maker.key(),
        orderbook: ctx.accounts.orderbook.key(),
        epoch_id: epoch.epoch_id,
        reward_lamports: reward,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        seeds = [
            RewardEpoch::SEED,
            orderbook.key().as_ref(),
            &reward_epoch.epoch_id.to_le_bytes(),
        ],
        bump = reward_epoch.bump,
        has_one = orderbook,
    )]
    pub reward_epoch: Account<'info, RewardEpoch>,

    #[account(
        mut,
        seeds = [
            MakerScore::SEED,
            maker.key().as_ref(),
            orderbook.key().as_ref(),
            &reward_epoch.epoch_id.to_le_bytes(),
        ],
        bump = maker_score.bump,
        has_one = maker,
    )]
    pub maker_score: Account<'info, MakerScore>,

    #[account(mut)]
    pub maker: Signer<'info>,

    /// CHECK: reward pool vault; address from global state.
    #[account(mut, address = global_state.reward_pool_vault)]
    pub reward_pool_vault: UncheckedAccount<'info>,

    /// PDA used as the vault's signing authority for transfers.
    /// CHECK: PDA validated by seeds.
    #[account(
        seeds = [b"reward_vault_authority"],
        bump
    )]
    pub reward_vault_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
