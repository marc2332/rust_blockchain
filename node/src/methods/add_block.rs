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

pub fn add_block(state: &Arc<Mutex<NodeState>>, block: Block) -> Result<String> {
    let mut state = state.lock().unwrap();

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
        state.blockchain.add_block(&block.clone());

        state.mempool.pending_transactions.clear();

        // WIP
        Ok(String::from("ok"))
    } else {
        Ok(String::from("err"))
    }
}
