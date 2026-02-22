use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use crate::state::{GlobalState, Loan, LoanState, OrderBook};

/// Repay a loan.
///
/// Borrower sends `repay_lamports` (principal + interest).
/// The protocol_fee portion is sent to the reward pool.
/// The remainder (principal + interest - fee) goes to the lender.
/// The NFT is thawed and the loan account is closed.
pub fn repay_loan(ctx: Context<RepayLoan>) -> Result<()> {
    let loan = &mut ctx.accounts.loan;
    let now = Clock::get()?.unix_timestamp;

    require!(loan.state == LoanState::Active, crate::error::LendingError::LoanNotActive);
    // Borrower can repay early — no check for now < end_ts

    let repay_amount = loan.repay_lamports;
    let protocol_fee = loan.protocol_fee_lamports;
    // Amount going to lender = repay - protocol_fee
    let lender_amount = repay_amount.checked_sub(protocol_fee).unwrap();

    // Transfer repay from borrower → lender
    let lender_transfer = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: ctx.accounts.borrower.to_account_info(),
            to: ctx.accounts.lender.to_account_info(),
        },
    );
    system_program::transfer(lender_transfer, lender_amount)?;

    // Transfer protocol fee → reward pool vault
    if protocol_fee > 0 {
        let fee_transfer = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.borrower.to_account_info(),
                to: ctx.accounts.reward_pool_vault.to_account_info(),
            },
        );
        system_program::transfer(fee_transfer, protocol_fee)?;
    }

    // Thaw the NFT
    let seeds = &[b"program_authority".as_ref(), &[ctx.bumps.program_authority]];
    let signer_seeds = &[&seeds[..]];

    let thaw_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::ThawAccount {
            account: ctx.accounts.nft_token_account.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::thaw_account(thaw_ctx)?;

    // Revoke program delegate from borrower's token account
    let revoke_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Revoke {
            source: ctx.accounts.nft_token_account.to_account_info(),
            authority: ctx.accounts.borrower.to_account_info(),
        },
    );
    token::revoke(revoke_ctx)?;

    // Update orderbook
    let ob = &mut ctx.accounts.orderbook;
    ob.active_loans = ob.active_loans.saturating_sub(1);

    // Mark loan as repaid (account is closed, event is the permanent record)
    emit!(crate::events::LoanRepaid {
        loan: loan.key(),
        orderbook: ob.key(),
        lender: loan.lender,
        borrower: loan.borrower,
        nft_mint: loan.nft_mint,
        principal_lamports: loan.principal_lamports,
        repay_lamports: repay_amount,
        protocol_fee_lamports: protocol_fee,
        repaid_at: now,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct RepayLoan<'info> {
    #[account(
        mut,
        seeds = [
            OrderBook::SEED,
            orderbook.collection_mint.as_ref(),
            &orderbook.duration_seconds.to_le_bytes(),
        ],
        bump = orderbook.bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        seeds = [GlobalState::SEED],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        close = borrower,   // Return loan account rent to borrower
        seeds = [
            Loan::SEED,
            loan.lender_order.as_ref(),
            loan.borrower_order.as_ref(),
        ],
        bump = loan.bump,
        has_one = orderbook,
        has_one = borrower,
        has_one = lender,
        has_one = nft_mint,
        has_one = nft_token_account,
    )]
    pub loan: Account<'info, Loan>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    /// CHECK: Lender receives payment. Address verified by has_one on loan.
    #[account(mut, address = loan.lender)]
    pub lender: UncheckedAccount<'info>,

    /// CHECK: Reward pool vault address from global state.
    #[account(mut, address = global_state.reward_pool_vault)]
    pub reward_pool_vault: UncheckedAccount<'info>,

    pub nft_mint: Account<'info, Mint>,

    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: Program authority PDA.
    #[account(
        seeds = [b"program_authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Foreclose a loan after its duration has expired.
///
/// The lender claims the NFT collateral. The loan account is closed.
/// The borrower loses the NFT but keeps the principal they borrowed.
pub fn foreclose_loan(ctx: Context<ForeCloseLoan>) -> Result<()> {
    let loan = &ctx.accounts.loan;
    let now = Clock::get()?.unix_timestamp;

    require!(loan.state == LoanState::Active, crate::error::LendingError::LoanNotActive);
    require!(now >= loan.end_ts, crate::error::LendingError::LoanNotExpired);

    // Thaw the NFT
    let seeds = &[b"program_authority".as_ref(), &[ctx.bumps.program_authority]];
    let signer_seeds = &[&seeds[..]];

    let thaw_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::ThawAccount {
            account: ctx.accounts.nft_token_account.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::thaw_account(thaw_ctx)?;

    // Transfer NFT from borrower's account → lender's token account
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.nft_token_account.to_account_info(),
            to: ctx.accounts.lender_nft_token_account.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(transfer_ctx, 1)?;

    // Update orderbook
    ctx.accounts.orderbook.active_loans =
        ctx.accounts.orderbook.active_loans.saturating_sub(1);

    emit!(crate::events::LoanForeclosed {
        loan: loan.key(),
        orderbook: ctx.accounts.orderbook.key(),
        lender: loan.lender,
        borrower: loan.borrower,
        nft_mint: loan.nft_mint,
        principal_lamports: loan.principal_lamports,
        foreclosed_at: now,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ForeCloseLoan<'info> {
    #[account(
        mut,
        seeds = [
            OrderBook::SEED,
            orderbook.collection_mint.as_ref(),
            &orderbook.duration_seconds.to_le_bytes(),
        ],
        bump = orderbook.bump
    )]
    pub orderbook: Account<'info, OrderBook>,

    #[account(
        mut,
        close = lender,
        seeds = [
            Loan::SEED,
            loan.lender_order.as_ref(),
            loan.borrower_order.as_ref(),
        ],
        bump = loan.bump,
        has_one = orderbook,
        has_one = lender,
        has_one = nft_mint,
        has_one = nft_token_account,
    )]
    pub loan: Account<'info, Loan>,

    #[account(mut)]
    pub lender: Signer<'info>,

    /// CHECK: validated by has_one = nft_token_account on loan
    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,

    pub nft_mint: Account<'info, Mint>,

    /// Lender's token account to receive the NFT.
    #[account(
        mut,
        constraint = lender_nft_token_account.owner == lender.key(),
        constraint = lender_nft_token_account.mint == nft_mint.key(),
    )]
    pub lender_nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA validated by seeds.
    #[account(
        seeds = [b"program_authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
