use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    Key,
    Transaction,
};

static HASH_VERSION: u8 = 1;

#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize, Default)]
pub struct BlockHash {
    pub hash: String,
    pub version: u8,
}

impl BlockHash {
    pub fn new(
        transactions: &[Transaction],
        timestamp: String,
        previous_hash: Option<BlockHash>,
        key: Key,
    ) -> Self {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        let transactions = serde_json::to_string(&transactions).unwrap();

        hasher.input_str(&HASH_VERSION.to_string());
        hasher.input_str(&transactions);
        hasher.input_str(&timestamp);
        hasher.input_str(&key.to_string());

        if let Some(previous_hash) = previous_hash {
            hasher.input_str(&previous_hash.hash);
        }

        let hash = hasher.result_str();
        Self {
            hash,
            version: HASH_VERSION,
        }
    }

    pub fn unite(&self) -> String {
        format!("{}x{}", self.version, self.hash)
    }
}
