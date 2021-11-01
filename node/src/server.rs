use blockchain::{
    Block,
    Transaction,
};
use jsonrpc_derive::rpc;

use crate::methods::{
    add_block,
    get_address_ammount,
    get_block_with_hash,
    get_block_with_prev_hash,
    get_chain_length,
    get_node_address,
    make_handshake,
};
use client::{
    HandshakeRequest,
    NodeClient,
};
use futures::executor::block_on;
use jsonrpc_core::{
    IoHandler,
    Result,
};
use std::{
    sync::{
        mpsc::{
            channel,
            Sender,
        },
        Arc,
        Mutex,
    },
    thread,
};

use crate::{
    methods::add_transaction,
    NodeState,
};

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
/*
 * Create a thread to receive transactions
 */
pub fn create_transaction_handler(state: Arc<Mutex<NodeState>>) -> Sender<ThreadMsg> {
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

/// Creates a separate thread that propagates the transactions to the specified Node
pub fn create_transaction_sender(hostname: String, rpc_ws_port: u16) -> Sender<ThreadMsg> {
    use tokio::sync::Mutex; // Use async Mutex

    let (tx, rx) = channel();
    let rx = Arc::new(Mutex::new(rx));

    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = NodeClient::new_ws(&format!("ws://{}:{}", hostname, rpc_ws_port))
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

/// Create a separate thread that manages to propagate new blocks to the given node
pub fn create_block_sender() -> Sender<ThreadMsg> {
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
                        let client = NodeClient::new(&format!("http://{}:{}", hostname, rpc_port))
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

pub struct RpcManager {
    pub state: Arc<Mutex<NodeState>>,
}

impl RpcMethods for RpcManager {
    /// Returns the current block height and the last block hash of the blockchain
    fn get_chain_length(&self) -> Result<(String, usize)> {
        get_chain_length(&self.state)
    }

    /// Makes a handshake to a node which ultimately opens WebSockets connection
    fn make_handshake(&self, req: HandshakeRequest) -> Result<()> {
        make_handshake(&self.state, req);
        Ok(())
    }

    /// Adds a new transaction into the mempool
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

    /// Same as add_transaction but instead of just 1, a group
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

    /// Try to add a block to the blockchain
    fn add_block(&self, block: Block) -> Result<()> {
        let state = self.state.clone();
        tokio::spawn(async move {
            add_block(&state, block).await;
        });
        Ok(())
    }

    /// Get a block by it's previous hash
    fn get_block_with_prev_hash(&self, prev_hash: String) -> Result<Option<Block>> {
        block_on(get_block_with_prev_hash(&self.state, prev_hash))
    }

    /// Return the Node's address
    fn get_node_address(&self) -> Result<String> {
        Ok(get_node_address(&self.state))
    }

    /// Get the ammount of the given address
    fn get_address_ammount(&self, address: String) -> Result<u64> {
        Ok(get_address_ammount(&self.state, address))
    }

    /// Get a block by the given hash
    fn get_block_with_hash(&self, hash: String) -> Result<Option<Block>> {
        block_on(get_block_with_hash(&self.state, hash))
    }
}

impl RpcManager {
    pub fn get_io_handler(state: &Arc<Mutex<NodeState>>) -> IoHandler {
        let mut io = IoHandler::default();
        let manager = RpcManager {
            state: state.clone(),
        };
        io.extend_with(manager.to_delegate());
        io
    }
}
