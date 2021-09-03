use std::sync::{
    Arc,
    Mutex,
};

use chrono::Utc;
use client::RPCClient;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
pub enum TransactionResult {
    Verified,
    BadVerification,
}

use crate::{
    mempool::Mempool,
    NodeState,
};
use blockchain::{
    BlockBuilder,
    Transaction,
    TransactionBuilder,
    TransactionType,
};
use jsonrpc_http_server::jsonrpc_core::*;

pub async fn add_transaction(state: &Arc<Mutex<NodeState>>, transaction: Transaction) {
    // Check the transaction isn't already added into the mempool
    let txs_is_not_added = state
        .lock()
        .unwrap()
        .mempool
        .pending_transactions
        .get(&transaction.get_hash())
        .is_none();

    if !txs_is_not_added {
        return;
    }

    // Make the transaction signature, hash... are ok and that the funds can be spent
    let tx_verification_is_ok = transaction.verify()
        && state
            .lock()
            .unwrap()
            .blockchain
            .state
            .verify_transaction_ammount(&transaction);

    if tx_verification_is_ok {
        let mut state = state.lock().unwrap();

        // Add the transaction to the memory pool
        state.mempool.add_transaction(&transaction);

        // Propagate the transactions to known peers
        for (i, (hostname, port)) in state.peers.values().enumerate() {
            if i == 2 {
                break;
            }

            let hostname = hostname.clone();
            let port = *port;
            let transaction = transaction.clone();

            tokio::spawn(async move {
                let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                    .await
                    .unwrap();
                client.add_transaction(transaction).await.ok();
            });
        }

        // Minimum transactions per block are harcoded for now
        let mempool_len = state.mempool.pending_transactions.len();
        if mempool_len > 499 && mempool_len % 50 == 0 {
            /*
             * The elected forget is the one who must forge the block
             * This block will then by propagated to other nodes
             * If another node tries to propagate a block with a wrong forger it should be punished and ignored
             * WIP
             */
            let elected_forger = state.next_forger.hash_it();

            if elected_forger == state.wallet.get_public().hash_it() {
                let mut pending_transactions = state
                    .mempool
                    .pending_transactions
                    .values()
                    .cloned()
                    .collect::<Vec<Transaction>>();
                pending_transactions.sort_by_key(|tx| tx.get_history());
                let mut chainstate = state.blockchain.state.clone();
                let (mut ok_txs, mut bad_txs) = Mempool::verify_veracity_of_transactions(
                    &mut pending_transactions,
                    &mut chainstate,
                );

                // Coinbase transaction sent to the block forger as a reward
                let reward_tx = TransactionBuilder::new()
                    .to_address(&state.wallet.get_public().hash_it())
                    .ammount(10)
                    .is_type(TransactionType::COINBASE)
                    .with_wallet(&mut state.wallet)
                    .build();

                ok_txs.push(reward_tx);

                let block_data = serde_json::to_string(&ok_txs).unwrap();

                let new_block = BlockBuilder::new()
                    .payload(&block_data)
                    .timestamp(Utc::now())
                    .key(&state.wallet.get_public())
                    .previous_hash(&state.blockchain.last_block_hash.clone().unwrap())
                    .hash_it()
                    .sign_with(&state.wallet)
                    .build();

                state.blockchain.add_block(&new_block).unwrap();

                // Elect the next forger
                state.next_forger = consensus::elect_forger(&state.blockchain).unwrap();

                ok_txs.append(&mut bad_txs);

                for tx in ok_txs {
                    state.mempool.remove_transaction(&tx.get_hash());
                }

                for (hostname, port) in state.peers.values() {
                    let hostname = hostname.clone();
                    let port = *port;
                    let new_block = new_block.clone();
                    let id = state.id;

                    tokio::spawn(async move {
                        let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                            .await
                            .unwrap();
                        let res = client.add_block(new_block).await;

                        if res.is_err() {
                            log::error!("(Node.{}) Failed propagating block", id);
                        }
                    });
                }
            }
        }
        log::info!(
            "(Node.{}) Verified transaction ({})",
            state.id,
            state.mempool.pending_transactions.len()
        );
    } else {
        log::error!(
            "(Node.{}) Verification of transaction failed",
            state.lock().unwrap().id
        );
    }
}
