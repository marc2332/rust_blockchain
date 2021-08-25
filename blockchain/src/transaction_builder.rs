use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};

use crate::{
    Key,
    Transaction,
    Wallet,
};

pub struct TransactionBuilder {
    pub author_public_key: Option<Key>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub ammount: Option<u64>,
    pub hash: Option<String>,
    pub signature: Option<Key>,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            author_public_key: None,
            from_address: None,
            to_address: None,
            ammount: None,
            hash: None,
            signature: None,
        }
    }

    pub fn from_address(&mut self, from_address: &str) -> &mut Self {
        self.from_address = Some(from_address.to_string());
        self
    }

    pub fn to_address(&mut self, to_address: &str) -> &mut Self {
        self.to_address = Some(to_address.to_string());
        self
    }

    pub fn ammount(&mut self, ammount: u64) -> &mut Self {
        self.ammount = Some(ammount);
        self
    }

    pub fn key(&mut self, key: &Key) -> &mut Self {
        self.author_public_key = Some(key.clone());
        self
    }

    pub fn hash_movement(&mut self) -> &mut Self {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(&self.author_public_key.as_ref().unwrap().to_string());
        hasher.input_str(self.from_address.as_ref().unwrap());
        hasher.input_str(self.to_address.as_ref().unwrap());
        hasher.input_str(&self.ammount.unwrap().to_string());
        self.hash = Some(hasher.result_str());
        self
    }

    pub fn hash_stake(&mut self) -> &mut Self {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(&self.author_public_key.as_ref().unwrap().to_string());
        hasher.input_str(self.from_address.as_ref().unwrap());
        hasher.input_str(&self.ammount.unwrap().to_string());
        self.hash = Some(hasher.result_str());
        self
    }

    pub fn hash_coinbase(&mut self) -> &mut Self {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(self.to_address.as_ref().unwrap());
        hasher.input_str(&self.ammount.unwrap().to_string());
        self.hash = Some(hasher.result_str());
        self
    }

    pub fn sign_with(&mut self, acc: &Wallet) -> &mut Self {
        self.signature = Some(acc.sign_data(self.hash.as_ref().unwrap().to_string()));
        self
    }

    pub fn build_movement(&self) -> Transaction {
        Transaction::MOVEMENT {
            author_public_key: self.author_public_key.as_ref().unwrap().clone(),
            signature: self.signature.as_ref().unwrap().clone(),
            from_address: self.from_address.as_ref().unwrap().clone(),
            to_address: self.to_address.as_ref().unwrap().clone(),
            ammount: *self.ammount.as_ref().unwrap(),
            hash: self.hash.as_ref().unwrap().clone(),
        }
    }

    pub fn build_stake(&self) -> Transaction {
        Transaction::STAKE {
            author_public_key: self.author_public_key.as_ref().unwrap().clone(),
            signature: self.signature.as_ref().unwrap().clone(),
            from_address: self.from_address.as_ref().unwrap().clone(),
            ammount: *self.ammount.as_ref().unwrap(),
            hash: self.hash.as_ref().unwrap().clone(),
        }
    }

    pub fn build_coinbase(&self) -> Transaction {
        Transaction::COINBASE {
            to_address: self.to_address.as_ref().unwrap().clone(),
            ammount: *self.ammount.as_ref().unwrap(),
            hash: self.hash.as_ref().unwrap().clone(),
        }
    }
}
