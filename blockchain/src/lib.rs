mod block;
mod block_builder;
mod block_chain;
mod block_hash;
mod configuration;
mod key;
mod public_address;
mod transaction;
mod wallet;

pub use block::Block;
pub use block_builder::BlockBuilder;
pub use block_chain::{
    Blockchain,
    BlockchainErrors,
};
pub use block_hash::BlockHash;
pub use configuration::Configuration;
pub use key::Key;
pub use public_address::PublicAddress;
pub use transaction::Transaction;
pub use wallet::Wallet;

pub trait SignVerifier {
    fn verify_signature(&self, signature: &Key, data: String) -> bool;
}
