#![feature(async_closure)]
use std::sync::{
    mpsc::Sender,
    Arc,
    Mutex,
};

use std::{
    collections::HashMap,
    sync::mpsc::channel,
    thread,
};

use blockchain::{
    Block,
    Blockchain,
    Configuration,
    Key,
    Wallet,
};

use futures::executor::block_on;
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
};
use methods::{
    add_block,
    add_transaction,
    get_address_ammount,
    get_block_with_hash,
    get_block_with_prev_hash,
    get_chain_length,
    get_node_address,
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
    fn get_chain_length(&self) -> Result<(String, usize)>;

    #[rpc(name = "make_handshake")]
    fn make_handshake(&self, req: HandshakeRequest) -> Result<()>;

    #[rpc(name = "add_transaction")]
    fn add_transaction(&self, transaction: Transaction) -> Result<()>;

    #[rpc(name = "add_block")]
    fn add_block(&self, block: Block) -> Result<()>;

    #[rpc(name = "get_block_with_prev_hash")]
    fn get_block_with_prev_hash(&self, prev_hash: String) -> Result<Option<Block>>;

    #[rpc(name = "get_node_address")]
    fn get_node_address(&self) -> Result<String>;

    #[rpc(name = "get_address_ammount")]
    fn get_address_ammount(&self, address: String) -> Result<u64>;

    #[rpc(name = "get_block_with_hash")]
    fn get_block_with_hash(&self, hash: String) -> Result<Option<Block>>;

    #[rpc(name = "add_transactions")]
    fn add_transactions(&self, transactions: Vec<Transaction>) -> Result<()>;
}

struct RpcManager {
    pub state: Arc<std::sync::Mutex<NodeState>>,
}

impl RpcMethods for RpcManager {
    fn get_chain_length(&self) -> Result<(String, usize)> {
        get_chain_length(&self.state)
    }

    fn make_handshake(&self, req: HandshakeRequest) -> Result<()> {
        make_handshake(&self.state, req);
        Ok(())
    }

    /*
     * Handle incoming transactions
     */
    fn add_transaction(&self, transaction: Transaction) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        state.transaction_handlers[state.available_tx_handler]
            .send(ThreadMsg::AddTransaction(transaction))
            .unwrap();
        state.available_tx_handler += 1;
        if state.available_tx_handler == state.transaction_handlers.len() {
            state.available_tx_handler = 0;
        }
        Ok(())
    }

    /*
     * Handle incoming blocks
     */
    fn add_block(&self, block: Block) -> Result<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            add_block(&state, block).await;
        });
        Ok(())
    }

    fn get_block_with_prev_hash(&self, prev_hash: String) -> Result<Option<Block>> {
        block_on(get_block_with_prev_hash(&self.state, prev_hash))
    }

    fn get_node_address(&self) -> Result<String> {
        Ok(get_node_address(&self.state))
    }

    fn get_address_ammount(&self, address: String) -> Result<u64> {
        Ok(get_address_ammount(&self.state, address))
    }

    fn get_block_with_hash(&self, hash: String) -> Result<Option<Block>> {
        block_on(get_block_with_hash(&self.state, hash))
    }

    fn add_transactions(&self, transactions: Vec<Transaction>) -> Result<()> {
        let mut state = self.state.lock().unwrap();

        for tx in transactions {
            state.transaction_handlers[state.available_tx_handler]
                .send(ThreadMsg::AddTransaction(tx))
                .unwrap();
            state.available_tx_handler += 1;
            if state.available_tx_handler == state.transaction_handlers.len() {
                state.available_tx_handler = 0;
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct NodeState {
    pub blockchain: Blockchain,
    pub lost_blocks: HashMap<String, Block>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub id: u16,
    pub next_forger: Option<Key>,
    pub transaction_handlers: Vec<Sender<ThreadMsg>>,
    pub available_tx_handler: usize,
    pub transaction_senders: Vec<Sender<ThreadMsg>>,
    pub available_tx_sender: usize,
    pub block_senders: Vec<Sender<ThreadMsg>>,
    pub available_block_sender: usize,
    pub peers: HashMap<String, (String, u16, u16)>,
}

impl NodeState {
    pub fn elect_new_forger(&mut self) {
        let next_forger = consensus::elect_forger(&mut self.blockchain).unwrap();
        self.next_forger = Some(next_forger);
    }
}

pub enum ThreadMsg {
    AddTransaction(Transaction),
    PropagateTransactions {
        transactions: Vec<Transaction>,
    },
    PropagateBlock {
        block: Block,
        hostname: String,
        rpc_port: u16,
    },
}

#[derive(Clone)]
pub struct Node {
    pub config: Configuration,
    pub state: Arc<Mutex<NodeState>>,
}

impl Node {
    pub async fn new(config: Configuration) -> Self {
        let blockchain = Blockchain::new(config.clone()).await;

        let wallet = config.wallet.clone();
        let id = config.id;

        // Create the node state
        let state = Arc::new(std::sync::Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            lost_blocks: HashMap::new(),
            wallet,
            id,
            next_forger: None,
            transaction_handlers: Vec::new(),
            available_tx_handler: 0,
            transaction_senders: Vec::new(),
            available_tx_sender: 0,
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
                .push(create_transaction_sender(hostname.clone(), *rpc_ws_port));
        }

        self.state.lock().unwrap().peers = peers;
    }

    pub async fn run(&mut self) {
        tracing::info!("(Node.{}) Booting up node...", self.config.id);

        // Setup the transactions handlers threads
        let transaction_handlers = (0..self.config.transaction_threads)
            .map(|_| create_transaction_handler(self.state.clone()))
            .collect::<Vec<Sender<ThreadMsg>>>();

        self.state.lock().unwrap().transaction_handlers = transaction_handlers;

        // Setup the blocks sender threads
        let block_senders = (0..5)
            .map(|_| create_block_sender())
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

                let client = RPCClient::new(&format!("http://{}:{}", hostname, node_rpc_port))
                    .await
                    .unwrap();

                client.make_handshake(handshake).await.unwrap();
            }
        });

        let mut ws_io = IoHandler::default();
        let ws_manager = RpcManager {
            state: self.state.clone(),
        };
        ws_io.extend_with(ws_manager.to_delegate());

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

        let mut http_io = IoHandler::default();
        let http_manager = RpcManager {
            state: self.state.clone(),
        };
        http_io.extend_with(http_manager.to_delegate());

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

/*
 * Create a thread to receive transactions
 */
fn create_transaction_handler(state: Arc<Mutex<NodeState>>) -> Sender<ThreadMsg> {
    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                let rx = rx.lock().unwrap();
                let state = state.clone();
                if let ThreadMsg::AddTransaction(transaction) = rx.recv().unwrap() {
                    tokio::spawn(async move {
                        add_transaction(&state, transaction).await;
                    });
                }
            }
        })
    });

    tx
}

/*
 * Create a thread for each WebSocket connection to known peers
 */
fn create_transaction_sender(hostname: String, rpc_ws_port: u16) -> Sender<ThreadMsg> {
    use tokio::sync::Mutex; // Use async Mutex

    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = RPCClient::new_ws(&format!("ws://{}:{}", hostname, rpc_ws_port))
                .await
                .unwrap();

            let client = Arc::new(Mutex::new(client));
            loop {
                let client = client.clone();
                let rx = rx.lock().await;
                if let ThreadMsg::PropagateTransactions { transactions } = rx.recv().unwrap() {
                    tokio::spawn(async move {
                        let client = client.lock().await;
                        client.add_transactions(transactions).await.ok();
                        drop(client)
                    });
                }
            }
        })
    });

    tx
}

/*
 * Create a thread to propagate blocks
 */
fn create_block_sender() -> Sender<ThreadMsg> {
    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                let rx = rx.lock().unwrap();
                if let ThreadMsg::PropagateBlock {
                    block,
                    hostname,
                    rpc_port,
                } = rx.recv().unwrap()
                {
                    tokio::spawn(async move {
                        let client = RPCClient::new(&format!("http://{}:{}", hostname, rpc_port))
                            .await
                            .unwrap();
                        client.add_block(block).await.ok();
                    });
                }
            }
        })
    });

    tx
}
