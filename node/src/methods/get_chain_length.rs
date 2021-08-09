use std::sync::{
    Arc,
    Mutex,
};

use crate::NodeState;
use jsonrpc_http_server::jsonrpc_core::*;

pub fn get_chain_length(state: &Arc<Mutex<NodeState>>) -> Result<usize> {
    Ok(state.lock().unwrap().blockchain.index)
}
