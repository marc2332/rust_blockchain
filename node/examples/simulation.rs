#![feature(slice_pattern)]
use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    Transaction,
    TransactionBuilder,
    TransactionType,
    Wallet,
};
use chrono::Utc;
use client::RPCClient;
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
    (0..5)
        .map(|i| {
            std::fs::remove_dir_all(&format!("db_{}", i)).ok();

            let config = Configuration::from_params(
                i,
                &format!("db_{}", i),
                2000 + i,
                "127.0.0.1",
                Wallet::default(),
                4,
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

    let mut nodes = create_nodes();

    let mut nodes_runtimes = Vec::new();

    let mut genesis_wallet = Wallet::default();

    let genesis_transaction = TransactionBuilder::new()
        .to_address(&genesis_wallet.get_public().hash_it())
        .ammount(20000000000)
        .is_type(TransactionType::COINBASE)
        .with_wallet(&mut genesis_wallet)
        .build();

    // Make a coinbase and a stake transaction fore very node
    let mut staking_transactions = nodes
        .iter_mut()
        .flat_map(|(_, config)| {
            vec![
                TransactionBuilder::new()
                    .to_address(&config.wallet.get_public().hash_it())
                    .ammount(10)
                    .is_type(TransactionType::MOVEMENT)
                    .with_wallet(&mut genesis_wallet)
                    .build(),
                TransactionBuilder::new()
                    .ammount(2)
                    .is_type(TransactionType::STAKE)
                    .with_wallet(&mut config.wallet)
                    .build(),
            ]
        })
        .collect::<Vec<Transaction>>();

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
        nodes_runtimes.push(thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut node = node.clone();
                node.run(config.clone()).await;
            })
        }));
    }

    tokio::spawn(async move {
        let delay = time::Duration::from_millis(2000);
        thread::sleep(delay);

        let client = RPCClient::new("http://localhost:2000").await.unwrap();

        let wallet_b = Wallet::default();

        for i in 0..100000 {
            // Build the transaction
            let sample_tx = TransactionBuilder::new()
                .to_address(&wallet_b.get_public().hash_it())
                .ammount(i)
                .is_type(TransactionType::MOVEMENT)
                .with_wallet(&mut genesis_wallet)
                .build();

            client.add_transaction(sample_tx).await.ok();
        }
    })
    .await
    .unwrap();
}
