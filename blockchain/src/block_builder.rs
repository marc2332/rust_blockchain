use chrono::{
    DateTime,
    Utc,
};

use crate::{
    Block,
    BlockHash,
    Key,
    Transaction,
    Wallet,
};

pub struct BlockBuilder {
    pub hash: Option<BlockHash>,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: Option<DateTime<Utc>>,
    pub transactions: Vec<Transaction>,
    pub key: Option<Key>,
    pub signature: Option<Key>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            hash: None,
            previous_hash: None,
            timestamp: None,
            transactions: vec![],
            key: None,
            signature: None,
        }
    }

    pub fn transactions(&mut self, transactions: &[Transaction]) -> &mut Self {
        self.transactions = transactions.to_vec();
        self
    }

    pub fn previous_hash(&mut self, previous_hash: &BlockHash) -> &mut Self {
        self.previous_hash = Some(previous_hash.clone());
        self
    }

    pub fn timestamp(&mut self, timestamp: DateTime<Utc>) -> &mut Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn key(&mut self, key: &Key) -> &mut Self {
        self.key = Some(key.clone());
        self
    }

    pub fn sign_with(&mut self, acc: &Wallet) -> &mut Self {
        self.signature = Some(acc.sign_data(self.hash.as_ref().unwrap().unite()));
        self
    }

    pub fn hash_it(&mut self) -> &mut Self {
        self.hash = Some(BlockHash::new(
            &self.transactions,
            self.timestamp.unwrap().to_string(),
            self.previous_hash.clone(),
            self.key.as_ref().unwrap().clone(),
        ));
        self
    }

    pub fn build(&self) -> Block {
        Block::new(
            self.transactions.clone(),
            self.timestamp.unwrap(),
            self.hash.as_ref().unwrap(),
            &self.previous_hash,
            self.key.as_ref().unwrap(),
            self.signature.as_ref().unwrap(),
        )
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}
