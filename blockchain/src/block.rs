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
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub hash: BlockHash,
    pub previous_hash: Option<BlockHash>,
    pub timestamp: String,
    pub payload: String,
    pub key: Key,
    pub signature: Key,
    pub index: Option<usize>,
}

impl Block {
    pub fn new(
        payload: &str,
        timestamp: DateTime<Utc>,
        previous_hash: &Option<BlockHash>,
        key: &Key,
        signature: &Key,
    ) -> Self {
        let timestamp = timestamp.to_string();
        let payload = payload.to_string();
        let signature = signature.clone();
        let previous_hash = previous_hash.clone();
        Self {
            hash: BlockHash::new(
                payload.clone(),
                timestamp.clone(),
                previous_hash.clone(),
                key.clone(),
            ),
            timestamp,
            payload,
            previous_hash,
            key: key.clone(),
            signature,
            index: None,
        }
    }

    pub fn verify_integrity(&self) -> Result<(), ()> {
        let must_hash = BlockHash::new(
            self.payload.clone(),
            self.timestamp.clone(),
            self.previous_hash.clone(),
            self.key.clone(),
        );

        if must_hash == self.hash {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn verify_sign_with(&self, acc: &impl SignVerifier) -> bool {
        // Terribly ugly, I know
        let data = format!(
            "{}{}{}{:?}",
            self.key, self.timestamp, self.payload, self.previous_hash
        );

        acc.verify_signature(&self.signature, data)
    }
}
