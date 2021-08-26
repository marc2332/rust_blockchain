#![feature(slice_pattern)]
use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    TransactionBuilder,
    Wallet,
};
use chrono::Utc;
use client::RPCClient;
use futures::future::join_all;
use jsonrpc_core::serde_json;
use log::LevelFilter;
use node::Node;
use simple_logger::SimpleLogger;
use std::{
    sync::Arc,
    thread,
    time,
};

fn create_nodes() -> Vec<(Node, Configuration)> {
    (0..15)
        .map(|i| {
            std::fs::remove_dir_all(&format!("db_{}", i)).ok();

            let config = Configuration::from_params(
                i,
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
    SimpleLogger::new()
        .with_timestamps(false)
        .with_colors(true)
        .with_level(LevelFilter::Off)
        .with_module_level("node", LevelFilter::Info)
        .with_module_level("blockchain", LevelFilter::Info)
        .init()
        .unwrap();

    let nodes = create_nodes();

    let mut nodes_runtimes = Vec::new();

    let wallet = nodes[0].1.wallet.clone();

    let genesis_transaction = TransactionBuilder::new()
        .to_address(&wallet.get_public().hash_it())
        .ammount(500000)
        .hash_coinbase()
        .sign_with(&wallet)
        .build_coinbase();

    let staking_transaction = TransactionBuilder::new()
        .key(&wallet.get_public())
        .from_address(&wallet.get_public().hash_it())
        .ammount(5)
        .hash_stake()
        .sign_with(&wallet)
        .build_stake();

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
            blockchain.add_block(&genesis_block).unwrap();
            /*
             * All other 14 nodes should also stake a small ammount to be able to participate in forgint the next block
             */
        }
        if nodes_runtimes.len() < 15 {
            nodes_runtimes.push(tokio::spawn(async move {
                let mut node = node.clone();
                node.run(config).await;
            }));
        }
    }

    tokio::spawn(async move {
        let delay = time::Duration::from_millis(2000);
        thread::sleep(delay);

        let client = RPCClient::new("http://localhost:3030").await.unwrap();

        let wallet_b = Wallet::new();

        for _ in 0..200000 {
            // Build the transaction
            let sample_tx = TransactionBuilder::new()
                .key(&wallet.get_public())
                .from_address(&wallet.get_public().hash_it())
                .to_address(&wallet_b.get_public().hash_it())
                .ammount(1)
                .hash_movement()
                .sign_with(&wallet)
                .build_movement();

            let client = client.clone();

            let res = client.add_transaction(sample_tx).await;

            println!("{:?}", res);
        }
    });

    join_all(nodes_runtimes).await;
}
