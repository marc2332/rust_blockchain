use std::sync::{
    Arc,
    Mutex,
};

use crate::NodeState;
use blockchain::{
    Block,
    Transaction,
};
use client::RPCClient;
use jsonrpc_http_server::jsonrpc_core::*;

pub async fn add_block(node_state: &Arc<Mutex<NodeState>>, block: Block) {
    /*
     * This should also make sure the forger is the right one
     */
    let is_block_ok = || {
        let transactions: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

        // Update chainstate with the new transactions
        for transaction in transactions.iter() {
            let tx_verification_is_ok = transaction.verify()
                && node_state
                    .lock()
                    .unwrap()
                    .blockchain
                    .state
                    .verify_transaction_ammount(transaction);

            if !tx_verification_is_ok {
                return false;
            }
        }
        true
    };

    if is_block_ok() {
        if node_state
            .lock()
            .unwrap()
            .blockchain
            .add_block(&block.clone())
            .is_ok()
        {
            node_state
                .lock()
                .unwrap()
                .mempool
                .pending_transactions
                .clear();
        } else {
            let node_state = node_state.clone();

            // Incredibly awful, should be improved

            let peers = node_state.lock().unwrap().peers.clone();
            let prev_hash = node_state
                .lock()
                .unwrap()
                .blockchain
                .last_block_hash
                .as_ref()
                .unwrap()
                .unite();

            for (_, (hostname, port)) in &peers {
                let hostname = hostname.clone();
                let port = *port;

                let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                    .await
                    .unwrap();

                let res = client.get_block_with_prev_hash(prev_hash.clone()).await;

                if let Ok(block) = res {
                    if let Some(block) = block {
                        node_state
                            .lock()
                            .unwrap()
                            .lost_blocks
                            .insert(block.hash.unite(), block);
                        break;
                    }
                }
            }

            let mut state = node_state.lock().unwrap();
            state.lost_blocks.insert(block.hash.unite(), block);

            let mut i = 0;

            let mut blocks_iter = state.lost_blocks.clone().into_iter().peekable();
            let mut blocks = state.lost_blocks.clone();

            while i < blocks.len() && blocks_iter.peek().is_some() {
                let (_, block) = blocks_iter.next().unwrap();
                let res = state.blockchain.add_block(&block.clone()).is_ok();
                if res {
                    i = 0;
                    blocks.remove(&block.hash.unite());
                    blocks_iter = blocks.clone().into_iter().peekable();
                } else {
                    i += 1;
                }
            }

            state.lost_blocks = blocks;

            log::warn!(
                "(Node.{}) Length of lost blocks is <{}>",
                state.id,
                state.lost_blocks.len()
            );
        }
    }
}
