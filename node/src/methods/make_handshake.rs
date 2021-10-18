use crate::NodeState;
use client::HandshakeRequest;
use std::sync::{
    Arc,
    Mutex,
};

pub fn make_handshake(state: &Arc<Mutex<NodeState>>, req: HandshakeRequest) {
    state
        .lock()
        .unwrap()
        .peers
        .insert(req.address, (req.ip.clone(), req.rpc_port, req.rpc_ws_port));
    tracing::info!(
        "(Node.{}) Handshaked by {}:{}",
        state.lock().unwrap().id,
        req.ip,
        req.rpc_port
    );
}
