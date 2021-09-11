use std::collections::HashMap;

use blockchain::Chainstate;

use crate::Transaction;

#[derive(Default, Clone)]
pub struct Mempool {
    pub pending_transactions: HashMap<String, Transaction>,
    pub cached_transactions: Vec<Transaction>,
}

impl Mempool {
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        self.pending_transactions
            .insert(transaction.hash_it(), transaction.clone());
        if self.cached_transactions.len() >= 1000 {
            self.cached_transactions.remove(0);
        }
        self.cached_transactions.push(transaction.clone());
    }

    pub fn transaction_was_cached(&self, transaction: &Transaction) -> bool {
        for tx in &self.cached_transactions {
            if tx.get_hash() == transaction.get_hash() {
                return true;
            }
        }
        false
    }

    pub fn remove_transaction(&mut self, transaction_hash: &str) {
        self.pending_transactions.remove(transaction_hash);
    }

    pub fn verify_veracity_of_incoming_transactions(
        pending_transactions: &mut Vec<Transaction>,
        temporal_chainstate: &mut Chainstate,
    ) -> bool {
        for tx in pending_transactions {
            if tx.verify()
                && temporal_chainstate.verify_transaction_ammount(tx)
                && temporal_chainstate.verify_transaction_history(tx)
            {
                temporal_chainstate.effect_transaction(tx);
            } else {
                return false;
            }
        }
        true
    }

    pub fn verify_veracity_of_transactions(
        pending_transactions: &mut Vec<Transaction>,
        temporal_chainstate: &mut Chainstate,
    ) -> (Vec<Transaction>, Vec<Transaction>) {
        let mut ok_txs = Vec::new();
        let mut bad_txs = Vec::new();

        for tx in pending_transactions {
            if ok_txs.len() > 500 {
                break;
            }

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
}
