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

pub enum TransactionType {
    MOVEMENT,
    STAKE,
    COINBASE,
}

pub struct TransactionBuilder {
    pub author_public_key: Option<Key>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub ammount: Option<u64>,
    pub history: Option<u64>,
    pub type_tx: Option<TransactionType>,
    pub wallet: Option<Wallet>,
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
            history: None,
            type_tx: None,
            wallet: None,
        }
    }

    pub fn to_address(&mut self, to_address: &str) -> &mut Self {
        self.to_address = Some(to_address.to_string());
        self
    }

    pub fn ammount(&mut self, ammount: u64) -> &mut Self {
        self.ammount = Some(ammount);
        self
    }

    pub fn is_type(&mut self, type_tx: TransactionType) -> &mut Self {
        self.type_tx = Some(type_tx);
        self
    }

    pub fn with_wallet(&mut self, wallet: &mut Wallet) -> &mut Self {
        self.wallet = Some(wallet.clone());
        self.history = Some(wallet.history);
        self.author_public_key = Some(wallet.get_public());
        self.from_address = Some(wallet.get_public().hash_it());
        match self.type_tx.as_ref().unwrap() {
            TransactionType::COINBASE { .. } => {}
            _ => {
                wallet.history += 1;
            }
        };
        self
    }

    pub fn build(&self) -> Transaction {
        let type_tx = self.type_tx.as_ref().unwrap();

        match type_tx {
            TransactionType::COINBASE => {
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);

                hasher.input_str(self.to_address.as_ref().unwrap());
                hasher.input_str(&self.ammount.unwrap().to_string());

                let hash = hasher.result_str();

                Transaction::COINBASE {
                    to_address: self.to_address.as_ref().unwrap().clone(),
                    ammount: *self.ammount.as_ref().unwrap(),
                    hash,
                }
            }
            TransactionType::MOVEMENT => {
                let wallet = self.wallet.as_ref().unwrap();
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);

                hasher.input_str(&self.author_public_key.as_ref().unwrap().to_string());
                hasher.input_str(self.from_address.as_ref().unwrap());
                hasher.input_str(self.to_address.as_ref().unwrap());
                hasher.input_str(&self.ammount.unwrap().to_string());
                hasher.input_str(&self.history.unwrap().to_string());

                let hash = hasher.result_str();
                let signature = wallet.sign_data(hash.clone());

                Transaction::MOVEMENT {
                    author_public_key: self.author_public_key.as_ref().unwrap().clone(),
                    signature,
                    from_address: self.from_address.as_ref().unwrap().clone(),
                    to_address: self.to_address.as_ref().unwrap().clone(),
                    ammount: *self.ammount.as_ref().unwrap(),
                    hash,
                    history: self.history.unwrap(),
                }
            }
            TransactionType::STAKE => {
                let wallet = self.wallet.as_ref().unwrap();
                let mut hasher = Sha3::new(Sha3Mode::Keccak256);

                hasher.input_str(&self.author_public_key.as_ref().unwrap().to_string());
                hasher.input_str(self.from_address.as_ref().unwrap());
                hasher.input_str(&self.ammount.unwrap().to_string());
                hasher.input_str(&self.history.unwrap().to_string());

                let hash = hasher.result_str();
                let signature = wallet.sign_data(hash.clone());

                Transaction::STAKE {
                    author_public_key: self.author_public_key.as_ref().unwrap().clone(),
                    signature,
                    from_address: self.from_address.as_ref().unwrap().clone(),
                    ammount: *self.ammount.as_ref().unwrap(),
                    hash,
                    history: self.history.unwrap(),
                }
            }
        }
    }
}
