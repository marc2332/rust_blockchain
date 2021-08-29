use blockchain::{
    Blockchain,
    Key,
    Transaction,
};
use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
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

    let txs_hash = {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        for tx in stakings {
            if let Transaction::STAKE { signature, .. } = tx {
                hasher.input_str(signature.hash_it().as_str());
            }
        }
        hasher.result_str()
    };

    let mut len = txs_hash.len();
    let mut forger = None;

    while len > 0 {
        for tx in stakings {
            if let Transaction::STAKE {
                author_public_key,
                hash,
                ..
            } = tx
            {
                if hash.contains(&txs_hash[0..len]) {
                    forger = Some(author_public_key);
                    break;
                }
            }
            len -= 1;
        }
        if forger.is_some() {
            break;
        }
    }

    Ok(forger.unwrap().clone())
}
