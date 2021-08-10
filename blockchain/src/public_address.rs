use openssl::{hash::MessageDigest, pkey::{PKey, Public}, sign::Verifier};

use crate::{Key, SignVerifier};

pub struct PublicAddress {
    pub keypair: PKey<Public>,
}

impl SignVerifier for PublicAddress {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(data.as_bytes()).unwrap();
        verifier.verify(&signature.0).unwrap()
    }
}