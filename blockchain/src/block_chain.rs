use std::sync::{
    Arc,
    Mutex,
};

use crate::{
    Block,
    BlockHash,
    Chainstate,
    Configuration,
    Metrics,
    PublicAddress,
    Transaction,
};

#[derive(Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub index: usize,
    pub last_block_hash: Option<BlockHash>,
    pub config: Arc<Mutex<Configuration>>,
    pub state: Chainstate,
    pub metrics: Arc<Mutex<Metrics>>,
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
    pub async fn new(config: Configuration, metrics: Arc<Mutex<Metrics>>) -> Self {
        let chain = config.get_blocks().await.unwrap();

        tracing::info!("(Node.{}) Loaded blockchain from database", config.id);

        let index = chain.len() as usize;

        let last_block_hash = if !chain.is_empty() {
            Some(chain[chain.len() - 1].hash.clone())
        } else {
            None
        };

        // Make sure the integrity of the chain is OK
        assert!(verify_integrity(&chain).is_ok());

        let config = Arc::new(Mutex::new(config));

        let mut state = Chainstate::new(config.clone());

        state.load_from_chain().await;

        Self {
            chain,
            index,
            last_block_hash,
            config,
            state,
            metrics,
        }
    }

    /*
     * Append a block to the chain
     */
    pub fn add_block(&mut self, block: &Block) -> Result<(), BlockchainErrors> {
        let mut block = block.clone();
        block.index = Some(self.index + 1);

        let transactions = &block.transactions;

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
                    tracing::warn!(
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
            self.config.lock().unwrap().add_block(&block);

            // Update chainstate with the new transactions
            for tx in transactions.iter() {
                self.state.effect_transaction(tx);
            }

            self.index += 1;

            // Keep the chain at length 1 for now
            if !self.chain.is_empty() {
                self.chain.remove(0);
            }

            self.chain.push(block.clone());

            self.last_block_hash = Some(block.hash.clone());

            // Announce to the metric listeners a new block was created
            self.metrics.lock().unwrap().new_block(block.clone());

            tracing::info!(
                "(Node.{}) Added block [{}] -> {:?} (size of {})",
                self.config.lock().unwrap().id,
                self.index,
                block.hash.unite(),
                transactions.len()
            );
            Ok(())
        } else {
            tracing::error!(
                "(Node.{}) Couldn't add the block to the database.",
                self.config.lock().unwrap().id
            );
            Err(BlockchainErrors::CouldntAddBlock(block.hash.unite()))
        }
    }

    /*
     * Return the chain iterator
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

    /*
     * Get a block that hash the same previous hash
     */
    pub async fn get_block_with_prev_hash(&self, prev_hash: String) -> Option<Block> {
        self.config
            .lock()
            .unwrap()
            .get_block_with_prev_hash(prev_hash)
            .await
    }

    /*
     * Get a block by it's corresponding hash
     */
    pub async fn get_block_with_hash(&self, hash: String) -> Option<Block> {
        self.config.lock().unwrap().get_block_with_hash(hash).await
    }
}

/*
 * This iterates over a chain of blocks and makes sure that all the blocks and transactions are correct
 * This only makes sense to be run on the node startup to make sure the DB has not been modified
 */
fn verify_integrity(chain: &[Block]) -> Result<(), BlockchainErrors> {
    for (i, block) in chain.iter().enumerate() {
        let block_hash = &block.hash;
        let block_author = block.key.hash_it();
        let block_txs = &block.transactions;

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
                    previous_hash.hash.to_string(),
                    previous_block.hash.hash.to_string(),
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
