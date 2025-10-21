use crate::types::*;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    DefaultMemoryImpl, StableBTreeMap, Log, Storable,
};
use std::cell::RefCell;
use candid::Principal;

type Memory = VirtualMemory<DefaultMemoryImpl>;

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = 
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
    

    static TOKEN_REGISTRY: RefCell<StableBTreeMap<TokenId, crate::types::StoredTokenMetadata, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::TOKEN_REGISTRY)))
        )
    );
    
    static BALANCE_STORAGE: RefCell<StableBTreeMap<[u8; 32], u128, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::BALANCE_STORAGE)))
        )
    );
    
    static ALLOWANCE_STORAGE: RefCell<StableBTreeMap<[u8; 32], u128, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::ALLOWANCE_STORAGE)))
        )
    );
    
    static TRANSACTION_LOG: RefCell<Log<crate::transaction::StoredTxV1, Memory, Memory>> = RefCell::new(
        Log::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::TRANSACTION_LOG))),
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::TX_INDEX_BUFFER)))
        ).expect("Failed to initialize transaction log")
    );
    
    static SYSTEM_STATE: RefCell<StableBTreeMap<[u8; 32], Vec<u8>, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::SYSTEM_STATE)))
        )
    );

    static CONTROLLERS: RefCell<StableBTreeMap<StoredPrincipal, u8, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::CONTROLLERS)))
        )
    );

    static DEDUP_MAP: RefCell<StableBTreeMap<[u8; 32], u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::DEDUP_MAP)))
        )
    );

    static ALLOWANCE_EXPIRY: RefCell<StableBTreeMap<[u8; 32], u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::ALLOWANCE_EXPIRY_INDEX)))
        )
    );

    static EXTENDED_MEMOS: RefCell<StableBTreeMap<u64, Vec<u8>, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::EXTENDED_MEMOS)))
        )
    );

    static HOLDER_COUNTS: RefCell<StableBTreeMap<TokenId, u64, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(memory_ids::HOLDER_COUNTS)))
        )
    );
}


const KEY_CONTROLLER: [u8; 32] = *b"icrc151:controller:v1\0\0\0\0\0\0\0\0\0\0\0";
const KEY_NEXT_TOKEN_NONCE: [u8; 32] = *b"icrc151:next_token_nonce:v1\0\0\0\0\0";
const KEY_GLOBAL_TX_COUNT: [u8; 32] = *b"icrc151:global_tx_count:v1\0\0\0\0\0\0";


pub fn init_state(controller: Principal) {
    SYSTEM_STATE.with(|s| {
        let mut state = s.borrow_mut();
        

        let controller_stored = StoredPrincipal::from_principal(&controller)
            .expect("Invalid controller principal");
        state.insert(KEY_CONTROLLER, controller_stored.to_bytes().to_vec());
        

        state.insert(KEY_NEXT_TOKEN_NONCE, 0u64.to_be_bytes().to_vec());
        state.insert(KEY_GLOBAL_TX_COUNT, 0u64.to_be_bytes().to_vec());
    });

    // Seed controllers set with initial controller
    CONTROLLERS.with(|c| {
        let mut map = c.borrow_mut();
        if let Ok(stored) = StoredPrincipal::from_principal(&controller) {
            map.insert(stored, 1u8);
        }
    });
}


pub fn get_controller() -> Option<Principal> {
    SYSTEM_STATE.with(|s| {
        s.borrow().get(&KEY_CONTROLLER).and_then(|bytes| {
            if bytes.len() == 30 {
                let stored = StoredPrincipal::from_bytes(std::borrow::Cow::Borrowed(&bytes));
                stored.to_principal().ok()
            } else {
                None
            }
        })
    })
}

pub fn only_controller() -> Result<(), String> {
    let caller = ic_cdk::caller();
    if !is_controller(&caller) {
        return Err("Only controller can call this method".to_string());
    }
    Ok(())
}


pub fn set_controller(new_controller: Principal) -> Result<(), String> {
    require_controller()?;
    SYSTEM_STATE.with(|s| -> Result<(), String> {
        let mut state = s.borrow_mut();
        let controller_stored = StoredPrincipal::from_principal(&new_controller)?;
        state.insert(KEY_CONTROLLER, controller_stored.to_bytes().to_vec());
        Ok(())
    })?;
    add_controller_internal(new_controller)
}


pub fn next_token_nonce() -> u64 {
    SYSTEM_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let current = state.get(&KEY_NEXT_TOKEN_NONCE)
            .map(|bytes| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(buf)
            })
            .unwrap_or(0);
        
        let next = current + 1;
        state.insert(KEY_NEXT_TOKEN_NONCE, next.to_be_bytes().to_vec());
        next
    })
}


pub fn get_global_tx_count() -> u64 {
    SYSTEM_STATE.with(|s| {
        s.borrow().get(&KEY_GLOBAL_TX_COUNT)
            .map(|bytes| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(buf)
            })
            .unwrap_or(0)
    })
}


pub fn increment_tx_count() -> u64 {
    SYSTEM_STATE.with(|s| {
        let mut state = s.borrow_mut();
        let current = state.get(&KEY_GLOBAL_TX_COUNT)
            .map(|bytes| {
                let mut buf = [0u8; 8];
                buf.copy_from_slice(&bytes[..8]);
                u64::from_be_bytes(buf)
            })
            .unwrap_or(0);
        
        let next = current + 1;
        state.insert(KEY_GLOBAL_TX_COUNT, next.to_be_bytes().to_vec());
        next
    })
}


pub fn get_balance(token_id: TokenId, account_key: AccountKey) -> u128 {
    let balance_key = hash_balance_key(token_id, account_key);
    BALANCE_STORAGE.with(|b| {
        b.borrow().get(&balance_key).unwrap_or(0)
    })
}


pub fn set_balance(token_id: TokenId, account_key: AccountKey, amount: u128) {
    let balance_key = hash_balance_key(token_id, account_key);

    let old_balance = BALANCE_STORAGE.with(|b| {
        b.borrow().get(&balance_key).unwrap_or(0)
    });

    BALANCE_STORAGE.with(|b| {
        let mut storage = b.borrow_mut();
        if amount == 0 {
            storage.remove(&balance_key);
        } else {
            storage.insert(balance_key, amount);
        }
    });

    if old_balance == 0 && amount > 0 {
        increment_holder_count(token_id);
    } else if old_balance > 0 && amount == 0 {
        decrement_holder_count(token_id);
    }
}


fn increment_holder_count(token_id: TokenId) {
    HOLDER_COUNTS.with(|h| {
        let mut counts = h.borrow_mut();
        let current = counts.get(&token_id).unwrap_or(0);
        counts.insert(token_id, current + 1);
    });
}

fn decrement_holder_count(token_id: TokenId) {
    HOLDER_COUNTS.with(|h| {
        let mut counts = h.borrow_mut();
        let current = counts.get(&token_id).unwrap_or(0);
        if current > 0 {
            counts.insert(token_id, current - 1);
        }
    });
}

pub fn get_holder_count(token_id: TokenId) -> u64 {
    HOLDER_COUNTS.with(|h| {
        h.borrow().get(&token_id).unwrap_or(0)
    })
}


pub fn get_allowance(token_id: TokenId, owner_key: AccountKey, spender_key: AccountKey) -> u128 {
    let allowance_key = hash_allowance_key(token_id, owner_key, spender_key);
    ALLOWANCE_STORAGE.with(|a| {
        a.borrow().get(&allowance_key).unwrap_or(0)
    })
}


pub fn set_allowance(token_id: TokenId, owner_key: AccountKey, spender_key: AccountKey, amount: u128) {
    let allowance_key = hash_allowance_key(token_id, owner_key, spender_key);
    ALLOWANCE_STORAGE.with(|a| {
        let mut storage = a.borrow_mut();
        if amount == 0 {
            storage.remove(&allowance_key);
        } else {
            storage.insert(allowance_key, amount);
        }
    });
}


pub fn add_transaction(tx: crate::transaction::StoredTxV1) -> u64 {
    TRANSACTION_LOG.with(|log| {
        log.borrow_mut().append(&tx).expect("Failed to append transaction")
    })
}


pub fn get_transaction_count() -> u64 {
    TRANSACTION_LOG.with(|log| {
        log.borrow().len()
    })
}


pub fn get_transaction(index: u64) -> Option<crate::transaction::StoredTxV1> {
    TRANSACTION_LOG.with(|log| {
        log.borrow().get(index)
    })
}


pub fn require_controller() -> Result<(), String> {
    let caller = ic_cdk::caller();
    if !is_controller(&caller) {
        return Err("Only controller can perform this operation".to_string());
    }
    
    Ok(())
}


pub fn is_controller(p: &Principal) -> bool {
    CONTROLLERS.with(|c| {
        if let Ok(stored) = StoredPrincipal::from_principal(p) {
            c.borrow().contains_key(&stored)
        } else {
            false
        }
    })
}


pub fn add_controller_internal(p: Principal) -> Result<(), String> {
    CONTROLLERS.with(|c| {
        let mut map = c.borrow_mut();
        let stored = StoredPrincipal::from_principal(&p)?;
        map.insert(stored, 1u8);
        Ok(())
    })
}


pub fn remove_controller_internal(p: Principal) -> Result<(), String> {
    CONTROLLERS.with(|c| {
        let mut map = c.borrow_mut();
        let stored = StoredPrincipal::from_principal(&p)?;
        map.remove(&stored);
        Ok(())
    })
}


pub fn list_controllers() -> Vec<Principal> {
    CONTROLLERS.with(|c| {
        c.borrow().iter().filter_map(|(stored, _)| stored.to_principal().ok()).collect()
    })
}



pub fn compute_dedup_key(
    caller: candid::Principal,
    token_id: crate::types::TokenId,
    created_at_time: u64,
    memo: Option<&[u8]>,
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"icrc151:dedup:v1");
    hasher.update(caller.as_slice());
    hasher.update(&token_id);
    hasher.update(&created_at_time.to_be_bytes());
    if let Some(memo_data) = memo {
        hasher.update(memo_data);
    }
    hasher.finalize().into()
}



pub fn check_duplicate(dedup_key: [u8; 32]) -> Option<u64> {
    DEDUP_MAP.with(|d| {
        d.borrow().get(&dedup_key)
    })
}


pub fn record_transaction_dedup(dedup_key: [u8; 32], tx_index: u64) {
    DEDUP_MAP.with(|d| {
        d.borrow_mut().insert(dedup_key, tx_index);
    });
}


pub fn register_token(token_id: crate::types::TokenId, metadata: crate::types::StoredTokenMetadata) {
    TOKEN_REGISTRY.with(|r| {
        r.borrow_mut().insert(token_id, metadata);
    });
}


pub fn get_token_metadata(token_id: crate::types::TokenId) -> Option<crate::types::StoredTokenMetadata> {
    TOKEN_REGISTRY.with(|r| {
        r.borrow().get(&token_id)
    })
}


pub fn token_exists(token_id: crate::types::TokenId) -> bool {
    TOKEN_REGISTRY.with(|r| {
        r.borrow().contains_key(&token_id)
    })
}


pub fn list_token_ids() -> Vec<crate::types::TokenId> {
    TOKEN_REGISTRY.with(|r| {
        let registry = r.borrow();
        registry.iter().map(|(k, _)| k).collect()
    })
}


pub fn update_token_fee(token_id: crate::types::TokenId, new_fee: u128) -> Result<(), String> {
    TOKEN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();

        match registry.get(&token_id) {
            Some(mut metadata) => {
                metadata.fee = new_fee;
                registry.insert(token_id, metadata);
                Ok(())
            }
            None => Err("Token not found".to_string())
        }
    })
}


pub fn update_total_supply(token_id: crate::types::TokenId, new_supply: u128) -> Result<(), String> {
    TOKEN_REGISTRY.with(|r| {
        let mut registry = r.borrow_mut();
        match registry.get(&token_id) {
            Some(mut metadata) => {
                metadata.total_supply = new_supply;
                registry.insert(token_id, metadata);
                Ok(())
            }
            None => Err("Token not found in registry".to_string()),
        }
    })
}


pub fn set_allowance_expiry(
    token_id: crate::types::TokenId,
    owner_key: crate::types::AccountKey,
    spender_key: crate::types::AccountKey,
    expires_at: u64,
) {
    let expiry_key = crate::types::hash_allowance_key(token_id, owner_key, spender_key);
    ALLOWANCE_EXPIRY.with(|e| {
        e.borrow_mut().insert(expiry_key, expires_at);
    });
}


pub fn get_allowance_expiry(
    token_id: crate::types::TokenId,
    owner_key: crate::types::AccountKey,
    spender_key: crate::types::AccountKey,
) -> Option<u64> {
    let expiry_key = crate::types::hash_allowance_key(token_id, owner_key, spender_key);
    ALLOWANCE_EXPIRY.with(|e| {
        e.borrow().get(&expiry_key)
    })
}


pub fn is_allowance_expired(expires_at: Option<u64>) -> bool {
    match expires_at {
        Some(exp) => ic_cdk::api::time() >= exp,
        None => false,
    }
}


pub fn store_extended_memo(tx_index: u64, memo: Vec<u8>) {
    EXTENDED_MEMOS.with(|m| {
        m.borrow_mut().insert(tx_index, memo);
    });
}


pub fn get_extended_memo(tx_index: u64) -> Option<Vec<u8>> {
    EXTENDED_MEMOS.with(|m| {
        m.borrow().get(&tx_index)
    })
}

pub fn get_dedup_map_size() -> u64 {
    DEDUP_MAP.with(|d| {
        d.borrow().len()
    })
}

pub fn get_allowance_expiry_size() -> u64 {
    ALLOWANCE_EXPIRY.with(|e| {
        e.borrow().len()
    })
}

pub fn get_extended_memos_size() -> u64 {
    EXTENDED_MEMOS.with(|m| {
        m.borrow().len()
    })
}

pub fn get_holder_counts_size() -> u64 {
    HOLDER_COUNTS.with(|h| {
        h.borrow().len()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_management() {


    }

    #[test] 
    fn test_balance_operations() {
        let token_id = [1u8; 32];
        let account_key = [2u8; 32];
        
        assert_eq!(get_balance(token_id, account_key), 0);
        
        set_balance(token_id, account_key, 1000);
        assert_eq!(get_balance(token_id, account_key), 1000);
        
        set_balance(token_id, account_key, 0);
        assert_eq!(get_balance(token_id, account_key), 0);
    }

    #[test]
    fn test_allowance_operations() {
        let token_id = [1u8; 32];
        let owner_key = [2u8; 32];
        let spender_key = [3u8; 32];
        
        assert_eq!(get_allowance(token_id, owner_key, spender_key), 0);
        
        set_allowance(token_id, owner_key, spender_key, 500);
        assert_eq!(get_allowance(token_id, owner_key, spender_key), 500);
        
        set_allowance(token_id, owner_key, spender_key, 0);
        assert_eq!(get_allowance(token_id, owner_key, spender_key), 0);
    }
}