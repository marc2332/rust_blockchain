use std::sync::{
    Arc,
    Mutex,
};

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
pub enum TransactionResult {
    Verified,
    BadVerification,
}

use crate::NodeState;
use blockchain::Transaction;
use jsonrpc_http_server::jsonrpc_core::*;

pub fn add_transaction(state: &Arc<Mutex<NodeState>>, transaction: Transaction) -> Result<String> {
    let tx_verification_is_ok = transaction.verify();

    if tx_verification_is_ok {
        state.lock().unwrap().mempool.add_transaction(transaction);

        Ok("Verified".to_string())
    } else {
        Ok("Bad verification".to_string())
    }
}
