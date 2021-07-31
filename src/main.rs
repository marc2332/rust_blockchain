use std::collections::HashMap;
use std::collections::hash_map;
use std::hash::Hash;
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use chrono::prelude::*;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct BlockHash {
    hash: String,
    version: u8
}

impl BlockHash {
    pub fn new(payload: String, timestamp: String) -> Self {
        let mut hasher = Sha1::new();
        
        hasher.input_str(&payload);
        hasher.input_str(&timestamp);

        let hash = hasher.result_str();
        Self {
            hash,
            version: 1
        }
    }
}

#[derive(Clone)]
struct Block {
    hash: BlockHash,
    timestamp: String
    // There are some missing fields
}

impl Block {
    pub fn new(payload: &str, timestamp: DateTime<Utc>) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        Self {
            hash: BlockHash::new(payload, timestamp.clone()),
            timestamp
        }
    }
}

struct Blockchain {
    chain: HashMap<BlockHash, Block>
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: HashMap::new()
        }
    }

    pub fn add_block(&mut self, block: Block){
        &self.chain.insert(block.hash.clone(), block.clone());
    }

    pub fn iter(&self) -> hash_map::Iter<BlockHash, Block> {
        self.chain.iter()
    }
}

fn main() {
    let mut blockchain = Blockchain::new();

    blockchain.add_block(Block::new("Block 1", Utc::now()));
    blockchain.add_block(Block::new("Block 2", Utc::now()));
    blockchain.add_block(Block::new("Block 3", Utc::now()));

    for (block_hash, block) in blockchain.iter(){
        let hash = &block_hash.hash;
        let timestamp = &block.timestamp;
        println!("[{hash}] - {timestamp}", hash=hash, timestamp=timestamp);
    }

}
