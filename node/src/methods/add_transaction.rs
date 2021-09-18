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
use chrono::{
    DateTime,
    Utc,
};
use jsonrpc_http_server::jsonrpc_core::*;
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    str::FromStr,
    sync::{
        Arc,
        Mutex,
    },
};

static BLOCK_TIME_MAX: i64 = 8000;
static MINIMUM_MEMPOOL_SIZE: usize = 50;
static TRANSACTIONS_CHUNK_SIZE: usize = 2;

#[derive(Serialize, Deserialize)]
pub enum TransactionResult {
    Verified,
    BadVerification,
}

pub async fn add_transaction(state: &Arc<Mutex<NodeState>>, transaction: Transaction) {
    // Check the transaction isn't already added into the mempool
    let is_tx_cached = state
        .lock()
        .unwrap()
        .mempool
        .is_transaction_cached(&transaction);

    if is_tx_cached {
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

        // Save the transaction for the next chunk
        state.mempool.chunked_transactions.push(transaction);

        // Propagate transactions as chunks
        if state.mempool.chunked_transactions.len() > TRANSACTIONS_CHUNK_SIZE {
            // Propagate the transactions chunk to known peers
            let peers = state.peers.clone();
            let transaction_senders = state.transaction_senders.clone();
            for (hostname, port) in peers.values() {
                let transactions = state.mempool.chunked_transactions.clone();

                transaction_senders[state.available_tx_sender]
                    .send(ThreadMsg::PropagateTransactions {
                        transactions,
                        hostname: hostname.clone(),
                        port: *port,
                    })
                    .unwrap();
                state.available_tx_sender += 1;

                if state.available_tx_sender == transaction_senders.len() {
                    state.available_tx_sender = 0;
                }
            }

            state.mempool.chunked_transactions.clear();
        }

        let mempool_len = state.mempool.pending_transactions.len();

        // Minimum transactions per block are harcoded for now
        if mempool_len > MINIMUM_MEMPOOL_SIZE {
            let elected_forger = state.next_forger.as_ref().unwrap().hash_it();

            // Only the elected forger can create new blocks
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
                    &pending_transactions,
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

            // Punish the current block forger if he missed his time to create a block
            if let Some(current_forger) = state.next_forger.clone() {
                /*
                 * Make sure he is not punished already (this shouldn't be the case anyway)
                 * And just to prevent initial issues, make sure the current chain height is greater than 5
                 */
                if !state
                    .blockchain
                    .state
                    .is_punished(&current_forger.hash_it())
                    && state.blockchain.index > 5
                {
                    let previous_forgers_are_blocked =
                        !state.blockchain.state.missed_forgers.is_empty();

                    /*
                     * It wouldn't be fair to diff the time between last block and now because there has been a blocked forger in between.
                     */
                    if !previous_forgers_are_blocked {
                        let last_block = state.blockchain.chain.last().unwrap();
                        let last_block_time: DateTime<Utc> =
                            DateTime::from_str(&last_block.timestamp).unwrap();

                        let current_time = Utc::now();

                        let time_diff = current_time.signed_duration_since(last_block_time);

                        // Punish the forger if he missed for configured time
                        if time_diff.num_milliseconds() > BLOCK_TIME_MAX {
                            // Block creation timeout
                            let block_index = state.blockchain.index;
                            state
                                .blockchain
                                .state
                                .missed_forgers
                                .insert(current_forger.hash_it(), block_index);

                            state.elect_new_forger();

                            log::warn!("Blocked forger = {}", current_forger.hash_it());
                        }
                    }
                }
            }

            // Forgive older forgers that missed it's block
            for (forger, block_index) in state.blockchain.state.missed_forgers.clone() {
                if block_index < state.blockchain.index {
                    log::warn!("Unblocked forger = {}", forger);
                    state.blockchain.state.missed_forgers.remove(&forger);
                }
            }
        }

        log::info!(
            "(Node.{}) Confirmed transaction ({})",
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
