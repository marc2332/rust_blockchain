use blockchain::{
    Block,
    Transaction,
};
use jsonrpc_client_transports::{
    transports::http,
    RpcChannel,
    RpcError,
    RpcResult,
    TypedClient,
};
use std::future::Future;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub address: String,
    pub ip: String,
    pub port: u16,
}

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
    pub fn make_handshake(&self, req: HandshakeRequest) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("make_handshake", "()", (req,))
    }
    pub fn add_transaction(&self, transaction: Transaction) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("add_transaction", "()", (transaction,))
    }
    pub fn add_block(&self, block: Block) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("add_block", "()", (block,))
    }
    pub fn get_block_with_prev_hash(
        &self,
        prev_hash: String,
    ) -> impl Future<Output = RpcResult<Option<Block>>> {
        self.0
            .call_method("get_block_with_prev_hash", "Option<Block>", (prev_hash,))
    }
}
