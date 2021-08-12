use std::sync::{
    Arc,
    Mutex,
};

use chrono::Utc;
use serde::{
    Deserialize,
    Serialize,
};

use consensus::{
    GoalBuilder,
    Player,
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
    let tx_verification_is_ok = transaction.verify();
    // Should validate if there are enough funds

    if tx_verification_is_ok {
        let mut state = state.lock().unwrap();

        state.mempool.add_transaction(transaction);

        // Minimum transactions are harcoded for now
        if state.mempool.pending_transactions.len() > 0 {
            let minner = Player::new(0);

            let last_hash = state.blockchain.peek().unwrap().hash.unite();

            let mut goal = GoalBuilder::new()
                .zeros(3)
                .data(last_hash)
                .player(minner)
                .build();

            let _result = goal.start().await;

            let block_data = state
                .mempool
                .pending_transactions
                .iter()
                .map(|tx| serde_json::to_string(tx).unwrap())
                .collect::<String>();

            let new_block = BlockBuilder::new()
                .payload(&block_data)
                .timestamp(Utc::now())
                .key(&state.wallet.get_public())
                .previous_hash(&state.blockchain.last_block_hash.clone().unwrap())
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
