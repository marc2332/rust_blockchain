mod block_hash;
mod block_builder;
mod block;
mod blockchain;
mod key;
mod wallet;
mod public_address;
mod configuration;

pub use block_hash::BlockHash;
pub use block_builder::BlockBuilder;
pub use block::Block;
pub use blockchain::{
    Blockchain,
    BlockchainErrors
};
pub use key::Key;
pub use wallet::Wallet;
pub use public_address::PublicAddress;
pub use configuration::Configuration;

pub trait SignVerifier {
    fn verify_signature(&self, signature: &Key, data: String) -> bool;
}

