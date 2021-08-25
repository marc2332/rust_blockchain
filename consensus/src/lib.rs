use blockchain::{
    Blockchain,
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
pub fn elect_forger(blockchain: &Blockchain) -> Result<String, ConsensusErrors> {
    let mut stakings = Vec::<Transaction>::new();
    for (i, block) in blockchain.iter().enumerate() {
        if i + 100 >= blockchain.chain.len() {
            let txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

            for tx in txs {
                if let Transaction::STAKE { .. } = tx {
                    let tx_verification_is_ok = tx.verify();

                    if tx_verification_is_ok {
                        if stakings.len() < 100 {
                            stakings.push(tx);
                        }
                    } else {
                        println!("Blockchain is broken.");
                        return Err(ConsensusErrors::TransactionBroken);
                    }
                }
            }
        }
    }

    let txs_hash = {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        for tx in &stakings {
            if let Transaction::STAKE { signature, .. } = tx {
                hasher.input_str(signature.hash_it().as_str());
            }
        }
        hasher.result_str()
    };

    let mut len = txs_hash.len();
    let mut forger = None;

    while len > 0 {
        for tx in &stakings {
            if let Transaction::STAKE {
                from_address, hash, ..
            } = tx
            {
                if hash.contains(&txs_hash[0..len]) {
                    forger = Some(from_address);
                    break;
                }
            }
            len -= 1;
        }
        if forger.is_some() {
            break;
        }
    }

    Ok(forger.unwrap().to_string())
}
