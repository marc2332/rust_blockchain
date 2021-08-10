use std::future::Future;

use jsonrpc_client_transports::{RpcChannel, RpcError, RpcResult, TypedClient, transports::http};
#[derive(Clone)]
pub struct RPCClient(TypedClient);

impl From<RpcChannel> for RPCClient {
    fn from(channel: RpcChannel) -> Self {
        RPCClient(channel.into())
    }
}

impl RPCClient {
    pub async fn new(uri: &str) -> Result<Self, RpcError> {
        http::connect(uri).await
    }
}

impl RPCClient {
    pub fn get_chain_length(&self) -> impl Future<Output = RpcResult<usize>> {
        self.0.call_method("get_chain_length", "Number", ())
    }
    pub fn make_handshake(&self) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("make_handshake", "()", ())
    }
}
