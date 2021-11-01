use std::sync::{
    Arc,
    Mutex,
};

use blockchain::{
    BlockBuilder,
    Blockchain,
    Configuration,
    Metrics,
    PublicAddress,
    Wallet,
};
use chrono::Utc;
use tokio_test::block_on;

#[test]

fn test() {
    let mut blockchain = block_on(Blockchain::new(
        Configuration::new(),
        Arc::new(Mutex::new(Metrics::new(vec![]))),
    ));

    assert!(blockchain.verify_integrity().is_ok());

    let account_a = Wallet::new();
    let public_key = account_a.get_public();

    if blockchain.last_block_hash.is_none() {
        blockchain
            .add_block(
                &BlockBuilder::new()
                    .transactions(&vec![])
                    .timestamp(Utc::now())
                    .key(&public_key)
                    .hash_it()
                    .sign_with(&account_a)
                    .build(),
            )
            .unwrap();
    }

    for _ in 1..5 {
        blockchain
            .add_block(
                &BlockBuilder::new()
                    .transactions(&vec![])
                    .timestamp(Utc::now())
                    .previous_hash(&blockchain.last_block_hash.clone().unwrap())
                    .key(&public_key)
                    .hash_it()
                    .sign_with(&account_a)
                    .build(),
            )
            .unwrap();
    }

    let block_3 = BlockBuilder::new()
        .transactions(&vec![])
        .timestamp(Utc::now())
        .key(&public_key)
        .hash_it()
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
