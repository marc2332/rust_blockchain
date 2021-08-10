use crate::{Block, BlockchainErrors};

#[derive(Clone)]
pub struct Configuration {
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

impl Default for Configuration {
    fn default() -> Self {
        Self::new()
    }
}

fn order_chain(chain: &[Block]) -> Vec<Block> {
    let mut ordered_list = chain.to_owned();

    for block in chain {
        ordered_list[block.index.unwrap() - 1] = block.clone();
    }

    ordered_list
}
