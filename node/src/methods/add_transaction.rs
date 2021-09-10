use std::sync::{
    Arc,
    Mutex,
};

use chrono::Utc;
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
    ThreadMsg,
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
    let was_tx_cached = state
        .lock()
        .unwrap()
        .mempool
        .transaction_was_cached(&transaction);

    if was_tx_cached {
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
        let peers = state.peers.clone();

        for (i, (hostname, port)) in peers.values().enumerate() {
            if i == 2 {
                break;
            }
            // Propagate the transactions to known peers
            let transaction_senders = state.transaction_senders.clone();

            transaction_senders[state.available_tx_sender]
                .send(ThreadMsg::PropagateTransaction {
                    transaction: transaction.clone(),
                    hostname: hostname.clone(),
                    port: *port,
                })
                .unwrap();
            state.available_tx_sender += 1;
            if state.available_tx_sender == transaction_senders.len() {
                state.available_tx_sender = 0;
            }
        }

        // Minimum transactions per block are harcoded for now
        let mempool_len = state.mempool.pending_transactions.len();
        if mempool_len > 50 {
            let elected_forger = state.next_forger.as_ref().unwrap().hash_it();

            if elected_forger == state.wallet.get_public().hash_it() {
                // Transform the pending transactions from a hashmap into a vector
                let mut pending_transactions = state
                    .mempool
                    .pending_transactions
                    .values()
                    .cloned()
                    .collect::<Vec<Transaction>>();

                // Sort transactions from lower history to higher
                pending_transactions.sort_by_key(|tx| tx.get_history());
                // Only get transactions that can be applied in the current chainstate (funds and history are ok)
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

                // Also add the block forging reward to the block
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

                // Add the block to the blockchain
                state.blockchain.add_block(&new_block).unwrap();

                // Elect the next forger
                state.elect_new_forger();

                ok_txs.append(&mut bad_txs);

                // Remove all good and bad transactions from the mempool
                for tx in ok_txs {
                    state.mempool.remove_transaction(&tx.get_hash());
                }

                // Propagate the block
                let block_senders = state.block_senders.clone();
                let peers = state.peers.clone();

                for (hostname, port) in peers.values() {
                    let hostname = hostname.clone();
                    let port = *port;
                    let block = new_block.clone();

                    block_senders[state.available_block_sender]
                        .send(ThreadMsg::PropagateBlock {
                            block,
                            hostname,
                            port,
                        })
                        .unwrap();
                    state.available_block_sender += 1;
                    if state.available_block_sender == block_senders.len() {
                        state.available_block_sender = 0;
                    }
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
