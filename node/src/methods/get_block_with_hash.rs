use crate::NodeState;
use blockchain::Block;
use jsonrpc_http_server::jsonrpc_core::*;
use std::sync::{
    Arc,
    Mutex,
};

pub async fn get_block_with_hash(
    state: &Arc<Mutex<NodeState>>,
    hash: String,
) -> Result<Option<Block>> {
    let res = state
        .lock()
        .unwrap()
        .blockchain
        .get_block_with_hash(hash)
        .await;

    Ok(res)
}
