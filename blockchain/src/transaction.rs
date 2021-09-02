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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Transaction {
    MOVEMENT {
        author_public_key: Key,
        signature: Key,
        from_address: String,
        to_address: String,
        ammount: u64,
        hash: String,
        history: u64,
    },
    COINBASE {
        to_address: String,
        ammount: u64,
        hash: String,
    },
    STAKE {
        author_public_key: Key,
        signature: Key,
        from_address: String,
        ammount: u64,
        hash: String,
        history: u64,
    },
}

impl Transaction {
    pub fn get_hash(&self) -> String {
        match self {
            Transaction::MOVEMENT { hash, .. } => hash,
            Transaction::COINBASE { hash, .. } => hash,
            Transaction::STAKE { hash, .. } => hash,
        }
        .to_string()
    }

    pub fn get_history(&self) -> u64 {
        *match self {
            Transaction::MOVEMENT { history, .. } => history,
            Transaction::COINBASE { .. } => &0_u64,
            Transaction::STAKE { history, .. } => history,
        }
    }

    pub fn hash_it(&self) -> String {
        match self {
            Transaction::MOVEMENT {
                author_public_key,
                from_address,
                to_address,
                ammount,
                history,
                ..
            } => {
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);
                hasher.input_str(&author_public_key.to_string());
                hasher.input_str(from_address);
                hasher.input_str(to_address);
                hasher.input_str(&ammount.to_string());
                hasher.input_str(&history.to_string());
                hasher.result_str()
            }
            Transaction::COINBASE {
                to_address,
                ammount,
                ..
            } => {
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);
                hasher.input_str(to_address);
                hasher.input_str(&ammount.to_string());
                hasher.result_str()
            }
            Transaction::STAKE {
                author_public_key,
                from_address,
                ammount,
                history,
                ..
            } => {
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);
                hasher.input_str(&author_public_key.to_string());
                hasher.input_str(from_address);
                hasher.input_str(&ammount.to_string());
                hasher.input_str(&history.to_string());
                hasher.result_str()
            }
        }
    }

    pub fn verify(&self) -> bool {
        match self {
            Transaction::MOVEMENT {
                author_public_key,
                signature,
                from_address,
                hash,
                ..
            } => {
                let public_key_hashed = author_public_key.hash_it();

                // Ensure the hashed public key is the same as the from_address
                if &public_key_hashed != from_address {
                    return false;
                }

                // Make sure the hash is not altered
                if &self.hash_it() != hash {
                    return false;
                }

                // Verify the signature
                let public_address = PublicAddress::from(author_public_key);

                public_address.verify_signature(signature, hash.to_string())
            }
            Transaction::COINBASE { hash, .. } => {
                // Make sure the hash is not altered
                if &self.hash_it() != hash {
                    return false;
                }

                true
            }
            Transaction::STAKE {
                author_public_key,
                signature,
                from_address,
                hash,
                ..
            } => {
                let public_key_hashed = author_public_key.hash_it();

                // Ensure the hashed public key is the same as the from_address
                if &public_key_hashed != from_address {
                    return false;
                }

                // Make sure the hash is not altered
                if &self.hash_it() != hash {
                    return false;
                }

                // Verify the signature
                let public_address = PublicAddress::from(author_public_key);

                public_address.verify_signature(signature, hash.to_string())
            }
        }
    }
}
