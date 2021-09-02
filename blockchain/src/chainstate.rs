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

#[derive(Default, Clone)]
pub struct AddressInfo {
    pub ammount: u64,
    // aka nonce
    pub history: u64,
}

#[derive(Clone)]
pub struct Chainstate {
    pub config: Arc<Mutex<Configuration>>,
    pub addresses: HashMap<String, AddressInfo>,
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
        self.addresses
            .get(&address)
            .unwrap_or(&AddressInfo {
                ammount: 0,
                history: 0,
            })
            .ammount
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
                if let Some(address_info) = &mut self.addresses.get(&from_address.clone()) {
                    address_info.ammount >= *ammount
                } else {
                    false
                }
            }
            Transaction::STAKE {
                from_address,
                ammount,
                ..
            } => {
                if let Some(address_info) = &mut self.addresses.get(&from_address.clone()) {
                    address_info.ammount >= *ammount
                } else {
                    false
                }
            }
            Transaction::COINBASE { .. } => true,
        }
    }

    /*
     * Verify the `history` of the transaction is accurate to the chainstate
     * This prevents transaction duplication
     */
    pub fn verify_transaction_history(&self, tx: &Transaction) -> bool {
        match tx {
            Transaction::MOVEMENT {
                from_address,
                history,
                ..
            } => {
                if let Some(address_info) = &mut self.addresses.get(&from_address.clone()) {
                    address_info.history == *history
                } else {
                    false
                }
            }
            Transaction::STAKE {
                from_address,
                history,
                ..
            } => {
                if let Some(address_info) = &mut self.addresses.get(&from_address.clone()) {
                    address_info.history == *history
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
                history,
                ..
            } => {
                let origin_is_valid = {
                    // Address does exist
                    if let Some(address_info) = self.addresses.get_mut(&from_address.clone()) {
                        // Has enough ammount and the history is correct
                        if &address_info.ammount >= ammount && &address_info.history == history {
                            // Remove the transaction ammount from the origin
                            address_info.ammount -= ammount;
                            address_info.history += 1;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                };

                if origin_is_valid {
                    if let Some(address_info) = self.addresses.get_mut(&to_address.clone()) {
                        address_info.ammount += ammount;
                    } else {
                        self.addresses.insert(
                            to_address.clone(),
                            AddressInfo {
                                ammount: *ammount,
                                history: 0,
                            },
                        );
                    }
                }
            }
            Transaction::COINBASE {
                to_address,
                ammount,
                ..
            } => {
                if let Some(address_info) = self.addresses.get_mut(&to_address.clone()) {
                    address_info.ammount += ammount;
                } else {
                    self.addresses.insert(
                        to_address.clone(),
                        AddressInfo {
                            ammount: *ammount,
                            history: 0,
                        },
                    );
                }
            }
            Transaction::STAKE {
                ammount,
                from_address,
                history,
                ..
            } => {
                if let Some(address_info) = self.addresses.get_mut(&from_address.clone()) {
                    // Has enough ammount and the history is correct
                    if &address_info.ammount >= ammount && &address_info.history == history {
                        address_info.ammount -= ammount;
                        address_info.history += 1;

                        self.last_staking_addresses.push(tx.clone());

                        if self.last_staking_addresses.len() > 100 {
                            self.last_staking_addresses.pop();
                        }
                    }
                }
            }
        };
    }
}
