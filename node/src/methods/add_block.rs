use std::sync::{
    Arc,
    Mutex,
};

use crate::NodeState;
use blockchain::{
    Block,
    Transaction,
};
use jsonrpc_http_server::jsonrpc_core::*;

pub fn add_block(node_state: &Arc<Mutex<NodeState>>, block: Block) -> Result<String> {
    let mut state = node_state.lock().unwrap();

    let is_block_ok = || {
        let transactions: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

        // Update chainstate with the new transactions
        for transaction in transactions.iter() {
            let tx_verification_is_ok = transaction.verify()
                && state
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
        if state.blockchain.add_block(&block.clone()).is_ok() {
            state.mempool.pending_transactions.clear();

            // WIP
            Ok(String::from("ok"))
        } else {
            let state = node_state.clone();

            // Incredibly awful, should be improved
            tokio::spawn(async move {
                let mut state = state.lock().unwrap();
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
            });

            Ok(String::from("err"))
        }
    } else {
        Ok(String::from("err"))
    }
}
