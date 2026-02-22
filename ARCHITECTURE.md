# NFT Lending Dual Orderbook — Architecture

> An NFT-backed P2P lending protocol on Solana with a dual-sided CLOB and Polymarket-style liquidity rewards, built to be maximally cost-efficient.

---

## What We're Building vs Sharky

| Dimension | Sharky.fi | This Protocol |
|---|---|---|
| Orderbook | Single-sided (lenders only post offers) | Dual-sided CLOB (both lenders AND borrowers post limit orders) |
| Matching | Borrower takes best available lender offer | Orders match when lender APR ≤ borrower APR |
| Liquidity Rewards | None | Polymarket-style: quadratic scoring for orders near mid-APR |
| Indexer | Hello Moon (paid 3rd party) | Self-hosted gRPC subscriber → PostgreSQL ($20/mo VPS) |
| RPC strategy | Heavy `getProgramAccounts` polling | Deterministic PDAs + event-driven gRPC push (10x cheaper) |
| Account discovery | getProgramAccounts (expensive) | Deterministic PDA seeds (free) |
| NFT support | Regular SPL tokens (freeze authority) | SPL + cNFT (delegate) |

---

## System Components

```
┌─────────────────────────────────────────────────────────────────────┐
│                        SOLANA MAINNET                               │
│                                                                     │
│   ┌─────────────────────────────────────────────────┐              │
│   │           NFT Lending Program (Anchor)           │              │
│   │                                                  │              │
│   │  OrderBook (per collection + duration)           │              │
│   │  LenderOrder  │  BorrowerOrder  │  Loan          │              │
│   │  RewardEpoch  │  MakerScore                      │              │
│   └──────────────────┬──────────────────────────────┘              │
│                       │ gRPC events                                 │
└───────────────────────┼─────────────────────────────────────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   Yellowstone gRPC Node   │  (Helius / Triton - much cheaper
          │   (account + tx stream)   │   than polling RPCs)
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   Indexer (Rust / Axum)   │  Self-hosted on $20/mo VPS
          │   - Parse program events  │
          │   - Maintain orderbook    │
          │   - Track reward scores   │
          └─────┬──────────┬──────────┘
                │          │
    ┌───────────▼──┐  ┌────▼──────────────┐
    │  PostgreSQL  │  │  WebSocket Server  │
    │  (state DB)  │  │  (live updates)    │
    └───────────┬──┘  └────────────────────┘
                │
    ┌───────────▼──────────────┐
    │   REST API (Axum)         │
    │   /orderbooks             │
    │   /orders                 │
    │   /loans                  │
    │   /rewards                │
    └───────────┬──────────────┘
                │
    ┌───────────▼──────────────┐
    │   Next.js Frontend        │
    │   - Dual orderbook UI     │
    │   - Wallet integration    │
    │   - Reward dashboard      │
    └──────────────────────────┘
                │
    ┌───────────▼──────────────┐
    │   Crank Bot (TypeScript)  │
    │   - Watches for matchable │
    │     orders in DB          │
    │   - Submits match txns    │
    │   - Updates reward scores │
    └──────────────────────────┘
```

---

## On-Chain Program Design

### The Dual Orderbook Mechanic

In traditional NFT lending (Sharky), only lenders post orders. The "price" is APR.

In our dual CLOB:
- **Lender bids**: "I will lend X SOL against [Collection] at ≤Y% APR for Z days"
  - Sorted **ascending by APR** (best offer = lowest APR lender)
- **Borrower asks**: "I want to borrow X SOL against [Collection] at ≤Y% APR for Z days"
  - Sorted **descending by max APR** (best offer = highest max-APR borrower)
- **Match condition**: `best_lender_apr_bps ≤ best_borrower_max_apr_bps`
- **Execution APR**: Lender's APR (maker-taker model — lender is maker, gets price preference)

```
Lender Orderbook (sorted ASC by APR):        Borrower Orderbook (sorted DESC by max-APR):
─────────────────────────────────────────    ──────────────────────────────────────────────
APR      Amount    Duration                  Max APR   Amount    Duration
18%      5 SOL     7 days   ← best offer     35%       5 SOL     7 days  ← best offer
22%      3 SOL     7 days                    28%       2 SOL     7 days
25%      8 SOL     7 days                    25%       1 SOL     7 days

Mid-APR = (18 + 35) / 2 = 26.5%

Match: Lender@18% + Borrower@35% → Loan executes at 18% APR (maker wins)
```

### Account Architecture

All accounts use deterministic PDAs — eliminating expensive `getProgramAccounts` calls.

```
ORDERBOOK PDA:
  seeds = ["orderbook", collection_mint, duration_seconds_le_bytes]

LENDER_ORDER PDA:
  seeds = ["lender_order", orderbook_pubkey, lender_pubkey, order_nonce_le_bytes]

BORROWER_ORDER PDA:
  seeds = ["borrower_order", orderbook_pubkey, borrower_pubkey, nft_mint_pubkey]
  (one active order per NFT, not per user — prevents double-pledging)

LOAN PDA:
  seeds = ["loan", lender_order_pubkey, borrower_order_pubkey]

REWARD_EPOCH PDA:
  seeds = ["reward_epoch", orderbook_pubkey, epoch_id_le_bytes]

MAKER_SCORE PDA:
  seeds = ["maker_score", maker_pubkey, orderbook_pubkey, epoch_id_le_bytes]

GLOBAL_STATE PDA:
  seeds = ["global_state"]
```

### NFT Locking for Borrower Orders

When a borrower posts an order:
1. The Lending program sets itself as the **freeze authority** on the NFT token account
2. NFT stays in the borrower's wallet but is frozen (cannot be listed/sold/transferred)
3. If the order is cancelled → NFT is thawed, freeze authority revoked
4. If order is matched → NFT stays frozen for the loan duration
5. On repayment → NFT thawed, borrower regains full control
6. On foreclosure → NFT transferred to lender's wallet

### Instructions

```
initialize_global      - Admin: set protocol fee, reward rate, admin key
initialize_orderbook   - Permissionless: create orderbook for collection+duration

place_lender_order     - Lender deposits SOL into order escrow
cancel_lender_order    - Lender withdraws SOL from escrow

place_borrower_order   - Borrower: freezes NFT, records max-APR + amount
cancel_borrower_order  - Borrower: unfreezes NFT, closes order

match_orders           - Permissionless crank: match lender + borrower order → Loan
                         (also auto-called internally when placing orders)

repay_loan             - Borrower: sends principal+interest → lender, NFT unfreezes
foreclose_loan         - Lender (after expiry): claims NFT

initialize_reward_epoch - Admin: set epoch params and reward pool
finalize_reward_epoch   - Admin: mark epoch done
claim_rewards           - Maker: claim accumulated reward for epoch
```

---

## Liquidity Reward System

Adapted from Polymarket's formula for the lending context.

### The "Spread" in APR Terms

Instead of bid-ask price spread, we measure **APR spread from the mid-APR**:

```
mid_apr = (best_lender_apr + best_borrower_apr) / 2
max_incentive_spread_bps = 500  (5% APR)

eligible: |order_apr - mid_apr| < max_incentive_spread_bps
```

### Scoring Formula (same as Polymarket)

Sampled every minute, 1440 samples/day.

```
S(order) = ((v - s) / v)^2 * b

where:
  v = max_incentive_spread_bps
  s = |order_apr - mid_apr| in bps
  b = order_amount_in_lamports / 1_000_000_000 (amount in SOL)

Aggregate score:
  Q_lender = Σ S(lender_orders in range)
  Q_borrower = Σ S(borrower_orders in range)
  Q_min = min(Q_lender, Q_borrower)  ← bottlenecked by weaker side

Daily reward for maker i:
  Reward_i = (maker_score_i / total_score) * daily_reward_pool
```

**Key behaviors:**
- Orders 1% from mid-APR score 4x more than orders 2% from mid-APR
- Both lender AND borrower liquidity must be present to score (min formula)
- Rewards paid daily in SOL from a protocol reward pool

---

## Cost-Efficiency Strategy

### The Problem with Sharky's Infrastructure

Sharky uses:
1. `getProgramAccounts` via paid RPC nodes to read all loan/offer accounts
2. Hello Moon paid API for historical data indexing
3. Likely Helius/Triton paid nodes at high-volume pricing

Estimated Sharky infra cost: **$2,000-5,000+/month** at scale

### Our Approach

| Component | Sharky approach | Our approach | Savings |
|---|---|---|---|
| Account discovery | `getProgramAccounts` (expensive) | Deterministic PDAs (free lookups) | ~80% |
| Event indexing | Hello Moon ($$$) | Self-hosted gRPC subscriber | ~90% |
| Order state | RPC polling | PostgreSQL from gRPC stream | ~95% |
| Historical data | Hello Moon / Dune | Own PostgreSQL | ~100% |
| RPC tier | Premium dedicated node | Shared + gRPC subscription | ~70% |

**Estimated our infra cost: $80-150/month**

### gRPC Subscription Architecture

Instead of polling `getProgramAccounts` thousands of times per day, we subscribe ONCE to program account updates via Yellowstone gRPC:

```
Helius/Triton gRPC endpoint
  └── subscribe to program_id = our_lending_program
       └── on every account change → push to indexer
            └── indexer updates PostgreSQL
                 └── API serves from PostgreSQL (not RPC)
```

Only transaction submissions hit the RPC node (write path). All reads come from our own DB.

### Deterministic PDA Benefits

Since all accounts have known PDA seeds, the frontend can:
- Compute the orderbook address for any collection+duration pair without any RPC call
- The backend can enumerate all orderbooks from its DB without RPC
- Only `getAccountInfo` (cheap) needed for single-account reads, never `getProgramAccounts`

---

## Technology Stack

| Layer | Technology | Why |
|---|---|---|
| Smart contract | Rust + Anchor 0.30.1 | Standard for Solana DeFi, mature tooling |
| Blockchain | Solana mainnet | Where NFT liquidity is |
| Indexer | Rust + Yellowstone gRPC | Cheapest real-time event stream on Solana |
| Database | PostgreSQL | Reliable, cheap, SQL for orderbook queries |
| API | Rust + Axum | Fast, type-safe, low memory footprint |
| WebSocket | tokio-tungstenite | Real-time orderbook updates to frontend |
| Frontend | Next.js 14 + TypeScript | Standard modern web stack |
| Wallet | @solana/wallet-adapter | Standard Solana wallet integration |
| Crank bot | TypeScript | Easy to iterate on matching logic |
| Hosting | Single $40/mo VPS (indexer+API) + Vercel (frontend) | Minimal cost |

---

## Project Structure

```
sharky-dual/
├── programs/
│   └── nft-lending/           # Anchor program (Rust)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── error.rs
│           ├── events.rs
│           ├── state/
│           │   ├── mod.rs
│           │   ├── global_state.rs
│           │   ├── orderbook.rs
│           │   ├── lender_order.rs
│           │   ├── borrower_order.rs
│           │   ├── loan.rs
│           │   └── reward_epoch.rs
│           └── instructions/
│               ├── mod.rs
│               ├── initialize.rs
│               ├── orderbook.rs
│               ├── lender_order.rs
│               ├── borrower_order.rs
│               ├── match_orders.rs
│               ├── loan.rs
│               └── rewards.rs
├── indexer/                   # Rust gRPC indexer + API
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── geyser.rs         # Yellowstone gRPC client
│       ├── parser.rs         # Decode program accounts/events
│       ├── db.rs             # PostgreSQL operations
│       ├── api/              # Axum REST + WebSocket
│       │   ├── mod.rs
│       │   ├── orderbooks.rs
│       │   ├── orders.rs
│       │   ├── loans.rs
│       │   └── rewards.rs
│       └── rewards/
│           └── scorer.rs     # Liquidity score calculator
├── crank/                     # TypeScript order matcher
│   ├── package.json
│   └── src/
│       ├── index.ts
│       ├── matcher.ts
│       └── rewards.ts
├── sdk/                       # TypeScript SDK
│   ├── package.json
│   └── src/
│       ├── index.ts
│       ├── client.ts
│       ├── instructions.ts
│       └── types.ts
├── frontend/                  # Next.js app
│   ├── package.json
│   └── src/
│       ├── app/
│       │   ├── page.tsx           # Orderbook list
│       │   ├── orderbook/[id]/    # Individual orderbook
│       │   └── rewards/           # Reward dashboard
│       ├── components/
│       │   ├── DualOrderbook.tsx
│       │   ├── PlaceOrderModal.tsx
│       │   └── RewardTracker.tsx
│       └── lib/
│           ├── api.ts
│           └── wallet.ts
├── Anchor.toml
├── Cargo.toml
└── migrations/
    └── 001_initial.sql
```

---

## Reward Pool Funding

The reward pool is funded by:
1. **Protocol fees**: 0.5% of each loan's interest goes to the reward pool
2. **Optional protocol token emissions**: future expansion
3. **Initial treasury funding**: admin seeds the pool at launch

Daily reward pool size = accumulated fees from previous day.

---

## Deployment Plan

1. Deploy Anchor program to Devnet, run full test suite
2. Deploy indexer + API on a cheap VPS ($20-40/mo Hetzner/DigitalOcean)
3. Deploy frontend to Vercel (free tier)
4. Run crank bot on same VPS as indexer
5. Audit program before mainnet
6. Migrate to mainnet with seeded reward pool
