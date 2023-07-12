use crate::block::{Hash, TimeStamp};
use crate::transaction::Transaction;
use sha2::{Digest, Sha256};

/// How many bits should be 0 in front of the hash value
// pub const TARGET_BITS: u8 = 24;
pub const TARGET_BITS: u8 = 1;

/// Proof of work algorithm, return the hash value which meet the requirements, and nonce value
pub fn pow(
    timestamp: TimeStamp,
    transactions: &Vec<Transaction>,
    prev_block_hash: &Option<Hash>,
) -> (Hash, u64) {
    let mut nonce = 1u64;
    let mut hash: Hash;
    loop {
        let mut hasher = Sha256::new()
            .chain_update(timestamp.to_string())
            .chain_update(hash_transactions(transactions));
        if let Some(pre_hash) = prev_block_hash {
            hasher.update(pre_hash);
        }
        // Nonce should be appended to the end (as bytes in little end order) to calculate hash value
        hasher.update(nonce.to_le_bytes());
        hash = hasher.finalize().try_into().unwrap();
        // Check if hash value is meet requirements
        if validate_hash(&hash) {
            break;
        }
        // Increase nonce until hash value is meet requirements
        match nonce.checked_add(1) {
            Some(new_nonce) => nonce = new_nonce,
            None => {
                // Overflow happen when increase nonce, which mean cannot find a valid hash value
                panic!("Can not find validate hash")
            }
        }
    }
    (hash, nonce)
}

fn hash_transactions(transactions: &Vec<Transaction>) -> Hash {
    let mut hasher = Sha256::new();
    for tx in transactions {
        hasher.update(tx.id);
    }
    hasher.finalize().try_into().unwrap()
}

/// Validate the hash value has meet the requirements, i.e. some bits in front of hash should be 0
pub fn validate_hash(hash: &Hash) -> bool {
    let mut checksum = 0u8;
    let mut count = TARGET_BITS;
    for byte in hash {
        if count >= 8 {
            checksum |= byte;
            if checksum != 0 {
                return false;
            }
            count -= 8;
        } else {
            if count > 0 {
                checksum |= byte >> (8 - count);
            }
            break;
        }
    }
    checksum == 0
}
