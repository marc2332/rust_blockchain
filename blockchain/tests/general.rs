use std::sync::{
    Arc,
    Mutex,
};

use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    PublicAddress,
    Wallet,
};
use chrono::Utc;

#[test]
fn test() {
    let config = Arc::new(Mutex::new(Configuration::new()));

    let mut blockchain = Blockchain::new("mars", config);

    assert!(blockchain.verify_integrity().is_ok());

    let account_a = Wallet::new();
    let public_key = account_a.get_public();

    if blockchain.last_block_hash.is_none() {
        blockchain.add_block(
            &BlockBuilder::new()
                .payload("block 1")
                .timestamp(Utc::now())
                .key(&public_key)
                .sign_with(&account_a)
                .build(),
        );
    }

    for i in 1..5 {
        blockchain.add_block(
            &BlockBuilder::new()
                .payload(&format!("Block {:?}", i))
                .timestamp(Utc::now())
                .previous_hash(&blockchain.last_block_hash.clone().unwrap())
                .key(&public_key)
                .sign_with(&account_a)
                .build(),
        );
    }

    let block_3 = BlockBuilder::new()
        .payload("Block 1")
        .timestamp(Utc::now())
        .key(&public_key)
        .sign_with(&account_a)
        .build();

    // Verifying the signing on the block should fail since this account hasn't signed it
    let account_b = Wallet::new();

    assert!(block_3.verify_sign_with(&account_a));
    assert!(!block_3.verify_sign_with(&account_b));

    assert!(blockchain.verify_integrity().is_ok());

    let public_account_a = PublicAddress::from(&public_key);

    assert!(block_3.verify_sign_with(&public_account_a));
}
