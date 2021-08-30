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
    let previous_forger = {
        if let Some(previous_hash) = last_block.previous_hash.as_ref() {
            let previous_block = blockchain
                .get_block_with_hash(previous_hash.unite())
                .unwrap();
            Some(previous_block.key.hash_it())
        } else {
            None
        }
    };

    let mut len = last_block.hash.hash.len();
    let mut forger = None;

    while len > 0 {
        for tx in stakings {
            if let Transaction::STAKE {
                author_public_key,
                hash,
                from_address,
                ..
            } = tx
            {
                if let Some(ref previous_forger) = previous_forger {
                    if from_address != previous_forger
                        && hash.contains(&last_block.hash.hash[0..len])
                    {
                        forger = Some(author_public_key);
                        break;
                    }
                } else if hash.contains(&last_block.hash.hash[0..len]) {
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
