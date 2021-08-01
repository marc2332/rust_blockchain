use std::collections::HashMap;
use std::collections::hash_map;
use std::hash::Hash;
use crypto::sha1::Sha1;
use crypto::digest::Digest;
use chrono::prelude::*;

static HASH_VERSION: u8 = 1;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct BlockHash {
    hash: String,
    version: u8
}

impl BlockHash {
    pub fn new(payload: String, timestamp: String, previous_hash: Option<BlockHash>) -> Self {
        let mut hasher = Sha1::new();
        
        hasher.input_str(&HASH_VERSION.to_string());
        hasher.input_str(&payload);
        hasher.input_str(&timestamp);

        if let Some(previous_hash) = previous_hash {
            hasher.input_str(&previous_hash.hash);
        }

        let hash = hasher.result_str();
        Self {
            hash,
            version: HASH_VERSION
        }
    }
}

#[derive(Clone, Debug)]
struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String
}

impl Block {
    pub fn new(payload: &str, timestamp: DateTime<Utc>, previous_hash: &Option<BlockHash>) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        let previous_hash = previous_hash.clone();
        Self {
            hash: BlockHash::new(payload.clone(), timestamp.clone(), previous_hash.clone()),
            timestamp,
            payload,
            previous_hash
        }
    }
}

struct Blockchain {
    pub chain: HashMap<BlockHash, Block>,
    pub index: u64,
    pub last_block_hash: Option<BlockHash>
}

impl Blockchain {
    pub fn new() -> Self {
        Self {
            chain: HashMap::new(),
            index: 0,
            last_block_hash: None
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: Block){
        &self.chain.insert(block.hash.clone(), block.clone());
        self.index += 1;
        self.last_block_hash = Some(block.hash.clone());
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

fn main() {
    let mut blockchain = Blockchain::new();

    blockchain.add_block(Block::new("Block 1", Utc::now(), &None));
    blockchain.add_block(Block::new("Block 2", Utc::now(), &blockchain.last_block_hash));
    blockchain.add_block(Block::new("Block 3", Utc::now(), &blockchain.last_block_hash));

    for (block_hash, block) in blockchain.iter(){
        let hash = &block_hash.hash;
        let timestamp = &block.timestamp;
        println!("[{hash}] - {timestamp}", hash=hash, timestamp=timestamp);
    }


    println!("Last block hash is {:?}", blockchain.peek().unwrap().hash);
}
