use crate::NodeState;
use std::sync::{
    Arc,
    Mutex,
};

pub fn get_address_ammount(state: &Arc<Mutex<NodeState>>, address: String) -> u64 {
    state
        .lock()
        .unwrap()
        .blockchain
        .state
        .get_address_ammount(address)
}
