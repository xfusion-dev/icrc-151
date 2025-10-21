use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ic_stable_structures::Storable;
use std::borrow::Cow;

pub type TokenId = [u8; 32];
pub type AccountKey = [u8; 32];

#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub owner: Principal,
    pub subaccount: Option<Vec<u8>>,
}

impl Account {
    pub fn to_key(&self) -> AccountKey {
        let mut hasher = Sha256::new();
        hasher.update(b"icrc151:account:v1");
        hasher.update(self.owner.as_slice());

        let subaccount_32 = match &self.subaccount {
            Some(sub) => sub.as_slice(),
            None => &[0u8; 32],
        };

        hasher.update(subaccount_32);
        hasher.finalize().into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoredPrincipal {
    pub len: u8,
    pub bytes: [u8; 29],
}

impl StoredPrincipal {
    pub fn from_principal(p: &Principal) -> Result<Self, String> {
        let bytes = p.as_slice();
        if bytes.is_empty() || bytes.len() > 29 {
            return Err(format!("Invalid principal length: {}", bytes.len()));
        }
        let mut stored = Self {
            len: bytes.len() as u8,
            bytes: [0; 29],
        };
        stored.bytes[..bytes.len()].copy_from_slice(bytes);
        Ok(stored)
    }
    
    pub fn to_principal(&self) -> Result<Principal, String> {
        if self.len == 0 || self.len > 29 {
            return Err(format!("Invalid stored principal length: {}", self.len));
        }
        Ok(Principal::from_slice(&self.bytes[..self.len as usize]))
    }
}

impl Storable for StoredPrincipal {
    const BOUND: ic_stable_structures::storable::Bound = 
        ic_stable_structures::storable::Bound::Bounded { 
            max_size: 30, 
            is_fixed_size: true 
        };
    
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        let mut buf = [0u8; 30];
        buf[0] = self.len;
        buf[1..30].copy_from_slice(&self.bytes);
        Cow::Owned(buf.to_vec())
    }
    
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        assert_eq!(bytes.len(), 30, "StoredPrincipal must be exactly 30 bytes");
        let mut stored = Self {
            len: bytes[0],
            bytes: [0; 29],
        };
        stored.bytes.copy_from_slice(&bytes[1..30]);
        stored
    }
}

pub mod memory_ids {
    pub const TOKEN_REGISTRY: u8 = 0;          // TokenId → TokenMetadata
    pub const BALANCE_STORAGE: u8 = 1;         // BalanceKey → u128
    pub const ALLOWANCE_STORAGE: u8 = 2;       // AllowanceKey → AllowanceValue
    pub const TRANSACTION_LOG: u8 = 3;         // StoredTxV1
    pub const TX_INDEX_RECENT: u8 = 4;         // Recent tx index (hot window)
    pub const ARCHIVE_INDEX: u8 = 5;           // start_idx → ArchiveManifest
    pub const SYSTEM_STATE: u8 = 6;            // System config and counters
    pub const TOKEN_ACCOUNTS_INDEX: u8 = 7;    // Token→Accounts mapping
    pub const ACCOUNT_TOKENS_INDEX: u8 = 8;    // Account→Tokens mapping
    pub const EXTENDED_MEMOS: u8 = 9;          // Extended memo storage
    pub const ALLOWANCE_EXPIRY_INDEX: u8 = 10; // Allowance expiry index
    pub const TX_INDEX_BUFFER: u8 = 11;        // Tx index buffer for archiving
    pub const DEDUP_MAP: u8 = 12;              // Deduplication: hash → tx_index
    pub const CONTROLLERS: u8 = 13;            // Controllers set: StoredPrincipal → u8
    pub const HOLDER_COUNTS: u8 = 14;          // Holder counts: TokenId → u64
    pub const RESERVED_START: u8 = 15;         // Reserved for future extensions
}

pub mod constants {
    pub const MAX_FUTURE_DRIFT: u64 = 300_000_000_000;
    pub const MAX_PAST_DRIFT: u64 = 600_000_000_000;
}
pub fn encode_tx_index_key(token_id: TokenId, local_index: u64) -> [u8; 44] {
    let mut key = [0u8; 44];
    key[0..4].copy_from_slice(b"txl/");
    key[4..36].copy_from_slice(&token_id);
    key[36..44].copy_from_slice(&local_index.to_be_bytes());
    key
}

pub fn encode_archive_key(start_index: u64) -> [u8; 12] {
    let mut key = [0u8; 12];
    key[0..4].copy_from_slice(b"arc/");
    key[4..12].copy_from_slice(&start_index.to_be_bytes());
    key
}

pub fn encode_token_account_key(token_id: TokenId, account_key: AccountKey) -> [u8; 64] {
    let mut key = [0u8; 64];
    key[0..32].copy_from_slice(&token_id);
    key[32..64].copy_from_slice(&account_key);
    key
}

pub fn encode_account_token_key(account_key: AccountKey, token_id: TokenId) -> [u8; 64] {
    let mut key = [0u8; 64];
    key[0..32].copy_from_slice(&account_key);
    key[32..64].copy_from_slice(&token_id);
    key
}

pub fn encode_allowance_expiry_key(expires_at: u64, allowance_key: [u8; 32]) -> [u8; 40] {
    let mut key = [0u8; 40];
    key[0..8].copy_from_slice(&expires_at.to_be_bytes());
    key[8..40].copy_from_slice(&allowance_key);
    key
}

pub fn hash_balance_key(token_id: TokenId, account_key: AccountKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"icrc151:balance:v1");
    hasher.update(&token_id);
    hasher.update(&account_key);
    hasher.finalize().into()
}

pub fn hash_allowance_key(token_id: TokenId, owner_key: AccountKey, spender_key: AccountKey) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"icrc151:allowance:v1");
    hasher.update(&token_id);
    hasher.update(&owner_key);
    hasher.update(&spender_key);
    hasher.finalize().into()
}

pub fn derive_token_id(ledger_principal: Principal, nonce: u64) -> TokenId {
    let mut hasher = Sha256::new();
    hasher.update(b"icrc151:token:v1");
    hasher.update(ledger_principal.as_slice());
    hasher.update(&nonce.to_be_bytes());
    hasher.finalize().into()
}

#[derive(candid::CandidType, serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct StoredTokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u128,
    pub fee: u128,
    pub fee_recipient: Account,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub created_at: u64,
    pub controller: Principal,
}

impl Storable for StoredTokenMetadata {
    const BOUND: ic_stable_structures::storable::Bound =
        ic_stable_structures::storable::Bound::Unbounded;

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        use candid::Encode;
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        use candid::Decode;
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}