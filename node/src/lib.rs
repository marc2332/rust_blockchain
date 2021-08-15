use std::sync::{
    Arc,
    Mutex,
};

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

static RPC_PORT: u16 = 3030;
static HOSTNAME: &str = "127.0.0.1";

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

pub async fn start_servers(state: Arc<Mutex<NodeState>>) {
    let mut io = MetaIoHandler::default();

    let manager = RpcManager { state };

    io.extend_with(manager.to_delegate());

    tokio::spawn(async move {
        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Null,
            ]))
            .meta_extractor(|_req: &hyper::Request<hyper::Body>| ReqInfo(String::from("_")))
            .start_http(&format!("{}:{}", HOSTNAME, RPC_PORT).parse().unwrap())
            .expect("Unable to start RPC server");

        server.wait();
    })
    .await
    .unwrap();
}

pub struct PeerNode {
    pub hostname: String,
}

pub struct NodeState {
    pub blockchain: Blockchain,
    pub peers: Vec<PeerNode>,
    pub mempool: Mempool,
    pub wallet: Wallet,
}

#[tokio::main]
pub async fn main() {
    let config = Arc::new(Mutex::new(Configuration::new()));

    let mut blockchain = Blockchain::new("mars", config);

    // Create a genesis block if there isn't
    if blockchain.last_block_hash.is_none() {
        let genesis_wallet = Wallet::new();

        println!("{:?}", genesis_wallet.get_private().0);

        let genesis_transaction = TransactionBuilder::new()
            .key(&genesis_wallet.get_public())
            .from_address("0x")
            .to_address(&genesis_wallet.get_public().hash_it())
            .ammount(100)
            .hash_it()
            .sign_with(&genesis_wallet)
            .build();

        let block_data = serde_json::to_string(&vec![genesis_transaction]).unwrap();

        let genesis_block = BlockBuilder::new()
            .payload(&block_data)
            .timestamp(Utc::now())
            .key(&genesis_wallet.get_public())
            .hash_it()
            .sign_with(&genesis_wallet)
            .build();

        blockchain.add_block(&genesis_block);
    }

    let state = Arc::new(Mutex::new(NodeState {
        blockchain,
        mempool: Mempool::default(),
        peers: vec![],
        wallet: Wallet::default(),
    }));

    assert!(state.lock().unwrap().blockchain.verify_integrity().is_ok());

    start_servers(state).await;
}
