# Deployment Guide

## Prerequisites

- Rust + Cargo
- Solana CLI (`solana --version` ≥ 1.18)
- Anchor CLI (`anchor --version` ≥ 0.30.1)
- Node.js 20+, Yarn/npm
- PostgreSQL 15+
- A Solana wallet with SOL for deployment gas

---

## Step 1: Build and Deploy the On-Chain Program

```bash
# Install Anchor if needed
cargo install --git https://github.com/coral-xyz/anchor anchor-cli --tag v0.30.1

# Build the program (generates IDL in target/idl/)
anchor build

# Deploy to Devnet first
solana config set --url devnet
anchor deploy --provider.cluster devnet

# Note the program ID from output, update Anchor.toml and .env files
```

After deploy, the actual program ID will be different from the placeholder.
Update `PROGRAM_ID` in all config files.

---

## Step 2: Set Up PostgreSQL

```bash
# Ubuntu/Debian
sudo apt install postgresql-15
sudo -u postgres psql -c "CREATE DATABASE nft_lending;"
sudo -u postgres psql -c "CREATE USER nft_lending WITH PASSWORD 'yourpass';"
sudo -u postgres psql -c "GRANT ALL ON DATABASE nft_lending TO nft_lending;"

# Apply schema
psql postgresql://nft_lending:yourpass@localhost/nft_lending \
     -f indexer/migrations/001_initial.sql
```

---

## Step 3: Deploy Indexer + API

```bash
# Copy env file
cp .env.example .env
# Fill in DATABASE_URL, GRPC_ENDPOINT, GRPC_API_KEY, PROGRAM_ID

# Build the indexer (release mode for production)
cargo build -p indexer --release

# Run (on VPS, use systemd or screen)
./target/release/indexer
```

**Systemd service** (recommended for VPS):

```ini
[Unit]
Description=NFT Lending Indexer
After=network.target postgresql.service

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu/nft-lending
ExecStart=/home/ubuntu/nft-lending/target/release/indexer
EnvironmentFile=/home/ubuntu/nft-lending/.env
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable nft-lending-indexer
sudo systemctl start nft-lending-indexer
```

---

## Step 4: Run the Crank Bot

```bash
cd crank
yarn install
cp ../.env.example .env  # fill in crank-specific vars

# Fund crank wallet (needs ~0.5 SOL for gas)
solana transfer <crank-wallet-pubkey> 0.5

# Start crank
yarn start
```

---

## Step 5: Deploy Frontend

```bash
cd frontend
yarn install

# Set environment variables
cat > .env.local << EOF
NEXT_PUBLIC_API_URL=https://api.your-domain.com
NEXT_PUBLIC_RPC_ENDPOINT=https://api.mainnet-beta.solana.com
NEXT_PUBLIC_PROGRAM_ID=<your-deployed-program-id>
EOF

# Deploy to Vercel (free tier)
vercel deploy --prod
```

---

## Infrastructure Cost Breakdown

| Service | Provider | Cost/Month |
|---|---|---|
| VPS (indexer + API + crank + DB) | Hetzner CX21 (2vCPU, 4GB) | $8 |
| PostgreSQL | Self-hosted on VPS | $0 |
| gRPC subscription | Helius Starter plan | $49 |
| Frontend hosting | Vercel free tier | $0 |
| Domain | Namecheap | $1 |
| **Total** | | **~$58/month** |

Compare to Sharky's estimated $2,000-5,000+/month.

### Scaling

At higher scale (>1000 TPS events):
- Upgrade VPS to Hetzner CX41 ($20/mo)
- Upgrade Helius to Growth plan ($200/mo)
- Add read replica for PostgreSQL ($15/mo)
- **Still under $250/month**

---

## gRPC Provider Options

| Provider | Endpoint | Notes |
|---|---|---|
| **Helius** | `mainnet.helius-rpc.com:443` | Best DX, $49/mo starter |
| **Triton** | Your node URL | Enterprise, more control |
| **QuickNode** | `your-node.quiknode.pro` | Good alternative |

---

## Security Checklist

Before mainnet:
- [ ] Get the Anchor program audited (Ottersec, Sec3, or Neodyme)
- [ ] Add admin multi-sig (Squads protocol) for all admin instructions
- [ ] Set a reasonable `max_incentive_spread_bps` per collection
- [ ] Add rate limiting to API (tower-governor)
- [ ] Enable PostgreSQL SSL in production
- [ ] Set up monitoring (Grafana + alerting on indexer lag)
- [ ] Test all loan lifecycle paths on devnet with real NFTs
