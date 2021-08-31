use std::collections::HashMap;

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
}
