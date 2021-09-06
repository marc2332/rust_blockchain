use std::collections::HashMap;

use blockchain::{
    Blockchain,
    Key,
    Transaction,
};

#[derive(Debug)]
pub enum ConsensusErrors {
    TransactionBroken,
}

/*
 * Algorithm to randomly take a block creator(block forger) from people who have staked a small ammount on previous blocks
 */
pub fn elect_forger(blockchain: &Blockchain) -> Result<Key, ConsensusErrors> {
    let stakings = &blockchain.state.last_staking_addresses;

    let last_block = blockchain.chain.last().unwrap();
    let previous_forgers = {
        let mut forgers = HashMap::new();

        for block in blockchain.chain.iter().rev() {
            forgers.insert(block.key.hash_it(), ());

            if forgers.len() == stakings.len() - 2 {
                break;
            }
        }

        forgers
    };

    let mut len = last_block.hash.hash.len();
    let mut forger = None;

    while len > 0 {
        for tx in stakings.iter() {
            if let Transaction::STAKE {
                author_public_key,
                hash,
                from_address,
                ..
            } = tx
            {
                if previous_forgers.get(from_address).is_none()
                    && hash.contains(&last_block.hash.hash[0..len])
                {
                    forger = Some(author_public_key);
                    break;
                }
            }
        }
        len -= 1;
        if forger.is_some() {
            break;
        }
    }

    Ok(forger.unwrap().clone())
}
