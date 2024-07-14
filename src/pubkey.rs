use std::fmt::{Debug, Display};
use std::hash::Hash;

use russh_keys::key::{KeyPair, PublicKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubKey(PublicKey);

impl PubKey {
    pub fn fingerprint(&self) -> String {
        self.0.fingerprint()
    }

    #[cfg(test)]
    pub fn long(&self) -> String {
        use russh_keys::PublicKeyBase64;
        let pk = self.0.public_key_base64();
        format!("{} {}", self.0.name(), pk)
    }
}

impl Into<PublicKey> for PubKey {
    fn into(self) -> PublicKey {
        self.0
    }
}

impl From<PublicKey> for PubKey {
    fn from(value: PublicKey) -> Self {
        Self(value)
    }
}

impl From<&PublicKey> for PubKey {
    fn from(value: &PublicKey) -> Self {
        Self(value.clone())
    }
}

impl Display for PubKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Hash for PubKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.fingerprint().hash(state);
    }
}

impl PartialEq<PublicKey> for PubKey {
    fn eq(&self, other: &PublicKey) -> bool {
        self.0.eq(other)
    }
}

impl Default for PubKey {
    fn default() -> Self {
        let key_pair = KeyPair::generate_ed25519().unwrap();
        let key = key_pair
            .clone_public_key()
            .expect("Public key of ed25519 algorithm");
        Self(key.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use russh_keys::key::{KeyPair, PublicKey};

    fn generate_test_key() -> PublicKey {
        let key_pair = KeyPair::generate_ed25519().unwrap();
        key_pair
            .clone_public_key()
            .expect("Public key of ed25519 algorithm")
    }

    #[test]
    fn test_pubkey_from_publickey() {
        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();
        assert_eq!(pubkey, PubKey(public_key));
    }

    #[test]
    fn test_pubkey_into_publickey() {
        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();
        let public_key_converted: PublicKey = pubkey.into();
        assert_eq!(public_key, public_key_converted);
    }

    #[test]
    fn test_pubkey_fingerprint() {
        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();
        assert_eq!(pubkey.fingerprint(), public_key.fingerprint());
    }

    #[test]
    fn test_pubkey_display() {
        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();
        assert_eq!(format!("{}", pubkey), format!("{:?}", public_key));
    }

    #[test]
    fn test_pubkey_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();

        let mut hasher1 = DefaultHasher::new();
        pubkey.hash(&mut hasher1);
        let pubkey_hash = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        public_key.fingerprint().hash(&mut hasher2);
        let public_key_hash = hasher2.finish();

        assert_eq!(pubkey_hash, public_key_hash);
    }

    #[test]
    fn test_pubkey_partial_eq_publickey() {
        let public_key = generate_test_key();
        let pubkey: PubKey = public_key.clone().into();
        assert_eq!(pubkey, public_key);
    }
}
