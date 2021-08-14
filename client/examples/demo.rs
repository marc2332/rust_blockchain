use blockchain::{
    TransactionBuilder,
    Wallet,
};
use client::RPCClient;

#[tokio::main]
async fn main() {
    // Connect to the node's RPC server
    let client = RPCClient::new("http://localhost:3030").await.unwrap();

    // Easily call methods remotely
    let _chain_length = client.get_chain_length().await.unwrap();

    //println!("{}", chain_length);

    //client.make_handshake().await.unwrap();

    let wallet_a = Wallet::new();
    let wallet_b = Wallet::new();

    let sample_tx = TransactionBuilder::new()
        .key(&wallet_a.get_public())
        .from_address(&wallet_a.get_public().hash_it())
        .to_address(&wallet_b.get_public().hash_it())
        .ammount(1)
        .hash_it()
        .sign_with(&wallet_a)
        .build();

    let res = client.add_transaction(sample_tx).await;

    println!("{:?}", res.unwrap());
}
