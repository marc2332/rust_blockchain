#![feature(async_closure)]
use std::sync::{
    mpsc::Sender,
    Arc,
    Mutex,
};

use std::collections::HashMap;

use blockchain::{
    Block,
    Blockchain,
    Configuration,
    Key,
    Metrics,
    Wallet,
};

use client::{
    HandshakeRequest,
    NodeClient,
};
use jsonrpc_core::serde_json;

pub mod mempool;
pub mod methods;
pub mod server;

use jsonrpc_http_server::{
    AccessControlAllowOrigin,
    DomainsValidation,
};

use mempool::Mempool;
use server::ThreadMsg;

use crate::server::RpcManager;

#[derive(Clone)]
pub struct NodeState {
    /// The chain of blocks
    pub blockchain: Blockchain,
    /// List of lost blocks
    pub lost_blocks: HashMap<String, Block>,
    /// Node memory pool
    pub mempool: Mempool,
    /// Internal Node's wallet
    pub wallet: Wallet,
    /// Internal Node ID
    pub id: u16,
    /// Calculated next forger Public Key
    pub next_forger: Option<Key>,
    /// List of transaction handlers
    pub transaction_handlers: Vec<Sender<ThreadMsg>>,
    /// Current free transaction handler
    pub available_tx_handler: usize,
    /// List of transaction senders
    pub transaction_senders: Vec<Sender<ThreadMsg>>,
    /// List of block senders
    pub block_senders: Vec<Sender<ThreadMsg>>,
    /// Current free block sender
    pub available_block_sender: usize,
    /// Known nodes
    pub peers: HashMap<String, (String, u16, u16)>,
}

impl NodeState {
    /// Calculate a new block forger given the current state of the blockchain
    pub fn elect_new_forger(&mut self) {
        let next_forger = consensus::elect_forger(&mut self.blockchain).unwrap();
        self.next_forger = Some(next_forger);
    }
}

#[derive(Clone)]
pub struct Node {
    pub config: Configuration,
    pub state: Arc<Mutex<NodeState>>,
}

impl Node {
    pub async fn new(config: Configuration) -> Self {
        // Create the metrics manager
        let metrics = Arc::new(Mutex::new(Metrics::new(vec![])));

        let blockchain = Blockchain::new(config.clone(), metrics).await;

        let wallet = config.wallet.clone();
        let id = config.id;

        // Create the node state
        let state = Arc::new(Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            lost_blocks: HashMap::new(),
            wallet,
            id,
            next_forger: None,
            transaction_handlers: Vec::new(),
            available_tx_handler: 0,
            transaction_senders: Vec::new(),
            block_senders: Vec::new(),
            available_block_sender: 0,
            peers: HashMap::new(),
        }));

        Self { config, state }
    }

    pub async fn sync_from_discovery_server(&mut self) {
        let wallet = self.config.wallet.clone();
        let rpc_port = self.config.rpc_port;
        let rpc_ws_port = self.config.rpc_ws_port;

        let sign = wallet.sign_data(wallet.get_public().hash_it());

        let obj = serde_json::json!({
            "address": wallet.get_public().hash_it(),
            "rpc_port": rpc_port,
            "rpc_ws_port": rpc_ws_port,
            "key": wallet.get_public(),
            "sign": sign,
        });
        let client = reqwest::Client::new();

        let res = client
            .post("http://localhost:33140/signal")
            .json(&obj)
            .send()
            .await;

        let mut peers = match res {
            Ok(res) => res
                .json::<HashMap<String, (String, u16, u16)>>()
                .await
                .unwrap(),
            _ => HashMap::new(),
        };

        let address = wallet.get_public().hash_it();

        if peers.get(&address).is_some() {
            peers.remove(&address);
        }

        for (hostname, _, rpc_ws_port) in peers.values() {
            self.state
                .lock()
                .unwrap()
                .transaction_senders
                .push(server::create_transaction_sender(
                    hostname.clone(),
                    *rpc_ws_port,
                ));
        }

        self.state.lock().unwrap().peers = peers;
    }

    pub async fn run(&mut self) {
        tracing::info!("(Node.{}) Booting up node...", self.config.id);

        // Setup the transactions handlers threads
        let transaction_handlers = (0..self.config.transaction_threads)
            .map(|_| server::create_transaction_handler(self.state.clone()))
            .collect::<Vec<Sender<ThreadMsg>>>();

        self.state.lock().unwrap().transaction_handlers = transaction_handlers;

        // Setup the blocks sender threads
        let block_senders = (0..5)
            .map(|_| server::create_block_sender())
            .collect::<Vec<Sender<ThreadMsg>>>();

        self.state.lock().unwrap().block_senders = block_senders;

        let wallet = self.config.wallet.clone();
        let rpc_port = self.config.rpc_port;
        let rpc_ws_port = self.config.rpc_ws_port;
        let peers = self.state.lock().unwrap().peers.clone();
        let hostname = self.config.hostname.clone();
        let id = self.config.id;

        // Handshake known nodes
        tokio::spawn(async move {
            for (hostname, node_rpc_port, _) in peers.values() {
                let handshake = HandshakeRequest {
                    ip: hostname.to_string(),
                    rpc_port,
                    rpc_ws_port,
                    address: wallet.get_public().hash_it(),
                };

                let client = NodeClient::new(&format!("http://{}:{}", hostname, node_rpc_port))
                    .await
                    .unwrap();

                client.make_handshake(handshake).await.unwrap();
            }
        });

        let ws_io = RpcManager::get_io_handler(&self.state);

        let ws_hostname = hostname.clone();

        tokio::task::spawn_blocking(move || {
            let server = jsonrpc_ws_server::ServerBuilder::new(ws_io)
                .start(&format!("{}:{}", ws_hostname, rpc_ws_port).parse().unwrap())
                .expect("Unable to start RPC server");

            tracing::info!(
                "(Node.{}) Running RPC WebSockets server on port {}",
                id,
                rpc_ws_port
            );

            server.wait().unwrap();
        });

        let http_io = RpcManager::get_io_handler(&self.state);

        tokio::task::spawn_blocking(move || {
            let server = jsonrpc_http_server::ServerBuilder::new(http_io)
                .cors(DomainsValidation::AllowOnly(vec![
                    AccessControlAllowOrigin::Null,
                ]))
                .threads(3)
                .start_http(&format!("{}:{}", hostname, rpc_port).parse().unwrap())
                .expect("Unable to start RPC HTTP server");

            tracing::info!("(Node.{}) Running RPC server on port {}", id, rpc_port);

            server.wait();
        })
        .await
        .unwrap();
    }
}
