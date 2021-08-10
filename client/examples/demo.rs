use client::RPCClient;

#[tokio::main]
async fn main() {
    // Connect to the node's RPC server
    let client = RPCClient::new("http://localhost:3030").await.unwrap();

    // Easily call methods remotely
    let chain_length = client.get_chain_length().await.unwrap();

    println!("{}", chain_length);

    client.make_handshake().await.unwrap();
}