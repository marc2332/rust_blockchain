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

use crate::NodeState;
use blockchain::{
    BlockBuilder,
    Transaction,
};
use jsonrpc_http_server::jsonrpc_core::*;

pub async fn add_transaction(
    state: &Arc<Mutex<NodeState>>,
    transaction: Transaction,
) -> Result<String> {
    let tx_verification_is_ok = transaction.verify()
        && state
            .lock()
            .unwrap()
            .blockchain
            .state
            .verify_transaction_ammount(&transaction);

    if tx_verification_is_ok {
        let mut state = state.lock().unwrap();

        state.mempool.add_transaction(transaction);

        // Minimum transactions per block are harcoded for now
        if !state.mempool.pending_transactions.is_empty() {

            /*
             * Algorithm to randomly take a block creator from people who have staked a small ammount on previous blocks
             */
            let _last_100_stakings = {
                let mut stakers = Vec::<Transaction>::new();
                for (i, block) in state.blockchain.iter().enumerate() {
                    if i + 100 >= state.blockchain.chain.len() {
                        let txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

                        for transaction in txs {
                            let tx_verification_is_ok = transaction.verify();

                            if tx_verification_is_ok {
                                if transaction.to_address == "stake" && stakers.len() < 100{
                                    stakers.push(transaction);
                                }
                            } else {
                                println!("Blockchain is broken.")
                            }
                        }
                    }
                }
                stakers
            };

            let block_data = serde_json::to_string(&state.mempool.pending_transactions).unwrap();

            let new_block = BlockBuilder::new()
                .payload(&block_data)
                .timestamp(Utc::now())
                .key(&state.wallet.get_public())
                .previous_hash(&state.blockchain.last_block_hash.clone().unwrap())
                .hash_it()
                .sign_with(&state.wallet)
                .build();

            state.blockchain.add_block(&new_block);

            // Clear mempool
            state.mempool.pending_transactions = Vec::new();
        }

        Ok("Verified".to_string())
    } else {
        Ok("Bad verification".to_string())
    }
}
