use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

use russh_keys::key::PublicKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubKey(PublicKey);

impl PubKey {
    pub fn new(key: PublicKey) -> Self {
        Self(key)
    }

    pub fn fingerprint(&self) -> String {
        self.0.fingerprint()
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
