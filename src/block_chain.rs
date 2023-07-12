use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use rusty_leveldb::{Options, DB};

use crate::block::{Block, Hash};
use crate::tools::hash2str;
use crate::transaction::{hash_transaction, TXInput, TXOutput, Transaction, UTXO};
use crate::wallet::{extract_pub_key_hash, Wallets};

pub struct BlockChain {
    /// The hash value of latest block
    pub tip: Hash,
    /// The database where store blocks
    db: DB,
}

const DB_FILE: &str = "blockchain";
const LATEST_HASH: &str = "l";
const GENESIS_COINBASE_DATA: &str = "";

impl BlockChain {
    pub fn create(address: String) -> Self {
        if Path::new(DB_FILE).exists() {
            panic!("Blockchain database {} already exists", DB_FILE);
        }
        // Create database file
        let opt = Options::default();
        let mut db = DB::open(DB_FILE, opt).unwrap();
        // Create genesis block
        let coinbase = Transaction::new_coinbase_tx(
            address.as_str(),
            Some(String::from(GENESIS_COINBASE_DATA)),
        );
        let genesis = Block::new_genesis_block(coinbase);
        println!("Create genesis block success: {}", genesis);
        // Save genesis block in database
        db.put_block(&genesis);
        db.put_hash(LATEST_HASH, &genesis.hash);
        let tip = genesis.hash;
        Self { db, tip }
    }

    pub fn get() -> Option<Self> {
        if !Path::new(DB_FILE).exists() {
            println!("Blockchain database {} not exists", DB_FILE);
            return None;
        }
        let opt = Options::default();
        let mut db = DB::open(DB_FILE, opt).unwrap();
        db.get_hash(LATEST_HASH).map(|tip| Self { db, tip })
    }

    /// Add a new block to the chain
    pub fn mine_block(&mut self, transactions: Vec<Transaction>) {
        println!("Add new block, mining...");
        // Get hash value of latest block
        let last_hash = self
            .db
            .get_hash(LATEST_HASH)
            .expect("Add block failed, there were no blocks");
        // Create and save block
        let new_block = Block::new(transactions, Some(last_hash));
        println!("Add block success:\n{}", new_block);
        self.db.put_block(&new_block);
        // Update latest hash value for blockchain and database
        self.tip = new_block.hash;
        self.db.put_hash(LATEST_HASH, &new_block.hash);
    }

    /// Print all of the blocks of the chain
    pub fn print_chain(&mut self) {
        let iter = BlockChainIter::new(self);
        for block in iter {
            println!("{}\n", block);
        }
    }

    /// Find unspent transaction outputs for specific address
    pub fn find_utxo(&mut self, pub_key_hash: &[u8]) -> UTXO {
        // Unspent transaction outputs
        let mut utxo: UTXO = HashMap::new();
        // Spent transaction outputs, key is transaction id, value is a set of spent output index
        let mut stxo: HashMap<Hash, HashSet<usize>> = HashMap::new();
        // Iterate each block in chain
        for b in BlockChainIter::new(self) {
            // Iterate each transaction in block
            for tx in b.transactions {
                // Iterate each output in transaction, collect unspent outputs
                for (i, txo) in tx.v_out.iter().enumerate() {
                    // Whether the output is spent
                    let is_spent = match stxo.get(&tx.id) {
                        Some(idx_set) => idx_set.contains(&i),
                        None => false,
                    };
                    // Whether the output can be unlocked by your address
                    let can_unlock = txo.is_locking_with_key(pub_key_hash);

                    if !is_spent && can_unlock {
                        // Collect your unspent outputs
                        let output = (Rc::clone(txo), i);
                        match utxo.entry(tx.id) {
                            Occupied(o) => {
                                o.into_mut().push(output);
                            }
                            Vacant(v) => {
                                v.insert(vec![output]);
                            }
                        };
                    }
                }

                if tx.is_coinbase_tx() {
                    continue;
                }

                // Iterate each input, collect spent outputs
                for txi in tx.v_in {
                    let can_unlock = txi.use_key(pub_key_hash);
                    if can_unlock {
                        // Collect your spent outputs
                        let out_idx = txi.v_out_idx.unwrap();
                        match stxo.entry(txi.tx_id.unwrap()) {
                            Occupied(o) => {
                                o.into_mut().insert(out_idx);
                            }
                            Vacant(v) => {
                                v.insert(HashSet::from([out_idx]));
                            }
                        };
                    }
                }
            }
        }
        utxo
    }

    /// New transaction, send `amount` of value from `from` to `to`
    pub fn new_tx(&mut self, from: &str, to: &str, amount: u64) -> Result<Transaction, String> {
        // Find minimum set of unspent outputs to transfer amount value
        let (utxo, valid_amount) = self.find_spendable_outputs(from, amount);
        if valid_amount < amount {
            return Err(format!(
                "Cannot transfer {} from {} to {}, not enough funds",
                amount, from, to
            ));
        }
        let mut inputs: Vec<Rc<TXInput>> = Vec::new();

        // Create inputs
        let wallets = Wallets::new();
        let wallet = wallets
            .get_wallet(from)
            .expect(format!("Can not get wallet for address {}", from).as_str());
        for (txid, idx_set) in utxo {
            for idx in idx_set {
                let input = TXInput {
                    tx_id: Some(txid),
                    v_out_idx: Some(idx),
                    pub_key: Vec::from(wallet.public_key()),
                    signature: Some(Vec::from("not implemented yet".as_bytes())),
                };
                inputs.push(Rc::new(input));
            }
        }
        // Create outputs
        let mut outputs = Vec::new();
        // Create output for `to` address
        let out1 = TXOutput::new(amount, to);
        outputs.push(Rc::new(out1));
        if valid_amount > amount {
            // A change for `from` address
            let out2 = TXOutput::new(valid_amount - amount, from);
            outputs.push(Rc::new(out2));
        }
        // Transaction hash
        let tx_hash = hash_transaction(&inputs, &outputs);

        Ok(Transaction {
            id: tx_hash,
            v_in: inputs,
            v_out: outputs,
        })
    }

    /// Find balance of address `addr`
    pub fn get_balance(&mut self, addr: &str) -> u64 {
        let mut balance = 0u64;
        let pub_key_hash = extract_pub_key_hash(addr);
        let utxo = self.find_utxo(pub_key_hash.as_slice());
        for (_, outs) in utxo.iter() {
            for (out, _) in outs {
                balance += out.value;
            }
        }
        balance
    }

    /// Find the unspent outputs of `address` which it's accumulated value are just bigger than amount
    ///
    /// Returns a tuple:
    /// The first element is a map, which key is transaction id, value is unspent
    /// outputs index array.
    /// The second element is accumulated value of unspent outputs in first element
    fn find_spendable_outputs(
        &mut self,
        address: &str,
        amount: u64,
    ) -> (HashMap<Hash, Vec<usize>>, u64) {
        // Find all unspent outputs
        let pub_key_hash = extract_pub_key_hash(address);
        let all_utxo = self.find_utxo(pub_key_hash.as_slice());
        // Returned unspent outputs map
        let mut utxo: HashMap<Hash, Vec<usize>> = HashMap::new();
        // Accumulate outputs value
        let mut acc_value = 0u64;
        'outer: for (txid, tx_outs) in all_utxo {
            for (out, out_idx) in tx_outs {
                // Collect unspent output
                match utxo.entry(txid) {
                    Occupied(o) => {
                        o.into_mut().push(out_idx);
                    }
                    Vacant(v) => {
                        v.insert(vec![out_idx]);
                    }
                }
                acc_value += out.value;
                // Stop collect if accumulated value is just bigger than amount
                if acc_value >= amount {
                    break 'outer;
                }
            }
        }
        (utxo, acc_value)
    }
}

impl Drop for BlockChain {
    fn drop(&mut self) {
        self.db.close().expect("BlockChain close database error");
    }
}

trait BlockDB {
    /// Get hash value from database
    fn get_hash(&mut self, key: &str) -> Option<Hash>;

    /// Save hash value to database
    fn put_hash(&mut self, key: &str, hash: &Hash);

    /// Get block from database
    fn get_block(&mut self, hash: &Hash) -> Option<Block>;

    /// Save block to database
    fn put_block(&mut self, block: &Block);
}

impl BlockDB for DB {
    fn get_hash(&mut self, key: &str) -> Option<Hash> {
        if let Some(data) = self.get(key.as_bytes()) {
            let hash = Hash::try_from(data.as_slice())
                .unwrap_or_else(|_| panic!("Invalid hash value: {:?}", data));
            return Some(hash);
        }
        None
    }

    fn put_hash(&mut self, key: &str, hash: &Hash) {
        self.put(key.as_bytes(), hash.as_slice())
            .unwrap_or_else(|_| panic!("Can not save hash {} to {}", hash2str(hash), key));
    }

    fn get_block(&mut self, hash: &Hash) -> Option<Block> {
        if let Some(data) = self.get(hash.as_slice()) {
            let block = Block::decode(data);
            return Some(block);
        }
        None
    }

    fn put_block(&mut self, block: &Block) {
        self.put(block.hash.as_slice(), block.encode().as_slice())
            .unwrap_or_else(|_| panic!("Can not save Block {} to database", block));
    }
}

pub struct BlockChainIter<'a> {
    /// The hash value of current iterated bock
    pub cur_hash: Option<Hash>,
    /// Database store the blocks
    db: &'a mut DB,
}

impl<'a> BlockChainIter<'a> {
    pub fn new(block_chain: &'a mut BlockChain) -> Self {
        Self {
            cur_hash: Some(block_chain.tip),
            db: &mut block_chain.db,
        }
    }
}

impl<'a> Iterator for BlockChainIter<'a> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let cur_hash = self.cur_hash?;
        // Get block via current hash
        let block = self.db.get_block(&cur_hash);
        // Update the current hash to previous block
        self.cur_hash = if let Some(Block {
            prev_block_hash: Some(hash),
            ..
        }) = block
        {
            // Update current hash to previous block
            Some(hash)
        } else {
            None
        };
        block
    }
}
