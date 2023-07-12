use bincode::{config, Decode, Encode};
use ring::rand;
use ring::signature::{Ed25519KeyPair, KeyPair, Signature, UnparsedPublicKey, ED25519};
use ripemd::Ripemd160;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;

use crate::block::ByteData;

/// Address version number
pub const ADDR_VERSION: u8 = 0;
/// Address checksum length
pub const ADDR_CHECKSUM_LEN: u8 = 4;

const WALLETS_FILE: &str = "wallets";

#[derive(Encode, Decode)]
pub struct Wallets {
    wallets: HashMap<String, ByteData>,
}

impl Wallets {
    pub fn new() -> Self {
        let wallets = Option::unwrap_or(Self::load(), HashMap::new());
        Self { wallets }
    }

    /// Create a new wallet, and return it's address
    pub fn create_wallet(&mut self) -> String {
        // Create a new wallet
        let key_pair = Wallet::create_key_pair();
        let wallet = Wallet::new(key_pair.as_slice());
        let address = wallet.get_address();

        // Save wallet to file
        self.wallets.insert(address.clone(), key_pair);
        if let Err(err) = self.save() {
            eprintln!("Save wallet error: {}", err)
        }

        address
    }

    /// Get addressed of saved wallets
    pub fn get_addresses(&self) -> Vec<String> {
        self.wallets.keys().map(String::clone).collect()
    }

    /// Get wallet for specified address
    pub fn get_wallet(&self, address: &str) -> Option<Wallet> {
        self.wallets
            .get(address)
            .map(|key_pair| Wallet::new(key_pair.as_slice()))
    }

    /// Load wallets data from file
    fn load() -> Option<HashMap<String, ByteData>> {
        // Open the file which save wallets data
        let mut file = match File::open(WALLETS_FILE) {
            Ok(f) => f,
            Err(err) => {
                println!("Open {} error: {}", WALLETS_FILE, err);
                return None;
            }
        };

        // Read wallets bytes data
        let mut raw_data = vec![];
        if let Err(err) = file.read_to_end(&mut raw_data) {
            println!("Read {} error: {}", WALLETS_FILE, err);
            return None;
        }

        // Convert bytes data to wallets instance
        let config = config::standard();
        let wallets = match bincode::decode_from_slice(raw_data.as_slice(), config) {
            Ok((wallets, _)) => wallets,
            Err(err) => {
                println!("Can not decode bytes to Wallets, {}", err);
                return None;
            }
        };
        Some(wallets)
    }

    /// Save wallets to file
    fn save(&self) -> Result<(), String> {
        // Create parent directory if it doesn't exist
        if let Some(p) = Path::new(WALLETS_FILE).parent() {
            if let Err(err) = create_dir_all(p) {
                return Err(format!("Create dir {} error: {}", p.to_string_lossy(), err));
            }
        }

        // Convert wallets struct instance to bytes
        let config = config::standard();
        let encoded = bincode::encode_to_vec(self, config);
        let bytes = match encoded {
            Ok(ref data) => data.as_slice(),
            Err(err) => return Err(format!("Encode wallets error: {}", err)),
        };

        // Save wallets bytes data to file
        if let Err(err) = File::create(WALLETS_FILE).and_then(|mut f| f.write_all(bytes)) {
            return Err(format!("Save wallets error: {}", err));
        }

        Ok(())
    }
}

pub struct Wallet {
    keypair: Ed25519KeyPair,
}

impl Wallet {
    /// Create key pair bytes data
    pub fn create_key_pair() -> ByteData {
        let rng = rand::SystemRandom::new();

        let pkcs8_bytes = match Ed25519KeyPair::generate_pkcs8(&rng) {
            Ok(v) => v,
            Err(err) => panic!("Generate key pair error: {}", err),
        };

        Vec::from(pkcs8_bytes.as_ref())
    }

    pub fn new(key_pair: &[u8]) -> Self {
        let keypair = Ed25519KeyPair::from_pkcs8(key_pair).unwrap();
        Self { keypair }
    }

    pub fn public_key(&self) -> &[u8] {
        self.keypair.public_key().as_ref()
    }

    pub fn sign(&self, data: &[u8]) -> Signature {
        self.keypair.sign(data)
    }

    pub fn verify(&self, data: &ByteData, signature: &[u8]) -> bool {
        let pub_key = UnparsedPublicKey::new(&ED25519, self.public_key());
        pub_key.verify(data, signature).is_ok()
    }

    /// Address are consists of three parts, version, public key hash, and checksum, the final
    /// address value is base58 encoded
    pub fn get_address(&self) -> String {
        // Bytes data of address version
        let version = ADDR_VERSION.to_le_bytes();
        // Public key
        let pub_key = self.keypair.public_key().as_ref();
        // The hash value of public key
        let pub_key_hash = hash_pub_key(pub_key);

        // Calculate checksum, it is former 4 bytes of SHA256(SHA256(version, public key)
        let hash1 = Sha256::new()
            .chain_update(version)
            .chain_update(pub_key_hash.as_slice())
            .finalize();
        let hash2 = Sha256::new().chain_update(hash1).finalize();
        let checksum = hash2[0..ADDR_CHECKSUM_LEN as usize].as_ref();

        // The address is a string base58 encode with version, public key hash and checksum
        bs58::encode([version.as_slice(), pub_key_hash.as_slice(), checksum].concat()).into_string()
    }
}

/// Calculate hash of the public key, it will be hashed twice with RIPEMD160(SHA256(public key))
pub fn hash_pub_key(pub_key: &[u8]) -> ByteData {
    let hash = Ripemd160::new()
        .chain_update(Sha256::new().chain_update(pub_key).finalize())
        .finalize();
    Vec::from(hash.as_slice())
}

/// Extract public key hash from address
pub fn extract_pub_key_hash(address: &str) -> ByteData {
    match bs58::decode(address).into_vec() {
        Ok(a) => {
            let start = 1;
            let end = a.len() - ADDR_CHECKSUM_LEN as usize;
            Vec::from(a[start..end].as_ref())
        }
        Err(err) => panic!("Decode address error: {}", err),
    }
}
