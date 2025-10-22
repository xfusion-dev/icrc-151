# ICRC-151 Candid Interface Reference

Complete reference for all public canister methods.

## Token Management (Controller Only)

### create_token

Creates a new token. Only callable by controller.

```candid
create_token : (
  name: text,
  symbol: text,
  decimals: nat8,
  total_supply: opt nat,
  fee: opt nat,
  logo: opt text,
  description: opt text
) -> variant { Ok: blob; Err: text }
```

**Parameters:**
- `name` - Token name (e.g., "Wrapped Solana")
- `symbol` - Token symbol (e.g., "ckSOL")
- `decimals` - Decimal places (typically 8 or 9)
- `total_supply` - Initial supply (optional, default: 0)
- `fee` - Transfer fee in smallest units (optional, default: 10_000)
- `logo` - Logo URL (optional)
- `description` - Token description (optional)

**Returns:**
- `Ok(token_id)` - 32-byte token identifier (blob)
- `Err(message)` - Error description

**Example:**
```bash
dfx canister call icrc151 create_token '(
  "Wrapped Solana",
  "ckSOL",
  9:nat8,
  opt (0:nat),
  opt (10_000:nat),
  opt "https://solana.com/logo.png",
  opt "Bridged Solana token on Internet Computer"
)'
```

**Response:**
```
(variant { Ok = blob "\ab\cd\ef..." })
```

---

### mint_tokens

Mints tokens to an account. Only callable by controller.

```candid
mint_tokens : (
  token_id: blob,
  to: Account,
  amount: nat,
  memo: opt blob
) -> variant { Ok: nat64; Err: text }

type Account = record {
  owner: principal;
  subaccount: opt blob;
}
```

**Parameters:**
- `token_id` - Token identifier from create_token
- `to` - Recipient account (owner + optional 32-byte subaccount)
- `amount` - Amount in smallest units (respects decimals)
- `memo` - Optional memo (max 32 bytes)

**Returns:**
- `Ok(tx_id)` - Transaction ID
- `Err(message)` - Error description

**Example:**
```bash
dfx canister call icrc151 mint_tokens '(
  blob "\ab\cd\ef\00\11\22\33\44\55\66\77\88\99\aa\bb\cc\dd\ee\ff\00\11\22\33\44\55\66\77\88\99\aa\bb\cc\dd\ee\ff",
  record {
    owner = principal "aaaaa-aa";
    subaccount = null;
  },
  1_000_000_000:nat,
  null
)'
```

**Response:**
```
(variant { Ok = 0 : nat64 })
```

---

### burn_tokens

Burns tokens from the caller's account. Anyone can burn their own tokens.

```candid
burn_tokens : (
  token_id: blob,
  amount: nat,
  memo: opt blob
) -> variant { Ok: nat64; Err: text }
```

**Parameters:**
- `token_id` - Token identifier
- `amount` - Amount to burn from caller's default account (no subaccount)
- `memo` - Optional memo (max 32 bytes for deduplication, larger stored separately)

**Returns:**
- `Ok(tx_id)` - Transaction ID
- `Err(message)` - Error description

**Example:**
```bash
dfx canister call icrc151 burn_tokens '(
  blob "\ab\cd\ef...",
  500_000_000:nat,
  null
)'
```

**Note:** This burns from the caller's default account only (no subaccount).

---

### burn_tokens_from

Burns tokens from any specified account. Only callable by controller.

```candid
burn_tokens_from : (
  token_id: blob,
  from: Account,
  amount: nat,
  memo: opt blob
) -> variant { Ok: nat64; Err: text }

type Account = record {
  owner: principal;
  subaccount: opt blob;
}
```

**Parameters:**
- `token_id` - Token identifier
- `from` - Account to burn from (owner + optional 32-byte subaccount)
- `amount` - Amount to burn in smallest units
- `memo` - Optional memo (max 32 bytes)

**Returns:**
- `Ok(tx_id)` - Transaction ID
- `Err(message)` - Error description

**Example:**
```bash
dfx canister call icrc151 burn_tokens_from '(
  blob "\ab\cd\ef...",
  record {
    owner = principal "6ka7a-jlr7k-k6pg7-bfjej-ystuc-tpubs-f2kvx-ecdzu-fxavt-etpih-qqe";
    subaccount = null;
  },
  100_000_000:nat,
  null
)'
```

**Use Case:** Bridge unwrapping - when a user initiates a withdrawal to the source chain, the controller (minter) burns their wrapped tokens.

---

### set_token_fee

Updates the transfer fee for a specific token. Only callable by controller.

```candid
set_token_fee : (token_id: blob, new_fee: nat) -> variant { Ok; Err: text }
```

**Parameters:**
- `token_id` - Token identifier (32-byte blob)
- `new_fee` - New fee amount in smallest units

**Returns:**
- `Ok` - Fee updated successfully
- `Err(message)` - Error message

**Example:**
```bash
dfx canister call icrc151 set_token_fee '(
  blob "\ab\cd\ef\00\11\22\33\44\55\66\77\88\99\aa\bb\cc\dd\ee\ff\00\11\22\33\44\55\66\77\88\99\aa\bb\cc\dd\ee\ff",
  20_000:nat
)'
```

**Response:**
```
(variant { Ok })
```

**Error Response:**
```
(variant { Err = "Token not found" })
```

**Use Cases:**
- Adjust fees based on network congestion
- Change fee structure for promotional periods
- Respond to token value changes without redeploying canister

---

### set_controller

Sets the primary controller. Only callable by an existing controller.

```candid
set_controller : (principal) -> variant { Ok; Err: text }
```

**Example:**
```bash
dfx canister call icrc151 set_controller '(principal "xxxxx-xxxxx")'
```

**Note:** This also adds the principal to the controllers set.

---

### add_controller

Adds a new controller principal. Only callable by an existing controller.

```candid
add_controller : (principal) -> variant { Ok; Err: text }
```

**Example:**
```bash
dfx canister call icrc151 add_controller '(principal "xxxxx-xxxxx")'
```

---

### remove_controller

Removes a controller principal. Cannot remove the last controller.

```candid
remove_controller : (principal) -> variant { Ok; Err: text }
```

**Example:**
```bash
dfx canister call icrc151 remove_controller '(principal "xxxxx-xxxxx")'
```

**Response on error:**
```
(variant { Err = "Cannot remove the last controller" })
```

---

### list_controllers

Lists all controller principals.

```candid
list_controllers : () -> vec principal query
```

**Example:**
```bash
dfx canister call icrc151 list_controllers '()'
```

**Response:**
```
(vec { principal "xxxxx-xxxxx"; principal "yyyyy-yyyyy" })
```

---

## ICRC-1 Transfer Operations

### transfer

Transfers tokens from caller to another account.

```candid
transfer : (Icrc151TransferArgs) -> TransferResult

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

**Parameters:**
- `token_id` - Token to transfer
- `from_subaccount` - Sender's subaccount (optional)
- `to` - Recipient account
- `amount` - Amount to transfer (excluding fee)
- `fee` - Expected fee (must match token's fee)
- `memo` - Optional memo
- `created_at_time` - Timestamp for deduplication (optional)

**Example:**
```bash
dfx canister call icrc151 transfer '(
  record {
    token_id = blob "\ab\cd\ef...";
    from_subaccount = null;
    to = record {
      owner = principal "bbbbb-bb";
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

## ICRC-2 Allowance Operations

### approve

Approves a spender to transfer tokens on behalf of caller.

```candid
approve : (Icrc151ApproveArgs) -> ApproveResult

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
dfx canister call icrc151 approve '(
  record {
    token_id = blob "\ab\cd\ef...";
    spender = record {
      owner = principal "ccccc-cc";
      subaccount = null;
    };
    amount = 1_000_000:nat;
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

### transfer_from

Transfers tokens using an allowance.

```candid
transfer_from : (Icrc151TransferFromArgs) -> TransferResult

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

**Example:**
```bash
dfx canister call icrc151 transfer_from '(
  record {
    token_id = blob "\ab\cd\ef...";
    spender_subaccount = null;
    from = record {
      owner = principal "aaaaa-aa";
      subaccount = null;
    };
    to = record {
      owner = principal "ddddd-dd";
      subaccount = null;
    };
    amount = 50_000:nat;
    fee = opt (10_000:nat);
    memo = null;
    created_at_time = null;
  }
)'
```

---

## Query Methods

### get_balance

Returns balance for an account and token.

```candid
get_balance : (token_id: blob, account: Account) -> variant { Ok: nat; Err: QueryError } query

type QueryError = variant {
  TokenNotFound;
  InvalidInput: text;
  InternalError: text;
}
```

**Example:**
```bash
dfx canister call icrc151 get_balance '(
  blob "\ab\cd\ef...",
  record {
    owner = principal "aaaaa-aa";
    subaccount = null;
  }
)'
```

**Response:**
```
(variant { Ok = 1_000_000_000 : nat })
```

---

### list_tokens

Lists all token IDs registered in the ledger.

```candid
list_tokens : () -> vec blob query
```

**Example:**
```bash
dfx canister call icrc151 list_tokens '()'
```

---

### get_balances_for

Returns non-zero balances for a given owner/subaccount across all tokens.

```candid
get_balances_for : (owner: principal, subaccount: opt blob) -> vec record {
  token_id: blob;
  balance: nat;
} query
```

**Notes:**
- Returns only tokens where balance > 0 for compact responses.
- Pass `null` for `subaccount` to query the default account.

**Example:**
```bash
dfx canister call icrc151 get_balances_for '(
  principal "aaaaa-aa",
  null
)'
```

---

### get_total_supply

Returns total supply of a token.

```candid
get_total_supply : (token_id: blob) -> variant { Ok: nat; Err: QueryError } query
```

**Example:**
```bash
dfx canister call icrc151 get_total_supply '(blob "\ab\cd\ef...")'
```

---

### get_holder_count

Returns the number of unique accounts with non-zero balance for a specific token.

```candid
get_holder_count : (token_id: blob) -> variant { Ok: nat64; Err: QueryError } query
```

**Parameters:**
- `token_id` - Token identifier (32-byte blob)

**Returns:**
- `Ok(count)` - Number of accounts with balance > 0
- `Err(QueryError)` - Token not found or invalid input

**Example:**
```bash
dfx canister call icrc151 get_holder_count '(blob "\ab\cd\ef...")'
```

**Response:**
```
(variant { Ok = 1_234 : nat64 })
```

**Use Cases:**
- Track token distribution and adoption metrics
- Monitor holder growth over time
- Analytics dashboards for token statistics

---

### get_token_metadata

Returns metadata for a token.

```candid
get_token_metadata : (token_id: blob) -> variant { Ok: TokenMetadata; Err: QueryError } query

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

**Example:**
```bash
dfx canister call icrc151 get_token_metadata '(blob "\ab\cd\ef...")'
```

**Response:**
```
(
  variant {
    Ok = record {
      name = "Wrapped Solana";
      symbol = "ckSOL";
      decimals = 9 : nat8;
      total_supply = 1_000_000_000 : nat;
      fee = 10_000 : nat;
      logo = opt "https://solana.com/logo.png";
      description = opt "Bridged Solana token";
    }
  }
)
```

---

### get_allowance

Returns allowance amount.

```candid
get_allowance : (token_id: blob, owner: Account, spender: Account) -> variant { Ok: nat; Err: QueryError } query
```

**Example:**
```bash
dfx canister call icrc151 get_allowance '(
  blob "\ab\cd\ef...",
  record { owner = principal "aaaaa-aa"; subaccount = null },
  record { owner = principal "ccccc-cc"; subaccount = null }
)'
```

---

### get_allowance_details

Returns full allowance details including expiration.

```candid
get_allowance_details : (token_id: blob, owner: Account, spender: Account) -> variant { Ok: Allowance; Err: QueryError } query

type Allowance = record {
  owner: Account;
  spender: Account;
  allowance: nat;
  expires_at: opt nat64;
}
```

---

### get_transactions

Returns transaction history.

```candid
get_transactions : (
  token_id: opt blob,
  start: opt nat64,
  limit: opt nat64
) -> variant { Ok: vec StoredTxV1; Err: QueryError } query

type StoredTxV1 = record {
  op: nat8;
  fee: blob;
  flags: nat8;
  token_id: blob;
  memo: blob;
  spender_key: blob;
  to_key: blob;
  _reserved: blob;
  timestamp: blob;
  from_key: blob;
  amount: blob;
}
```

**Parameters:**
- `token_id` - Filter by token (optional, null = all tokens)
- `start` - Starting transaction ID (optional, default: 0)
- `limit` - Max transactions (optional, default: 100, max: 1000)

**Example:**
```bash
# Get all transactions
dfx canister call icrc151 get_transactions '(null, null, opt (100:nat64))'

# Get transactions for specific token
dfx canister call icrc151 get_transactions '(opt blob "\ab\cd\ef...", null, opt (50:nat64))'
```

---

### get_transaction_count

Returns total number of transactions.

```candid
get_transaction_count : () -> nat64 query
```

**Example:**
```bash
dfx canister call icrc151 get_transaction_count '()'
```

---

### health_check

Returns "OK" if canister is healthy.

```candid
health_check : () -> text query
```

**Example:**
```bash
dfx canister call icrc151 health_check '()'
```

---

### get_storage_stats

Returns storage usage statistics for monitoring.

```candid
get_storage_stats : () -> StorageStats query

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
- `transaction_log_size` - Number of transactions stored
- `dedup_map_size` - Number of deduplication entries (grows unbounded)
- `allowance_expiry_size` - Number of allowance expiry tracking entries
- `extended_memos_size` - Number of memos >32 bytes stored separately
- `holder_counts_size` - Number of holder count entries tracked
- `token_count` - Total tokens registered
- `estimated_memory_bytes` - Rough memory usage estimate

**Example:**
```bash
dfx canister call icrc151 get_storage_stats '()'
```

**Response:**
```
(
  record {
    transaction_log_size = 1_234 : nat64;
    dedup_map_size = 987 : nat64;
    allowance_expiry_size = 42 : nat64;
    extended_memos_size = 5 : nat64;
    token_count = 3 : nat64;
    estimated_memory_bytes = 321_000 : nat64;
  }
)
```

**Use Case:** Monitor storage growth. When `dedup_map_size` approaches 10M+ entries, consider implementing cleanup to prevent memory exhaustion.

---

### get_info

Returns canister information.

```candid
get_info : () -> CanisterInfo query

type CanisterInfo = record {
  name: text;
  version: text;
  controller: text;
  transaction_count: nat64;
  global_tx_count: nat64;
}
```

**Example:**
```bash
dfx canister call icrc151 get_info '()'
```

**Response:**
```
(
  record {
    name = "ICRC-151 Multi-Token Ledger";
    version = "0.1.0";
    controller = "xxxxx-xxxxx-xxxxx-xxxxx-xxx";
    transaction_count = 42 : nat64;
    global_tx_count = 42 : nat64;
  }
)
```

---

## Common Types

### Account

```candid
type Account = record {
  owner: principal;
  subaccount: opt blob;
}
```

**Notes:**
- `owner` - Principal of the account owner
- `subaccount` - Optional 32-byte subaccount identifier
- If `subaccount` is `null`, uses default account

### Token ID

```candid
type TokenId = blob;
```

**Notes:**
- Always 32 bytes
- Generated as SHA-256 hash of (name, symbol, decimals)
- Deterministic: same metadata = same token_id

### Timestamps

All timestamps are in **nanoseconds since epoch** (IC time).

**Validation:**
- `created_at_time` must be within ±5 minutes of ledger time (future)
- `created_at_time` must be within 10 minutes of ledger time (past)

### Memo

- Maximum size: 32 bytes (truncated if longer)
- Stored as-is in transaction log
- Optional on all operations

---

## Error Handling

### TransferError

- `BadFee` - Fee doesn't match token's configured fee
- `InsufficientFunds` - Balance too low for amount + fee
- `TooOld` - created_at_time > 10 minutes in the past
- `CreatedInFuture` - created_at_time > 5 minutes in the future
- `Duplicate` - Same transaction submitted within deduplication window

### ApproveError

- `AllowanceChanged` - Current allowance ≠ expected_allowance
- `Expired` - Allowance expiration time has passed
- Other errors same as TransferError

### QueryError

- `TokenNotFound` - Token ID doesn't exist
- `InvalidInput` - Invalid parameters provided
- `InternalError` - Unexpected internal error

---

## Integration Examples

### Minter: Create Token

```bash
TOKEN_ID=$(dfx canister call icrc151 create_token '(
  "Wrapped SOL",
  "ckSOL",
  9:nat8,
  opt (0:nat),
  opt (10_000:nat),
  null,
  null
)' | grep -o 'blob ".*"' | cut -d'"' -f2)
```

### Minter: Mint on Deposit

```bash
# When user deposits 1 SOL to bridge
dfx canister call icrc151 mint_tokens "(
  blob \"$TOKEN_ID\",
  record {
    owner = principal \"$USER_PRINCIPAL\";
    subaccount = null;
  },
  1_000_000_000:nat,
  opt blob \"deposit_tx_hash\"
)"
```

### Minter: Burn on Withdrawal

```bash
# When user requests withdrawal
dfx canister call icrc151 burn_tokens "(
  blob \"$TOKEN_ID\",
  1_000_000_000:nat,
  opt blob \"withdrawal_request_id\"
)"
```

### User: Check Balance

```bash
dfx canister call icrc151 get_balance "(
  blob \"$TOKEN_ID\",
  record {
    owner = principal \"$MY_PRINCIPAL\";
    subaccount = null;
  }
)"
```

### User: Transfer Tokens

```bash
dfx canister call icrc151 transfer "(
  record {
    token_id = blob \"$TOKEN_ID\";
    from_subaccount = null;
    to = record {
      owner = principal \"$RECIPIENT_PRINCIPAL\";
      subaccount = null;
    };
    amount = 100_000_000:nat;
    fee = opt (10_000:nat);
    memo = null;
    created_at_time = null;
  }
)"
```
