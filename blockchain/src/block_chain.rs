use std::sync::{
    Arc,
    Mutex,
};

use crate::{
    Block,
    BlockHash,
    Chainstate,
    Configuration,
    PublicAddress,
    Transaction,
};

#[derive(Clone)]
pub struct Blockchain {
    pub name: String,
    pub chain: Vec<Block>,
    pub index: usize,
    pub last_block_hash: Option<BlockHash>,
    pub config: Arc<Mutex<Configuration>>,
    pub state: Chainstate,
}

#[derive(Debug)]
pub enum BlockchainErrors {
    InvalidPrevioushHash(String, String),
    InvalidSignature,
    InvalidHash,
    CouldntLoadBlock(String),
    CouldntAddBlock(String),
    MultipleCoinbase(String),
    InvalidCoinbaseAddress(String),
    InvalidBlockForger(String),
}

impl Blockchain {
    pub fn new(name: &str, config: Arc<Mutex<Configuration>>) -> Self {
        let chain = config.lock().unwrap().get_blocks(name).unwrap();

        log::info!(
            "(Node.{}) Loaded blockchain from database",
            config.lock().unwrap().id
        );

        let index = chain.len() as usize;

        let last_block_hash = if !chain.is_empty() {
            Some(chain[chain.len() - 1].hash.clone())
        } else {
            None
        };

        let mut state = Chainstate::new(config.clone());

        state.load_from_chain(name);

        assert!(verify_integrity(&chain).is_ok());

        Self {
            name: name.to_string(),
            chain,
            index,
            last_block_hash,
            config,
            state,
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: &Block) -> Result<(), BlockchainErrors> {
        let mut block = block.clone();
        block.index = Some(self.index + 1);

        let transactions: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

        // Make sure that adding the block to the chain won't break it's integrity
        let block_can_be_added = {
            /*
             * If the last chain hash is the same as the new block hash that means that the block was already added,
             * so there is not need to verify the complete integrity of the chain
             */
            if self.last_block_hash.as_ref() == Some(&block.hash) {
                false
            } else if let Some(block_hash) = self.last_block_hash.as_ref() {
                if block_hash.clone() != block.clone().previous_hash.unwrap() {
                    log::warn!(
                        "(Node.{}) Tried to add a faulty block ({}) to the chain.",
                        self.config.lock().unwrap().id,
                        block.hash.unite()
                    );
                    false
                } else {
                    true
                }
            } else {
                true
            }
        };

        if block_can_be_added {
            // Add the block to the database
            let db_result = self.config.lock().unwrap().add_block(&block, &self.name);

            if db_result.is_ok() {
                // Update chainstate with the new transactions
                for tx in transactions.iter() {
                    self.state.effect_transaction(tx);
                }

                self.index += 1;
                self.chain.push(block.clone());
                self.last_block_hash = Some(block.hash.clone());
                log::info!(
                    "(Node.{}) Added block -> {:?}",
                    self.config.lock().unwrap().id,
                    block.hash.unite()
                );
                Ok(())
            } else {
                log::error!(
                    "(Node.{}) Couldn't add the block to the database.",
                    self.config.lock().unwrap().id
                );
                Err(BlockchainErrors::CouldntAddBlock(block.hash.unite()))
            }
        } else {
            Err(BlockchainErrors::CouldntAddBlock(block.hash.unite()))
        }
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
        verify_integrity(&self.chain)
    }

    pub fn get_block_with_prev_hash(&self, prev_hash: String) -> Option<Block> {
        for block in &self.chain {
            if let Some(block_prev_hash) = &block.previous_hash {
                if block_prev_hash.unite() == prev_hash {
                    return Some(block.clone());
                }
            }
        }
        None
    }

    pub fn get_block_with_hash(&self, hash: String) -> Option<Block> {
        for block in &self.chain {
            if block.hash.unite() == hash {
                return Some(block.clone());
            }
        }
        None
    }
}

fn verify_integrity(chain: &[Block]) -> Result<(), BlockchainErrors> {
    for (i, block) in chain.iter().enumerate() {
        let block_hash = &block.hash;
        let block_author = block.key.hash_it();
        let block_txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

        for (tx_i, tx) in block_txs.iter().enumerate() {
            if let Transaction::COINBASE { hash, .. } = tx {
                // There can only be one coinbase in each block
                if tx_i > 0 {
                    return Err(BlockchainErrors::MultipleCoinbase(hash.to_string()));
                }
            }

            if tx_i == 0 {
                // The first transaction must always be a coinbase rewarding the block creator
                if let Transaction::COINBASE {
                    to_address, hash, ..
                } = tx
                {
                    if to_address != &block_author {
                        return Err(BlockchainErrors::InvalidCoinbaseAddress(hash.clone()));
                    }
                } else {
                    return Err(BlockchainErrors::MultipleCoinbase(block_hash.unite()));
                }
            }
        }

        if i > 0 {
            let previous_block = &chain[i - 1];
            let previous_hash = block.previous_hash.as_ref().unwrap();

            // The previous hash must be the same as the previous block's hash
            if previous_hash.unite() != previous_block.hash.unite() {
                return Err(BlockchainErrors::InvalidPrevioushHash(
                    previous_hash.hash.clone(),
                    previous_block.hash.hash.clone(),
                ));
            }

            //It should also check if the block forger isn't the same as the previous one
            if previous_block.key == block.key {
                return Err(BlockchainErrors::InvalidBlockForger(block.hash.unite()));
            }
        }

        let block_signer = PublicAddress::from(&block.key);

        // The signature must be correct according the public key and the block data
        if !block.verify_sign_with(&block_signer) {
            return Err(BlockchainErrors::InvalidSignature);
        }
    }
    Ok(())
}
