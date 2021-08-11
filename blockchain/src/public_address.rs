use openssl::{
    hash::MessageDigest,
    pkey::{
        PKey,
        Public,
    },
    rsa::Rsa,
    sign::Verifier,
};

use crate::{
    Key,
    SignVerifier,
};

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

impl From<&Key> for PublicAddress {
    fn from(bytes: &Key) -> Self {
        PublicAddress {
            keypair: PKey::from_rsa(Rsa::public_key_from_pem(bytes.0.as_slice()).unwrap()).unwrap(),
        }
    }
}
