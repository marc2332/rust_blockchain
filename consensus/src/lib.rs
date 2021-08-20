use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};

use blockchain::{
    Blockchain,
    Transaction,
};

#[derive(Debug)]
pub enum ConsensusErrors {
    TransactionBroken,
}

/*
 * Algorithm to randomly take a block creator(block forger) from people who have staked a small ammount on previous blocks
 */
pub fn elect_forger(blockchain: &Blockchain) -> Result<Vec<Transaction>, ConsensusErrors> {
    let mut stakers = Vec::<Transaction>::new();
    for (i, block) in blockchain.iter().enumerate() {
        if i + 100 >= blockchain.chain.len() {
            let txs: Vec<Transaction> = serde_json::from_str(&block.payload).unwrap();

            for transaction in txs {
                let tx_verification_is_ok = transaction.verify();

                if tx_verification_is_ok {
                    if transaction.to_address == "stake" && stakers.len() < 100 {
                        stakers.push(transaction);
                    }
                } else {
                    println!("Blockchain is broken.");
                    return Err(ConsensusErrors::TransactionBroken);
                }
            }
        }
    }
    Ok(stakers)
}
