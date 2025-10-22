# Deployment Guide

Complete guide for building, testing, and deploying ICRC-151 ledger.

## Prerequisites

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Add WebAssembly Target

```bash
rustup target add wasm32-unknown-unknown
```

### 3. Install DFX (DFINITY SDK)

```bash
DFX_VERSION=0.29.1 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"
```

Verify installation:
```bash
dfx --version
# Should output: dfx 0.29.1
```

### 4. Install Candid Extractor

```bash
cargo install candid-extractor
```

### 5. Install ic-wasm (Optional)

For WASM optimization:
```bash
cargo install ic-wasm
```

---

## Local Deployment

### 1. Start Local Replica

```bash
dfx start --background --clean
```

This starts a local Internet Computer replica on `http://127.0.0.1:8000`.

### 2. Deploy Canister

```bash
dfx deploy icrc151
```

This will:
- Build the WASM binary
- Extract Candid interface
- Create canister ID
- Install code
- Generate TypeScript declarations

**Output:**
```
Deployed canisters.
URLs:
  Backend canister via Candid interface:
    icrc151: http://127.0.0.1:8000/?canisterId=...&id=...
```

### 3. Generate Declarations (Optional)

If declarations weren't generated:
```bash
dfx generate icrc151
```

This creates `src/declarations/icrc151/` with:
- `index.js` - JavaScript client
- `icrc151.did.js` - Candid bindings
- `icrc151.did.d.ts` - TypeScript types

---

## Manual Build

If you prefer to build manually:

```bash
./build.sh
```

This script:
1. Builds WASM with `cargo build`
2. Extracts Candid interface
3. Optimizes WASM with `ic-wasm` (if installed)

---

## Mainnet Deployment

### 1. Create Identity

```bash
# Create new identity for mainnet
dfx identity new mainnet-deployer

# Use the identity
dfx identity use mainnet-deployer

# Get principal (save this!)
dfx identity get-principal
```

**⚠️ IMPORTANT:** Backup your identity:
```bash
cp ~/.config/dfx/identity/mainnet-deployer/identity.pem ~/safe-backup-location/
```

### 2. Add Cycles

You need ICP to create canisters on mainnet.

**Option A: Using ICP from wallet**
1. Send ICP to your principal
2. Convert ICP to cycles

**Option B: Using cycles wallet**
```bash
# If you have a cycles wallet
dfx identity --network ic set-wallet <wallet-canister-id>
```

### 3. Deploy to Mainnet

```bash
# Create canister
dfx canister --network ic create icrc151

# Install code
dfx deploy --network ic icrc151

# Or combined
dfx deploy --network ic icrc151
```

### 4. Verify Deployment

```bash
# Get canister ID
dfx canister --network ic id icrc151

# Check canister info
dfx canister --network ic status icrc151
```

---

## Configuration

### dfx.json

```json
{
  "canisters": {
    "icrc151": {
      "type": "rust",
      "package": "icrc151",
      "candid": "candid/icrc151.did",
      "build": [
        "cargo build --target wasm32-unknown-unknown --release --package icrc151",
        "candid-extractor target/wasm32-unknown-unknown/release/icrc151.wasm > candid/icrc151.did"
      ],
      "wasm": "target/wasm32-unknown-unknown/release/icrc151.wasm",
      "declarations": {
        "output": "src/declarations/icrc151",
        "node_compatibility": true
      }
    }
  },
  "networks": {
    "local": {
      "bind": "127.0.0.1:8000",
      "type": "ephemeral"
    }
  }
}
```

### Cargo.toml

```toml
[package]
name = "icrc151"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
candid = { version = "0.10", features = ["value"] }
ic-cdk = "0.16"
ic-stable-structures = "0.6"
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
num-traits = "0.2"

[profile.release]
lto = true
opt-level = 3
```

---

## Post-Deployment Setup

### 1. Verify Controller

The deploying principal becomes the controller:

```bash
dfx canister --network ic info icrc151
```

Look for `Controllers:` field.

### 2. Create First Token

```bash
dfx canister call icrc151 create_token '(
  "Wrapped SOL",
  "ckSOL",
  9:nat8,
  opt (0:nat),
  opt (10_000:nat),
  opt "https://solana.com/logo.png",
  opt "Wrapped Solana on Internet Computer"
)'
```

Save the returned `token_id`.

### 3. Test Transfer

```bash
# First, mint some tokens
dfx canister call icrc151 mint_tokens '(
  blob "\ab\cd\ef...",
  record {
    owner = principal "xxxxx";
    subaccount = null;
  },
  1_000_000_000:nat,
  null
)'

# Then transfer
dfx canister call icrc151 transfer '(
  record {
    token_id = blob "\ab\cd\ef...";
    from_subaccount = null;
    to = record {
      owner = principal "yyyyy";
      subaccount = null;
    };
    amount = 100_000_000:nat;
    fee = opt (10_000:nat);
    memo = null;
    created_at_time = null;
  }
)'
```

---

## Upgrade Procedure

### Local Testing

1. Make code changes
2. Test locally:
   ```bash
   dfx deploy icrc151 --upgrade-unchanged
   ```

3. Verify state persisted:
   ```bash
   dfx canister call icrc151 get_transaction_count
   dfx canister call icrc151 get_info
   ```

### Mainnet Upgrade

```bash
# Build and test locally first!
dfx deploy --network ic icrc151 --mode upgrade
```

**⚠️ WARNING:** This is a live upgrade. Test thoroughly on local replica first!

### Upgrade with Validation

Add pre/post upgrade hooks in `src/lib.rs`:

```rust
#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let tx_count = state::get_transaction_count();
    ic_cdk::println!("Pre-upgrade: {} transactions", tx_count);

    // Validate state
    // All data already in stable memory
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let tx_count = state::get_transaction_count();
    ic_cdk::println!("Post-upgrade: {} transactions", tx_count);

    // Re-validate state
}
```

---

## Monitoring

### Check Canister Status

```bash
dfx canister --network ic status icrc151
```

**Output:**
- Memory size
- Cycles balance
- Module hash
- Controllers

### Monitor Cycles

```bash
# Check cycles balance
dfx canister --network ic status icrc151 | grep "Balance:"

# Top up cycles (if needed)
dfx canister --network ic deposit-cycles 1000000000000 icrc151
```

### Cycles Consumption Estimates

| Storage | Cycles/Month | USD/Month (approx) |
|---------|--------------|-------------------|
| 1 GB | 127 M | $0.17 |
| 10 GB | 1.27 B | $1.70 |
| 100 GB | 12.7 B | $17 |
| 300 GB | 38.1 B | $51 |

**At 300 GB:** ~$51/month in cycles

---

## Backup & Recovery

### Backup Canister ID

```bash
# Save canister ID
dfx canister --network ic id icrc151 > canister_id.txt

# Save to .env
echo "CANISTER_ID=$(dfx canister --network ic id icrc151)" >> .env
```

### Backup Controller Keys

```bash
# Backup identity
cp -r ~/.config/dfx/identity/mainnet-deployer ~/backup/

# Or export as PEM
cp ~/.config/dfx/identity/mainnet-deployer/identity.pem ~/backup/controller-key.pem
```

### Export State (Future)

Once ICRC-3 is implemented:
```bash
# Export all transactions
dfx canister call icrc151 icrc3_get_blocks '(
  record {
    start = 0:nat;
    length = 1000000:nat;
  }
)'
```

---

## Troubleshooting

### Build Fails

```bash
# Clean build
cargo clean
dfx canister --network ic stop icrc151
dfx deploy --network ic icrc151
```

### Candid Mismatch

```bash
# Regenerate Candid
cargo build --target wasm32-unknown-unknown --release
candid-extractor target/wasm32-unknown-unknown/release/icrc151.wasm > candid/icrc151.did
```

### Out of Cycles

```bash
# Top up immediately
dfx canister --network ic deposit-cycles 5000000000000 icrc151
```

### Canister Stuck

```bash
# Stop and restart
dfx canister --network ic stop icrc151
dfx canister --network ic start icrc151
```

### Upgrade Fails

```bash
# Check current status
dfx canister --network ic status icrc151

# Try forced reinstall (⚠️ LOSES STATE!)
# dfx deploy --network ic icrc151 --mode reinstall  # DO NOT USE unless you want to wipe data
```

---

## Security Checklist

Before mainnet deployment:

- [ ] Controller identity backed up
- [ ] Canister ID documented
- [ ] Pre/post upgrade hooks implemented
- [ ] Local testing completed
- [ ] Cycles topped up (recommend >1T for safety)
- [ ] Multi-sig considered for controller
- [ ] Code audited
- [ ] Transaction deduplication tested
- [ ] Time drift tolerance validated
- [ ] Fee validation tested

---

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Deploy ICRC-151

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install DFX
        run: DFX_VERSION=0.29.1 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          target: wasm32-unknown-unknown

      - name: Build
        run: dfx build icrc151

      - name: Deploy (if main)
        if: github.ref == 'refs/heads/main'
        run: |
          echo "${{ secrets.DFX_IDENTITY }}" > identity.pem
          dfx identity import deployer identity.pem
          dfx identity use deployer
          dfx deploy --network ic icrc151
```

---

## Cost Analysis

### One-Time Costs
- Canister creation: ~1T cycles (~$1.30 USD)

### Recurring Costs
- Stable memory: 127M cycles/GB/month
- Compute: Negligible for ledger operations
- Network: Negligible for typical usage

### Estimated Monthly Costs

| Usage | Storage | Cycles | USD |
|-------|---------|--------|-----|
| Low (10 TPS) | 10 GB/year | 127M/month | $0.17 |
| Medium (50 TPS) | 50 GB/year | 635M/month | $0.85 |
| High (100 TPS) | 100 GB/year | 1.27B/month | $1.70 |

**Recommendation:** Start with 5T cycles (~$6.50) for multi-month runway.
