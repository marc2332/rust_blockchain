use blockchain::{Block, Transaction};

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

pub mod Server {

    use std::{sync::{Arc, Mutex, mpsc::{Sender, channel}}, thread};

    use client::RPCClient;

    use crate::{NodeState, methods::add_transaction};

    use super::ThreadMsg;

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

    /*
     * Create a thread for each WebSocket connection to known peers
     */
    pub fn create_transaction_sender(hostname: String, rpc_ws_port: u16) -> Sender<ThreadMsg> {
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
                            let client =
                                RPCClient::new(&format!("http://{}:{}", hostname, rpc_port))
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
}
