use std::sync::{
    Arc,
    Mutex,
};

use crate::NodeState;
use blockchain::{
    Block,
    PublicAddress,
    Transaction,
};
use client::RPCClient;
use jsonrpc_http_server::jsonrpc_core::*;

pub async fn add_block(state: &Arc<Mutex<NodeState>>, block: Block) {
    /*
     * This should also make sure Fthe forger is the right one
     */
    let is_block_ok = || {
        let elected_forger = consensus::elect_forger(&state.lock().unwrap().blockchain).unwrap();

        // Make sure elected forger is the right one
        if !block.verify_sign_with(&PublicAddress::from(&elected_forger)) {
            return false;
        }

        let transactions: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

        // Make sure the transactions movements are correct
        for transaction in transactions.iter() {
            let tx_verification_is_ok = transaction.verify()
                && state
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
        if state
            .lock()
            .unwrap()
            .blockchain
            .add_block(&block.clone())
            .is_ok()
        {
            // Elect the next forger
            let next_forger = consensus::elect_forger(&state.lock().unwrap().blockchain).unwrap();
            state.lock().unwrap().next_forger = next_forger;

            let block_txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

            // Remove thee bundled transactions from the mempool
            for tx in block_txs {
                state
                    .lock()
                    .unwrap()
                    .mempool
                    .remove_transaction(&tx.get_hash())
            }
        } else {
            let state = state.clone();

            // Incredibly awful, should be improved

            let peers = state.lock().unwrap().peers.clone();
            let prev_hash = state
                .lock()
                .unwrap()
                .blockchain
                .last_block_hash
                .as_ref()
                .unwrap()
                .unite();

            for (hostname, port) in peers.values() {
                let hostname = hostname.clone();
                let port = *port;

                let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                    .await
                    .unwrap();

                let res = client.get_block_with_prev_hash(prev_hash.clone()).await;

                if let Ok(Some(block)) = res {
                    state
                        .lock()
                        .unwrap()
                        .lost_blocks
                        .insert(block.hash.unite(), block);
                    break;
                }
            }

            let mut state = state.lock().unwrap();
            state.lost_blocks.insert(block.hash.unite(), block);

            let mut blocks_iter = state.lost_blocks.clone().into_iter().peekable();
            let mut blocks = state.lost_blocks.clone();

            while blocks_iter.peek().is_some() {
                let (_, block) = blocks_iter.next().unwrap();
                let res = state.blockchain.add_block(&block.clone()).is_ok();
                if res {
                    blocks.remove(&block.hash.unite());
                    blocks_iter = blocks.clone().into_iter().peekable();

                    // Elect next block forger
                    let next_forger = consensus::elect_forger(&state.blockchain).unwrap();
                    state.next_forger = next_forger;

                    // Remove confirmed transactions from the mempool
                    let block_txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

                    for tx in block_txs {
                        state.mempool.remove_transaction(&tx.get_hash());
                    }
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
