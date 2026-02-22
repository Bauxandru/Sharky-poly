use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Mint};
use mpl_token_metadata::accounts::Metadata;
use crate::state::{BorrowerOrder, OrderBook};

/// Place a borrower limit order.
///
/// The borrower's NFT is frozen in their token account.
/// The program sets itself as freeze authority via CPI to Token program.
///
/// Note: `principal_lamports` is the amount the borrower WANTS to receive.
/// `max_apr_bps` is the maximum APR they will accept.
pub fn place_borrower_order(
    ctx: Context<PlaceBorrowerOrder>,
    principal_lamports: u64,
    max_apr_bps: u64,
    expiry_ts: i64,
) -> Result<()> {
    let ob = &mut ctx.accounts.orderbook;
    require!(ob.is_active, crate::error::LendingError::OrderBookInactive);
    require!(principal_lamports >= 100_000_000, crate::error::LendingError::AmountTooSmall);
    require!(max_apr_bps > 0 && max_apr_bps <= 100_000, crate::error::LendingError::InvalidApr);

    let now = Clock::get()?.unix_timestamp;
    require!(expiry_ts == 0 || expiry_ts > now, crate::error::LendingError::AlreadyExpired);

    // Verify the NFT belongs to the declared collection (Metaplex metadata check).
    verify_collection(
        &ctx.accounts.nft_metadata,
        &ctx.accounts.nft_mint.key(),
        &ob.collection_mint,
    )?;

    // Verify borrower owns exactly 1 token in the NFT token account.
    let token_account = &ctx.accounts.nft_token_account;
    require!(
        token_account.owner == ctx.accounts.borrower.key(),
        crate::error::LendingError::Unauthorized
    );
    require!(token_account.amount == 1, crate::error::LendingError::InvalidNft);
    require!(
        token_account.mint == ctx.accounts.nft_mint.key(),
        crate::error::LendingError::InvalidNft
    );

    // Freeze the NFT in borrower's wallet.
    // Approve program_authority PDA as delegate, then freeze.

    // Approve program as delegate on the token account (needed to freeze)
    let approve_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Approve {
            to: ctx.accounts.nft_token_account.to_account_info(),
            delegate: ctx.accounts.program_authority.to_account_info(),
            authority: ctx.accounts.borrower.to_account_info(),
        },
    );
    token::approve(approve_ctx, 1)?;

    // Freeze the token account via the program authority PDA
    let seeds = &[b"program_authority".as_ref(), &[ctx.bumps.program_authority]];
    let signer_seeds = &[&seeds[..]];

    let freeze_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::FreezeAccount {
            account: ctx.accounts.nft_token_account.to_account_info(),
            mint: ctx.accounts.nft_mint.to_account_info(),
            authority: ctx.accounts.program_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::freeze_account(freeze_ctx)?;

    // Record the order
    let order = &mut ctx.accounts.borrower_order;
    order.orderbook = ob.key();
    order.borrower = ctx.accounts.borrower.key();
    order.nft_mint = ctx.accounts.nft_mint.key();
    order.nft_token_account = ctx.accounts.nft_token_account.key();
    order.principal_lamports = principal_lamports;
    order.max_apr_bps = max_apr_bps;
    order.expiry_ts = expiry_ts;
    order.created_at = now;
    order.bump = ctx.bumps.borrower_order;

    // Update orderbook stats
    ob.open_borrower_orders += 1;
    if ob.best_borrower_max_apr_bps == 0 || max_apr_bps > ob.best_borrower_max_apr_bps {
        ob.best_borrower_max_apr_bps = max_apr_bps;
        ob.update_mid_apr();
    }

    emit!(crate::events::BorrowerOrderPlaced {
        orderbook: ob.key(),
        order: ctx.accounts.borrower_order.key(),
        borrower: ctx.accounts.borrower.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
        principal_lamports,
        max_apr_bps,
        expiry_ts,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct PlaceBorrowerOrder<'info> {
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

    /// One active borrower order per NFT (seeds include nft_mint, not borrower).
    /// This prevents the same NFT from being pledged in multiple orders.
    #[account(
        init,
        payer = borrower,
        space = BorrowerOrder::SIZE,
        seeds = [
            BorrowerOrder::SEED,
            orderbook.key().as_ref(),
            nft_mint.key().as_ref(),
        ],
        bump
    )]
    pub borrower_order: Account<'info, BorrowerOrder>,

    pub nft_mint: Account<'info, Mint>,

    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: Metaplex metadata account for NFT. Verified in instruction body.
    pub nft_metadata: UncheckedAccount<'info>,

    /// Program's authority PDA used as freeze authority.
    /// Seeds: ["program_authority"]
    /// CHECK: PDA validated by seeds constraint.
    #[account(
        seeds = [b"program_authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Cancel a borrower order. Unfreezes the NFT and closes the order account.
pub fn cancel_borrower_order(ctx: Context<CancelBorrowerOrder>) -> Result<()> {
    let order = &ctx.accounts.borrower_order;
    require!(
        order.borrower == ctx.accounts.borrower.key(),
        crate::error::LendingError::Unauthorized
    );

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

    // Revoke program delegate
    let revoke_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Revoke {
            source: ctx.accounts.nft_token_account.to_account_info(),
            authority: ctx.accounts.borrower.to_account_info(),
        },
    );
    token::revoke(revoke_ctx)?;

    let ob = &mut ctx.accounts.orderbook;
    ob.open_borrower_orders = ob.open_borrower_orders.saturating_sub(1);
    if ob.open_borrower_orders == 0 {
        ob.best_borrower_max_apr_bps = 0;
        ob.update_mid_apr();
    }

    emit!(crate::events::BorrowerOrderCancelled {
        orderbook: ob.key(),
        order: ctx.accounts.borrower_order.key(),
        borrower: ctx.accounts.borrower.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
    });

    Ok(())
}

#[derive(Accounts)]
pub struct CancelBorrowerOrder<'info> {
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
        close = borrower,
        seeds = [
            BorrowerOrder::SEED,
            orderbook.key().as_ref(),
            nft_mint.key().as_ref(),
        ],
        bump = borrower_order.bump,
        has_one = orderbook,
        has_one = borrower,
        has_one = nft_mint,
        has_one = nft_token_account,
    )]
    pub borrower_order: Account<'info, BorrowerOrder>,

    pub nft_mint: Account<'info, Mint>,

    #[account(mut)]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// CHECK: PDA validated by seeds constraint.
    #[account(
        seeds = [b"program_authority"],
        bump
    )]
    pub program_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub borrower: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

/// Verify that an NFT's Metaplex metadata contains the expected collection.
fn verify_collection(
    metadata_info: &UncheckedAccount,
    nft_mint: &Pubkey,
    expected_collection: &Pubkey,
) -> Result<()> {
    // Derive expected metadata PDA and verify the account address matches
    let (expected_metadata, _) = Metadata::find_pda(nft_mint);
    require!(
        metadata_info.key() == expected_metadata,
        crate::error::LendingError::InvalidMetadata
    );

    // Parse metadata and check verified collection
    let metadata = Metadata::safe_deserialize(&*metadata_info.data.borrow())
        .map_err(|_| error!(crate::error::LendingError::InvalidMetadata))?;

    if let Some(collection) = metadata.collection {
        require!(collection.verified, crate::error::LendingError::CollectionNotVerified);
        require!(
            collection.key == *expected_collection,
            crate::error::LendingError::WrongCollection
        );
    } else {
        return Err(crate::error::LendingError::NoCollection.into());
    }

    Ok(())
}
