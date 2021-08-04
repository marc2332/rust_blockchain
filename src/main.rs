use chrono::prelude::*;
use core::fmt;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::Rsa;
use openssl::sign::{Signer, Verifier};
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

#[derive(Clone, Debug)]
struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String,
    pub key: Key,
    pub signature: Key,
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
    pub chain: Vec<Block>,
    pub index: u64,
    pub last_block_hash: Option<BlockHash>,
}

#[derive(Debug)]
enum BlockchainIntegrityErrors {
    InvalidPrevioushHash(String, String),
    InvalidSignature,
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: Vec::new(),
            index: 0,
            last_block_hash: None,
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: &Block) {
        self.chain.push(block.clone());
        self.index += 1;
        self.last_block_hash = Some(block.hash.clone());
    }

    /*
     * Return HashMap's iterator
     */
    pub fn iter(&self) -> std::slice::Iter<Block> {
        self.chain.iter()
    }

    /*
     * Return the last block's hash if there is
     */
    pub fn peek(&self) -> Option<&Block> {
        self.chain.last()
    }

    /*
     * Verify the integrity of the blockchain
     */
    pub fn verify_integrity(&self) -> Result<(), BlockchainIntegrityErrors> {
        for (i, block) in self.chain.iter().enumerate() {
            if i > 0 {
                let previous_block = &self.chain[i - 1];

                /*
                 * The previous hash must be the same as the previous block's hash
                 */
                let previous_hash = block.previous_hash.as_ref().unwrap();
                if previous_hash != &previous_block.hash {
                    return Err(BlockchainIntegrityErrors::InvalidPrevioushHash(
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
                return Err(BlockchainIntegrityErrors::InvalidSignature);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
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
    let mut blockchain = Blockchain::new();

    let account_a = Wallet::new();
    let public_key = account_a.get_public();

    blockchain.add_block(
        &BlockBuilder::new()
            .payload("Block 1")
            .timestamp(Utc::now())
            .key(&public_key)
            .sign_with(&account_a)
            .build(),
    );

    for i in 0..99 {
        blockchain.add_block(
            &BlockBuilder::new()
                .payload(&format!("Block {:?}", i))
                .timestamp(Utc::now())
                .previous_hash(&blockchain.peek().unwrap().hash)
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
