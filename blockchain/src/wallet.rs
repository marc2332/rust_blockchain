use openssl::{hash::MessageDigest, pkey::{PKey, Private}, rsa::Rsa, sign::{Signer, Verifier}};

use crate::{Key, SignVerifier};


pub struct Wallet {
    pub keypair: PKey<Private>,
}

impl SignVerifier for Wallet {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(data.as_bytes()).unwrap();
        verifier.verify(&signature.0).unwrap()
    }
}

impl Wallet {
    pub fn new() -> Self {
        let keypair = Rsa::generate(1024).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        Self { keypair }
    }

    pub fn sign_data(&self, data: String) -> Key {
        let data = data.as_bytes();

        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        signer.update(data).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        Key(signature)
    }

    pub fn get_public(&self) -> Key {
        let public_key = self.keypair.public_key_to_pem().unwrap();
        Key(public_key)
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}