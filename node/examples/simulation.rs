#![feature(slice_pattern)]
#![feature(async_closure)]
use blockchain::{
    Block,
    BlockBuilder,
    Configuration,
    Transaction,
    TransactionBuilder,
    TransactionType,
    Wallet,
};
use chrono::Utc;
use client::RPCClient;
use futures::Future;
use node::Node;
use std::{
    thread,
    time,
};
use tracing_subscriber::{
    filter::EnvFilter,
    fmt,
    prelude::*,
    Registry,
};

fn create_configs() -> Vec<Configuration> {
    (0..5)
        .map(|i| {
            Configuration::from_params(
                i,
                5000 + i,
                7000 + i,
                "127.0.0.1",
                Wallet::default(),
                5,
                "mars",
            )
        })
        .collect()
}

/*
 * The idea of this simulation is to run a few nodes that share the same blockchain, verify transactions and get rewarded
 */
#[tokio::main]
async fn main() {
    let file_appender = tracing_appender::rolling::minutely("./", "simulation.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::default()
        .add_directive("node=info".parse().unwrap())
        .add_directive("blockchain=info".parse().unwrap());

    let subscriber = Registry::default()
        .with(filter)
        .with(fmt::Layer::default().with_writer(non_blocking))
        .with(fmt::Layer::default());

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set global subscriber");

    let mut node_configurations = create_configs();

    let mut nodes_runtimes = Vec::new();

    let mut genesis_wallet = Wallet::default();

    tracing::info!("Starting simulation");

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

    for _ in 0..1 {
        for i in 0..5 {
            let (tx, sender) = create_sender(&mut genesis_wallet, i);
            transactions.push(tx);
            senders_threads.push(sender);
        }
    }

    let genesis_block = BlockBuilder::new()
        .transactions(&transactions)
        .timestamp(Utc::now())
        .key(&genesis_wallet.get_public())
        .hash_it()
        .sign_with(&genesis_wallet)
        .build();

    for config in node_configurations {
        let mongo_client = config.mongo_client.clone();

        let db = mongo_client.database(&format!("db_{}", config.id));

        let coll = db.collection::<Block>("blocks");

        coll.drop(None).await.unwrap();

        let mut node = Node::new(config.clone()).await;
        let genesis_block = genesis_block.clone();

        nodes_runtimes.push(thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                node.sync_from_discovery_server().await;

                // Create a genesis block if there isn't
                if node
                    .state
                    .lock()
                    .unwrap()
                    .blockchain
                    .last_block_hash
                    .is_none()
                {
                    node.state
                        .lock()
                        .unwrap()
                        .blockchain
                        .add_block(&genesis_block)
                        .unwrap();
                    node.state.lock().unwrap().elect_new_forger();
                }

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
        let client = RPCClient::new_ws(&format!("ws://127.0.0.1:{}", 7000 + i))
            .await
            .unwrap();

        let temp_wallet = Wallet::default();

        for _ in 0..100000 {
            // Build the transaction
            let sample_tx = TransactionBuilder::new()
                .to_address(&temp_wallet.get_public().hash_it())
                .ammount(1)
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
