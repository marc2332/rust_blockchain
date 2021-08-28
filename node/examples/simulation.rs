#![feature(slice_pattern)]
use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    Transaction,
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
    (0..10)
        .map(|i| {
            std::fs::remove_dir_all(&format!("db_{}", i)).ok();

            let config = Configuration::from_params(
                i,
                &format!("db_{}", i),
                2000 + i,
                "127.0.0.1",
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

    let genesis_wallet = nodes[0].1.wallet.clone();

    let genesis_transaction = TransactionBuilder::new()
        .to_address(&genesis_wallet.get_public().hash_it())
        .ammount(20000000000)
        .hash_coinbase()
        .sign_with(&genesis_wallet)
        .build_coinbase();

    /*
    let mut staking_transactions = nodes
        .iter()
        .flat_map(|(_, config)| {
            let wallet = config.wallet.clone();
            vec![
                TransactionBuilder::new()
                    .key(&genesis_wallet.get_public())
                    .from_address(&genesis_wallet.get_public().hash_it())
                    .to_address(&wallet.get_public().hash_it())
                    .ammount(10)
                    .hash_movement()
                    .sign_with(&wallet)
                    .build_movement(),
                TransactionBuilder::new()
                    .key(&wallet.get_public())
                    .from_address(&wallet.get_public().hash_it())
                    .ammount(2)
                    .hash_stake()
                    .sign_with(&wallet)
                    .build_stake(),
            ]
        })
        .collect::<Vec<Transaction>>();
        */
    let mut staking_transactions = vec![TransactionBuilder::new()
        .key(&genesis_wallet.get_public())
        .from_address(&genesis_wallet.get_public().hash_it())
        .ammount(2)
        .hash_stake()
        .sign_with(&genesis_wallet)
        .build_stake()];

    let mut transactions = vec![genesis_transaction];
    transactions.append(&mut staking_transactions);

    let block_data = serde_json::to_string(&transactions).unwrap();

    let genesis_block = BlockBuilder::new()
        .payload(&block_data)
        .timestamp(Utc::now())
        .key(&genesis_wallet.get_public())
        .hash_it()
        .sign_with(&genesis_wallet)
        .build();

    println!("{:?}", genesis_wallet.get_private().0);

    for (node, config) in nodes.clone() {
        let mut blockchain =
            Blockchain::new("mars", Arc::new(std::sync::Mutex::new(config.clone())));

        // Create a genesis block if there isn't
        if blockchain.last_block_hash.is_none() {
            blockchain.add_block(&genesis_block).unwrap();
            /*
             * All other 14 nodes should also stake a small ammount to be able to participate in forgint the next block
             */
        }
        nodes_runtimes.push(tokio::spawn(async move {
            let mut node = node.clone();
            node.run(config).await;
        }));
    }

    tokio::spawn(async move {
        let delay = time::Duration::from_millis(2000);
        thread::sleep(delay);

        let client = RPCClient::new("http://localhost:2000").await.unwrap();

        let wallet_b = Wallet::default();

        for i in 0..10000 {
            // Build the transaction
            let sample_tx = TransactionBuilder::new()
                .key(&genesis_wallet.get_public())
                .from_address(&genesis_wallet.get_public().hash_it())
                .to_address(&wallet_b.get_public().hash_it())
                .ammount(i)
                .hash_movement()
                .sign_with(&genesis_wallet)
                .build_movement();

            client.add_transaction(sample_tx).await.ok();

            let delay = time::Duration::from_millis(100);
            thread::sleep(delay);
        }
    });

    join_all(nodes_runtimes).await;
}
