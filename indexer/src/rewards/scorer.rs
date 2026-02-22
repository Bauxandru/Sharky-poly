/// Polymarket-style quadratic liquidity reward scorer.
///
/// Runs every 60 seconds. For each open order on each active reward epoch,
/// computes a score contribution and records it in the DB.
/// At epoch end, makers claim their proportional share of the reward pool.
///
/// Score formula (identical to Polymarket):
///   S(v, s) = ((v - s) / v)^2 * amount_sol
///
///   v = max_incentive_spread_bps
///   s = |order_apr - mid_apr| in bps  (spread from midpoint)
///   amount_sol = principal_lamports / 1e9
///
/// Two-sided enforcement:
///   Q_lender  = Σ S(order) for open lender orders in range
///   Q_borrower = Σ S(order) for open borrower orders in range
///   Q_min = min(Q_lender, Q_borrower)  (both sides must be present)

use sqlx::{PgPool, Row};
use tracing::warn;
use std::time::Duration;

pub async fn run_scorer(pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(e) = score_epoch(&pool).await {
            warn!("Scorer error: {}", e);
        }
    }
}

async fn score_epoch(pool: &PgPool) -> anyhow::Result<()> {
    // Get all active epochs
    let epochs = sqlx::query(
        r#"
        SELECT re.orderbook, re.epoch_id, ob.mid_apr_bps, ob.max_incentive_spread_bps
        FROM reward_epochs re
        JOIN orderbooks ob ON ob.pubkey = re.orderbook
        WHERE re.finalized = false
          AND re.start_ts <= extract(epoch from NOW())
          AND re.end_ts > extract(epoch from NOW())
        "#,
    )
    .fetch_all(pool)
    .await?;

    for epoch in epochs {
        let orderbook: String = epoch.try_get("orderbook")?;
        let epoch_id: i64 = epoch.try_get("epoch_id")?;
        let mid_apr: i64 = epoch.try_get("mid_apr_bps")?;
        let max_spread: i64 = epoch.try_get("max_incentive_spread_bps").unwrap_or(0);
        let mid_apr = mid_apr as u64;
        let max_spread = max_spread as u64;

        if mid_apr == 0 || max_spread == 0 {
            continue;
        }

        // Score lender orders
        let lender_scores = score_lender_orders(pool, &orderbook, mid_apr, max_spread).await?;

        // Score borrower orders
        let borrower_scores = score_borrower_orders(pool, &orderbook, mid_apr, max_spread).await?;

        // Compute Q_min per-maker and record
        let mut combined: std::collections::HashMap<String, (u64, u64)> =
            std::collections::HashMap::new();

        for (maker, score) in &lender_scores {
            combined.entry(maker.clone()).or_default().0 += score;
        }
        for (maker, score) in &borrower_scores {
            combined.entry(maker.clone()).or_default().1 += score;
        }

        for (maker, (q_lender, q_borrower)) in combined {
            // Q_min: bottlenecked by weaker side
            let q_min = q_lender.min(q_borrower);
            if q_min > 0 {
                sqlx::query(
                    r#"
                    INSERT INTO maker_scores (maker, orderbook, epoch_id, score, claimed, updated_at)
                    VALUES ($1, $2, $3, $4, false, NOW())
                    ON CONFLICT (maker, orderbook, epoch_id) DO UPDATE SET
                        score = maker_scores.score + $4,
                        updated_at = NOW()
                    "#,
                )
                .bind(&maker)
                .bind(&orderbook)
                .bind(epoch_id)
                .bind(q_min as i64)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(())
}

async fn score_lender_orders(
    pool: &PgPool,
    orderbook: &str,
    mid_apr: u64,
    max_spread: u64,
) -> anyhow::Result<Vec<(String, u64)>> {
    let orders = sqlx::query(
        r#"
        SELECT lender, apr_bps, principal_lamports
        FROM lender_orders
        WHERE orderbook = $1 AND status = 'open'
        "#,
    )
    .bind(orderbook)
    .fetch_all(pool)
    .await?;

    let mut scores = Vec::new();
    for order in orders {
        let lender: String = order.try_get("lender")?;
        let apr: i64 = order.try_get("apr_bps")?;
        let principal: i64 = order.try_get("principal_lamports")?;
        let apr = apr as u64;
        let spread = apr.abs_diff(mid_apr);
        if spread < max_spread {
            let score = quadratic_score(max_spread, spread, principal as u64);
            scores.push((lender, score));
        }
    }
    Ok(scores)
}

async fn score_borrower_orders(
    pool: &PgPool,
    orderbook: &str,
    mid_apr: u64,
    max_spread: u64,
) -> anyhow::Result<Vec<(String, u64)>> {
    let orders = sqlx::query(
        r#"
        SELECT borrower, max_apr_bps, principal_lamports
        FROM borrower_orders
        WHERE orderbook = $1 AND status = 'open'
        "#,
    )
    .bind(orderbook)
    .fetch_all(pool)
    .await?;

    let mut scores = Vec::new();
    for order in orders {
        let borrower: String = order.try_get("borrower")?;
        let apr: i64 = order.try_get("max_apr_bps")?;
        let principal: i64 = order.try_get("principal_lamports")?;
        let apr = apr as u64;
        let spread = apr.abs_diff(mid_apr);
        if spread < max_spread {
            let score = quadratic_score(max_spread, spread, principal as u64);
            scores.push((borrower, score));
        }
    }
    Ok(scores)
}

/// S(v, s) = ((v - s) / v)^2 * b
/// Returns score in units of 1e-6 SOL (microlamports / 1000)
fn quadratic_score(max_spread: u64, spread: u64, principal_lamports: u64) -> u64 {
    if spread >= max_spread {
        return 0;
    }
    let numerator = (max_spread - spread) as u128;
    let denominator = max_spread as u128;
    let ratio_1e6 = numerator * 1_000_000 / denominator;
    let ratio_sq_1e12 = ratio_1e6 * ratio_1e6;
    let amount_usol = principal_lamports as u128 / 1_000;
    let score = ratio_sq_1e12 / 1_000_000_000_000 * amount_usol;
    score as u64
}
