#![feature(async_closure)]
use std::sync::Arc;

use std::collections::HashMap;

use blockchain::{
    Block,
    Blockchain,
    Configuration,
    Wallet,
};

use futures::executor::block_on;
use jsonrpc_http_server::{
    jsonrpc_core::*,
    *,
};

use jsonrpc_derive::rpc;

pub mod mempool;
pub mod methods;

use methods::{
    add_block,
    add_transaction,
    get_chain_length,
    make_handshake,
};

use blockchain::Transaction;
use mempool::Mempool;

use serde::{
    Deserialize,
    Serialize,
};

#[rpc]
pub trait RpcMethods {
    type Metadata;

    #[rpc(name = "get_chain_length")]
    fn get_chain_length(&self) -> Result<usize>;

    #[rpc(meta, name = "make_handshake")]
    fn make_handshake(&self, req_info: Self::Metadata) -> Result<()>;

    #[rpc(name = "add_transaction")]
    fn add_transaction(&self, transaction: Transaction) -> Result<String>;

    #[rpc(name = "add_block")]
    fn add_block(&self, block: Block) -> Result<String>;
}

struct RpcManager {
    pub state: Arc<std::sync::Mutex<NodeState>>,
}

impl RpcMethods for RpcManager {
    type Metadata = ReqInfo;

    fn get_chain_length(&self) -> Result<usize> {
        get_chain_length(&self.state)
    }

    fn make_handshake(&self, req_info: Self::Metadata) -> Result<()> {
        make_handshake::<Self::Metadata>(req_info);
        Ok(())
    }

    fn add_transaction(&self, transaction: Transaction) -> Result<String> {
        block_on(add_transaction(&self.state, transaction))
    }

    fn add_block(&self, block: Block) -> Result<String> {
        block_on(add_block(&self.state, block))
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ReqInfo(String);

impl Metadata for ReqInfo {}

pub struct NodeState {
    pub blockchain: Blockchain,
    pub peers: HashMap<String, (String, u16)>,
    pub mempool: Mempool,
    pub wallet: Wallet,
}

#[derive(Clone)]
pub struct Node {}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

impl Node {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&mut self, config: Configuration) {
        println!("[ INFO ] Node starting on {}", config.rpc_port);

        let blockchain = Blockchain::new("mars", Arc::new(std::sync::Mutex::new(config.clone())));

        let wallet = config.wallet.clone();

        let sign = wallet.sign_data(wallet.get_public().hash_it());

        let obj = serde_json::json!({
            "address": wallet.get_public().hash_it(),
            "port": config.rpc_port,
            "key": wallet.get_public(),
            "sign": sign,
        });

        let client = reqwest::Client::new();

        let peers = {
            let res = client
                .post("http://localhost:33140/signal")
                .json(&obj)
                .send()
                .await;

            match res {
                Ok(res) => res.json::<HashMap<String, (String, u16)>>().await.unwrap(),
                _ => HashMap::new(),
            }
        };

        let state = Arc::new(std::sync::Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            peers,
            wallet,
        }));

        //assert!(state.lock().unwrap().blockchain.verify_integrity().is_ok());

        let mut io = MetaIoHandler::default();

        let manager = RpcManager { state };

        io.extend_with(manager.to_delegate());

        let hostname = config.hostname.clone();
        let rpc_port = config.rpc_port;

        tokio::task::spawn_blocking(move || {
            let server = ServerBuilder::new(io)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Null,
                ]))
                .start_http(&format!("{}:{}", hostname, rpc_port).parse().unwrap())
                .expect("Unable to start RPC server");

            println!("[ RPC ] Running on port {}", rpc_port);

            server.wait();
        })
        .await
        .unwrap();
    }
}
