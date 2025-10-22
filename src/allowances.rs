use crate::types::{Account, TokenId};
use crate::state;
use crate::validation::{validate_approve_params, validate_account, validate_token_id, ValidationError};
use crate::transaction::StoredTxV1;
use candid::CandidType;
use serde::{Deserialize, Serialize};
use num_traits::cast::ToPrimitive;


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Icrc151ApproveArgs {
    pub token_id: TokenId,
    pub spender: Account,
    pub amount: candid::Nat,
    pub expires_at: Option<u64>,
    pub expected_allowance: Option<candid::Nat>,
    pub memo: Option<Vec<u8>>,
    pub fee: Option<candid::Nat>,
    pub from_subaccount: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum ApproveResult {
    Ok(u64),
    Err(ApproveError),
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub enum ApproveError {
    BadFee { expected_fee: candid::Nat },
    InsufficientFunds { balance: candid::Nat },
    AllowanceChanged { current_allowance: candid::Nat },
    Expired { ledger_time: u64 },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: u64 },
    TemporarilyUnavailable,
    GenericError { error_code: candid::Nat, message: String },
}

impl From<ValidationError> for ApproveError {
    fn from(err: ValidationError) -> Self {
        ApproveError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: err.to_string(),
        }
    }
}


#[derive(CandidType, Serialize, Deserialize, Clone, Debug)]
pub struct Icrc151TransferFromArgs {
    pub token_id: TokenId,
    pub spender_subaccount: Option<Vec<u8>>,
    pub from: Account,
    pub to: Account,
    pub amount: candid::Nat,
    pub fee: Option<candid::Nat>,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}


pub use crate::operations::{TransferResult, TransferError};


#[ic_cdk::update]
pub fn approve(args: Icrc151ApproveArgs) -> ApproveResult {
    let caller = ic_cdk::caller();
    

    let owner_account = Account {
        owner: caller,
        subaccount: args.from_subaccount.clone(),
    };
    

    let amount = match args.amount.0.to_u128() {
        Some(a) => a,
        None => return ApproveResult::Err(ApproveError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: "Amount exceeds maximum value (u128::MAX)".to_string(),
        }),
    };

    let fee = match args.fee.as_ref() {
        Some(f) => match f.0.to_u128() {
            Some(val) => Some(val),
            None => return ApproveResult::Err(ApproveError::GenericError {
                error_code: candid::Nat::from(400u64),
                message: "Fee exceeds maximum value (u128::MAX)".to_string(),
            }),
        },
        None => None,
    };

    let expected_allowance = match args.expected_allowance.as_ref() {
        Some(a) => match a.0.to_u128() {
            Some(val) => Some(val),
            None => return ApproveResult::Err(ApproveError::GenericError {
                error_code: candid::Nat::from(400u64),
                message: "Expected allowance exceeds maximum value (u128::MAX)".to_string(),
            }),
        },
        None => None,
    };

    match approve_internal(
        args.token_id,
        owner_account,
        args.spender,
        amount,
        args.expires_at,
        expected_allowance,
        fee,
        args.memo.as_deref(),
        args.created_at_time,
    ) {
        Ok(tx_index) => ApproveResult::Ok(tx_index),
        Err(err) => ApproveResult::Err(err),
    }
}


fn approve_internal(
    token_id: TokenId,
    owner: Account,
    spender: Account,
    amount: u128,
    expires_at: Option<u64>,
    expected_allowance: Option<u128>,
    fee: Option<u128>,
    memo: Option<&[u8]>,
    created_at_time: Option<u64>,
) -> Result<u64, ApproveError> {

    validate_token_id(&token_id)?;


    let metadata = state::get_token_metadata(token_id)
        .ok_or(ApproveError::GenericError {
            error_code: candid::Nat::from(404u64),
            message: "Token not found".to_string(),
        })?;

    let expected_fee = metadata.fee;
    let fee_amount = fee.unwrap_or(expected_fee);


    if let Some(provided_fee) = fee {
        if provided_fee != expected_fee {
            return Err(ApproveError::BadFee {
                expected_fee: candid::Nat::from(expected_fee),
            });
        }
    }

    validate_approve_params(&owner, &spender, amount, Some(fee_amount), memo)?;
    

    let timestamp = created_at_time.unwrap_or_else(|| ic_cdk::api::time());
    if let Some(provided_time) = created_at_time {
        let current_time = ic_cdk::api::time();

        if provided_time > current_time + crate::types::constants::MAX_FUTURE_DRIFT {
            return Err(ApproveError::CreatedInFuture { ledger_time: current_time });
        }

        if provided_time < current_time.saturating_sub(crate::types::constants::MAX_PAST_DRIFT) {
            return Err(ApproveError::TooOld);
        }
    }
    

    if let Some(exp_time) = expires_at {
        if exp_time <= timestamp {
            return Err(ApproveError::Expired { ledger_time: timestamp });
        }
    }
    

    let owner_key = owner.to_key();
    let spender_key = spender.to_key();
    

    let current_allowance = state::get_allowance(token_id, owner_key, spender_key);
    if let Some(expected) = expected_allowance {
        if current_allowance != expected {
            return Err(ApproveError::AllowanceChanged {
                current_allowance: candid::Nat::from(current_allowance),
            });
        }
    }
    

    let owner_balance = if fee_amount > 0 {
        let balance = state::get_balance(token_id, owner_key);
        if balance < fee_amount {
            return Err(ApproveError::InsufficientFunds {
                balance: candid::Nat::from(balance),
            });
        }
        balance
    } else {
        0
    };

    let fee_recipient_key = metadata.fee_recipient.to_key();
    let fee_balance = state::get_balance(token_id, fee_recipient_key);
    let new_fee_balance = if fee_amount > 0 {
        fee_balance.checked_add(fee_amount)
            .ok_or(ApproveError::GenericError {
                error_code: candid::Nat::from(500u64),
                message: "Fee recipient balance overflow".to_string(),
            })?
    } else {
        fee_balance
    };

    if fee_amount > 0 {
        state::set_balance(token_id, owner_key, owner_balance - fee_amount);
        state::set_balance(token_id, fee_recipient_key, new_fee_balance);
    }
    

    let dedup_key = state::compute_dedup_key(
        owner.owner,
        token_id,
        timestamp,
        memo,
    );

    if let Some(duplicate_tx_index) = state::check_duplicate(dedup_key) {
        return Err(ApproveError::Duplicate {
            duplicate_of: duplicate_tx_index,
        });
    }


    state::set_allowance(token_id, owner_key, spender_key, amount);


    if let Some(exp_time) = expires_at {
        state::set_allowance_expiry(token_id, owner_key, spender_key, exp_time);
    }


    let tx = StoredTxV1::new_approve(
        token_id,
        owner_key,
        spender_key,
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
pub fn transfer_from(args: Icrc151TransferFromArgs) -> TransferResult {
    let caller = ic_cdk::caller();
    

    let spender_account = Account {
        owner: caller,
        subaccount: args.spender_subaccount.clone(),
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

    match transfer_from_internal(
        args.token_id,
        spender_account,
        args.from,
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


fn transfer_from_internal(
    token_id: TokenId,
    spender: Account,
    from: Account,
    to: Account,
    amount: u128,
    fee: Option<u128>,
    memo: Option<&[u8]>,
    created_at_time: Option<u64>,
) -> Result<u64, TransferError> {

    validate_token_id(&token_id).map_err(|e| TransferError::GenericError {
        error_code: candid::Nat::from(400u64),
        message: e.to_string(),
    })?;
    
    validate_account(&spender).map_err(|e| TransferError::GenericError {
        error_code: candid::Nat::from(400u64),
        message: e.to_string(),
    })?;
    
    validate_account(&from).map_err(|e| TransferError::GenericError {
        error_code: candid::Nat::from(400u64),
        message: e.to_string(),
    })?;
    
    validate_account(&to).map_err(|e| TransferError::GenericError {
        error_code: candid::Nat::from(400u64),
        message: e.to_string(),
    })?;
    
    if amount == 0 {
        return Err(TransferError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: "Amount must be greater than 0".to_string(),
        });
    }


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
    

    let spender_key = spender.to_key();
    let from_key = from.to_key();
    let to_key = to.to_key();
    

    let expiry = state::get_allowance_expiry(token_id, from_key, spender_key);
    if state::is_allowance_expired(expiry) {
        return Err(TransferError::GenericError {
            error_code: candid::Nat::from(403u64),
            message: "Allowance expired".to_string(),
        });
    }


    let current_allowance = state::get_allowance(token_id, from_key, spender_key);
    let total_amount = amount.checked_add(fee_amount)
        .ok_or(TransferError::GenericError {
            error_code: candid::Nat::from(400u64),
            message: "Amount + fee overflow".to_string(),
        })?;

    if current_allowance < total_amount {
        return Err(TransferError::InsufficientFunds {
            balance: candid::Nat::from(current_allowance),
        });
    }

    let from_balance = state::get_balance(token_id, from_key);
    if from_balance < total_amount {
        return Err(TransferError::InsufficientFunds {
            balance: candid::Nat::from(from_balance),
        });
    }

    let dedup_key = state::compute_dedup_key(
        spender.owner,
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
    state::set_allowance(token_id, from_key, spender_key, current_allowance - total_amount);
    if fee_amount > 0 {
        state::set_balance(token_id, fee_recipient_key, new_fee_balance);
    }


    let tx = StoredTxV1::new_transfer_from(
        token_id,
        from_key,
        to_key,
        spender_key,
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

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn test_approve_args_conversion() {
        let args = Icrc151ApproveArgs {
            token_id: [1u8; 32],
            spender: Account {
                owner: Principal::from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2]),
                subaccount: None,
            },
            amount: candid::Nat::from(1000u64),
            expires_at: None,
            expected_allowance: Some(candid::Nat::from(0u64)),
            memo: Some(b"test".to_vec()),
            fee: Some(candid::Nat::from(10u64)),
            from_subaccount: None,
            created_at_time: None,
        };
        

        let amount = args.amount.0.to_u128().unwrap_or(0);
        let fee = args.fee.as_ref().map(|f| f.0.to_u128().unwrap_or(0));
        let expected = args.expected_allowance.as_ref().map(|a| a.0.to_u128().unwrap_or(0));
        
        assert_eq!(amount, 1000);
        assert_eq!(fee, Some(10));
        assert_eq!(expected, Some(0));
    }

    #[test]
    fn test_transfer_from_args_conversion() {
        let principal_bytes1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let principal_bytes2 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD3];

        let args = Icrc151TransferFromArgs {
            token_id: [1u8; 32],
            spender_subaccount: None,
            from: Account {
                owner: Principal::from_slice(&principal_bytes1),
                subaccount: None,
            },
            to: Account {
                owner: Principal::from_slice(&principal_bytes2),
                subaccount: None,
            },
            amount: candid::Nat::from(1000u64),
            fee: Some(candid::Nat::from(10u64)),
            memo: Some(b"transfer_from_test".to_vec()),
            created_at_time: None,
        };
        

        let amount = args.amount.0.to_u128().unwrap_or(0);
        let fee = args.fee.as_ref().map(|f| f.0.to_u128().unwrap_or(0));
        
        assert_eq!(amount, 1000);
        assert_eq!(fee, Some(10));
    }

    #[test]
    fn test_approve_validation() {
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
        


        assert!(validate_token_id(&token_id).is_ok());
        assert!(validate_approve_params(&owner, &spender, 1000, Some(10), None).is_ok());
    }
}