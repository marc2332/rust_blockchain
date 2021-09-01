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

use crate::NodeState;
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
        if state.mempool.pending_transactions.len() > 499 {
            /*
             * The elected forget is the one who must forge the block
             * This block will then by propagated to other nodes
             * If another node tries to propagate a block with a wrong forger it should be punished and ignored
             * WIP
             */
            let elected_forger = state.next_forger.hash_it();

            if elected_forger == state.wallet.get_public().hash_it() {
                let (mut ok_txs, mut bad_txs) = verify_veracity_of_transactions(&state);

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

fn verify_veracity_of_transactions(state: &NodeState) -> (Vec<Transaction>, Vec<Transaction>) {
    let mut ok_txs = Vec::new();
    let mut bad_txs = Vec::new();

    let mut mempool = state.mempool.clone();
    let pending_transactions = mempool.pending_transactions.clone();

    mempool.pending_transactions_list.sort_by(|hash_a, hash_b| {
        let tx_a = pending_transactions.get(hash_a).unwrap();
        let tx_b = pending_transactions.get(hash_b).unwrap();

        tx_a.get_history().cmp(&tx_b.get_history())
    });

    let mut temporal_chainstate = state.blockchain.state.clone();

    for tx_hash in mempool.pending_transactions_list {
        let tx = mempool.pending_transactions.get(&tx_hash).unwrap();
        // Make sure the funds are enough and the history is accurate
        if temporal_chainstate.verify_transaction_ammount(tx)
            && temporal_chainstate.verify_transaction_history(tx)
        {
            temporal_chainstate.effect_transaction(tx);
            ok_txs.push(tx.clone());
        } else {
            bad_txs.push(tx.clone());
        }
    }

    (ok_txs, bad_txs)
}
