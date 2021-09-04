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
    get_address_ammount,
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
    fn get_chain_length(&self) -> Result<usize>;

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

    fn get_node_address(&self) -> Result<String> {
        Ok(get_node_address(&self.state))
    }

    fn get_address_ammount(&self, address: String) -> Result<u64> {
        Ok(get_address_ammount(&self.state, address))
    }
}

#[derive(Clone)]
pub struct NodeState {
    pub blockchain: Blockchain,
    pub lost_blocks: HashMap<String, Block>,
    pub peers: HashMap<String, (String, u16)>,
    pub mempool: Mempool,
    pub wallet: Wallet,
    pub id: u16,
    pub next_forger: Key,
    pub transaction_handlers: Vec<Sender<ThreadMsg>>,
    pub available_tx_handler: usize,
    pub transaction_senders: Vec<Sender<ThreadMsg>>,
    pub available_tx_sender: usize,
    pub block_senders: Vec<Sender<ThreadMsg>>,
    pub available_block_sender: usize,
}

pub enum ThreadMsg {
    AddTransaction(Transaction),
    PropagateTransaction {
        transaction: Transaction,
        hostname: String,
        port: u16,
    },
    PropagateBlock {
        block: Block,
        hostname: String,
        port: u16,
    },
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

        // Setup the blockchain
        let blockchain = Blockchain::new("mars", Arc::new(std::sync::Mutex::new(config.clone())));

        let wallet = config.wallet.clone();
        let id = config.id;
        let hostname = config.hostname.clone();
        let rpc_port = config.rpc_port;

        // Fetch new peers from a discovery server (WIP)
        let peers = {
            let sign = wallet.sign_data(wallet.get_public().hash_it());

            let obj = serde_json::json!({
                "address": wallet.get_public().hash_it(),
                "port": rpc_port,
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
                Ok(res) => res.json::<HashMap<String, (String, u16)>>().await.unwrap(),
                _ => HashMap::new(),
            };

            let address = wallet.get_public().hash_it();

            if peers.get(&address).is_some() {
                peers.remove(&address);
            }

            peers
        };

        // Elect the next block forger
        let next_forger = consensus::elect_forger(&blockchain).unwrap();

        // Create the node state
        let state = Arc::new(std::sync::Mutex::new(NodeState {
            blockchain,
            mempool: Mempool::default(),
            peers: peers.clone(),
            lost_blocks: HashMap::new(),
            wallet: wallet.clone(),
            id: config.id,
            next_forger: next_forger.clone(),
            transaction_handlers: Vec::new(),
            available_tx_handler: 0,
            transaction_senders: Vec::new(),
            available_tx_sender: 0,
            block_senders: Vec::new(),
            available_block_sender: 0,
        }));

        // Setup the transactions handlers threads
        let transaction_handlers = (0..config.transaction_threads)
            .map(|_| create_transaction_handler(state.clone()))
            .collect::<Vec<Sender<ThreadMsg>>>();

        state.lock().unwrap().transaction_handlers = transaction_handlers;

        // Setup the transactions sender threads
        let transaction_senders = (0..1)
            .map(|_| create_transaction_sender())
            .collect::<Vec<Sender<ThreadMsg>>>();

        state.lock().unwrap().transaction_senders = transaction_senders;

        // Setup the blocks sender threads
        let block_senders = (0..1)
            .map(|_| create_block_sender())
            .collect::<Vec<Sender<ThreadMsg>>>();

        state.lock().unwrap().block_senders = block_senders;

        // Verify the integrity of the blockchain
        assert!(state.lock().unwrap().blockchain.verify_integrity().is_ok());

        // Handshark known nodes
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

        // Setup the JSON RPC server
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

/*
 * Create a thread to recive transactions
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
                    add_transaction(&state, transaction).await
                }
            }
        })
    });

    tx
}

/*
 * Create a thread to propagate transactions
 */
fn create_transaction_sender() -> Sender<ThreadMsg> {
    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                let rx = rx.lock().unwrap();
                if let ThreadMsg::PropagateTransaction {
                    transaction,
                    hostname,
                    port,
                } = rx.recv().unwrap()
                {
                    let hostname = hostname.clone();
                    let port = port;
                    let transaction = transaction.clone();

                    let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                        .await
                        .unwrap();
                    client.add_transaction(transaction).await.ok();
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
                    port,
                } = rx.recv().unwrap()
                {
                    let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                        .await
                        .unwrap();
                    client.add_block(block).await.ok();
                }
            }
        })
    });

    tx
}
