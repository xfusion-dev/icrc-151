use crate::types::{Account, TokenId};
use crate::state;
use crate::validation::{validate_account, validate_token_id, ValidationError};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenMetadata {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u128,
    pub fee: u128,
    pub logo: Option<String>,
    pub description: Option<String>,
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Balance {
    pub account: Account,
    pub balance: u128,
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Allowance {
    pub owner: Account,
    pub spender: Account,
    pub allowance: u128,
    pub expires_at: Option<u64>,
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenInfo {
    pub token_id: TokenId,
    pub metadata: TokenMetadata,
    pub created_at: u64,
    pub controller: Principal,
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum QueryError {
    TokenNotFound,
    InvalidInput(String),
    InternalError(String),
}

impl From<ValidationError> for QueryError {
    fn from(err: ValidationError) -> Self {
        QueryError::InvalidInput(err.to_string())
    }
}


#[ic_cdk::query]
pub fn get_balance(token_id: TokenId, account: Account) -> Result<u128, QueryError> {
    validate_token_id(&token_id)?;
    validate_account(&account)?;
    
    let account_key = account.to_key();
    Ok(state::get_balance(token_id, account_key))
}


#[ic_cdk::query]
pub fn get_allowance(token_id: TokenId, owner: Account, spender: Account) -> Result<u128, QueryError> {
    validate_token_id(&token_id)?;
    validate_account(&owner)?;
    validate_account(&spender)?;

    let owner_key = owner.to_key();
    let spender_key = spender.to_key();

    Ok(state::get_allowance(token_id, owner_key, spender_key))
}


#[ic_cdk::query]
pub fn get_allowance_details(token_id: TokenId, owner: Account, spender: Account) -> Result<Allowance, QueryError> {
    validate_token_id(&token_id)?;
    validate_account(&owner)?;
    validate_account(&spender)?;

    let owner_key = owner.to_key();
    let spender_key = spender.to_key();

    let allowance_amount = state::get_allowance(token_id, owner_key, spender_key);
    let expires_at = state::get_allowance_expiry(token_id, owner_key, spender_key);

    Ok(Allowance {
        owner,
        spender,
        allowance: allowance_amount,
        expires_at,
    })
}


#[ic_cdk::query]
pub fn get_total_supply(token_id: TokenId) -> Result<u128, QueryError> {
    validate_token_id(&token_id)?;

    match state::get_token_metadata(token_id) {
        Some(metadata) => Ok(metadata.total_supply),
        None => Err(QueryError::TokenNotFound),
    }
}


#[ic_cdk::query]
pub fn get_holder_count(token_id: TokenId) -> Result<u64, QueryError> {
    validate_token_id(&token_id)?;

    if !state::token_exists(token_id) {
        return Err(QueryError::TokenNotFound);
    }

    Ok(state::get_holder_count(token_id))
}


#[ic_cdk::query]
pub fn get_token_metadata(token_id: TokenId) -> Result<TokenMetadata, QueryError> {
    validate_token_id(&token_id)?;

    match state::get_token_metadata(token_id) {
        Some(stored) => Ok(TokenMetadata {
            name: stored.name,
            symbol: stored.symbol,
            decimals: stored.decimals,
            total_supply: stored.total_supply,
            fee: stored.fee,
            logo: stored.logo,
            description: stored.description,
        }),
        None => Err(QueryError::TokenNotFound),
    }
}


#[ic_cdk::query]
pub fn get_transaction_count() -> u64 {
    state::get_transaction_count()
}


#[ic_cdk::query]
pub fn get_transactions(
    token_id: Option<TokenId>,
    start: Option<u64>,
    length: Option<u64>,
) -> Result<Vec<crate::transaction::StoredTxV1>, QueryError> {
    if let Some(tid) = token_id {
        validate_token_id(&tid)?;
    }

    const MAX_RESULTS: u64 = 1000;

    let total_count = state::get_transaction_count();
    let start_idx = start.unwrap_or(0);
    let requested_length = length.unwrap_or(100).min(MAX_RESULTS);


    if start_idx >= total_count {
        return Ok(vec![]);
    }


    let end_idx = (start_idx + requested_length).min(total_count);

    let mut results = Vec::new();

    for idx in start_idx..end_idx {
        if let Some(tx) = state::get_transaction(idx) {

            if let Some(filter_token_id) = token_id {
                if tx.token_id == filter_token_id {
                    results.push(tx);
                }
            } else {

                results.push(tx);
            }
        }
    }

    Ok(results)
}


#[ic_cdk::query]
pub fn health_check() -> String {
    format!(
        "ICRC-151 Canister v0.1.0 - Controller: {:?} - Transactions: {}",
        state::get_controller(),
        state::get_transaction_count()
    )
}


#[ic_cdk::query]
pub fn get_info() -> CanisterInfo {
    CanisterInfo {
        name: "ICRC-151 Multi-Token Ledger".to_string(),
        version: "0.1.0".to_string(),
        controller: state::get_controller()
            .map(|p| p.to_text())
            .unwrap_or("None".to_string()),
        transaction_count: state::get_transaction_count(),
        global_tx_count: state::get_global_tx_count(),
    }
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct CanisterInfo {
    pub name: String,
    pub version: String,
    pub controller: String,
    pub transaction_count: u64,
    pub global_tx_count: u64,
}


#[ic_cdk::query]
pub fn list_tokens() -> Vec<TokenId> {
    state::list_token_ids()
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct TokenBalance {
    pub token_id: TokenId,
    pub balance: u128,
}


#[ic_cdk::query]
pub fn get_balances_for(owner: candid::Principal, subaccount: Option<Vec<u8>>) -> Vec<TokenBalance> {
    let account = Account { owner, subaccount };
    let account_key = account.to_key();
    let token_ids = state::list_token_ids();

    let mut results = Vec::with_capacity(token_ids.len());
    for token_id in token_ids.into_iter() {
        let amount = state::get_balance(token_id, account_key);
        if amount > 0 {
            results.push(TokenBalance { token_id, balance: amount });
        }
    }
    results
}

#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct StorageStats {
    pub transaction_log_size: u64,
    pub dedup_map_size: u64,
    pub allowance_expiry_size: u64,
    pub extended_memos_size: u64,
    pub holder_counts_size: u64,
    pub token_count: u64,
    pub estimated_memory_bytes: u64,
}

#[ic_cdk::query]
pub fn get_storage_stats() -> StorageStats {
    let tx_count = state::get_transaction_count();
    let dedup_size = state::get_dedup_map_size();
    let expiry_size = state::get_allowance_expiry_size();
    let memo_size = state::get_extended_memos_size();
    let holder_counts_size = state::get_holder_counts_size();
    let token_count = state::list_token_ids().len() as u64;

    let estimated_memory = (tx_count * 256)
        + (dedup_size * 40)
        + (expiry_size * 40)
        + (memo_size * 100)
        + (holder_counts_size * 40);

    StorageStats {
        transaction_log_size: tx_count,
        dedup_map_size: dedup_size,
        allowance_expiry_size: expiry_size,
        extended_memos_size: memo_size,
        holder_counts_size,
        token_count,
        estimated_memory_bytes: estimated_memory,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn test_balance_queries() {
        let principal_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let account = Account {
            owner: Principal::from_slice(&principal_bytes),
            subaccount: None,
        };
        

        let token_id = [1u8; 32];
        assert_eq!(get_balance(token_id, account.clone()).unwrap(), 0);
    }

    #[test]
    fn test_allowance_queries() {
        let principal_bytes1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let principal_bytes2 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD3];
        
        let owner = Account {
            owner: Principal::from_slice(&principal_bytes1),
            subaccount: None,
        };
        let spender = Account {
            owner: Principal::from_slice(&principal_bytes2),
            subaccount: None,
        };
        
        let token_id = [1u8; 32];
        assert_eq!(get_allowance(token_id, owner, spender).unwrap(), 0);
    }

    #[test]
    fn test_validation_errors() {
        let zero_token = [0u8; 32];
        let valid_account = Account {
            owner: Principal::from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2]),
            subaccount: None,
        };
        
        assert!(get_balance(zero_token, valid_account).is_err());
    }
}