use chrono::{
    DateTime,
    Utc,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    BlockHash,
    Key,
    SignVerifier,
    Transaction,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub transactions: Vec<Transaction>,
    pub key: Key,
    pub signature: Key,
    pub index: Option<usize>,
}

pub enum BlocksErrors {
    WrongHash,
}

impl Block {
    pub fn new(
        transactions: Vec<Transaction>,
        timestamp: DateTime<Utc>,
        hash: &BlockHash,
        previous_hash: &Option<BlockHash>,
        key: &Key,
        signature: &Key,
    ) -> Self {
        let timestamp = timestamp.to_string();
        let signature = signature.clone();
        let hash = hash.clone();
        let previous_hash = previous_hash.clone();
        Self {
            hash,
            timestamp,
            transactions,
            previous_hash,
            key: key.clone(),
            signature,
            index: None,
        }
    }

    pub fn verify_integrity(&self) -> Result<(), BlocksErrors> {
        let must_hash = BlockHash::new(
            &self.transactions,
            self.timestamp.clone(),
            self.previous_hash.clone(),
            self.key.clone(),
        );

        if must_hash == self.hash {
            Ok(())
        } else {
            Err(BlocksErrors::WrongHash)
        }
    }

    pub fn verify_sign_with(&self, acc: &impl SignVerifier) -> bool {
        acc.verify_signature(&self.signature, self.hash.unite())
    }
}
