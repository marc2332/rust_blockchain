use crate::{
    Key,
    SignVerifier,
};
use k256::{
    ecdsa::{
        recoverable::Signature as RecoverableSignature,
        signature::{
            Signature,
            Signer,
            Verifier,
        },
        Signature as NormalSignature,
        SigningKey,
        VerifyingKey,
    },
    SecretKey,
};
use rand_core::OsRng;

/// A Wallet that holds a private key and it's `history`
#[derive(Clone)]
pub struct Wallet {
    /// The sign key
    pub sign_key: SigningKey,
    /// The current history of a wallet
    pub history: u64,
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = Key(self.sign_key.to_bytes().to_vec());
        f.debug_struct("Wallet")
            .field("public key", &key.hash_it())
            .field("history", &self.history)
            .finish()
    }
}

impl SignVerifier for Wallet {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let verify_key = VerifyingKey::from(&self.sign_key);
        let signature: NormalSignature = NormalSignature::from_bytes(&signature.0).unwrap();
        verify_key.verify(data.as_bytes(), &signature).is_ok()
    }
}

impl Wallet {
    /// Creates a random private key
    pub fn new() -> Self {
        let sign_key = SigningKey::random(&mut OsRng);

        Self {
            sign_key,
            history: 0,
        }
    }

    /// Returns a signature from the given data using the private key
    ///
    /// # Example
    ///
    /// ```
    /// let wallet = Wallet::new();
    /// // The signature now certifies that the wallet signed `Hello World`
    /// let signature = wallet.sign_data("Hello World".to_string());
    /// ```
    ///
    pub fn sign_data(&self, data: String) -> Key {
        let data = data.as_bytes();
        let signature: RecoverableSignature = self.sign_key.sign(data);
        Key(signature.as_bytes().to_vec())
    }

    /// Returns the public key of the wallet
    ///
    /// # Example
    ///
    /// ```
    /// let wallet = Wallet::new();
    /// let public_key = wallet.get_public();
    /// // It can be hashed, like an address
    /// let address = public_key.hash_it();
    /// ```
    ///
    pub fn get_public(&self) -> Key {
        let public_key = VerifyingKey::from(&self.sign_key).to_bytes().to_vec();
        Key(public_key)
    }

    /// Returns the private key of the wallet
    ///
    /// # Example
    ///
    /// ```
    /// let wallet = Wallet::new();
    /// // You can save it and then use `from_private`
    /// let private_key = wallet.get_private();
    /// ```
    ///
    pub fn get_private(&self) -> Key {
        let public_key = SecretKey::from(&self.sign_key).to_bytes().to_vec();
        Key(public_key)
    }

    /// Returns a wallet from the given Private Key and history
    ///
    /// # Example
    ///
    /// ```
    /// let wallet = Wallet::from_private(private_key, history);
    /// ```
    ///
    pub fn from_private(private_key: &[u8], history: u64) -> Self {
        let sign_key = SigningKey::from_bytes(private_key).unwrap();

        Self { sign_key, history }
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}
