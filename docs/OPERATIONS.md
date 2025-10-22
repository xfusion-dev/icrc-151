# Operations Guide

Detailed workflows for all ledger operations.

## Transfer Operation

### Flow

```
User → transfer() → Validate → Update Balances → Record Tx → Return TxId
```

### Steps

1. **Validation** (`src/validation.rs`)
   - Check token exists
   - Verify fee matches token's configured fee
   - Check `created_at_time` within 5 minutes of ledger time
   - Check for duplicate transaction (24-hour window)
   - Verify sufficient balance (amount + fee)

2. **Balance Updates** (`src/operations.rs`)
   - Deduct `amount + fee` from sender
   - Add `amount` to recipient
   - Fee is not credited to anyone (burned)

3. **Transaction Recording**
   - Create `StoredTxV1` with op=0 (Transfer)
   - Append to transaction log
   - Store deduplication hash
   - Increment global transaction counter

4. **Return**
   - `Ok(tx_id)` on success
   - `Err(TransferError)` on failure

### Error Handling

| Error | Condition |
|-------|-----------|
| `BadFee` | Provided fee doesn't match token fee |
| `InsufficientFunds` | Balance < amount + fee |
| `TooOld` | created_at_time > 24 hours ago |
| `CreatedInFuture` | created_at_time > ledger_time + 5 min |
| `Duplicate` | Same tx submitted within 24 hours |

---

## Mint Operation

### Flow

```
Controller → mint_tokens() → Check Auth → Update Balance → Record Tx → Return TxId
```

### Steps

1. **Authorization**
   - Verify caller is controller
   - Return error if not authorized

2. **Validation**
   - Check token exists
   - Validate amount > 0
   - Check memo length ≤ 32 bytes

3. **State Updates**
   - Add `amount` to recipient balance
   - Add `amount` to token total_supply
   - No fee charged for minting

4. **Transaction Recording**
   - Create `StoredTxV1` with op=1 (Mint)
   - from_key = all zeros (minting source)
   - No deduplication for mint operations

### Security

- **Only controller can mint**
- No supply cap enforcement (bridge use case requires unlimited minting)
- Minting does not charge fees
- Events logged for audit trail

---

## Burn Operation

### Flow

```
Controller → burn_tokens() → Check Auth → Update Balance → Record Tx → Return TxId
```

### Steps

1. **Authorization**
   - Verify caller is controller

2. **Validation**
   - Check token exists
   - Verify controller has sufficient balance
   - Validate amount > 0

3. **State Updates**
   - Deduct `amount` from controller balance
   - Deduct `amount` from token total_supply
   - No fee charged for burning

4. **Transaction Recording**
   - Create `StoredTxV1` with op=2 (Burn)
   - to_key = all zeros (burning destination)
   - No deduplication for burn operations

---

## Approve Operation

### Flow

```
User → approve() → Validate → Update Allowance → Record Tx → Return TxId
```

### Steps

1. **Validation**
   - Check token exists
   - Verify fee matches token fee
   - Check `created_at_time` within window
   - Check for duplicate
   - If `expected_allowance` provided, verify current allowance matches

2. **Allowance Update**
   - Set allowance for (owner, spender) = amount
   - Store expiration time if provided
   - Deduct fee from owner balance

3. **Transaction Recording**
   - Create `StoredTxV1` with op=3 (Approve)
   - from_key = owner
   - to_key = spender
   - spender_key = spender
   - amount = allowance amount

### Security Features

- **Expected Allowance**: Prevents race conditions
  - If current allowance ≠ expected_allowance → reject
  - Protects against double-spend attacks

- **Expiration**: Time-limited allowances
  - expires_at is optional
  - Checked during transfer_from

---

## TransferFrom Operation

### Flow

```
Spender → transfer_from() → Validate → Check Allowance → Update Balances → Update Allowance → Record Tx → Return TxId
```

### Steps

1. **Validation**
   - Check token exists
   - Verify fee matches
   - Check timestamps
   - Check for duplicates

2. **Allowance Check**
   - Verify allowance exists
   - Check allowance ≥ amount + fee
   - Check not expired

3. **Balance Updates**
   - Deduct amount + fee from `from` account
   - Add amount to `to` account
   - Fee is burned

4. **Allowance Update**
   - Deduct amount + fee from allowance
   - If allowance becomes 0, optionally remove entry

5. **Transaction Recording**
   - Create `StoredTxV1` with op=4 (TransferFrom)
   - from_key = from account
   - to_key = to account
   - spender_key = caller (spender)

### Error Cases

| Error | Condition |
|-------|-----------|
| `InsufficientFunds` | From account balance too low |
| `InsufficientAllowance` | Allowance < amount + fee |
| `AllowanceExpired` | expires_at < current_time |
| `AllowanceChanged` | Concurrent modification detected |

---

## Token Creation

### Flow

```
Controller → create_token() → Check Auth → Generate TokenId → Store Metadata → Return TokenId
```

### Steps

1. **Authorization**
   - Verify caller is controller

2. **Token ID Generation**
   ```rust
   token_id = SHA256(name || symbol || decimals)
   ```
   - Deterministic: same metadata = same token_id
   - Prevents duplicate tokens

3. **Metadata Storage**
   ```rust
   TokenMetadata {
       name,
       symbol,
       decimals,
       total_supply: 0,  // or provided value
       fee: fee.unwrap_or(10_000),
       logo,
       description,
   }
   ```

4. **Initialization**
   - If `total_supply` > 0, credit to controller
   - No transaction recorded for initial supply

### Idempotency

Creating the same token twice:
- Same (name, symbol, decimals) → Same token_id
- Attempt to insert will fail if token_id already exists
- Returns error: "Token already exists"

---

## Deduplication

### Purpose

Prevents processing the same transaction multiple times due to:
- Network retries
- Client retries
- Replay attacks

### Mechanism

1. **Hash Calculation**
   ```rust
   hash = SHA256(
       token_id ||
       from ||
       to ||
       amount ||
       memo ||
       created_at_time
   )
   ```

2. **Deduplication Check**
   - Look up hash in `TX_DEDUP` map
   - If found and < 24 hours old → Return `Duplicate` error with original tx_id
   - If found and > 24 hours old → Remove old entry, process as new

3. **Storage**
   - Store hash → tx_id mapping
   - Cleaned up automatically on subsequent transactions

### Window

- **24 hours** deduplication window
- Balances need for safety vs storage overhead
- Configurable in future versions

---

## Query Operations

All query operations are **read-only** and don't modify state.

### get_balance

```rust
fn get_balance(token_id, account) -> Result<u128> {
    let key = make_balance_key(token_id, account);
    BALANCES.get(&key).unwrap_or(0)
}
```

- Returns 0 for non-existent accounts
- No error on missing balance entry

### get_transactions

```rust
fn get_transactions(token_id, start, limit) -> Result<Vec<StoredTxV1>> {
    let start = start.unwrap_or(0);
    let limit = min(limit.unwrap_or(100), 1000);

    let txs = TX_LOG.iter()
        .skip(start)
        .take(limit)
        .filter(|tx| token_id.is_none() || tx.token_id == token_id.unwrap())
        .collect();

    Ok(txs)
}
```

- Default limit: 100
- Maximum limit: 1000
- Optionally filter by token_id

---

## State Consistency

### Invariants

These must **always** be true:

1. **Supply Conservation**
   ```
   token.total_supply = SUM(all balances for token)
   ```

2. **Transaction Log Integrity**
   ```
   tx_count = TX_LOG.len()
   ```

3. **No Negative Balances**
   ```
   ALL balances ≥ 0
   ```

4. **Controller Exists**
   ```
   CONTROLLER.get().is_some()
   ```

### Validation

Add pre-upgrade validation:
```rust
#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    // Verify all invariants
    verify_supply_conservation();
    verify_no_negative_balances();
    verify_transaction_integrity();
}
```

---

## Error Recovery

### Failed Transactions

If a transaction fails:
1. State is NOT modified (atomic operations)
2. Error returned to caller
3. No transaction recorded
4. No deduplication entry created

### Partial Failures

Not possible - operations are atomic:
- All balance updates succeed or none
- Transaction log append is atomic
- Deduplication insert is atomic

### Stuck Transactions

If canister traps mid-operation:
- Stable memory state is consistent (last committed state)
- Caller receives error and should retry
- Deduplication prevents double-processing on retry
