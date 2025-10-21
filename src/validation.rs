use crate::types::{Account, TokenId, AccountKey};
use candid::Principal;


#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    InvalidAccount(String),
    InvalidAmount(String),
    InvalidTokenId(String),
    InvalidPrincipal(String),
    InvalidMemo(String),
    InvalidFee(String),
    InvalidTimestamp(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidAccount(msg) => write!(f, "Invalid account: {}", msg),
            ValidationError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            ValidationError::InvalidTokenId(msg) => write!(f, "Invalid token ID: {}", msg),
            ValidationError::InvalidPrincipal(msg) => write!(f, "Invalid principal: {}", msg),
            ValidationError::InvalidMemo(msg) => write!(f, "Invalid memo: {}", msg),
            ValidationError::InvalidFee(msg) => write!(f, "Invalid fee: {}", msg),
            ValidationError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
        }
    }
}


pub fn validate_account(account: &Account) -> Result<(), ValidationError> {

    if account.owner == Principal::anonymous() {
        return Err(ValidationError::InvalidAccount(
            "Anonymous principal not allowed".to_string()
        ));
    }
    

    let principal_bytes = account.owner.as_slice();
    if principal_bytes.is_empty() || principal_bytes.len() > 29 {
        return Err(ValidationError::InvalidAccount(
            format!("Principal length {} not in range 1-29", principal_bytes.len())
        ));
    }
    

    if let Some(ref subaccount) = account.subaccount {
        if subaccount.len() != 32 {
            return Err(ValidationError::InvalidAccount(
                format!("Subaccount must be exactly 32 bytes, got {}", subaccount.len())
            ));
        }
    }
    
    Ok(())
}


pub fn validate_amount(amount: u128, allow_zero: bool) -> Result<(), ValidationError> {
    if !allow_zero && amount == 0 {
        return Err(ValidationError::InvalidAmount(
            "Amount must be greater than 0".to_string()
        ));
    }
    

    if amount > u128::MAX / 2 {
        return Err(ValidationError::InvalidAmount(
            "Amount too large, may cause overflow".to_string()
        ));
    }
    
    Ok(())
}


pub fn validate_transfer_fee(_fee: u128, _amount: u128) -> Result<(), ValidationError> {
    Ok(())
}

pub fn validate_approve_fee(_fee: u128) -> Result<(), ValidationError> {
    Ok(())
}


pub fn validate_memo(memo: &[u8]) -> Result<(), ValidationError> {

    if memo.len() > 65536 {
        return Err(ValidationError::InvalidMemo(
            format!("Memo size {} exceeds 64KB limit", memo.len())
        ));
    }
    

    if memo.len() > 0 && memo.len() <= 1024 {

        if let Ok(text) = std::str::from_utf8(memo) {
            if text.contains('\0') {
                return Err(ValidationError::InvalidMemo(
                    "Text memo contains null bytes".to_string()
                ));
            }
        }
    }
    
    Ok(())
}


pub fn validate_token_id(token_id: &TokenId) -> Result<(), ValidationError> {
    if token_id == &[0u8; 32] {
        return Err(ValidationError::InvalidTokenId(
            "Token ID cannot be all zeros".to_string()
        ));
    }
    
    Ok(())
}


pub fn validate_account_key(account_key: &AccountKey) -> Result<(), ValidationError> {
    if account_key == &[0u8; 32] {
        return Err(ValidationError::InvalidAccount(
            "Account key cannot be all zeros".to_string()
        ));
    }
    
    Ok(())
}


pub fn validate_timestamp(timestamp: u64) -> Result<(), ValidationError> {
    const MIN_TIMESTAMP: u64 = 1_600_000_000_000_000_000;

    if timestamp < MIN_TIMESTAMP {
        return Err(ValidationError::InvalidTimestamp(
            "Timestamp too far in the past".to_string()
        ));
    }

    let current_time = ic_cdk::api::time();
    if timestamp > current_time + crate::types::constants::MAX_FUTURE_DRIFT {
        return Err(ValidationError::InvalidTimestamp(
            "Timestamp too far in the future".to_string()
        ));
    }

    Ok(())
}


pub fn validate_admin_principal(principal: &Principal) -> Result<(), ValidationError> {
    if *principal == Principal::anonymous() {
        return Err(ValidationError::InvalidPrincipal(
            "Anonymous principal cannot be admin".to_string()
        ));
    }
    

    if *principal == Principal::management_canister() {
        return Err(ValidationError::InvalidPrincipal(
            "Management canister cannot be admin".to_string()
        ));
    }
    
    let principal_bytes = principal.as_slice();
    if principal_bytes.is_empty() || principal_bytes.len() > 29 {
        return Err(ValidationError::InvalidPrincipal(
            format!("Principal length {} not in range 1-29", principal_bytes.len())
        ));
    }
    
    Ok(())
}


pub fn validate_transfer_params(
    from: &Account,
    to: &Account,
    amount: u128,
    fee: Option<u128>,
    memo: Option<&[u8]>,
) -> Result<(), ValidationError> {
    validate_account(from)?;
    validate_account(to)?;
    validate_amount(amount, false)?;

    if let Some(fee_amount) = fee {
        validate_transfer_fee(fee_amount, amount)?;
    }

    if let Some(memo_data) = memo {
        validate_memo(memo_data)?;
    }

    if from == to {
        return Err(ValidationError::InvalidAccount(
            "Cannot transfer to same account".to_string()
        ));
    }

    Ok(())
}


pub fn validate_approve_params(
    owner: &Account,
    spender: &Account,
    amount: u128,
    fee: Option<u128>,
    memo: Option<&[u8]>,
) -> Result<(), ValidationError> {
    validate_account(owner)?;
    validate_account(spender)?;
    validate_amount(amount, true)?;

    if let Some(fee_amount) = fee {
        validate_approve_fee(fee_amount)?;
    }

    if let Some(memo_data) = memo {
        validate_memo(memo_data)?;
    }

    if owner == spender {
        return Err(ValidationError::InvalidAccount(
            "Cannot approve spending to self".to_string()
        ));
    }

    Ok(())
}


pub fn validate_mint_params(
    to: &Account,
    amount: u128,
    memo: Option<&[u8]>,
) -> Result<(), ValidationError> {
    validate_account(to)?;
    validate_amount(amount, false)?;
    
    if let Some(memo_data) = memo {
        validate_memo(memo_data)?;
    }
    
    Ok(())
}


pub fn validate_burn_params(
    from: &Account,
    amount: u128,
    memo: Option<&[u8]>,
) -> Result<(), ValidationError> {
    validate_account(from)?;
    validate_amount(amount, false)?;
    
    if let Some(memo_data) = memo {
        validate_memo(memo_data)?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    #[test]
    fn test_validate_account() {

        let principal_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let valid_account = Account {
            owner: Principal::from_slice(&principal_bytes),
            subaccount: None,
        };
        assert!(validate_account(&valid_account).is_ok());
        

        let account_with_sub = Account {
            owner: Principal::from_slice(&principal_bytes),
            subaccount: Some(vec![1u8; 32]),
        };
        assert!(validate_account(&account_with_sub).is_ok());
        

        let anonymous_account = Account {
            owner: Principal::anonymous(),
            subaccount: None,
        };
        assert!(validate_account(&anonymous_account).is_err());
        

        let invalid_sub = Account {
            owner: Principal::from_slice(&principal_bytes),
            subaccount: Some(vec![1u8; 31]),
        };
        assert!(validate_account(&invalid_sub).is_err());
    }

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(1000, false).is_ok());
        assert!(validate_amount(0, true).is_ok());
        assert!(validate_amount(0, false).is_err());
        assert!(validate_amount(u128::MAX, false).is_err());
    }

    #[test]
    fn test_validate_transfer_fee() {
        assert!(validate_transfer_fee(10, 1000).is_ok());
        assert!(validate_transfer_fee(0, 1000).is_ok());
        assert!(validate_transfer_fee(1000, 1000).is_ok());
        assert!(validate_transfer_fee(5000, 1000).is_ok());
        assert!(validate_transfer_fee(10000, 1000).is_ok());
        assert!(validate_transfer_fee(10001, 1000).is_ok());
        assert!(validate_transfer_fee(u128::MAX, u128::MAX).is_ok());
    }

    #[test]
    fn test_validate_memo() {
        assert!(validate_memo(b"valid memo").is_ok());
        assert!(validate_memo(&[]).is_ok());
        assert!(validate_memo(&vec![1u8; 1000]).is_ok());
        assert!(validate_memo(&vec![0u8; 70000]).is_err());
        assert!(validate_memo(b"invalid\0memo").is_err());
    }

    #[test]
    fn test_validate_token_id() {
        let valid_id = [1u8; 32];
        let zero_id = [0u8; 32];
        
        assert!(validate_token_id(&valid_id).is_ok());
        assert!(validate_token_id(&zero_id).is_err());
    }

    #[test]
    fn test_validate_transfer_params() {
        let principal_bytes1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD2];
        let principal_bytes2 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0xD3];
        
        let from = Account {
            owner: Principal::from_slice(&principal_bytes1),
            subaccount: None,
        };
        let to = Account {
            owner: Principal::from_slice(&principal_bytes2),
            subaccount: None,
        };
        
        assert!(validate_transfer_params(&from, &to, 1000, Some(10), None).is_ok());
        assert!(validate_transfer_params(&from, &from, 1000, Some(10), None).is_err());
        assert!(validate_transfer_params(&from, &to, 0, Some(10), None).is_err());
    }
}