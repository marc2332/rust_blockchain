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
    tokio,
    NodeState,
};
use blockchain::{
    BlockBuilder,
    Transaction,
};
use jsonrpc_http_server::jsonrpc_core::*;

pub async fn add_transaction(
    state: &Arc<Mutex<NodeState>>,
    transaction: Transaction,
) -> Result<String> {
    // Make the transaction signature, hash... is ok and that the funds can be spent
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
        state.mempool.add_transaction(transaction);

        // Minimum transactions per block are harcoded for now
        if !state.mempool.pending_transactions.len() > 10 {
            /*
             * The elected forget is the one who must forge the block
             * This block will then by propagated to other nodes
             * If another node tries to propagate a block with a wrong forger it should be punished and ignored
             * WIP
             */
            let elected_forger = consensus::elect_forger(&state.blockchain).unwrap();

            if elected_forger == state.wallet.get_public().hash_it() {
                let block_data =
                    serde_json::to_string(&state.mempool.pending_transactions).unwrap();

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

                for (hostname, port) in state.peers.values() {
                    let hostname = hostname.clone();
                    let port = *port;
                    let new_block = new_block.clone();

                    tokio::spawn(async move {
                        let client = RPCClient::new(&format!("http://{}:{}", hostname, port))
                            .await
                            .unwrap();
                        client.add_block(new_block).await.unwrap();
                    });
                }
            }
        }

        Ok("Verified".to_string())
    } else {
        Ok("Bad verification".to_string())
    }
}
