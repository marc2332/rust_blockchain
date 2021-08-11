use crate::Transaction;

#[derive(Default)]
pub struct Mempool {
    pub pending_transactions: Vec<Transaction>,
}

impl Mempool {
    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.pending_transactions.push(transaction);
    }
}
