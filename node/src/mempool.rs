use std::collections::HashMap;

use blockchain::Chainstate;

use crate::Transaction;

#[derive(Default, Clone)]
pub struct Mempool {
    pub pending_transactions: HashMap<String, Transaction>,
}

impl Mempool {
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        self.pending_transactions
            .insert(transaction.hash_it(), transaction.clone());
    }
    pub fn remove_transaction(&mut self, transaction_hash: &str) {
        self.pending_transactions.remove(transaction_hash);
    }
    pub fn verify_veracity_of_transactions(
        pending_transactions: &mut Vec<Transaction>,
        temporal_chainstate: &mut Chainstate,
    ) -> (Vec<Transaction>, Vec<Transaction>) {
        let mut ok_txs = Vec::new();
        let mut bad_txs = Vec::new();

        for tx in pending_transactions {
            // Make sure the funds are enough and the history is accurate
            if tx.verify()
                && temporal_chainstate.verify_transaction_ammount(tx)
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
}
