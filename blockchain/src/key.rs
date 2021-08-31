use std::fmt;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Key(pub Vec<u8>);

#[allow(dead_code)]
impl Key {
    pub fn hash_it(&self) -> String {
        let str_key = self.to_string();
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(&str_key);
        hasher.result_str()
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .join(" ")
        )
    }
}
