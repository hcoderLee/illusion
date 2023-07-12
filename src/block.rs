use bincode::{config, Decode, Encode};
use std::fmt::{Display, Formatter};

use crate::pow::pow;
use crate::tools::{get_timestamp, hash2str};
use crate::transaction::Transaction;

pub type ByteData = Vec<u8>;
pub type Hash = [u8; 32];
pub type TimeStamp = u128;

#[derive(Encode, Decode)]
pub struct Block {
    /// Block created time
    pub timestamp: TimeStamp,
    /// The transactions recorded in this block
    pub transactions: Vec<Transaction>,
    /// Previous block hash
    pub prev_block_hash: Option<Hash>,
    /// Hash of the block
    pub hash: Hash,
    /// Random number to participate in hash calculation
    pub nonce: u64,
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let prev_hash_str = match self.prev_block_hash {
            Some(ref hash) => hash2str(hash),
            None => String::from("None"),
        };
        let mut transactions = String::from("\n");
        for t in &self.transactions {
            transactions += format!("\t{}", hash2str(&t.id)).as_str();
        }
        write!(
            f,
            "timestamp: {}\nprevious block hash: {}\ntransactions: {}\nhash: {}",
            self.timestamp,
            prev_hash_str,
            transactions,
            hash2str(&self.hash),
        )
    }
}

impl Block {
    /// Create a new block
    pub fn new(transactions: Vec<Transaction>, prev_block_hash: Option<Hash>) -> Self {
        let timestamp = get_timestamp();
        let (hash, nonce) = pow(timestamp, &transactions, &prev_block_hash);
        Self {
            timestamp,
            transactions,
            prev_block_hash,
            hash,
            nonce,
        }
    }

    /// Create a genesis block
    pub fn new_genesis_block(coinbase: Transaction) -> Self {
        Self::new(vec![coinbase], None)
    }

    /// Serialize block to bytes
    pub fn encode(&self) -> Vec<u8> {
        let config = config::standard();
        bincode::encode_to_vec(self, config).expect("Can not encode Block to byte data")
    }

    pub fn decode(data: Vec<u8>) -> Self {
        let config = config::standard();
        let (block, _): (Block, usize) = bincode::decode_from_slice(data.as_slice(), config)
            .expect("Can not decode bytes to Block");
        block
    }
}

#[cfg(test)]
mod block_test {
    use super::*;
}
