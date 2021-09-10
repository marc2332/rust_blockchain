#![feature(slice_pattern)]
#![feature(async_closure)]
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
use futures::Future;
use jsonrpc_core::serde_json;
use log::LevelFilter;
use node::Node;
use simple_logger::SimpleLogger;
use std::{
    sync::Arc,
    thread,
    time,
};

fn create_configs() -> Vec<Configuration> {
    (0..7)
        .map(|i| {
            std::fs::remove_dir_all(&format!("db_{}", i)).ok();

            Configuration::from_params(
                i,
                &format!("db_{}", i),
                2000 + i,
                "127.0.0.1",
                Wallet::default(),
                1,
            )
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

    let mut node_configurations = create_configs();

    let mut nodes_runtimes = Vec::new();

    let mut genesis_wallet = Wallet::default();

    log::info!("Starting simulation");

    let genesis_transaction = TransactionBuilder::new()
        .to_address(&genesis_wallet.get_public().hash_it())
        .ammount(200000000000)
        .is_type(TransactionType::COINBASE)
        .with_wallet(&mut genesis_wallet)
        .build();

    // Make a coinbase and a stake transaction fore very node
    let mut staking_transactions = node_configurations
        .iter_mut()
        .flat_map(|config| {
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

    let mut senders_threads = Vec::new();

    for _ in 0..2 {
        for i in 0..7 {
            let (tx, sender) = create_sender(&mut genesis_wallet, i);
            transactions.push(tx);
            senders_threads.push(sender);
        }
    }

    let block_data = serde_json::to_string(&transactions).unwrap();

    let genesis_block = BlockBuilder::new()
        .payload(&block_data)
        .timestamp(Utc::now())
        .key(&genesis_wallet.get_public())
        .hash_it()
        .sign_with(&genesis_wallet)
        .build();

    for config in node_configurations {
        let mut node = Node::new(config.clone());
        let genesis_block = genesis_block.clone();

        nodes_runtimes.push(thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                node.sync_from_discovery_server().await;
                let mut state = node.state.lock().unwrap();

                // Create a genesis block if there isn't
                if state.blockchain.last_block_hash.is_none() {
                    state.blockchain.add_block(&genesis_block).unwrap();
                    state.elect_new_forger();
                }

                drop(state);

                node.run().await;
            })
        }));
    }

    std::thread::spawn(move || {
        discovery_server::main().unwrap();
    });

    let delay = time::Duration::from_millis(2000);
    thread::sleep(delay);

    futures::future::join_all(senders_threads).await;
}

fn create_sender(genesis_wallet: &mut Wallet, i: u16) -> (Transaction, impl Future<Output = ()>) {
    let mut sender_wallet = Wallet::default();

    let transaction = TransactionBuilder::new()
        .to_address(&sender_wallet.get_public().hash_it())
        .ammount(2000000)
        .is_type(TransactionType::MOVEMENT)
        .with_wallet(genesis_wallet)
        .build();

    let sender = std::thread::spawn(async move || {
        let client = RPCClient::new(&format!("http://localhost:{}", 2000 + i))
            .await
            .unwrap();

        let temp_wallet = Wallet::default();

        for _ in 0..100000 {
            // Build the transaction
            let sample_tx = TransactionBuilder::new()
                .to_address(&temp_wallet.get_public().hash_it())
                .ammount(100)
                .is_type(TransactionType::MOVEMENT)
                .with_wallet(&mut sender_wallet)
                .build();

            client.add_transaction(sample_tx).await.ok();
        }
    })
    .join()
    .unwrap();

    (transaction, sender)
}
