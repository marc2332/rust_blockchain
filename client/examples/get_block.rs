use client::NodeClient;

#[tokio::main]
async fn main() {
    // Connect to the node's RPC server
    let client = NodeClient::new("http://localhost:5001").await.unwrap();

    let mut args = std::env::args();

    let hash = args.nth(1).unwrap();

    let block = client.get_block_with_hash(hash.clone()).await.unwrap();

    println!("{} -> {:?}", hash, block);
}
