use std::sync::{
    mpsc,
    Arc,
    Mutex,
};

use crate::{
    Block,
    BlockchainErrors,
    Transaction,
    Wallet,
};
use std::{
    sync::mpsc::Sender,
    thread,
};

pub enum DbMessages {
    AddBlock(Block),
}

#[derive(Clone, Debug)]
pub struct Configuration {
    pub id: u16,
    pub db: Arc<Mutex<sled::Db>>,
    pub db_handler: Sender<DbMessages>,
    pub rpc_port: u16,
    pub hostname: String,
    pub wallet: Wallet,
    pub transaction_threads: u16,
    pub chain_memory_length: u16,
    pub chain_name: String,
}

fn create_block_handler(db: Arc<Mutex<sled::Db>>) -> Sender<DbMessages> {
    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                let db = db.clone();
                let rx = rx.lock().unwrap();
                let DbMessages::AddBlock(mut block) = rx.recv().unwrap();
                tokio::spawn(async move {
                    let block_txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

                    block.payload = "".to_string();

                    let all_blocks_tree = db
                        .lock()
                        .unwrap()
                        .open_tree("_chain_blocks_mars".as_bytes())
                        .unwrap();

                    all_blocks_tree
                        .insert(
                            &block.index.unwrap().to_string(),
                            serde_json::to_string(&block).unwrap().as_bytes(),
                        )
                        .unwrap();

                    let block_tree = db
                        .lock()
                        .unwrap()
                        .open_tree(format!("block_{}", block.hash.unite()).as_bytes())
                        .unwrap();

                    // https://github.com/spacejam/sled/issues/941
                    block_tree
                        .transaction::<_, (), ()>(|tx_db| {
                            for tx in &block_txs {
                                tx_db
                                    .insert(
                                        tx.get_hash().as_bytes(),
                                        serde_json::to_string(&tx).unwrap().as_bytes(),
                                    )
                                    .unwrap();
                            }
                            Ok(())
                        })
                        .unwrap();
                });
            }
        })
    });

    tx
}

impl Configuration {
    pub fn new() -> Self {
        let db = sled::Config::new()
            .path("db")
            .mode(sled::Mode::HighThroughput)
            .open()
            .unwrap();

        let db = Arc::new(Mutex::new(db));

        let tx = create_block_handler(db.clone());

        Self {
            id: 0,
            db,
            db_handler: tx,
            rpc_port: 2000,
            hostname: "0.0.0.0".to_string(),
            wallet: Wallet::default(),
            transaction_threads: 2,
            chain_memory_length: 20,
            chain_name: "mars".to_string(),
        }
    }

    pub fn from_params(
        id: u16,
        db_name: &str,
        rpc_port: u16,
        hostname: &str,
        wallet: Wallet,
        transaction_threads: u16,
        chain_memory_length: u16,
        chain_name: &str,
    ) -> Self {
        let db = sled::Config::new()
            .path(db_name)
            .mode(sled::Mode::HighThroughput)
            .flush_every_ms(Some(8000))
            .use_compression(true)
            .open()
            .unwrap();

        let db = Arc::new(Mutex::new(db));

        let tx = create_block_handler(db.clone());

        Self {
            id,
            db,
            db_handler: tx,
            rpc_port,
            hostname: hostname.to_string(),
            wallet,
            transaction_threads,
            chain_memory_length,
            chain_name: chain_name.to_string(),
        }
    }

    /*
     * Get all the blocks on the blockchain
     */
    pub fn get_blocks(&self) -> Result<Vec<Block>, BlockchainErrors> {
        let mut chain = Vec::new();

        // Blocks tree
        let blocks = self
            .db
            .lock()
            .unwrap()
            .open_tree("_chain_blocks_mars".as_bytes())
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
                    let mut block: Block = block;

                    block.payload = self.get_blocks_txs(&block.hash.unite());

                    if block.verify_integrity().is_ok() {
                        chain.push(block)
                    } else {
                        return Err(BlockchainErrors::InvalidHash);
                    }
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

    fn get_blocks_txs(&self, block_hash: &str) -> String {
        let mut txs = String::new();

        let block_txs = self
            .db
            .lock()
            .unwrap()
            .open_tree(format!("blocks{}", block_hash).as_bytes())
            .unwrap();

        for (_, block_tx) in block_txs.iter().flatten() {
            let tx = String::from_utf8(block_tx.to_vec()).unwrap();
            txs += &tx;
        }

        txs
    }

    /*
     * Add a block to the database
     */
    pub fn add_block(&mut self, block: &Block) -> Result<(), BlockchainErrors> {
        let mut block = block.clone();

        let db = self.db.clone();

        //self.db_handler.send(DbMessages::AddBlock(block)).unwrap();

        tokio::spawn(async move {
            let block_txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

            block.payload = "".to_string();

            let all_blocks_tree = db
                .lock()
                .unwrap()
                .open_tree("_chain_blocks_mars".as_bytes())
                .unwrap();

            all_blocks_tree
                .insert(
                    &block.index.unwrap().to_string(),
                    serde_json::to_string(&block).unwrap().as_bytes(),
                )
                .unwrap();

            let block_tree = db
                .lock()
                .unwrap()
                .open_tree(format!("block_{}", block.hash.unite()).as_bytes())
                .unwrap();

            // https://github.com/spacejam/sled/issues/941
            block_tree
                .transaction::<_, (), ()>(|tx_db| {
                    for tx in &block_txs {
                        tx_db
                            .insert(
                                tx.get_hash().as_bytes(),
                                serde_json::to_string(&tx).unwrap().as_bytes(),
                            )
                            .unwrap();
                    }
                    Ok(())
                })
                .unwrap();
        });

        Ok(())

        /* Err(BlockchainErrors::CouldntAddBlock(
               block.hash.hash.to_string(),
           ))
        */
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
