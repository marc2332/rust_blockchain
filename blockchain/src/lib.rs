mod block;
mod block_builder;
mod block_chain;
mod block_hash;
mod chainstate;
mod configuration;
mod key;
mod metrics;
mod public_address;
mod transaction;
mod transaction_builder;
mod wallet;

pub use block::Block;
pub use block_builder::BlockBuilder;
pub use block_chain::{
    Blockchain,
    BlockchainErrors,
};
pub use block_hash::BlockHash;
pub use chainstate::Chainstate;
pub use configuration::Configuration;
pub use key::Key;
pub use metrics::{
    Metrics,
    MetricsClient,
};
pub use public_address::PublicAddress;
pub use transaction::Transaction;
pub use transaction_builder::{
    TransactionBuilder,
    TransactionType,
};
pub use wallet::Wallet;

pub trait SignVerifier {
    /// Makes sure the given data was correctly signed by the signature
    ///
    /// # Example
    ///
    /// ```
    /// let wallet = Wallet::new();
    /// // Some data
    /// let data = "Hello World".to_string();
    /// // The signature that certifies that the wallet signed `Hello World`
    /// let signature = wallet.sign_data(data.clone());
    /// // The verification of the signature
    /// let is_signature_ok = wallet.verify_signature(signature, data);
    /// ```
    ///
    fn verify_signature(&self, signature: &Key, data: String) -> bool;
}
