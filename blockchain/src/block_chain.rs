use std::sync::{
    Arc,
    Mutex,
};

use openssl::{
    pkey::PKey,
    rsa::Rsa,
};

use crate::{
    Block,
    BlockHash,
    Configuration,
    PublicAddress,
};

pub struct Blockchain {
    pub name: String,
    pub chain: Vec<Block>,
    pub index: usize,
    pub last_block_hash: Option<BlockHash>,
    pub config: Arc<Mutex<Configuration>>,
}

#[derive(Debug)]
pub enum BlockchainErrors {
    InvalidPrevioushHash(String, String),
    InvalidSignature,
    InvalidHash,
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
