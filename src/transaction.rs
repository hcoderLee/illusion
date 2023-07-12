use bincode::{config, Decode, Encode};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::rc::Rc;

use crate::block::{ByteData, Hash};
use crate::wallet::{extract_pub_key_hash, hash_pub_key};

const SUBSIDY: u64 = 50;

/// Transaction are composed of inputs and outputs, one input must refer to a output in another
/// transaction, the generated output may have no inputs referred to
#[derive(Encode, Decode)]
pub struct Transaction {
    pub id: Hash,
    pub v_in: Vec<Rc<TXInput>>,
    pub v_out: Vec<Rc<TXOutput>>,
}

impl Transaction {
    /// Create a coinbase transaction, which will be inserted at start of each block, it's have no
    /// referred outputs (it's only have one empty input), the generated output is rewards for miners
    pub fn new_coinbase_tx(to: &str, data: Option<String>) -> Self {
        let data = data.unwrap_or(format!("Reword to {}", to));
        let tx_in = vec![Rc::new(TXInput {
            tx_id: None,
            v_out_idx: None,
            signature: None,
            pub_key: Vec::from(data),
        })];
        let tx_out = vec![Rc::new(TXOutput::new(SUBSIDY, to))];
        Self {
            id: hash_transaction(&tx_in, &tx_out),
            v_in: tx_in,
            v_out: tx_out,
        }
    }

    /// Determine whether it is a coinbase transaction
    pub fn is_coinbase_tx(&self) -> bool {
        self.v_in.len() == 1 && self.v_in[0].tx_id.is_none()
    }
}

pub fn hash_transaction(v_in: &Vec<Rc<TXInput>>, v_out: &Vec<Rc<TXOutput>>) -> Hash {
    let mut hasher = Sha256::new();
    let config = config::standard();
    let inputs = bincode::encode_to_vec(v_in, config).expect("Can not encode transaction inputs");
    hasher.update(inputs);
    let outputs =
        bincode::encode_to_vec(v_out, config).expect("Can not encode transaction outputs");
    hasher.update(outputs);
    hasher.finalize().try_into().unwrap()
}

/// Transaction input
#[derive(Encode, Decode)]
pub struct TXInput {
    /// The id of referenced transaction
    pub tx_id: Option<Hash>,
    /// The index of referenced output in transaction outputs
    pub v_out_idx: Option<usize>,
    pub signature: Option<ByteData>,
    pub pub_key: ByteData,
}

impl TXInput {
    /// Check whether a input was uses a specific key to unlock an output
    pub fn use_key(&self, pub_key_hash: &[u8]) -> bool {
        let locking_hash = hash_pub_key(self.pub_key.as_ref());
        pub_key_hash == locking_hash
    }
}

/// Transaction output
#[derive(Encode, Decode)]
pub struct TXOutput {
    /// The amount of "coin" stored in output, and it's indivisible
    pub value: u64,
    pub pub_key_hash: ByteData,
}

impl TXOutput {
    pub fn new(value: u64, address: &str) -> Self {
        Self {
            value,
            pub_key_hash: extract_pub_key_hash(address),
        }
    }

    /// Check whether provided public key hash was used to lock the output
    pub fn is_locking_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash == pub_key_hash
    }
}

/// Unspent transaction outputs, key is transaction id, value is unspend output and it's index in
/// this transaction
pub type UTXO = HashMap<Hash, Vec<(Rc<TXOutput>, usize)>>;
