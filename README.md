# ICRC-151 Multi-Token Ledger

Production-ready implementation of the ICRC-151 multi-token ledger standard for the Internet Computer.

## Overview

ICRC-151 is a multi-token ledger that allows a single canister to manage multiple fungible tokens, each with its own supply, metadata, and balances. This implementation is optimized for cross-chain bridge use cases where each blockchain ecosystem gets its own ledger instance.

**Key Features:**
- Multiple tokens in a single ledger
- ICRC-1-like transfer operations
- ICRC-2-like approve/transfer_from operations
- Persistent storage using IC stable memory
- Controller-only token creation and minting
- Complete transaction history
- Deduplication and replay protection
- Fixed 256-byte transaction format for predictable storage

## Architecture

### Use Case: Cross-Chain Bridge

This ledger is designed for cross-chain bridge architectures where:

```
Solana → ckSOL Minter → ICRC-151 Ledger (Solana tokens)
Aptos  → ckAPT Minter → ICRC-151 Ledger (Aptos tokens)
```

Each blockchain ecosystem gets its own ledger instance. The minter canister acts as the controller and handles all bridge-specific logic (multi-sig, relayer authorization, replay prevention, etc.).

### Components

```
src/
├── lib.rs          - Canister entry point and initialization
├── types.rs        - Core type definitions (Account, TokenId, etc.)
├── state.rs        - Stable memory state management
├── transaction.rs  - Transaction storage (StoredTxV1)
├── operations.rs   - Transfer, mint, burn operations
├── allowances.rs   - ICRC-2 approve/transfer_from
├── queries.rs      - Balance and metadata queries
└── validation.rs   - Input validation and deduplication
```

## Documentation

- [API Reference](./docs/API.md) - Complete API documentation
- [Storage](./docs/STORAGE.md) - Memory layout and data structures
- [Operations](./docs//OPERATIONS.md) - Detailed operation workflows
- [Deployment](./docs//DEPLOYMENT.md) - Build and deployment guide
- [Future: ICRC-3 Support] - ICRC-3 implementation - on roadmap

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WebAssembly target
rustup target add wasm32-unknown-unknown

# Install dfx
DFX_VERSION=0.29.1 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"

# Install candid-extractor (for Candid generation)
cargo install candid-extractor

# Install ic-wasm (optional, for WASM optimization)
cargo install ic-wasm
```

### Local Deployment

```bash
# Start local IC replica
dfx start --background --clean

# Deploy canister
dfx deploy icrc151

# Generate TypeScript declarations
dfx generate icrc151
```

### Create and Mint Token

```bash
# Get your principal
dfx identity get-principal

# Create a token (controller only)
dfx canister call icrc151 create_token '(
  "Wrapped SOL",
  "ckSOL",
  9:nat8,
  opt (1_000_000:nat),
  opt (10_000:nat),
  opt "https://solana.com/logo.png",
  opt "Wrapped Solana token on IC"
)'

# Returns: (variant { Ok = blob "\00\01\02..." })
# Save this token_id for next steps

# Mint tokens to an account
dfx canister call icrc151 mint_tokens '(
  blob "\00\01\02...",
  record {
    owner = principal "xxxxx-xxxxx-xxxxx-xxxxx-xxx";
    subaccount = null;
  },
  1_000_000_000:nat,
  null
)'
```

## Memory Limits

- **Maximum storage**: 400 GB stable memory
- **Transactions per canister**: ~1.37 billion (256 bytes each)
- **Timeline at 100 TPS**: 4 months before archiving needed
- **Timeline at 10 TPS**: 3+ years before archiving needed
- **Recommendation**: Implement archiving at 50% memory usage (300 GB)

## Standards Compliance

- ✅ ICRC-1: Basic transfer operations
- ✅ ICRC-2: Approve and transfer_from
- ⏳ ICRC-3: Transaction history and archiving (future)

## License

MIT
