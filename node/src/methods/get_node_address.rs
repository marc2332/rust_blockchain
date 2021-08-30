use crate::NodeState;
use std::sync::{
    Arc,
    Mutex,
};

pub fn get_node_address(state: &Arc<Mutex<NodeState>>) -> String {
    state.lock().unwrap().wallet.get_public().hash_it()
}
