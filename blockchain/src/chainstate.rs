use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};

use crate::{
    Configuration,
    Transaction,
};

#[derive(Clone)]
pub struct Chainstate {
    pub config: Arc<Mutex<Configuration>>,
    pub addresses: HashMap<String, u64>,
    pub last_staking_addresses: Vec<Transaction>,
}

impl Chainstate {
    pub fn new(config: Arc<Mutex<Configuration>>) -> Self {
        Self {
            config,
            addresses: HashMap::new(),
            last_staking_addresses: Vec::new(),
        }
    }

    pub fn get_address_ammount(&self, address: String) -> u64 {
        *self.addresses.get(&address).unwrap_or(&0)
    }

    /*
     * Calculate the chainstate from the begining of the blockchain
     */
    pub fn load_from_chain(&mut self, name: &str) {
        let chain = self.config.lock().unwrap().get_blocks(name).unwrap();

        for block in chain.iter() {
            let transactions: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();
            for tx in transactions.iter() {
                self.effect_transaction(tx);
            }
        }
    }

    /*
     * Make sure a transaction can be spent
     */
    pub fn verify_transaction_ammount(&self, tx: &Transaction) -> bool {
        match tx {
            Transaction::MOVEMENT {
                from_address,
                ammount,
                ..
            } => {
                if let Some(address_amm) = &mut self.addresses.get(&from_address.clone()) {
                    *address_amm >= ammount
                } else {
                    false
                }
            }
            Transaction::STAKE {
                from_address,
                ammount,
                ..
            } => {
                if let Some(address_amm) = &mut self.addresses.get(&from_address.clone()) {
                    *address_amm >= ammount
                } else {
                    false
                }
            }
            Transaction::COINBASE { .. } => true,
        }
    }

    /*
     * Apply the proper changes to the chainstate when a transaction is ocurred
     */
    pub fn effect_transaction(&mut self, tx: &Transaction) {
        match tx {
            Transaction::MOVEMENT {
                from_address,
                to_address,
                ammount,
                ..
            } => {
                if let Some(address_amm) = self.addresses.get(&from_address.clone()) {
                    // Address is loaded
                    if address_amm >= ammount {
                        // Has enough ammount, OK

                        #[allow(mutable_borrow_reservation_conflict)]
                        self.addresses
                            .insert(from_address.clone(), address_amm - ammount);

                        if let Some(address_amm) = self.addresses.get(&to_address.clone()) {
                            #[allow(mutable_borrow_reservation_conflict)]
                            self.addresses
                                .insert(to_address.clone(), *address_amm + ammount);
                        } else {
                            self.addresses.insert(to_address.clone(), *ammount);
                        }
                    }
                }
            }
            Transaction::COINBASE {
                to_address,
                ammount,
                ..
            } => {
                if let Some(address_amm) = self.addresses.get(&to_address.clone()) {
                    #[allow(mutable_borrow_reservation_conflict)]
                    self.addresses
                        .insert(to_address.clone(), *address_amm + ammount);
                } else {
                    self.addresses.insert(to_address.clone(), *ammount);
                }
            }
            Transaction::STAKE { .. } => {
                self.last_staking_addresses.push(tx.clone());

                if self.last_staking_addresses.len() > 100 {
                    self.last_staking_addresses.pop();
                }
            }
        };
    }
}
