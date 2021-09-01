use std::collections::HashMap;

use crate::Transaction;

#[derive(Default, Clone)]
pub struct Mempool {
    pub pending_transactions: HashMap<String, Transaction>,
    pub pending_transactions_list: Vec<String>,
}

impl Mempool {
    pub fn add_transaction(&mut self, transaction: &Transaction) {
        self.pending_transactions
            .insert(transaction.hash_it(), transaction.clone());
        self.pending_transactions_list.push(transaction.get_hash());
    }
    pub fn remove_transaction(&mut self, transaction_hash: &str) {
        self.pending_transactions_list = self
            .pending_transactions_list
            .iter()
            .filter_map(|tx_hash| {
                if tx_hash.as_str() == transaction_hash {
                    None
                } else {
                    Some(tx_hash.clone())
                }
            })
            .collect::<Vec<String>>();
        self.pending_transactions.remove(transaction_hash);
    }
}
