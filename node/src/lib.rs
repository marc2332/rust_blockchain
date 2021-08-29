use std::sync::Arc;

use std::collections::HashMap;

use blockchain::{
    Block,
    Blockchain,
    Configuration,
    Key,
    Wallet,
};

use jsonrpc_core::{
    serde_json,
    IoHandler,
    Result,
};
use jsonrpc_derive::rpc;

pub mod mempool;
pub mod methods;

use jsonrpc_http_server::{
    AccessControlAllowOrigin,
    DomainsValidation,
    ServerBuilder,
};
use methods::{
    add_block,
    add_transaction,
    get_block_with_prev_hash,
    get_chain_length,
    make_handshake,
};

use client::{
    HandshakeRequest,
    RPCClient,
};

use blockchain::Transaction;
use mempool::Mempool;

#[rpc]
pub trait RpcMethods {
    #[rpc(name = "get_chain_length")]
    fn get_chain_length(&self) -> Result<usize>;

    #[rpc(name = "make_handshake")]
    fn make_handshake(&self, req: HandshakeRequest) -> Result<()>;

    #[rpc(name = "add_transaction")]
    fn add_transaction(&self, transaction: Transaction) -> Result<()>;

    #[rpc(name = "add_block")]
    fn add_block(&self, block: Block) -> Result<()>;

    #[rpc(name = "get_block_with_prev_hash")]
    fn get_block_with_prev_hash(&self, prev_hash: String) -> Result<Option<Block>>;
}

struct RpcManager {
    pub state: Arc<std::sync::Mutex<NodeState>>,
}

impl RpcMethods for RpcManager {
    fn get_chain_length(&self) -> Result<usize> {
        get_chain_length(&self.state)
    }

    fn make_handshake(&self, req: HandshakeRequest) -> Result<()> {
        make_handshake(&self.state, req);
        Ok(())
    }

    fn add_transaction(&self, transaction: Transaction) -> Result<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            add_transaction(&state, transaction).await;
        });
        Ok(())
    }

    fn add_block(&self, block: Block) -> Result<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            add_block(&state, block).await;
        });
        Ok(())
    }

    fn get_block_with_prev_hash(&self, prev_hash: String) -> Result<Option<Block>> {
        get_block_with_prev_hash(&self.state, prev_hash)
    }
}

pub struct NodeState {
    pub blockchain: Blockchain,
    pub lost_blocks: HashMap<String, Block>,
    pub peers: HashMap<String, (String, u16)>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub id: u16,
    pub next_forger: Key,
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
        log::info!("(Node.{}) Booting up node...", config.id);

        let blockchain = Blockchain::new("mars", Arc::new(std::sync::Mutex::new(config.clone())));

        let wallet = config.wallet.clone();
        let id = config.id;
        let hostname = config.hostname.clone();
        let rpc_port = config.rpc_port;

        let sign = wallet.sign_data(wallet.get_public().hash_it());

        let obj = serde_json::json!({
            "address": wallet.get_public().hash_it(),
            "port": rpc_port,
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

            let mut peers = match res {
                Ok(res) => res.json::<HashMap<String, (String, u16)>>().await.unwrap(),
                _ => HashMap::new(),
            };
            let address = wallet.get_private().hash_it();
            if peers.get(&address).is_some() {
                peers.remove(&address);
            }
            peers
        };

        let next_forger = consensus::elect_forger(&blockchain).unwrap();

        let state = Arc::new(std::sync::Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            peers: peers.clone(),
            lost_blocks: HashMap::new(),
            wallet: wallet.clone(),
            id: config.id,
            next_forger,
        }));

        assert!(state.lock().unwrap().blockchain.verify_integrity().is_ok());

        tokio::spawn(async move {
            for (hostname, port) in peers.values() {
                let handshake = HandshakeRequest {
                    ip: hostname.to_string(),
                    port: rpc_port,
                    address: wallet.get_public().hash_it(),
                };

                let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                    .await
                    .unwrap();

                client.make_handshake(handshake).await.unwrap();
            }
        });

        let mut io = IoHandler::default();

        let manager = RpcManager { state };

        io.extend_with(manager.to_delegate());

        tokio::task::spawn_blocking(move || {
            let server = ServerBuilder::new(io)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Null,
                ]))
                .start_http(&format!("{}:{}", hostname, rpc_port).parse().unwrap())
                .expect("Unable to start RPC server");

            log::info!("(Node.{}) Running RPC server on port {}", id, rpc_port);

            server.wait();
        })
        .await
        .unwrap();
    }
}
