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
pub fn elect_forger(blockchain: &mut Blockchain) -> Result<Key, ConsensusErrors> {
    let stakings = blockchain.state.last_staking_addresses.clone();

    let last_block = blockchain.chain.last().unwrap();

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
                // Forger wasn't recently chose
                let forger_is_not_recent = !blockchain.state.has_recent_forger(from_address);
                // Forger is not punished because of missing it's slot
                let forger_is_not_punished = !blockchain.state.is_punished(from_address);
                // The forger is elected
                let forger_won = hash.contains(&last_block.hash.hash[0..len]);

                if forger_is_not_recent && forger_is_not_punished && forger_won {
                    blockchain.state.add_recent_forger(from_address);
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
