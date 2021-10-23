use crate::{
    Block,
    BlockchainErrors,
    Wallet,
};

use futures_util::{
    stream::StreamExt,
    TryStreamExt,
};
use mongodb::{
    bson::doc,
    options::{
        ClientOptions,
        ServerAddress,
    },
    Client,
};

#[derive(Clone, Debug)]
pub struct Configuration {
    pub id: u16,
    pub mongo_client: Client,
    pub rpc_port: u16,
    pub rpc_ws_port: u16,
    pub hostname: String,
    pub wallet: Wallet,
    pub transaction_threads: u16,
    pub chain_name: String,
}

impl Configuration {
    pub fn new() -> Self {
        let mut client_options = ClientOptions::default();
        client_options.hosts = vec![ServerAddress::Tcp {
            host: "localhost".to_string(),
            port: Some(27017),
        }];

        let mongo_client = Client::with_options(client_options).unwrap();

        Self {
            id: 0,
            mongo_client,
            rpc_port: 2000,
            rpc_ws_port: 7000,
            hostname: "0.0.0.0".to_string(),
            wallet: Wallet::default(),
            transaction_threads: 2,
            chain_name: "mars".to_string(),
        }
    }

    pub fn from_params(
        id: u16,
        rpc_port: u16,
        rpc_ws_port: u16,
        hostname: &str,
        wallet: Wallet,
        transaction_threads: u16,
        chain_name: &str,
    ) -> Self {
        let mut client_options = ClientOptions::default();
        client_options.hosts = vec![ServerAddress::Tcp {
            host: "localhost".to_string(),
            port: Some(27017),
        }];

        let mongo_client = Client::with_options(client_options).unwrap();

        Self {
            id,
            mongo_client,
            rpc_port,
            rpc_ws_port,
            hostname: hostname.to_string(),
            wallet,
            transaction_threads,
            chain_name: chain_name.to_string(),
        }
    }

    /*
     * Get all the blocks on the blockchain
     */
    pub async fn get_blocks(&self) -> Result<Vec<Block>, BlockchainErrors> {
        let mut chain = Vec::new();

        let id = self.id;
        let db = self.mongo_client.database(&format!("db_{}", id));

        // Blocks tree
        let blocks = db.collection::<Block>("blocks");
        let mut cursor = blocks.find(None, None).await.unwrap();

        while let Some(Ok(block)) = cursor.next().await {
            if block.verify_integrity().is_ok() {
                chain.push(block)
            } else {
                return Err(BlockchainErrors::InvalidHash);
            }
        }
        chain = order_chain(&chain);
        Ok(chain)
    }

    /*
     * Add a block to the database
     */
    pub fn add_block(&mut self, block: &Block) {
        let block = block.clone();

        let id = self.id;
        let mongo_client = self.mongo_client.clone();

        tokio::spawn(async move {
            let db = mongo_client.database(&format!("db_{}", id));

            let coll = db.collection::<Block>("blocks");

            coll.insert_one(block, None).await.ok();
        });
    }

    pub async fn get_block_with_prev_hash(&self, prev_hash: String) -> Option<Block> {
        let db = self.mongo_client.database(&format!("db_{}", self.id));

        let coll = db.collection::<Block>("blocks");

        let mut cursor = coll
            .find(
                doc! {
                    "prev_hash.hash": prev_hash
                },
                None,
            )
            .await
            .unwrap();

        match cursor.try_next().await {
            Ok(block) => block,
            _ => None,
        }
    }

    pub async fn get_block_with_hash(&self, hash: String) -> Option<Block> {
        let db = self.mongo_client.database(&format!("db_{}", self.id));

        let coll = db.collection::<Block>("blocks");
        let mut cursor = coll
            .find(
                doc! {
                    "hash.hash": hash
                },
                None,
            )
            .await
            .unwrap();

        match cursor.try_next().await {
            Ok(block) => block,
            _ => None,
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
