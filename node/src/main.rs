use std::sync::{
    Arc,
    Mutex,
};

use blockchain::{
    Blockchain,
    Configuration,
};

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
        add_transaction(&self.state, transaction)
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
}

#[tokio::main]
async fn main() {
    let config = Arc::new(Mutex::new(Configuration::new()));

    let state = Arc::new(Mutex::new(NodeState {
        blockchain: Blockchain::new("mars", config),
        mempool: Mempool::default(),
        peers: vec![],
    }));

    start_servers(state).await;
}
