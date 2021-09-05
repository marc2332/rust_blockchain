use std::sync::{
    Arc,
    Mutex,
};

use crate::NodeState;
use jsonrpc_http_server::jsonrpc_core::*;

/*
 * Return the length of the blockchaim ( also known as the block height ) and the last block hash
 */
pub fn get_chain_length(state: &Arc<Mutex<NodeState>>) -> Result<(String, usize)> {
    let index = state.lock().unwrap().blockchain.index;
    let last_hash = {
        if let Some(hash) = &state.lock().unwrap().blockchain.last_block_hash {
            hash.unite()
        } else {
            String::new()
        }
    };
    Ok((last_hash.clone(), index))
}
