use blockchain::{
    Transaction,
    Wallet,
};

use client::RPCClient;
use criterion::{
    criterion_group,
    criterion_main,
    Criterion,
};
use crypto::{
    digest::Digest,
    sha3::{
        Sha3,
        Sha3Mode,
    },
};

#[tokio::main]
async fn execute_100_transactions() {
    let client = RPCClient::new("http://localhost:3030").await.unwrap();

    let wallet_a = Wallet::new();
    let wallet_b = Wallet::new();

    let public_key_a = wallet_a.get_public();
    let public_key_b = wallet_b.get_public();

    let data = format!(
        "{}{}{}{}",
        public_key_a,
        public_key_a.hash_it(),
        public_key_b.hash_it(),
        5
    );

    let sig = wallet_a.sign_data(data);

    let hash = {
        let mut hasher = Sha3::new(Sha3Mode::Keccak256);
        hasher.input_str(&public_key_a.to_string());
        hasher.input_str(&sig.hash_it());
        hasher.input_str(&public_key_a.hash_it());
        hasher.input_str(&public_key_b.hash_it());
        hasher.input_str(&5.to_string());
        hasher.result_str()
    };

    let sample_tx = Transaction {
        author_public_key: public_key_a.clone(),
        signature: sig,
        from_address: public_key_a.hash_it(),
        to_address: public_key_b.hash_it(),
        ammount: 5,
        hash,
    };

    client.add_transaction(sample_tx.clone()).await.unwrap();
}

fn benchs(c: &mut Criterion) {
    c.bench_function("execute_100_transactions", |b| {
        b.iter(|| execute_100_transactions());
    });
}

criterion_group!(benches, benchs);
criterion_main!(benches);
