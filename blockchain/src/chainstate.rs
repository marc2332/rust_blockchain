use std::{collections::HashMap, sync::{Arc, Mutex}};

use crate::{Configuration, Transaction};

pub struct Chainstate {
    pub config: Arc<Mutex<Configuration>>,
    pub addresses: HashMap<String, u64>
}

impl Chainstate {

    pub fn new(config: Arc<Mutex<Configuration>>) -> Self {
        Self {
            config,
            addresses: HashMap::new()
        }
    }

    /*
     * Calculate the chainstate from the begining of the blockchain
     */
    pub fn load_from_chain(&mut self, name:&str) {

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
        if tx.from_address != "0x" {
            if let Some(address_amm) = &mut self.addresses.get(&tx.from_address.clone()) {
                **address_amm >= tx.ammount
            } else {
                false
            }
        } else {
            true
        }
    }

    /*
     * Apply the proper changes to the chainstate when a transaction is ocurred
     */
    pub fn effect_transaction(&mut self, tx: &Transaction) {
        if tx.from_address != "0x" {
            if let Some(address_amm) = self.addresses.get(&tx.from_address.clone()) {
                // Address is loaded
                if *address_amm >= tx.ammount {
                    // Has enough ammount, OK

                    #[allow(mutable_borrow_reservation_conflict)]
                    self.addresses.insert(tx.from_address.clone(), address_amm - tx.ammount);

                    if let Some(address_amm) = self.addresses.get(&tx.to_address.clone()) {
                        #[allow(mutable_borrow_reservation_conflict)]
                        self.addresses.insert(tx.to_address.clone(), *address_amm + tx.ammount);
                    } else {
                        self.addresses.insert(tx.to_address.clone(),  tx.ammount);
                    }
                }
            }
        } else {
            if let Some(address_amm) = self.addresses.get(&tx.to_address.clone()) {
                #[allow(mutable_borrow_reservation_conflict)]
                self.addresses.insert(tx.to_address.clone(), *address_amm + tx.ammount);
            } else {
                self.addresses.insert(tx.to_address.clone(),  tx.ammount);
            } 
        }

        println!("\nFrom: {} has {}", tx.from_address, self.addresses.get(&tx.from_address.clone()).unwrap_or(&0));
        println!("To: {} has {}", tx.to_address, self.addresses.get(&tx.to_address.clone()).unwrap());
    }
}