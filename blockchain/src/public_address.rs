use k256::ecdsa::{
    VerifyingKey,
    signature::{
        Signature,
        Verifier,
    }
};

use crate::{
    Key,
    SignVerifier,
};

pub struct PublicAddress {
    pub public_key: VerifyingKey,
}

impl PublicAddress {
    pub fn from_signature(signature: &[u8], data: &[u8]) -> Self {
        let signature: k256::ecdsa::recoverable::Signature =
            Signature::from_bytes(signature).unwrap();
        let public_key = signature.recover_verify_key(data).unwrap();
        Self { public_key }
    }

    pub fn get_public(&self) -> Key {
        Key(self.public_key.to_bytes().to_vec())
    }
}

impl SignVerifier for PublicAddress {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let signature: k256::ecdsa::recoverable::Signature =
            Signature::from_bytes(&signature.0).unwrap();
        let result: Result<(), k256::ecdsa::Error> =
            self.public_key.verify(data.as_bytes(), &signature);
        result.is_ok()
    }
}

impl From<&Key> for PublicAddress {
    fn from(key: &Key) -> Self {
        PublicAddress {
            public_key: VerifyingKey::from_sec1_bytes(&key.0).unwrap(),
        }
    }
}
