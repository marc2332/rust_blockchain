use openssl::{
    hash::MessageDigest,
    pkey::{
        PKey,
        Private,
    },
    rsa::Rsa,
    sign::{
        Signer,
        Verifier,
    },
};

use crate::{
    Key,
    SignVerifier,
};

/// A Wallet that holds a pair of keys and it's `history`
#[derive(Clone)]
pub struct Wallet {
    /// The private and public key
    pub keypair: PKey<Private>,
    /// The current history of a wallet
    pub history: u64,
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = Key(self.keypair.public_key_to_pem().unwrap());
        f.debug_struct("Wallet")
            .field("keypair", &key.hash_it())
            .field("history", &self.history)
            .finish()
    }
}

impl SignVerifier for Wallet {
    fn verify_signature(&self, signature: &Key, data: String) -> bool {
        let mut verifier = Verifier::new(MessageDigest::sha256(), &self.keypair).unwrap();
        verifier.update(data.as_bytes()).unwrap();
        verifier.verify(&signature.0).unwrap()
    }
}

impl Wallet {
    /// Creates a wallet with a random pair of keys
    pub fn new() -> Self {
        let keypair = Rsa::generate(1024).unwrap();
        let keypair = PKey::from_rsa(keypair).unwrap();

        Self {
            keypair,
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

        let mut signer = Signer::new(MessageDigest::sha256(), &self.keypair).unwrap();
        signer.update(data).unwrap();
        let signature = signer.sign_to_vec().unwrap();

        Key(signature)
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
        let public_key = self.keypair.public_key_to_pem().unwrap();
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
        let public_key = self.keypair.private_key_to_pem_pkcs8().unwrap();
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
        let keypair = PKey::private_key_from_pem(private_key).unwrap();

        Self { keypair, history }
    }
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}
