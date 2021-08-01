use chrono::prelude::*;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use rand::rngs::OsRng;
use rsa::pkcs1::ToRsaPublicKey;
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::collections::hash_map;
use std::collections::HashMap;
use std::hash::Hash;

static HASH_VERSION: u8 = 1;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct BlockHash {
    hash: String,
    version: u8,
}

impl BlockHash {
    pub fn new(
        payload: String,
        timestamp: String,
        previous_hash: Option<BlockHash>,
        address: String,
    ) -> Self {
        let mut hasher = Sha1::new();

        hasher.input_str(&HASH_VERSION.to_string());
        hasher.input_str(&payload);
        hasher.input_str(&timestamp);
        hasher.input_str(&address);

        if let Some(previous_hash) = previous_hash {
            hasher.input_str(&previous_hash.hash);
        }

        let hash = hasher.result_str();
        Self {
            hash,
            version: HASH_VERSION,
        }
    }
}

#[derive(Clone, Debug)]
struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String,
    pub address: String,
}

impl Block {
    pub fn new(
        payload: &str,
        timestamp: DateTime<Utc>,
        previous_hash: &Option<BlockHash>,
        address: &str,
    ) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        let address = address.to_string();
        let previous_hash = previous_hash.clone();
        Self {
            hash: BlockHash::new(
                payload.clone(),
                timestamp.clone(),
                previous_hash.clone(),
                address.clone(),
            ),
            timestamp,
            payload,
            previous_hash,
            address,
        }
    }
}

struct Blockchain {
    pub chain: HashMap<BlockHash, Block>,
    pub index: u64,
    pub last_block_hash: Option<BlockHash>,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: HashMap::new(),
            index: 0,
            last_block_hash: None,
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: Block) {
        self.chain.insert(block.hash.clone(), block.clone());
        self.index += 1;
        self.last_block_hash = Some(block.hash);
    }

    /*
     * Return HashMap's iterator
     */
    pub fn iter(&self) -> hash_map::Iter<BlockHash, Block> {
        self.chain.iter()
    }

    /*
     * Return the last block's hash if there is
     */
    pub fn peek(&self) -> Option<&Block> {
        if let Some(last_block_hash) = &self.last_block_hash {
            self.chain.get(&last_block_hash)
        } else {
            None
        }
    }
}

struct Address {
    pub public_address: String,
}

impl Address {
    /*
     * Just for testing
     */
    pub fn random() -> Self {
        let mut rng = OsRng;
        let bits = 2048;
        let private_key = RsaPrivateKey::new(&mut rng, bits).unwrap();
        let public_key = RsaPublicKey::from(&private_key);
        let mut hasher = Sha1::new();
        hasher.input_str(&public_key.to_pkcs1_pem().unwrap());
        let public_address = hasher.result_str();
        Self { public_address }
    }
}

fn main() {
    let mut blockchain = Blockchain::new();

    println!("Creating address...");

    let address_1 = Address::random();

    println!("Address: {} \n", address_1.public_address);

    blockchain.add_block(Block::new(
        "Block 1",
        Utc::now(),
        &None,
        &address_1.public_address,
    ));
    blockchain.add_block(Block::new(
        "Block 2",
        Utc::now(),
        &blockchain.last_block_hash,
        &address_1.public_address,
    ));
    blockchain.add_block(Block::new(
        "Block 3",
        Utc::now(),
        &blockchain.last_block_hash,
        &address_1.public_address,
    ));

    for (block_hash, block) in blockchain.iter() {
        let hash = &block_hash.hash;
        let timestamp = &block.timestamp;
        let address = &block.address;
        println!(
            "[{hash}] - {timestamp} - made by {address}",
            hash = hash,
            timestamp = timestamp,
            address = address
        );
    }

    println!(
        "\nLast block hash is {:?}",
        blockchain.peek().unwrap().hash.hash
    );
}
