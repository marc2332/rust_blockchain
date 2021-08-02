use chrono::prelude::*;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::rsa::Rsa;
use openssl::sign::{Signer, Verifier};
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

struct BlockBuilder {
    pub hash: Option<BlockHash>,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: Option<DateTime<Utc>>,
    pub payload: Option<String>,
    pub address: Option<String>,
    pub signature: Option<String>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            hash: None,
            previous_hash: None,
            timestamp: None,
            payload: None,
            address: None,
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

    pub fn address(&mut self, address: &str) -> &mut Self {
        self.address = Some(address.to_string());
        self
    }

    pub fn sign_with(&mut self, acc: &SignedInfo) -> &mut Self {
        // Terribly ugly, I know
        let data = format!(
            "{}{}{}{:?}",
            self.address.as_ref().unwrap(),
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
            &self.address.as_ref().unwrap(),
            &self.signature.as_ref().unwrap(),
        )
    }
}

#[derive(Clone, Debug)]
struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String,
    pub address: String,
    pub signature: String,
}

impl Block {
    pub fn new(
        payload: &str,
        timestamp: DateTime<Utc>,
        previous_hash: &Option<BlockHash>,
        address: &str,
        signature: &str,
    ) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        let address = address.to_string();
        let signature = signature.to_string();
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
            signature,
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

struct SignedInfo {
    pub keypair: PKey<Private>,
}

impl SignedInfo {
    pub fn new() -> Self {
        let keypair = Rsa::generate(515).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        Self { keypair }
    }

    pub fn sign_data(&self, data: String) -> String {
        let data = data.as_bytes();

        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        signer.update(data).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        signature
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(" ")
    }
    /*
    pub fn verify_signature(&self, signature: Vec<u8>) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(&self.data.as_bytes()).unwrap();
        verifier.verify(&signature).is_ok()
    }
    */
    pub fn get_public(&self) -> String {
        let public_key = self.keypair.public_key_to_pem().unwrap();
        let public_key = String::from_utf8(public_key).unwrap();
        let mut hasher = Sha1::new();
        hasher.input_str(&public_key);
        return hasher.result_str();
    }
}

fn main() {
    let mut blockchain = Blockchain::new();

    let account = SignedInfo::new();
    let public_address = account.get_public();

    blockchain.add_block(
        BlockBuilder::new()
            .payload("Block 1")
            .timestamp(Utc::now())
            .address(&public_address)
            .sign_with(&account)
            .build(),
    );

    blockchain.add_block(
        BlockBuilder::new()
            .payload("Block 2")
            .timestamp(Utc::now())
            .previous_hash(&blockchain.peek().unwrap().hash)
            .address(&public_address)
            .sign_with(&account)
            .build(),
    );

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
