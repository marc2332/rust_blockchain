use std::sync::{
    Arc,
    Mutex,
};

use std::collections::HashMap;

use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    TransactionBuilder,
    Wallet,
};

use chrono::Utc;
use futures::executor::block_on;
use jsonrpc_http_server::{
    jsonrpc_core::*,
    *,
};

use jsonrpc_derive::rpc;

pub mod mempool;
pub mod methods;

use methods::{
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
}

struct RpcManager {
    pub state: Arc<Mutex<NodeState>>,
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
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ReqInfo(String);

impl Metadata for ReqInfo {}

pub struct NodeState {
    pub blockchain: Blockchain,
    pub peers: HashMap<String, String>,
    pub mempool: Mempool,
    pub wallet: Wallet,
}

pub struct Node {}

impl Node {
    pub fn new() -> Self {
        Self {}
    }

    #[tokio::main]
    pub async fn run(&mut self, config: Arc<Mutex<Configuration>>) {
        let mut blockchain = Blockchain::new("mars", config.clone());

        let wallet = Wallet::default();

        // Create a genesis block if there isn't
        if blockchain.last_block_hash.is_none() {
            println!("{:?}", wallet.get_private().0);

            let genesis_transaction = TransactionBuilder::new()
                .key(&wallet.get_public())
                .from_address("0x")
                .to_address(&wallet.get_public().hash_it())
                .ammount(100)
                .hash_it()
                .sign_with(&wallet)
                .build();

            let block_data = serde_json::to_string(&vec![genesis_transaction]).unwrap();

            let genesis_block = BlockBuilder::new()
                .payload(&block_data)
                .timestamp(Utc::now())
                .key(&wallet.get_public())
                .hash_it()
                .sign_with(&wallet)
                .build();

            blockchain.add_block(&genesis_block);
        }

        /*
        println!("Finding peers...");

        let sign = wallet.sign_data(wallet.get_public().hash_it());

        let obj = serde_json::json!({
            "address": wallet.get_public().hash_it(),
            "key": wallet.get_public(),
            "sign": sign,
        });

        let client = reqwest::Client::new();

        let peers = client.post("http://localhost:33140/signal")
            .json(&obj)
            .send()
            .await.unwrap()
            .json::<HashMap<String, String>>()
            .await.unwrap();
            */

        let state = Arc::new(Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            peers: HashMap::new(),
            wallet,
        }));

        assert!(state.lock().unwrap().blockchain.verify_integrity().is_ok());

        let mut io = MetaIoHandler::default();

        let manager = RpcManager { state };

        io.extend_with(manager.to_delegate());

        let config = config.lock().unwrap();

        let hostname = config.hostname.clone();
        let rpc_port = config.rpc_port.clone();

        drop(config);

        tokio::spawn(async move {
            let server = ServerBuilder::new(io)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Null,
                ]))
                .meta_extractor(|_req: &hyper::Request<hyper::Body>| ReqInfo(String::from("_")))
                .start_http(&format!("{}:{}", hostname, rpc_port).parse().unwrap())
                .expect("Unable to start RPC server");

            server.wait();
        })
        .await
        .unwrap();
    }
}