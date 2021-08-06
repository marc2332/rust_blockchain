use chrono::prelude::*;
use core::fmt;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::Rsa;
use openssl::sign::{Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::sync::{Arc, Mutex};

static HASH_VERSION: u8 = 1;

#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
struct BlockHash {
    hash: String,
    version: u8,
}

impl BlockHash {
    pub fn new(
        payload: String,
        timestamp: String,
        previous_hash: Option<BlockHash>,
        key: Key,
    ) -> Self {
        let mut hasher = Sha1::new();

        hasher.input_str(&HASH_VERSION.to_string());
        hasher.input_str(&payload);
        hasher.input_str(&timestamp);
        hasher.input_str(&key.to_string());

        if let Some(previous_hash) = previous_hash {
            hasher.input_str(&previous_hash.hash);
        }

        let hash = hasher.result_str();
        Self {
            hash,
            version: HASH_VERSION,
        }
    }

    pub fn unite(&self) -> String {
        format!("{}x{}", self.version, self.hash)
    }
}

struct BlockBuilder {
    pub hash: Option<BlockHash>,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: Option<DateTime<Utc>>,
    pub payload: Option<String>,
    pub key: Option<Key>,
    pub signature: Option<Key>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            hash: None,
            previous_hash: None,
            timestamp: None,
            payload: None,
            key: None,
            signature: None,
        }
    }

    pub fn payload(&mut self, payload: &str) -> &mut Self {
        self.payload = Some(payload.to_string());
        self
    }

    pub fn previous_hash(&mut self, previous_hash: &BlockHash) -> &mut Self {
        self.previous_hash = Some(previous_hash.clone());
        self
    }

    pub fn timestamp(&mut self, timestamp: DateTime<Utc>) -> &mut Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn key(&mut self, key: &Key) -> &mut Self {
        self.key = Some(key.clone());
        self
    }

    pub fn sign_with(&mut self, acc: &Wallet) -> &mut Self {
        // Terribly ugly, I know
        let data = format!(
            "{}{}{}{:?}",
            self.key.as_ref().unwrap(),
            self.timestamp.as_ref().unwrap(),
            self.payload.as_ref().unwrap(),
            self.previous_hash
        );
        self.signature = Some(acc.sign_data(data));
        self
    }

    pub fn build(&self) -> Block {
        Block::new(
            &self.payload.as_ref().unwrap(),
            self.timestamp.unwrap(),
            &self.previous_hash,
            &self.key.as_ref().unwrap(),
            &self.signature.as_ref().unwrap(),
        )
    }
}

trait SignVerifier {
    fn verify_signature(&self, signature: &Key, data: String) -> bool;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String,
    pub key: Key,
    pub signature: Key,
    pub index: Option<usize>,
}

impl Block {
    pub fn new(
        payload: &str,
        timestamp: DateTime<Utc>,
        previous_hash: &Option<BlockHash>,
        key: &Key,
        signature: &Key,
    ) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        let signature = signature.clone();
        let previous_hash = previous_hash.clone();
        Self {
            hash: BlockHash::new(
                payload.clone(),
                timestamp.clone(),
                previous_hash.clone(),
                key.clone(),
            ),
            timestamp,
            payload,
            previous_hash,
            key: key.clone(),
            signature,
            index: None,
        }
    }

    pub fn verify_sign_with(&self, acc: &impl SignVerifier) -> bool {
        // Terribly ugly, I know
        let data = format!(
            "{}{}{}{:?}",
            self.key, self.timestamp, self.payload, self.previous_hash
        );

        acc.verify_signature(&self.signature, data)
    }
}

struct Blockchain {
    pub name: String,
    pub chain: Vec<Block>,
    pub index: usize,
    pub last_block_hash: Option<BlockHash>,
    pub config: Arc<Mutex<Configuration>>,
}

#[derive(Debug)]
enum BlockchainErrors {
    InvalidPrevioushHash(String, String),
    InvalidSignature,
    CouldntLoadBlock(String),
    CouldntAddBlock(String),
}

impl Blockchain {
    pub fn new(name: &str, config: Arc<Mutex<Configuration>>) -> Self {
        let chain = config.lock().unwrap().get_blocks(name).unwrap();

        let index = chain.len() as usize;

        let last_block_hash = if !chain.is_empty() {
            Some(chain[chain.len() - 1].hash.clone())
        } else {
            None
        };

        Self {
            name: name.to_string(),
            chain,
            index,
            last_block_hash,
            config,
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: &Block) {
        self.index += 1;

        let mut block = block.clone();
        block.index = Some(self.index);

        let db_result = self.config.lock().unwrap().add_block(&block, &self.name);

        if db_result.is_ok() {
            self.chain.push(block.clone());

            self.last_block_hash = Some(block.hash);
        } else {
            // WIP
            println!("error");
        }

        assert!(self.verify_integrity().is_ok());
    }

    /*
     * Return HashMap's iterator
     */
    pub fn iter(&self) -> std::slice::Iter<Block> {
        self.chain.iter()
    }

    /*
     * Return the last block's if there is
     */
    pub fn peek(&self) -> Option<&Block> {
        self.chain.last()
    }

    /*
     * Verify the integrity of the blockchain
     */
    pub fn verify_integrity(&self) -> Result<(), BlockchainErrors> {
        for (i, block) in self.chain.iter().enumerate() {
            if i > 0 {
                let previous_block = &self.chain[i - 1];

                /*
                 * The previous hash must be the same as the previous block's hash
                 */
                let previous_hash = block.previous_hash.as_ref().unwrap();

                if previous_hash.unite() != previous_block.hash.unite() {
                    return Err(BlockchainErrors::InvalidPrevioushHash(
                        previous_hash.hash.clone(),
                        previous_block.hash.hash.clone(),
                    ));
                }
            }

            let block_signer = PublicAddress {
                keypair: PKey::from_rsa(Rsa::public_key_from_pem(block.key.0.as_slice()).unwrap())
                    .unwrap(),
            };

            /*
             * The signature must be correct according the public key and the block data
             */
            if !block.verify_sign_with(&block_signer) {
                return Err(BlockchainErrors::InvalidSignature);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Key(Vec<u8>);

#[allow(dead_code)]
impl Key {
    pub fn hash_it(&self) -> String {
        let str_key = self.to_string();
        let mut hasher = Sha1::new();
        hasher.input_str(&str_key);
        hasher.result_str()
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}

struct Wallet {
    pub keypair: PKey<Private>,
}

impl SignVerifier for Wallet {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(data.as_bytes()).unwrap();
        verifier.verify(&signature.0).unwrap()
    }
}

impl Wallet {
    pub fn new() -> Self {
        let keypair = Rsa::generate(515).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        Self { keypair }
    }

    pub fn sign_data(&self, data: String) -> Key {
        let data = data.as_bytes();

        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        signer.update(data).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        Key(signature)
    }

    pub fn get_public(&self) -> Key {
        let public_key = self.keypair.public_key_to_pem().unwrap();
        Key(public_key)
    }
}

struct PublicAddress {
    keypair: PKey<Public>,
}

impl SignVerifier for PublicAddress {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(data.as_bytes()).unwrap();
        verifier.verify(&signature.0).unwrap()
    }
}

fn main() {
    let config = Arc::new(Mutex::new(Configuration::new()));

    let mut blockchain = Blockchain::new("mars", config);

    let account_a = Wallet::new();
    let public_key = account_a.get_public();

    if blockchain.last_block_hash.is_none() {
        blockchain.add_block(
            &BlockBuilder::new()
                .payload("block 1")
                .timestamp(Utc::now())
                .key(&public_key)
                .sign_with(&account_a)
                .build(),
        );
    }

    for i in 1..5 {
        blockchain.add_block(
            &BlockBuilder::new()
                .payload(&format!("Block {:?}", i))
                .timestamp(Utc::now())
                .previous_hash(&blockchain.last_block_hash.clone().unwrap())
                .key(&public_key)
                .sign_with(&account_a)
                .build(),
        );
    }

    let block_3 = BlockBuilder::new()
        .payload("Block 1")
        .timestamp(Utc::now())
        .key(&public_key)
        .sign_with(&account_a)
        .build();

    // Verifying the signing on the block should fail since this account hasn't signed it
    let account_b = Wallet::new();

    assert!(block_3.verify_sign_with(&account_a));
    assert!(!block_3.verify_sign_with(&account_b));

    for block in blockchain.iter() {
        let hash = &block.hash.hash;
        let timestamp = &block.timestamp;
        let key = &block.key;
        println!(
            "[{hash}] - {timestamp} - made by {key}",
            hash = hash,
            timestamp = timestamp,
            key = key.hash_it()
        );
    }

    assert!(blockchain.verify_integrity().is_ok());

    let public_account_a = PublicAddress {
        keypair: PKey::from_rsa(
            Rsa::public_key_from_pem(account_a.get_public().0.as_slice()).unwrap(),
        )
        .unwrap(),
    };

    assert!(block_3.verify_sign_with(&public_account_a));

    println!(
        "\nLast block hash is {:?}",
        blockchain.peek().unwrap().hash.hash
    );
}

#[derive(Deserialize, Serialize)]
struct BlockchainConfig {
    name: String,
}

struct Configuration {
    db: sled::Db,
}

impl Configuration {
    pub fn new() -> Self {
        let db = sled::open("db").unwrap();
        Self { db }
    }

    /*
     * Get all the blocks on the blockchain
     */
    pub fn get_blocks(&self, chain_name: &str) -> Result<Vec<Block>, BlockchainErrors> {
        let mut chain = Vec::new();

        // Blocks tree
        let blocks: sled::Tree = self
            .db
            .open_tree(format!("{}_chain_blocks", chain_name).as_bytes())
            .unwrap();

        // Get the first and the last block's hash
        if let Some((first_hash, _)) = blocks.first().unwrap() {
            // Get a range between the first and the last block (all blocks)
            let all_blocks = blocks.range(first_hash..);

            for block in all_blocks {
                let (block_hash, block) = block.unwrap();

                // Stringified block
                let block_info = String::from_utf8(block.to_vec()).unwrap();

                // Block serialized
                if let Ok(block) = serde_json::from_str(&block_info) {
                    let block: Block = block;
                    println!("Loaded block {}", &block.hash.unite());
                    chain.push(block)
                } else {
                    return Err(BlockchainErrors::CouldntLoadBlock(
                        String::from_utf8(block_hash.to_vec()).unwrap(),
                    ));
                }
            }
        }

        chain = order_chain(&chain);

        Ok(chain)
    }

    /*
     * Add a block to the database
     */
    pub fn add_block(&mut self, block: &Block, chain_name: &str) -> Result<(), BlockchainErrors> {
        let blocks: sled::Tree = self
            .db
            .open_tree(format!("{}_chain_blocks", chain_name).as_bytes())
            .unwrap();

        let result = blocks.insert(
            &block.index.unwrap().to_string(),
            serde_json::to_string(&block).unwrap().as_bytes(),
        );

        if result.is_ok() {
            Ok(())
        } else {
            Err(BlockchainErrors::CouldntAddBlock(block.hash.hash.clone()))
        }
    }
}

fn order_chain(chain: &[Block]) -> Vec<Block> {
    let mut ordered_list = chain.to_owned();

    for block in chain {
        ordered_list[block.index.unwrap() - 1] = block.clone();
    }

    ordered_list
}
