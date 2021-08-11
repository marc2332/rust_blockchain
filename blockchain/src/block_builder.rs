use chrono::{
    DateTime,
    Utc,
};

use crate::{
    Block,
    BlockHash,
    Key,
    Wallet,
};

pub struct BlockBuilder {
    pub hash: Option<BlockHash>,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: Option<DateTime<Utc>>,
    pub payload: Option<String>,
    pub key: Option<Key>,
    pub signature: Option<Key>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            hash: None,
            previous_hash: None,
            timestamp: None,
            payload: None,
            key: None,
            signature: None,
        }
    }

    pub fn payload(&mut self, payload: &str) -> &mut Self {
        self.payload = Some(payload.to_string());
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
        // Terribly ugly, I know
        let data = format!(
            "{}{}{}{:?}",
            self.key.as_ref().unwrap(),
            self.timestamp.as_ref().unwrap(),
            self.payload.as_ref().unwrap(),
            self.previous_hash
        );
        self.signature = Some(acc.sign_data(data));
        self
    }

    pub fn build(&self) -> Block {
        Block::new(
            &self.payload.as_ref().unwrap(),
            self.timestamp.unwrap(),
            &self.previous_hash,
            &self.key.as_ref().unwrap(),
            &self.signature.as_ref().unwrap(),
        )
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}
