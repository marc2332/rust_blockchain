use std::future::Future;

use jsonrpc_client_transports::{
    transports::http,
    RpcChannel,
    RpcResult,
    TypedClient,
};
#[derive(Clone)]
struct RPCCLient(TypedClient);

impl From<RpcChannel> for RPCCLient {
    fn from(channel: RpcChannel) -> Self {
        RPCCLient(channel.into())
    }
}

impl RPCCLient {
    fn get_chain_length(&self) -> impl Future<Output = RpcResult<usize>> {
        self.0.call_method("get_chain_length", "Number", ())
    }
    fn make_handshake(&self) -> impl Future<Output = RpcResult<()>> {
        self.0.call_method("make_handshake", "()", ())
    }
}

#[tokio::main]
async fn main() {
    let client: RPCCLient = http::connect("http://localhost:3030").await.unwrap();

    let n = client.get_chain_length().await.unwrap();

    println!("{}", n);

    client.make_handshake().await.unwrap();
}
