use blockchain::Transaction;
use jsonrpc_client_transports::{
    transports::http,
    RpcChannel,
    RpcError,
    RpcResult,
    TypedClient,
};
use std::future::Future;

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
    pub fn add_transaction(
        &self,
        transaction: Transaction,
    ) -> impl Future<Output = RpcResult<String>> {
        self.0
            .call_method("add_transaction", "String", (transaction,))
    }
}
