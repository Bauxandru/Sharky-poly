-- NFT Lending Protocol — Initial Schema
-- All reads by the frontend go through this DB, not Solana RPC.

CREATE TABLE IF NOT EXISTS orderbooks (
    pubkey                    TEXT PRIMARY KEY,
    collection_mint           TEXT NOT NULL,
    duration_seconds          BIGINT NOT NULL,
    best_lender_apr_bps       BIGINT NOT NULL DEFAULT 0,
    best_borrower_max_apr_bps BIGINT NOT NULL DEFAULT 0,
    mid_apr_bps               BIGINT NOT NULL DEFAULT 0,
    max_incentive_spread_bps  BIGINT,
    open_lender_orders        BIGINT NOT NULL DEFAULT 0,
    open_borrower_orders      BIGINT NOT NULL DEFAULT 0,
    active_loans              BIGINT NOT NULL DEFAULT 0,
    total_loans               BIGINT NOT NULL DEFAULT 0,
    total_volume_lamports     BIGINT NOT NULL DEFAULT 0,
    is_active                 BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orderbooks_collection ON orderbooks (collection_mint);
CREATE INDEX idx_orderbooks_volume ON orderbooks (total_volume_lamports DESC);

-- ─── Lender Orders ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS lender_orders (
    pubkey             TEXT PRIMARY KEY,
    orderbook          TEXT NOT NULL REFERENCES orderbooks(pubkey),
    lender             TEXT NOT NULL,
    principal_lamports BIGINT NOT NULL,
    apr_bps            BIGINT NOT NULL,
    expiry_ts          BIGINT NOT NULL DEFAULT 0,
    nonce              BIGINT NOT NULL,
    signature          TEXT NOT NULL,
    slot               BIGINT NOT NULL,
    -- open | filled | cancelled
    status             TEXT NOT NULL DEFAULT 'open',
    placed_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- For orderbook query: open orders sorted by APR ascending
CREATE INDEX idx_lender_orders_ob_status_apr
    ON lender_orders (orderbook, status, apr_bps ASC)
    WHERE status = 'open';

CREATE INDEX idx_lender_orders_lender ON lender_orders (lender);

-- ─── Borrower Orders ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS borrower_orders (
    pubkey             TEXT PRIMARY KEY,
    orderbook          TEXT NOT NULL REFERENCES orderbooks(pubkey),
    borrower           TEXT NOT NULL,
    nft_mint           TEXT NOT NULL,
    principal_lamports BIGINT NOT NULL,
    max_apr_bps        BIGINT NOT NULL,
    expiry_ts          BIGINT NOT NULL DEFAULT 0,
    signature          TEXT NOT NULL,
    slot               BIGINT NOT NULL,
    -- open | filled | cancelled
    status             TEXT NOT NULL DEFAULT 'open',
    placed_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (nft_mint, status)  -- enforce one open order per NFT
);

CREATE INDEX idx_borrower_orders_ob_status_apr
    ON borrower_orders (orderbook, status, max_apr_bps DESC)
    WHERE status = 'open';

CREATE INDEX idx_borrower_orders_borrower ON borrower_orders (borrower);
CREATE INDEX idx_borrower_orders_nft ON borrower_orders (nft_mint);

-- ─── Loans ────────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS loans (
    pubkey             TEXT PRIMARY KEY,
    orderbook          TEXT NOT NULL REFERENCES orderbooks(pubkey),
    lender             TEXT NOT NULL,
    borrower           TEXT NOT NULL,
    nft_mint           TEXT NOT NULL,
    principal_lamports BIGINT NOT NULL,
    apr_bps            BIGINT NOT NULL,
    duration_seconds   BIGINT NOT NULL,
    start_ts           BIGINT NOT NULL,
    end_ts             BIGINT NOT NULL,
    repay_lamports     BIGINT NOT NULL,
    lender_order       TEXT,
    borrower_order     TEXT,
    -- active | repaid | foreclosed
    state              TEXT NOT NULL DEFAULT 'active',
    signature          TEXT NOT NULL,
    repaid_at          TIMESTAMPTZ,
    foreclosed_at      TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_loans_orderbook_state ON loans (orderbook, state);
CREATE INDEX idx_loans_lender ON loans (lender);
CREATE INDEX idx_loans_borrower ON loans (borrower);
CREATE INDEX idx_loans_nft ON loans (nft_mint);
CREATE INDEX idx_loans_end_ts ON loans (end_ts) WHERE state = 'active';

-- ─── Reward Epochs ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS reward_epochs (
    id                    BIGSERIAL PRIMARY KEY,
    orderbook             TEXT NOT NULL REFERENCES orderbooks(pubkey),
    epoch_id              BIGINT NOT NULL,
    start_ts              BIGINT NOT NULL,
    end_ts                BIGINT NOT NULL,
    total_reward_lamports BIGINT NOT NULL,
    total_score           BIGINT NOT NULL DEFAULT 0,
    finalized             BOOLEAN NOT NULL DEFAULT FALSE,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (orderbook, epoch_id)
);

CREATE INDEX idx_reward_epochs_active
    ON reward_epochs (orderbook)
    WHERE finalized = FALSE;

-- ─── Maker Scores ─────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS maker_scores (
    maker      TEXT NOT NULL,
    orderbook  TEXT NOT NULL REFERENCES orderbooks(pubkey),
    epoch_id   BIGINT NOT NULL,
    score      BIGINT NOT NULL DEFAULT 0,
    claimed    BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (maker, orderbook, epoch_id)
);

CREATE INDEX idx_maker_scores_maker ON maker_scores (maker);

-- ─── Helpers ──────────────────────────────────────────────────────────────────

-- Auto-update updated_at
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER orderbooks_updated_at BEFORE UPDATE ON orderbooks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER lender_orders_updated_at BEFORE UPDATE ON lender_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER borrower_orders_updated_at BEFORE UPDATE ON borrower_orders
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
CREATE TRIGGER loans_updated_at BEFORE UPDATE ON loans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
