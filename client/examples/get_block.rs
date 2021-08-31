use client::RPCClient;

#[tokio::main]
async fn main() {
    // Connect to the node's RPC server
    let client = RPCClient::new("http://localhost:2000").await.unwrap();

    let mut args = std::env::args();

    args.next();

    let hash = args.next().unwrap();

    println!("{}", hash);

    let block = client.get_block_with_prev_hash(hash).await.unwrap();

    println!("{:?}", block);
}
