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
    let stakings = &blockchain.state.last_staking_addresses;

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
                if !blockchain.state.has_forger(from_address)
                    && hash.contains(&last_block.hash.hash[0..len])
                {
                    if blockchain.state.last_forgers.len() > stakings.len() - 2 {
                        blockchain.state.last_forgers.remove(0);
                    }

                    blockchain.state.last_forgers.push(from_address.to_string());

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
