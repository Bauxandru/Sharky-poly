/// PostgreSQL database layer.
///
/// All state that the frontend needs is stored here.
/// Frontend reads from this DB via our REST API — NOT from Solana RPC.
/// This is the primary cost-saving measure vs Sharky's architecture.

use anyhow::Result;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

pub async fn connect(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

// ─── Orderbook ────────────────────────────────────────────────────────────────

pub async fn upsert_orderbook(
    pool: &PgPool,
    orderbook: &str,
    collection_mint: &str,
    duration_seconds: u64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO orderbooks (pubkey, collection_mint, duration_seconds,
            best_lender_apr_bps, best_borrower_max_apr_bps, mid_apr_bps,
            open_lender_orders, open_borrower_orders, active_loans,
            total_loans, total_volume_lamports, is_active, created_at)
        VALUES ($1, $2, $3, 0, 0, 0, 0, 0, 0, 0, 0, true, NOW())
        ON CONFLICT (pubkey) DO NOTHING
        "#,
    )
    .bind(orderbook)
    .bind(collection_mint)
    .bind(duration_seconds as i64)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_orderbook_best_apr(
    pool: &PgPool,
    orderbook: &str,
    best_lender_apr_bps: u64,
    best_borrower_max_apr_bps: u64,
) -> Result<()> {
    let mid = if best_lender_apr_bps > 0 && best_borrower_max_apr_bps > 0 {
        (best_lender_apr_bps + best_borrower_max_apr_bps) / 2
    } else {
        best_lender_apr_bps.max(best_borrower_max_apr_bps)
    };

    sqlx::query(
        r#"
        UPDATE orderbooks
        SET best_lender_apr_bps = $2,
            best_borrower_max_apr_bps = $3,
            mid_apr_bps = $4,
            updated_at = NOW()
        WHERE pubkey = $1
        "#,
    )
    .bind(orderbook)
    .bind(best_lender_apr_bps as i64)
    .bind(best_borrower_max_apr_bps as i64)
    .bind(mid as i64)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── Lender Orders ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn upsert_lender_order(
    pool: &PgPool,
    order: &str,
    orderbook: &str,
    lender: &str,
    principal_lamports: u64,
    apr_bps: u64,
    expiry_ts: i64,
    nonce: u64,
    signature: &str,
    slot: u64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO lender_orders
            (pubkey, orderbook, lender, principal_lamports, apr_bps,
             expiry_ts, nonce, signature, slot, status, placed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'open', NOW())
        ON CONFLICT (pubkey) DO UPDATE SET
            status = 'open',
            updated_at = NOW()
        "#,
    )
    .bind(order)
    .bind(orderbook)
    .bind(lender)
    .bind(principal_lamports as i64)
    .bind(apr_bps as i64)
    .bind(expiry_ts)
    .bind(nonce as i64)
    .bind(signature)
    .bind(slot as i64)
    .execute(pool)
    .await?;

    // Recalculate best lender APR for this orderbook
    refresh_orderbook_best_apr(pool, orderbook).await?;
    Ok(())
}

pub async fn cancel_lender_order(pool: &PgPool, order: &str) -> Result<()> {
    let row = sqlx::query(
        "UPDATE lender_orders SET status = 'cancelled', updated_at = NOW()
         WHERE pubkey = $1 RETURNING orderbook",
    )
    .bind(order)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let ob: String = r.try_get("orderbook")?;
        refresh_orderbook_best_apr(pool, &ob).await?;
    }
    Ok(())
}

// ─── Borrower Orders ──────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn upsert_borrower_order(
    pool: &PgPool,
    order: &str,
    orderbook: &str,
    borrower: &str,
    nft_mint: &str,
    principal_lamports: u64,
    max_apr_bps: u64,
    expiry_ts: i64,
    signature: &str,
    slot: u64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO borrower_orders
            (pubkey, orderbook, borrower, nft_mint, principal_lamports,
             max_apr_bps, expiry_ts, signature, slot, status, placed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'open', NOW())
        ON CONFLICT (pubkey) DO UPDATE SET
            status = 'open',
            updated_at = NOW()
        "#,
    )
    .bind(order)
    .bind(orderbook)
    .bind(borrower)
    .bind(nft_mint)
    .bind(principal_lamports as i64)
    .bind(max_apr_bps as i64)
    .bind(expiry_ts)
    .bind(signature)
    .bind(slot as i64)
    .execute(pool)
    .await?;

    refresh_orderbook_best_apr(pool, orderbook).await?;
    Ok(())
}

pub async fn cancel_borrower_order(pool: &PgPool, order: &str) -> Result<()> {
    let row = sqlx::query(
        "UPDATE borrower_orders SET status = 'cancelled', updated_at = NOW()
         WHERE pubkey = $1 RETURNING orderbook",
    )
    .bind(order)
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        let ob: String = r.try_get("orderbook")?;
        refresh_orderbook_best_apr(pool, &ob).await?;
    }
    Ok(())
}

// ─── Loans ────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn insert_loan(
    pool: &PgPool,
    loan: &str,
    orderbook: &str,
    lender: &str,
    borrower: &str,
    nft_mint: &str,
    principal_lamports: u64,
    apr_bps: u64,
    duration_seconds: u64,
    start_ts: i64,
    end_ts: i64,
    repay_lamports: u64,
    signature: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO loans
            (pubkey, orderbook, lender, borrower, nft_mint,
             principal_lamports, apr_bps, duration_seconds,
             start_ts, end_ts, repay_lamports, state, signature, created_at)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'active',$12,NOW())
        ON CONFLICT (pubkey) DO NOTHING
        "#,
    )
    .bind(loan)
    .bind(orderbook)
    .bind(lender)
    .bind(borrower)
    .bind(nft_mint)
    .bind(principal_lamports as i64)
    .bind(apr_bps as i64)
    .bind(duration_seconds as i64)
    .bind(start_ts)
    .bind(end_ts)
    .bind(repay_lamports as i64)
    .bind(signature)
    .execute(pool)
    .await?;

    // Mark matched orders as filled
    sqlx::query(
        "UPDATE lender_orders SET status = 'filled', updated_at = NOW() WHERE pubkey IN
         (SELECT lender_order FROM loans WHERE pubkey = $1)",
    )
    .bind(loan)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE borrower_orders SET status = 'filled', updated_at = NOW() WHERE pubkey IN
         (SELECT borrower_order FROM loans WHERE pubkey = $1)",
    )
    .bind(loan)
    .execute(pool)
    .await?;

    refresh_orderbook_best_apr(pool, orderbook).await?;
    Ok(())
}

pub async fn mark_loan_repaid(pool: &PgPool, loan: &str, repaid_at: i64) -> Result<()> {
    sqlx::query(
        "UPDATE loans SET state = 'repaid', repaid_at = to_timestamp($2), updated_at = NOW()
         WHERE pubkey = $1",
    )
    .bind(loan)
    .bind(repaid_at as f64)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_loan_foreclosed(pool: &PgPool, loan: &str, foreclosed_at: i64) -> Result<()> {
    sqlx::query(
        "UPDATE loans SET state = 'foreclosed', foreclosed_at = to_timestamp($2), updated_at = NOW()
         WHERE pubkey = $1",
    )
    .bind(loan)
    .bind(foreclosed_at as f64)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── Reward Scores ────────────────────────────────────────────────────────────

pub async fn upsert_maker_score(
    pool: &PgPool,
    maker: &str,
    orderbook: &str,
    epoch_id: u64,
    score_delta: u64,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO maker_scores (maker, orderbook, epoch_id, score, claimed, updated_at)
        VALUES ($1, $2, $3, $4, false, NOW())
        ON CONFLICT (maker, orderbook, epoch_id) DO UPDATE SET
            score = maker_scores.score + $4,
            updated_at = NOW()
        "#,
    )
    .bind(maker)
    .bind(orderbook)
    .bind(epoch_id as i64)
    .bind(score_delta as i64)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Recompute the best lender/borrower APR from open orders and update the orderbook row.
/// Called after any order insert/cancel/fill on an orderbook.
async fn refresh_orderbook_best_apr(pool: &PgPool, orderbook: &str) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE orderbooks ob SET
            best_lender_apr_bps = COALESCE(
                (SELECT MIN(apr_bps) FROM lender_orders
                 WHERE orderbook = $1 AND status = 'open'), 0),
            best_borrower_max_apr_bps = COALESCE(
                (SELECT MAX(max_apr_bps) FROM borrower_orders
                 WHERE orderbook = $1 AND status = 'open'), 0),
            open_lender_orders = (
                SELECT COUNT(*) FROM lender_orders
                WHERE orderbook = $1 AND status = 'open'),
            open_borrower_orders = (
                SELECT COUNT(*) FROM borrower_orders
                WHERE orderbook = $1 AND status = 'open'),
            active_loans = (
                SELECT COUNT(*) FROM loans
                WHERE orderbook = $1 AND state = 'active'),
            updated_at = NOW()
        WHERE ob.pubkey = $1
        "#,
    )
    .bind(orderbook)
    .execute(pool)
    .await?;

    // Recalculate mid APR
    sqlx::query(
        r#"
        UPDATE orderbooks SET
            mid_apr_bps = CASE
                WHEN best_lender_apr_bps > 0 AND best_borrower_max_apr_bps > 0
                    THEN (best_lender_apr_bps + best_borrower_max_apr_bps) / 2
                WHEN best_lender_apr_bps > 0 THEN best_lender_apr_bps
                ELSE best_borrower_max_apr_bps
            END
        WHERE pubkey = $1
        "#,
    )
    .bind(orderbook)
    .execute(pool)
    .await?;

    Ok(())
}
