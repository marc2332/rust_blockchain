use std::sync::Arc;

use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    TransactionBuilder,
    Wallet,
};
use chrono::Utc;
use futures::future::join_all;
use jsonrpc_core::serde_json;
use node::Node;

fn create_nodes() -> Vec<(Node, Configuration)> {
    (0..15)
        .map(|i| {
            let config = Configuration::from_params(
                &format!("db_{}", i),
                3030 + i,
                "0.0.0.0",
                Wallet::default(),
            );

            let node = Node::new();

            (node, config)
        })
        .collect()
}

/*
 * The idea of this simulation is to run a few nodes that share the same blockchain and verify transactions
 */
#[tokio::main]
async fn main() {
    let nodes = create_nodes();

    let mut nodes_runtimes = Vec::new();

    let wallet = nodes[0].1.wallet.clone();

    let genesis_transaction = TransactionBuilder::new()
        .key(&wallet.get_public())
        .from_address("0x")
        .to_address(&wallet.get_public().hash_it())
        .ammount(100)
        .hash_it()
        .sign_with(&wallet)
        .build();

    let staking_transaction = TransactionBuilder::new()
        .key(&wallet.get_public())
        .from_address(&wallet.get_public().hash_it())
        .to_address("stake")
        .ammount(5)
        .hash_it()
        .sign_with(&wallet)
        .build();

    let block_data =
        serde_json::to_string(&vec![genesis_transaction, staking_transaction]).unwrap();

    let genesis_block = BlockBuilder::new()
        .payload(&block_data)
        .timestamp(Utc::now())
        .key(&wallet.get_public())
        .hash_it()
        .sign_with(&wallet)
        .build();

    println!("{:?}", wallet.get_private().0);

    for (node, config) in nodes {
        let mut blockchain =
            Blockchain::new("mars", Arc::new(std::sync::Mutex::new(config.clone())));

        // Create a genesis block if there isn't
        if blockchain.last_block_hash.is_none() {
            blockchain.add_block(&genesis_block);
            /*
             * All other 14 nodes should also stake a small ammount to be able to participate in forgint the next block
             */
        }
        nodes_runtimes.push(tokio::spawn(async move {
            let mut node = node.clone();
            node.run(config).await;
        }));
    }

    join_all(nodes_runtimes).await;
}
