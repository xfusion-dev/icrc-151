# Storage Architecture

Complete overview of the stable memory layout and data structures.

## Stable Memory Layout

All data is stored in IC stable memory, which persists across canister upgrades. Total capacity: **400 GB**.

### Memory IDs

Memory is organized into separate regions identified by Memory IDs (0-255). Each ID corresponds to a specific data structure.

```rust
// Active Memory IDs
const MEMORY_ID_TOKENS: u8 = 0;              // Token metadata
const MEMORY_ID_BALANCES: u8 = 1;            // Account balances
const MEMORY_ID_CONTROLLER: u8 = 2;          // Controller principal
const MEMORY_ID_TX_LOG: u8 = 3;              // Transaction log
const MEMORY_ID_TX_DEDUP: u8 = 6;            // Deduplication hashes
const MEMORY_ID_TX_COUNT: u8 = 9;            // Global transaction counter
const MEMORY_ID_ALLOWANCES: u8 = 10;         // ICRC-2 allowances
const MEMORY_ID_ALLOWANCE_DEDUP: u8 = 12;    // Allowance dedup hashes

// Reserved for Future Use
const MEMORY_ID_ARCHIVE_INFO: u8 = 4;        // Archive canister info
const MEMORY_ID_ARCHIVE_METADATA: u8 = 5;    // Archive metadata
const MEMORY_ID_METADATA: u8 = 7;            // Canister metadata
const MEMORY_ID_SETTINGS: u8 = 8;            // Canister settings
const MEMORY_ID_TOKEN_INDEX: u8 = 11;        // Token lookup index
```

**IMPORTANT:** Memory IDs are **permanent**. Never change existing IDs. New features can use IDs 13-255.

---

## Data Structures

### 1. Token Metadata (Memory ID: 0)

**Structure:** `StableBTreeMap<TokenId, TokenMetadata>`

```rust
type TokenId = Vec<u8>;  // 32-byte SHA-256 hash

struct TokenMetadata {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u128,
    fee: u128,
    logo: Option<String>,
    description: Option<String>,
}
```

**Key:** SHA-256 hash of `(name, symbol, decimals)`
**Size:** Variable (typically 100-500 bytes per token)

---

### 2. Account Balances (Memory ID: 1)

**Structure:** `StableBTreeMap<(TokenId, AccountKey), u128>`

```rust
type AccountKey = Vec<u8>;  // 64-byte serialized Account

struct Account {
    owner: Principal,
    subaccount: Option<[u8; 32]>,
}
```

**Key:** `(token_id: [u8; 32], account_key: [u8; 64])`
**Value:** Balance as u128
**Size:** 96 bytes key + 16 bytes value = **112 bytes per balance entry**

**Account Key Format (64 bytes):**
```
[32 bytes: owner principal (padded)] [32 bytes: subaccount or zeros]
```

---

### 3. Controller (Memory ID: 2)

**Structure:** `StableCell<Principal>`

Stores the controller principal that can create tokens and mint/burn.

**Size:** ~32 bytes

---

### 4. Transaction Log (Memory ID: 3)

**Structure:** `StableLog<StoredTxV1>`

Append-only log storing all transactions in chronological order.

```rust
struct StoredTxV1 {
    op: u8,              // Operation type (0=Transfer, 1=Mint, 2=Burn, 3=Approve, 4=TransferFrom)
    flags: u8,           // Feature flags (currently unused, reserved)
    token_id: [u8; 32],  // Token identifier
    from_key: [u8; 64],  // Sender account key
    to_key: [u8; 64],    // Recipient account key
    spender_key: [u8; 64], // Spender key (for approve/transfer_from)
    amount: [u8; 16],    // Amount (u128 big-endian)
    fee: [u8; 16],       // Fee (u128 big-endian)
    timestamp: [u8; 8],  // Timestamp (u64 big-endian)
    memo: [u8; 32],      // Memo (padded or truncated)
    _reserved: [u8; 16], // Reserved for future use
}
```

**Size:** Fixed **256 bytes per transaction**

**Operation Types:**
- `0` - Transfer
- `1` - Mint
- `2` - Burn
- `3` - Approve
- `4` - TransferFrom

**Capacity Calculation:**
```
400 GB / 256 bytes = 1,562,500,000 transactions (1.56 billion)
```

**Practical Limit (with other data):**
```
~300 GB for transactions = 1,171,875,000 transactions (1.17 billion)
```

---

### 5. Transaction Deduplication (Memory ID: 6)

**Structure:** `StableBTreeMap<TxHash, u64>`

Stores transaction hashes to prevent duplicates within 24-hour window.

```rust
type TxHash = [u8; 32];  // SHA-256 of transaction parameters
type TxId = u64;         // Transaction ID that processed this hash
```

**Hash Input:**
```
SHA-256(token_id || from || to || amount || memo || created_at_time)
```

**Cleanup:** Entries older than 24 hours are removed during validation.

**Size:** 32 bytes key + 8 bytes value = **40 bytes per entry**

---

### 6. Transaction Counter (Memory ID: 9)

**Structure:** `StableCell<u64>`

Global counter for transaction IDs. Incremented on each transaction.

**Size:** 8 bytes

---

### 7. Allowances (Memory ID: 10)

**Structure:** `StableBTreeMap<AllowanceKey, AllowanceValue>`

```rust
struct AllowanceKey {
    token_id: [u8; 32],
    owner: [u8; 64],
    spender: [u8; 64],
}

struct AllowanceValue {
    allowance: u128,
    expires_at: Option<u64>,
}
```

**Key Size:** 32 + 64 + 64 = **160 bytes**
**Value Size:** 16 + 9 = **25 bytes**
**Total:** **185 bytes per allowance**

---

### 8. Allowance Deduplication (Memory ID: 12)

**Structure:** `StableBTreeMap<TxHash, u64>`

Similar to transaction deduplication but for approve operations.

**Size:** 40 bytes per entry

---

## Memory Usage Estimates

### Per Token
- Metadata: ~200 bytes
- Average 1,000 holders: 112 KB (balances)
- **Total per token: ~112 KB**

### Per Transaction
- Transaction log: 256 bytes
- Deduplication (temporary): 40 bytes
- **Total: 296 bytes (dedup cleaned after 24h)**

### Growth Scenarios

| Scenario | Tokens | Holders/Token | Tx Rate | Time to 300 GB |
|----------|--------|---------------|---------|----------------|
| Bridge - Low | 100 | 10,000 | 10 TPS | 3+ years |
| Bridge - Med | 500 | 50,000 | 50 TPS | 8 months |
| Bridge - High | 1,000 | 100,000 | 100 TPS | 4 months |

**Recommendation:** Implement ICRC-3 archiving when reaching 50% capacity (200 GB).

---

## Upgrade Safety

### ✅ Persists Across Upgrades
- All stable memory data (tokens, balances, transactions, allowances)
- Controller principal
- Transaction counter

### ❌ Does NOT Persist (Not in Stable Memory)
- None - everything critical is in stable memory

### Pre/Post Upgrade Hooks

Currently not implemented. Can be added for:
- Data validation
- Migration logic
- Version compatibility checks

```rust
#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    // Validate state before upgrade
    // All data already in stable memory
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    // Validate state after upgrade
    // Optionally run migrations
}
```

---

## Adding New Memory Regions

When adding new features that need persistent storage:

1. Choose unused Memory ID (13-255)
2. Document it in `src/types.rs`
3. Initialize in `src/state.rs`
4. **Never change existing Memory IDs**

Example:
```rust
// Add to types.rs
const MEMORY_ID_NEW_FEATURE: u8 = 13;

// Add to state.rs
thread_local! {
    static NEW_FEATURE_STORAGE: RefCell<StableBTreeMap<K, V, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(13)))
        ));
}
```

---

## Limitations

- **Maximum canister stable memory:** 400 GB
- **Maximum Memory IDs:** 255
- **StableBTreeMap key size:** No hard limit, but smaller is better for performance
- **StableLog:** Append-only, cannot delete individual entries
- **Transaction size:** Fixed 256 bytes (cannot change without breaking compatibility)

---

## Future Considerations

### ICRC-3 Archive Support

When implementing ICRC-3:
- Use Memory IDs 4, 5 for archive metadata
- Transactions remain in Memory ID 3 until archived
- Old transactions moved to separate archive canisters
- Main ledger stores pointers to archive canisters

### StoredTxV2

If block hash + parent hash are needed (ICRC-3 requirement):
- Would increase transaction size to 320 bytes (256 + 64 for hashes)
- **Breaking change** - requires data migration
- Reduces capacity by 20%
- Recommended to defer until archiving is actually needed
