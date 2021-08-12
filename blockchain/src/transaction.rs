use crate::{
    Key,
    PublicAddress,
    SignVerifier,
};
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub author_public_key: Key,
    pub signature: Key,
    // The Hashed public key must be the same as the from_address
    pub from_address: String,
    pub to_address: String,
    pub ammount: u64,
    pub hash: String,
}

impl Transaction {
    pub fn hash_it(&self) -> String {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(&self.author_public_key.to_string());
        hasher.input_str(&self.signature.hash_it());
        hasher.input_str(&self.from_address);
        hasher.input_str(&self.to_address);
        hasher.input_str(&self.ammount.to_string());
        hasher.result_str()
    }

    pub fn verify(&self) -> bool {
        let public_key_hashed = self.author_public_key.hash_it();

        // Ensure the hashed public key is the same as the from_address
        if public_key_hashed != self.from_address {
            return false;
        }

        // Make sure the hash is not altered
        if self.hash_it() != self.hash {
            return false;
        }

        // Verify the signature
        let public_address = PublicAddress::from(&self.author_public_key);

        let data = format!(
            "{}{}{}{}",
            self.author_public_key, self.from_address, self.to_address, self.ammount
        );

        public_address.verify_signature(&self.signature, data)
    }
}
