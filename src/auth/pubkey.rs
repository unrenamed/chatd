use std::fmt::{Debug, Display};
use std::hash::Hash;

use russh_keys::key::PublicKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubKey(PublicKey);

impl PubKey {
    pub fn fingerprint(&self) -> String {
        self.0.fingerprint()
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
