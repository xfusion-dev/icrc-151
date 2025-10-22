use crate::types::{Account, TokenId, derive_token_id};
use crate::state;
use crate::validation::{validate_transfer_params, validate_account, validate_token_id, ValidationError};
use crate::transaction::StoredTxV1;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use num_traits::cast::ToPrimitive;


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum TransferResult {
    Ok(u64),
    Err(TransferError),
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum TransferError {
    BadFee { expected_fee: candid::Nat },
    BadBurn { min_burn_amount: candid::Nat },
    InsufficientFunds { balance: candid::Nat },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: u64 },
    TemporarilyUnavailable,
    GenericError { error_code: candid::Nat, message: String },
}

impl From<ValidationError> for TransferError {
    fn from(err: ValidationError) -> Self {
        TransferError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: err.to_string(),
        }
    }
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Icrc151TransferArgs {
    pub token_id: TokenId,
    pub from_subaccount: Option<Vec<u8>>,
    pub to: Account,
    pub amount: candid::Nat,
    pub fee: Option<candid::Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}


#[ic_cdk::update]
pub fn transfer(args: Icrc151TransferArgs) -> TransferResult {
    let caller = ic_cdk::caller();
    

    let from_account = Account {
        owner: caller,
        subaccount: args.from_subaccount.clone(),
    };
    

    let amount = match args.amount.0.to_u128() {
        Some(a) => a,
        None => return TransferResult::Err(TransferError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: "Amount exceeds maximum value (u128::MAX)".to_string(),
        }),
    };

    let fee = match args.fee.as_ref() {
        Some(f) => match f.0.to_u128() {
            Some(val) => Some(val),
            None => return TransferResult::Err(TransferError::GenericError {
                error_code: candid::Nat::from(400u64),
                message: "Fee exceeds maximum value (u128::MAX)".to_string(),
            }),
        },
        None => None,
    };

    match transfer_internal(
        args.token_id,
        from_account,
        args.to,
        amount,
        fee,
        args.memo.as_deref(),
        args.created_at_time,
    ) {
        Ok(tx_index) => TransferResult::Ok(tx_index),
        Err(err) => TransferResult::Err(err),
    }
}


fn transfer_internal(
    token_id: TokenId,
    from: Account,
    to: Account,
    amount: u128,
    fee: Option<u128>,
    memo: Option<&[u8]>,
    created_at_time: Option<u64>,
) -> Result<u64, TransferError> {

    validate_token_id(&token_id)?;


    let metadata = state::get_token_metadata(token_id)
        .ok_or(TransferError::GenericError {
            error_code: candid::Nat::from(404u64),
            message: "Token not found".to_string(),
        })?;

    let expected_fee = metadata.fee;
    let fee_amount = fee.unwrap_or(expected_fee);


    if let Some(provided_fee) = fee {
        if provided_fee != expected_fee {
            return Err(TransferError::BadFee {
                expected_fee: candid::Nat::from(expected_fee),
            });
        }
    }

    validate_transfer_params(&from, &to, amount, Some(fee_amount), memo)?;
    

    let timestamp = created_at_time.unwrap_or_else(|| ic_cdk::api::time());
    if let Some(provided_time) = created_at_time {
        let current_time = ic_cdk::api::time();

        if provided_time > current_time + crate::types::constants::MAX_FUTURE_DRIFT {
            return Err(TransferError::CreatedInFuture { ledger_time: current_time });
        }

        if provided_time < current_time.saturating_sub(crate::types::constants::MAX_PAST_DRIFT) {
            return Err(TransferError::TooOld);
        }
    }
    

    let from_key = from.to_key();
    let to_key = to.to_key();
    

    let from_balance = state::get_balance(token_id, from_key);
    let total_amount = amount.checked_add(fee_amount)
        .ok_or(TransferError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: "Amount + fee overflow".to_string(),
        })?;

    if from_balance < total_amount {
        return Err(TransferError::InsufficientFunds {
            balance: candid::Nat::from(from_balance),
        });
    }

    let dedup_key = state::compute_dedup_key(
        from.owner,
        token_id,
        timestamp,
        memo,
    );

    if let Some(duplicate_tx_index) = state::check_duplicate(dedup_key) {
        return Err(TransferError::Duplicate {
            duplicate_of: duplicate_tx_index,
        });
    }

    let to_balance = state::get_balance(token_id, to_key);
    let new_to_balance = to_balance.checked_add(amount)
        .ok_or(TransferError::GenericError {
            error_code: candid::Nat::from(500u64),
            message: "Recipient balance overflow".to_string(),
        })?;

    let fee_recipient_key = metadata.fee_recipient.to_key();
    let fee_balance = state::get_balance(token_id, fee_recipient_key);
    let new_fee_balance = if fee_amount > 0 {
        fee_balance.checked_add(fee_amount)
            .ok_or(TransferError::GenericError {
                error_code: candid::Nat::from(500u64),
                message: "Fee recipient balance overflow".to_string(),
            })?
    } else {
        fee_balance
    };

    state::set_balance(token_id, from_key, from_balance - total_amount);
    state::set_balance(token_id, to_key, new_to_balance);
    if fee_amount > 0 {
        state::set_balance(token_id, fee_recipient_key, new_fee_balance);
    }


    let tx = StoredTxV1::new_transfer(
        token_id,
        from_key,
        to_key,
        amount,
        fee_amount,
        timestamp,
        memo,
    );

    let tx_index = state::add_transaction(tx);
    state::increment_tx_count();


    if let Some(memo_bytes) = memo {
        if memo_bytes.len() > 32 {
            state::store_extended_memo(tx_index, memo_bytes.to_vec());
        }
    }


    state::record_transaction_dedup(dedup_key, tx_index);

    Ok(tx_index)
}


#[ic_cdk::update]
pub fn create_token(
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: Option<candid::Nat>,
    fee: Option<candid::Nat>,
    logo: Option<String>,
    description: Option<String>,
) -> Result<TokenId, String> {

    state::require_controller()?;


    if name.is_empty() || name.len() > 255 {
        return Err("Invalid token name length".to_string());
    }
    if symbol.is_empty() || symbol.len() > 32 {
        return Err("Invalid token symbol length".to_string());
    }
    if decimals > 18 {
        return Err("Decimals cannot exceed 18".to_string());
    }


    let nonce = state::next_token_nonce();
    let ledger_principal = ic_cdk::id();
    let token_id = derive_token_id(ledger_principal, nonce);


    let fee_amount = match fee {
        Some(f) => f.0.to_u128().ok_or("Fee exceeds maximum value (u128::MAX)".to_string())?,
        None => 10_000,
    };


    let controller = state::get_controller().ok_or("No controller set")?;
    let fee_recipient = Account {
        owner: controller,
        subaccount: None,
    };

    let metadata = crate::types::StoredTokenMetadata {
        name,
        symbol,
        decimals,
        total_supply: 0,
        fee: fee_amount,
        fee_recipient,
        logo,
        description,
        created_at: ic_cdk::api::time(),
        controller,
    };

    state::register_token(token_id, metadata);


    if let Some(supply) = initial_supply {
        let supply_amount = supply.0.to_u128()
            .ok_or("Initial supply exceeds maximum value (u128::MAX)".to_string())?;
        if supply_amount > 0 {
            let controller = state::get_controller().ok_or("No controller set")?;
            let controller_account = Account {
                owner: controller,
                subaccount: None,
            };
            
            mint_internal(token_id, controller_account, supply_amount, None, None)?;
        }
    }
    
    Ok(token_id)
}


#[ic_cdk::update]
pub fn mint_tokens(
    token_id: TokenId,
    to: Account,
    amount: candid::Nat,
    memo: Option<Vec<u8>>,
) -> Result<u64, String> {

    state::require_controller()?;

    let amount_u128 = amount.0.to_u128()
        .ok_or("Amount exceeds maximum value (u128::MAX)".to_string())?;
    mint_internal(token_id, to, amount_u128, memo.as_deref(), None)
}


fn mint_internal(
    token_id: TokenId,
    to: Account,
    amount: u128,
    memo: Option<&[u8]>,
    created_at_time: Option<u64>,
) -> Result<u64, String> {

    validate_token_id(&token_id).map_err(|e| e.to_string())?;
    validate_account(&to).map_err(|e| e.to_string())?;
    
    if amount == 0 {
        return Err("Amount must be greater than 0".to_string());
    }
    
    let timestamp = created_at_time.unwrap_or_else(|| ic_cdk::api::time());
    let to_key = to.to_key();


    let dedup_key = state::compute_dedup_key(
        to.owner,
        token_id,
        timestamp,
        memo,
    );

    if let Some(duplicate_tx_index) = state::check_duplicate(dedup_key) {
        return Err(format!("Duplicate mint transaction, original tx_index: {}", duplicate_tx_index));
    }


    let current_balance = state::get_balance(token_id, to_key);
    let new_balance = current_balance.checked_add(amount)
        .ok_or("Balance overflow")?;

    state::set_balance(token_id, to_key, new_balance);


    if let Some(metadata) = state::get_token_metadata(token_id) {
        let new_supply = metadata.total_supply.checked_add(amount)
            .ok_or("Total supply overflow")?;
        state::update_total_supply(token_id, new_supply)?;
    }


    let tx = StoredTxV1::new_mint(
        token_id,
        to_key,
        amount,
        timestamp,
        memo,
    );

    let tx_index = state::add_transaction(tx);
    state::increment_tx_count();


    if let Some(memo_bytes) = memo {
        if memo_bytes.len() > 32 {
            state::store_extended_memo(tx_index, memo_bytes.to_vec());
        }
    }


    state::record_transaction_dedup(dedup_key, tx_index);

    Ok(tx_index)
}


#[ic_cdk::update]
pub fn burn_tokens(
    token_id: TokenId,
    amount: candid::Nat,
    memo: Option<Vec<u8>>,
) -> Result<u64, String> {
    let caller = ic_cdk::caller();
    let from_account = Account {
        owner: caller,
        subaccount: None,
    };

    let amount_u128 = amount.0.to_u128()
        .ok_or("Amount exceeds maximum value (u128::MAX)".to_string())?;
    burn_internal(token_id, from_account, amount_u128, memo.as_deref(), None)
}

#[ic_cdk::update]
pub fn burn_tokens_from(
    token_id: TokenId,
    from: Account,
    amount: candid::Nat,
    memo: Option<Vec<u8>>,
) -> Result<u64, String> {
    state::only_controller()?;

    let amount_u128 = amount.0.to_u128()
        .ok_or("Amount exceeds maximum value (u128::MAX)".to_string())?;
    burn_internal(token_id, from, amount_u128, memo.as_deref(), None)
}


fn burn_internal(
    token_id: TokenId,
    from: Account,
    amount: u128,
    memo: Option<&[u8]>,
    created_at_time: Option<u64>,
) -> Result<u64, String> {

    validate_token_id(&token_id).map_err(|e| e.to_string())?;
    validate_account(&from).map_err(|e| e.to_string())?;
    
    if amount == 0 {
        return Err("Amount must be greater than 0".to_string());
    }
    
    let timestamp = created_at_time.unwrap_or_else(|| ic_cdk::api::time());
    let from_key = from.to_key();


    let dedup_key = state::compute_dedup_key(
        from.owner,
        token_id,
        timestamp,
        memo,
    );

    if let Some(duplicate_tx_index) = state::check_duplicate(dedup_key) {
        return Err(format!("Duplicate burn transaction, original tx_index: {}", duplicate_tx_index));
    }


    let current_balance = state::get_balance(token_id, from_key);
    if current_balance < amount {
        return Err(format!(
            "Insufficient balance: {} < {}",
            current_balance, amount
        ));
    }


    state::set_balance(token_id, from_key, current_balance - amount);


    if let Some(metadata) = state::get_token_metadata(token_id) {
        let new_supply = metadata.total_supply.checked_sub(amount)
            .ok_or("Total supply underflow")?;
        state::update_total_supply(token_id, new_supply)?;
    }


    let tx = StoredTxV1::new_burn(
        token_id,
        from_key,
        amount,
        timestamp,
        memo,
    );

    let tx_index = state::add_transaction(tx);
    state::increment_tx_count();


    if let Some(memo_bytes) = memo {
        if memo_bytes.len() > 32 {
            state::store_extended_memo(tx_index, memo_bytes.to_vec());
        }
    }


    state::record_transaction_dedup(dedup_key, tx_index);

    Ok(tx_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn test_transfer_validation() {

        let principal_bytes1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let principal_bytes2 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD3];
        
        let from = Account {
            owner: candid::Principal::from_slice(&principal_bytes1),
            subaccount: None,
        };
        let to = Account {
            owner: candid::Principal::from_slice(&principal_bytes2),
            subaccount: None,
        };
        let token_id = [1u8; 32];
        


        assert!(crate::validation::validate_token_id(&token_id).is_ok());
        assert!(crate::validation::validate_transfer_params(&from, &to, 1000, Some(10), None).is_ok());
    }

    #[test]
    fn test_transfer_args_conversion() {
        let args = Icrc151TransferArgs {
            token_id: [1u8; 32],
            from_subaccount: None,
            to: Account {
                owner: Principal::from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2]),
                subaccount: None,
            },
            amount: candid::Nat::from(1000u64),
            fee: Some(candid::Nat::from(10u64)),
            memo: Some(b"test".to_vec()),
            created_at_time: None,
        };
        

        let amount = args.amount.0.to_u128().unwrap_or(0);
        let fee = args.fee.as_ref().map(|f| f.0.to_u128().unwrap_or(0));
        
        assert_eq!(amount, 1000);
        assert_eq!(fee, Some(10));
    }

    #[test]
    fn test_token_creation_validation() {

        assert!(validate_token_name("").is_err());
        assert!(validate_token_name(&"a".repeat(256)).is_err());
        assert!(validate_token_name("Valid Token").is_ok());
        

        assert!(validate_token_symbol("").is_err());
        assert!(validate_token_symbol(&"A".repeat(33)).is_err());
        assert!(validate_token_symbol("VALID").is_ok());
    }
    
    fn validate_token_name(name: &str) -> Result<(), &'static str> {
        if name.is_empty() || name.len() > 255 {
            Err("Invalid token name length")
        } else {
            Ok(())
        }
    }
    
    fn validate_token_symbol(symbol: &str) -> Result<(), &'static str> {
        if symbol.is_empty() || symbol.len() > 32 {
            Err("Invalid token symbol length")
        } else {
            Ok(())
        }
    }
}

#[ic_cdk::update]
pub fn set_controller(new_controller: candid::Principal) -> Result<(), String> {
    state::set_controller(new_controller)
}


#[ic_cdk::update]
pub fn add_controller(p: candid::Principal) -> Result<(), String> {
    state::require_controller()?;
    state::add_controller_internal(p)
}


#[ic_cdk::update]
pub fn remove_controller(p: candid::Principal) -> Result<(), String> {
    state::require_controller()?;
    let controllers = state::list_controllers();
    if controllers.len() <= 1 && controllers.contains(&p) {
        return Err("Cannot remove the last controller".to_string());
    }
    state::remove_controller_internal(p)
}


#[ic_cdk::query]
pub fn list_controllers() -> Vec<candid::Principal> {
    state::list_controllers()
}


#[ic_cdk::update]
pub fn set_token_fee(token_id: TokenId, new_fee: candid::Nat) -> Result<(), String> {
    state::require_controller()?;

    let fee_amount = new_fee.0.to_u128()
        .ok_or("Fee exceeds maximum value (u128::MAX)".to_string())?;

    state::update_token_fee(token_id, fee_amount)
}