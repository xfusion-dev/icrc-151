use crate::types::{TokenId, AccountKey};
use ic_stable_structures::Storable;
use std::borrow::Cow;
use candid::CandidType;


#[repr(C)]
#[derive(Clone, Copy, Debug, CandidType)]
pub struct StoredTxV1 {
    pub op: u8,
    pub flags: u8,
    pub token_id: [u8; 32],
    pub from_key: [u8; 32],
    pub to_key: [u8; 32],
    pub spender_key: [u8; 32],
    pub amount: [u8; 16],
    pub fee: [u8; 16],
    pub timestamp: [u8; 8],
    pub memo: [u8; 32],
    pub _reserved: [u8; 54],
}


const _: () = assert!(std::mem::size_of::<StoredTxV1>() == 256);


pub const FLAG_HAS_FEE: u8 = 1;
pub const FLAG_HAS_MEMO: u8 = 2;
pub const FLAG_HAS_SPENDER: u8 = 4;
pub const FLAG_MEMO_EXTENDED: u8 = 8;

impl StoredTxV1 {

    pub fn new_transfer(
        token_id: TokenId,
        from_key: AccountKey,
        to_key: AccountKey,
        amount: u128,
        fee: u128,
        timestamp: u64,
        memo: Option<&[u8]>,
    ) -> Self {
        let mut tx = Self {
            op: 0,
            flags: 0,
            token_id,
            from_key,
            to_key,
            spender_key: [0; 32],
            amount: amount.to_le_bytes(),
            fee: fee.to_le_bytes(),
            timestamp: timestamp.to_le_bytes(),
            memo: [0; 32],
            _reserved: [0; 54],
        };

        if fee > 0 {
            tx.flags |= FLAG_HAS_FEE;
        }

        if let Some(memo_bytes) = memo {
            tx.flags |= FLAG_HAS_MEMO;
            let copy_len = memo_bytes.len().min(32);
            tx.memo[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
            
            if memo_bytes.len() > 32 {
                tx.flags |= FLAG_MEMO_EXTENDED;
            }
        }

        tx
    }


    pub fn new_mint(
        token_id: TokenId,
        to_key: AccountKey,
        amount: u128,
        timestamp: u64,
        memo: Option<&[u8]>,
    ) -> Self {
        let mut tx = Self {
            op: 1,
            flags: 0,
            token_id,
            from_key: [0; 32],
            to_key,
            spender_key: [0; 32],
            amount: amount.to_le_bytes(),
            fee: [0; 16],
            timestamp: timestamp.to_le_bytes(),
            memo: [0; 32],
            _reserved: [0; 54],
        };

        if let Some(memo_bytes) = memo {
            tx.flags |= FLAG_HAS_MEMO;
            let copy_len = memo_bytes.len().min(32);
            tx.memo[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
            
            if memo_bytes.len() > 32 {
                tx.flags |= FLAG_MEMO_EXTENDED;
            }
        }

        tx
    }


    pub fn new_burn(
        token_id: TokenId,
        from_key: AccountKey,
        amount: u128,
        timestamp: u64,
        memo: Option<&[u8]>,
    ) -> Self {
        let mut tx = Self {
            op: 2,
            flags: 0,
            token_id,
            from_key,
            to_key: [0; 32],
            spender_key: [0; 32],
            amount: amount.to_le_bytes(),
            fee: [0; 16],
            timestamp: timestamp.to_le_bytes(),
            memo: [0; 32],
            _reserved: [0; 54],
        };

        if let Some(memo_bytes) = memo {
            tx.flags |= FLAG_HAS_MEMO;
            let copy_len = memo_bytes.len().min(32);
            tx.memo[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
            
            if memo_bytes.len() > 32 {
                tx.flags |= FLAG_MEMO_EXTENDED;
            }
        }

        tx
    }


    pub fn new_approve(
        token_id: TokenId,
        owner_key: AccountKey,
        spender_key: AccountKey,
        amount: u128,
        fee: u128,
        timestamp: u64,
        memo: Option<&[u8]>,
    ) -> Self {
        let mut tx = Self {
            op: 3,
            flags: FLAG_HAS_SPENDER,
            token_id,
            from_key: owner_key,
            to_key: [0; 32],
            spender_key,
            amount: amount.to_le_bytes(),
            fee: fee.to_le_bytes(),
            timestamp: timestamp.to_le_bytes(),
            memo: [0; 32],
            _reserved: [0; 54],
        };

        if fee > 0 {
            tx.flags |= FLAG_HAS_FEE;
        }

        if let Some(memo_bytes) = memo {
            tx.flags |= FLAG_HAS_MEMO;
            let copy_len = memo_bytes.len().min(32);
            tx.memo[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
            
            if memo_bytes.len() > 32 {
                tx.flags |= FLAG_MEMO_EXTENDED;
            }
        }

        tx
    }


    pub fn new_transfer_from(
        token_id: TokenId,
        from_key: AccountKey,
        to_key: AccountKey,
        spender_key: AccountKey,
        amount: u128,
        fee: u128,
        timestamp: u64,
        memo: Option<&[u8]>,
    ) -> Self {
        let mut tx = Self {
            op: 4,
            flags: FLAG_HAS_SPENDER,
            token_id,
            from_key,
            to_key,
            spender_key,
            amount: amount.to_le_bytes(),
            fee: fee.to_le_bytes(),
            timestamp: timestamp.to_le_bytes(),
            memo: [0; 32],
            _reserved: [0; 54],
        };

        if fee > 0 {
            tx.flags |= FLAG_HAS_FEE;
        }

        if let Some(memo_bytes) = memo {
            tx.flags |= FLAG_HAS_MEMO;
            let copy_len = memo_bytes.len().min(32);
            tx.memo[..copy_len].copy_from_slice(&memo_bytes[..copy_len]);
            
            if memo_bytes.len() > 32 {
                tx.flags |= FLAG_MEMO_EXTENDED;
            }
        }

        tx
    }


    pub fn get_amount(&self) -> u128 {
        u128::from_le_bytes(self.amount)
    }


    pub fn get_fee(&self) -> u128 {
        u128::from_le_bytes(self.fee)
    }


    pub fn get_timestamp(&self) -> u64 {
        u64::from_le_bytes(self.timestamp)
    }


    pub fn has_fee(&self) -> bool {
        self.flags & FLAG_HAS_FEE != 0
    }


    pub fn has_memo(&self) -> bool {
        self.flags & FLAG_HAS_MEMO != 0
    }


    pub fn has_spender(&self) -> bool {
        self.flags & FLAG_HAS_SPENDER != 0
    }


    pub fn has_extended_memo(&self) -> bool {
        self.flags & FLAG_MEMO_EXTENDED != 0
    }


    pub fn to_bytes(&self) -> [u8; 256] {
        let mut buf = [0u8; 256];
        buf[0] = self.op;
        buf[1] = self.flags;
        buf[2..34].copy_from_slice(&self.token_id);
        buf[34..66].copy_from_slice(&self.from_key);
        buf[66..98].copy_from_slice(&self.to_key);
        buf[98..130].copy_from_slice(&self.spender_key);
        buf[130..146].copy_from_slice(&self.amount);
        buf[146..162].copy_from_slice(&self.fee);
        buf[162..170].copy_from_slice(&self.timestamp);
        buf[170..202].copy_from_slice(&self.memo);
        buf[202..256].copy_from_slice(&self._reserved);
        buf
    }
    

    pub fn from_bytes(buf: &[u8; 256]) -> Self {
        let mut tx = Self {
            op: buf[0],
            flags: buf[1],
            token_id: [0; 32],
            from_key: [0; 32],
            to_key: [0; 32],
            spender_key: [0; 32],
            amount: [0; 16],
            fee: [0; 16],
            timestamp: [0; 8],
            memo: [0; 32],
            _reserved: [0; 54],
        };
        
        tx.token_id.copy_from_slice(&buf[2..34]);
        tx.from_key.copy_from_slice(&buf[34..66]);
        tx.to_key.copy_from_slice(&buf[66..98]);
        tx.spender_key.copy_from_slice(&buf[98..130]);
        tx.amount.copy_from_slice(&buf[130..146]);
        tx.fee.copy_from_slice(&buf[146..162]);
        tx.timestamp.copy_from_slice(&buf[162..170]);
        tx.memo.copy_from_slice(&buf[170..202]);
        tx._reserved.copy_from_slice(&buf[202..256]);
        
        tx
    }
}

impl Storable for StoredTxV1 {
    const BOUND: ic_stable_structures::storable::Bound = 
        ic_stable_structures::storable::Bound::Bounded { 
            max_size: 256, 
            is_fixed_size: true 
        };
    
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        Cow::Owned(self.to_bytes().to_vec())
    }
    
    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        let buf: [u8; 256] = bytes.as_ref().try_into()
            .expect("StoredTxV1 must be exactly 256 bytes");
        Self::from_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stored_tx_size() {
        assert_eq!(std::mem::size_of::<StoredTxV1>(), 256);
    }

    #[test]
    fn test_transfer_creation() {
        let token_id = [1u8; 32];
        let from_key = [2u8; 32];
        let to_key = [3u8; 32];
        let amount = 1000u128;
        let fee = 10u128;
        let timestamp = 1693564800000000000u64;

        let tx = StoredTxV1::new_transfer(
            token_id,
            from_key,
            to_key,
            amount,
            fee,
            timestamp,
            Some(b"test memo"),
        );

        assert_eq!(tx.op, 0);
        assert_eq!(tx.flags, FLAG_HAS_FEE | FLAG_HAS_MEMO);
        assert_eq!(tx.token_id, token_id);
        assert_eq!(tx.from_key, from_key);
        assert_eq!(tx.to_key, to_key);
        assert_eq!(tx.get_amount(), amount);
        assert_eq!(tx.get_fee(), fee);
        assert_eq!(tx.get_timestamp(), timestamp);
        assert!(tx.has_fee());
        assert!(tx.has_memo());
        assert!(!tx.has_spender());
        assert!(!tx.has_extended_memo());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let token_id = [1u8; 32];
        let from_key = [2u8; 32];
        let to_key = [3u8; 32];
        
        let tx = StoredTxV1::new_transfer(
            token_id,
            from_key,
            to_key,
            1000,
            10,
            1693564800000000000,
            Some(b"test"),
        );

        let bytes = tx.to_bytes();
        let tx2 = StoredTxV1::from_bytes(&bytes);

        assert_eq!(tx.op, tx2.op);
        assert_eq!(tx.flags, tx2.flags);
        assert_eq!(tx.token_id, tx2.token_id);
        assert_eq!(tx.from_key, tx2.from_key);
        assert_eq!(tx.to_key, tx2.to_key);
        assert_eq!(tx.amount, tx2.amount);
        assert_eq!(tx.fee, tx2.fee);
        assert_eq!(tx.timestamp, tx2.timestamp);
        assert_eq!(tx.memo, tx2.memo);
    }
}