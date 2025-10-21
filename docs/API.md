# API Reference

Complete reference for all ICRC-151 ledger methods.

## Token Management (Controller Only)

### create_token

Creates a new token in the ledger. Only callable by the controller.

```candid
create_token : (
  name: text,
  symbol: text,
  decimals: nat8,
  total_supply: opt nat,
  fee: opt nat,
  logo: opt text,
  description: opt text
) -> (variant { Ok: blob; Err: text })
```

**Parameters:**
- `name` - Token name (e.g., "Wrapped SOL")
- `symbol` - Token symbol (e.g., "ckSOL")
- `decimals` - Number of decimal places (e.g., 9)
- `total_supply` - Optional initial supply (defaults to 0)
- `fee` - Optional transfer fee (defaults to 10_000)
- `logo` - Optional logo URL
- `description` - Optional token description

**Returns:**
- `Ok(token_id)` - 32-byte token identifier (SHA-256 hash of metadata)
- `Err(message)` - Error message

**Example:**
```bash
dfx canister call icrc151 create_token '(
  "Wrapped SOL",
  "ckSOL",
  9:nat8,
  opt (1_000_000_000:nat),
  opt (10_000:nat),
  opt "https://solana.com/logo.png",
  opt "Wrapped Solana on IC"
)'
```

---

### mint_tokens

Mints new tokens to an account. Only callable by the controller.

```candid
mint_tokens : (
  token_id: blob,
  to: Account,
  amount: nat,
  memo: opt blob
) -> (variant { Ok: nat64; Err: text })
```

**Parameters:**
- `token_id` - Token identifier from create_token
- `to` - Recipient account
- `amount` - Amount to mint (in smallest units)
- `memo` - Optional memo (max 32 bytes)

**Returns:**
- `Ok(tx_id)` - Transaction ID
- `Err(message)` - Error message

**Example:**
```bash
dfx canister call icrc151 mint_tokens '(
  blob "\ab\cd\ef...",
  record {
    owner = principal "xxxxx-xxxxx";
    subaccount = null;
  },
  1_000_000_000:nat,
  null
)'
```

---

### burn_tokens

Burns tokens from the caller's account. Only callable by the controller.

```candid
burn_tokens : (
  token_id: blob,
  amount: nat,
  memo: opt blob
) -> (variant { Ok: nat64; Err: text })
```

**Parameters:**
- `token_id` - Token identifier
- `amount` - Amount to burn
- `memo` - Optional memo

**Returns:**
- `Ok(tx_id)` - Transaction ID
- `Err(message)` - Error message

---

## ICRC-1 Transfer Operations

### icrc151_transfer

Transfers tokens from caller to another account.

```candid
icrc151_transfer : (Icrc151TransferArgs) -> (TransferResult)

type Icrc151TransferArgs = record {
  token_id: blob;
  from_subaccount: opt blob;
  to: Account;
  amount: nat;
  fee: opt nat;
  memo: opt blob;
  created_at_time: opt nat64;
}

type TransferResult = variant {
  Ok: nat64;
  Err: TransferError;
}

type TransferError = variant {
  BadFee: record { expected_fee: nat };
  BadBurn: record { min_burn_amount: nat };
  InsufficientFunds: record { balance: nat };
  TooOld;
  CreatedInFuture: record { ledger_time: nat64 };
  Duplicate: record { duplicate_of: nat64 };
  TemporarilyUnavailable;
  GenericError: record { error_code: nat; message: text };
}
```

**Validations:**
- Fee must match token's configured fee
- Caller must have sufficient balance
- `created_at_time` must be within 5 minutes of ledger time
- Deduplication window: 24 hours

**Example:**
```bash
dfx canister call icrc151 icrc151_transfer '(
  record {
    token_id = blob "\ab\cd\ef...";
    from_subaccount = null;
    to = record {
      owner = principal "xxxxx-xxxxx";
      subaccount = null;
    };
    amount = 1_000_000:nat;
    fee = opt (10_000:nat);
    memo = null;
    created_at_time = null;
  }
)'
```

---

## ICRC-2 Allowance Operations

### icrc151_approve

Approves a spender to transfer tokens on behalf of the caller.

```candid
icrc151_approve : (Icrc151ApproveArgs) -> (ApproveResult)

type Icrc151ApproveArgs = record {
  token_id: blob;
  spender: Account;
  amount: nat;
  expires_at: opt nat64;
  expected_allowance: opt nat;
  memo: opt blob;
  fee: opt nat;
  from_subaccount: opt blob;
  created_at_time: opt nat64;
}

type ApproveResult = variant {
  Ok: nat64;
  Err: ApproveError;
}

type ApproveError = variant {
  BadFee: record { expected_fee: nat };
  InsufficientFunds: record { balance: nat };
  AllowanceChanged: record { current_allowance: nat };
  Expired: record { ledger_time: nat64 };
  TooOld;
  CreatedInFuture: record { ledger_time: nat64 };
  Duplicate: record { duplicate_of: nat64 };
  TemporarilyUnavailable;
  GenericError: record { error_code: nat; message: text };
}
```

**Example:**
```bash
dfx canister call icrc151 icrc151_approve '(
  record {
    token_id = blob "\ab\cd\ef...";
    spender = record {
      owner = principal "xxxxx-xxxxx";
      subaccount = null;
    };
    amount = 5_000_000:nat;
    expires_at = null;
    expected_allowance = null;
    memo = null;
    fee = opt (10_000:nat);
    from_subaccount = null;
    created_at_time = null;
  }
)'
```

---

### icrc151_transfer_from

Transfers tokens using an allowance.

```candid
icrc151_transfer_from : (Icrc151TransferFromArgs) -> (TransferResult)

type Icrc151TransferFromArgs = record {
  token_id: blob;
  spender_subaccount: opt blob;
  from: Account;
  to: Account;
  amount: nat;
  fee: opt nat;
  memo: opt blob;
  created_at_time: opt nat64;
}
```

**Validations:**
- Caller must have sufficient allowance
- From account must have sufficient balance
- Allowance must not be expired

---

---

### set_token_fee

Updates the transfer fee for a specific token. Only callable by controller.

```candid
set_token_fee : (token_id: blob, new_fee: nat) -> (variant { Ok; Err: text })
```

**Parameters:**
- `token_id` - Token identifier
- `new_fee` - New fee amount in smallest units

**Returns:**
- `Ok` - Fee updated successfully
- `Err(message)` - Error message (e.g., "Token not found")

**Example:**
```bash
dfx canister call icrc151 set_token_fee '(
  blob "\ab\cd\ef...",
  20_000:nat
)'
```

**Use Case:** Adjust fees based on network conditions or token economics without redeploying.

---

### set_controller

Sets the primary controller. Only callable by an existing controller.

```candid
set_controller : (principal) -> (variant { Ok; Err: text })
```

**Parameters:**
- `principal` - New controller principal

**Returns:**
- `Ok` - Controller updated successfully
- `Err(message)` - Error message

**Note:** This also adds the principal to the controllers set.

---

### add_controller

Adds a new controller principal. Only callable by an existing controller.

```candid
add_controller : (principal) -> (variant { Ok; Err: text })
```

**Parameters:**
- `principal` - Principal to add as controller

**Returns:**
- `Ok` - Controller added successfully
- `Err(message)` - Error message

**Example:**
```bash
dfx canister call icrc151 add_controller '(principal "xxxxx-xxxxx")'
```

---

### remove_controller

Removes a controller principal. Cannot remove the last controller.

```candid
remove_controller : (principal) -> (variant { Ok; Err: text })
```

**Parameters:**
- `principal` - Principal to remove from controllers

**Returns:**
- `Ok` - Controller removed successfully
- `Err(message)` - Error message (e.g., "Cannot remove the last controller")

---

### list_controllers

Lists all controller principals.

```candid
list_controllers : () -> (vec principal) query
```

**Returns:**
- Vector of all controller principals

**Example:**
```bash
dfx canister call icrc151 list_controllers '()'
```

---

## Query Methods

### get_balance

Returns the balance of an account for a specific token.

```candid
get_balance : (token_id: blob, account: Account) -> (variant { Ok: nat; Err: QueryError }) query
```

---

### get_total_supply

Returns the total supply of a token.

```candid
get_total_supply : (token_id: blob) -> (variant { Ok: nat; Err: QueryError }) query
```

---

### get_holder_count

Returns the number of accounts with non-zero balance for a specific token.

```candid
get_holder_count : (token_id: blob) -> (variant { Ok: nat64; Err: QueryError }) query
```

**Parameters:**
- `token_id` - Token identifier

**Returns:**
- `Ok(count)` - Number of unique holders with balance > 0
- `Err(QueryError)` - Token not found or invalid input

**Example:**
```bash
dfx canister call icrc151 get_holder_count '(blob "\ab\cd\ef...")'
```

**Use Case:** Track token distribution metrics and monitor adoption.

---

### get_token_metadata

Returns metadata for a token.

```candid
get_token_metadata : (token_id: blob) -> (variant { Ok: TokenMetadata; Err: QueryError }) query

type TokenMetadata = record {
  name: text;
  symbol: text;
  decimals: nat8;
  total_supply: nat;
  fee: nat;
  logo: opt text;
  description: opt text;
}
```

---

### list_tokens

Returns all registered token IDs.

```candid
list_tokens : () -> (vec blob) query
```

**Returns:**
- Vector of all token IDs (32-byte blobs)

**Example:**
```bash
dfx canister call icrc151 list_tokens '()'
```

---

### get_balances_for

Returns non-zero balances for a principal across all tokens.

```candid
get_balances_for : (owner: principal, subaccount: opt blob) -> (vec TokenBalance) query

type TokenBalance = record {
  token_id: blob;
  balance: nat;
}
```

**Parameters:**
- `owner` - Principal to query
- `subaccount` - Optional 32-byte subaccount (null for default account)

**Returns:**
- Vector of TokenBalance records (only includes tokens with balance > 0)

**Example:**
```bash
dfx canister call icrc151 get_balances_for '(principal "xxxxx-xxxxx", null)'
```

---

### get_storage_stats

Returns storage usage statistics for monitoring.

```candid
get_storage_stats : () -> (StorageStats) query

type StorageStats = record {
  transaction_log_size: nat64;
  dedup_map_size: nat64;
  allowance_expiry_size: nat64;
  extended_memos_size: nat64;
  holder_counts_size: nat64;
  token_count: nat64;
  estimated_memory_bytes: nat64;
}
```

**Returns:**
- `transaction_log_size` - Number of transactions in the log
- `dedup_map_size` - Number of deduplication entries
- `allowance_expiry_size` - Number of allowance expiry entries
- `extended_memos_size` - Number of extended memo entries
- `holder_counts_size` - Number of holder count entries
- `token_count` - Total number of registered tokens
- `estimated_memory_bytes` - Estimated total memory usage in bytes

**Example:**
```bash
dfx canister call icrc151 get_storage_stats '()'
```

**Use Case:** Monitor storage growth to plan cleanup/archiving before hitting memory limits.

---

### get_allowance

Returns the allowance amount.

```candid
get_allowance : (token_id: blob, owner: Account, spender: Account) -> (variant { Ok: nat; Err: QueryError }) query
```

---

### get_allowance_details

Returns full allowance details including expiration.

```candid
get_allowance_details : (token_id: blob, owner: Account, spender: Account) -> (variant { Ok: Allowance; Err: QueryError }) query

type Allowance = record {
  owner: Account;
  spender: Account;
  allowance: nat;
  expires_at: opt nat64;
}
```

---

### get_transactions

Returns transaction history with optional filtering.

```candid
get_transactions : (
  token_id: opt blob,
  start: opt nat64,
  limit: opt nat64
) -> (variant { Ok: vec StoredTxV1; Err: QueryError }) query
```

**Parameters:**
- `token_id` - Optional filter by token
- `start` - Starting transaction ID (default: 0)
- `limit` - Max transactions to return (default: 100, max: 1000)

---

### get_transaction_count

Returns total number of transactions.

```candid
get_transaction_count : () -> (nat64) query
```

---

### health_check

Returns "OK" if canister is healthy.

```candid
health_check : () -> (text) query
```

---

### get_info

Returns canister information.

```candid
get_info : () -> (CanisterInfo) query

type CanisterInfo = record {
  name: text;
  version: text;
  controller: text;
  transaction_count: nat64;
  global_tx_count: nat64;
}
```

## Type Definitions

### Account

```candid
type Account = record {
  owner: principal;
  subaccount: opt blob;  // 32 bytes
}
```

### QueryError

```candid
type QueryError = variant {
  TokenNotFound;
  InvalidInput: text;
  InternalError: text;
}
```
