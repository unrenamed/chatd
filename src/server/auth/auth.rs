use russh_keys::key::PublicKey;
use std::collections::HashSet;
use std::fmt::Display;
use std::time::Duration;

use crate::utils::TimedHashSet;

use super::pk::PubKey;
use super::PublicKeyLoader;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthKeyLoadError {
    NoWhitelist,
    NoOplist,
    IOParse(String),
}

impl Display for AuthKeyLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthKeyLoadError::NoWhitelist => write!(f, "server has no whitelist file to load from"),
            AuthKeyLoadError::NoOplist => write!(f, "server has no oplist file to load from"),
            AuthKeyLoadError::IOParse(err) => write!(f, "I/O or parse error. {}", err),
        }
    }
}

impl std::error::Error for AuthKeyLoadError {}

#[derive(Clone)]
pub struct Auth {
    is_whitelist_enabled: bool,
    oplist_loader: Option<PublicKeyLoader>,
    whitelist_loader: Option<PublicKeyLoader>,
    operators: HashSet<PubKey>,
    trusted_keys: HashSet<PubKey>,
    banned_usernames: TimedHashSet<String>,
    banned_fingerprints: TimedHashSet<String>,
}

impl Auth {
    pub fn new(
        oplist_loader: Option<PublicKeyLoader>,
        whitelist_loader: Option<PublicKeyLoader>,
    ) -> Self {
        Self {
            is_whitelist_enabled: whitelist_loader.is_some(),
            oplist_loader,
            whitelist_loader,
            operators: HashSet::new(),
            trusted_keys: HashSet::new(),
            banned_fingerprints: TimedHashSet::new(),
            banned_usernames: TimedHashSet::new(),
        }
    }

    pub fn enable_whitelist_mode(&mut self) {
        self.is_whitelist_enabled = true;
    }

    pub fn disable_whitelist_mode(&mut self) {
        self.is_whitelist_enabled = false;
    }

    pub fn is_whitelist_enabled(&self) -> bool {
        self.is_whitelist_enabled
    }

    pub fn trusted_keys(&self) -> &HashSet<PubKey> {
        &self.trusted_keys
    }

    pub fn clear_trusted_keys(&mut self) {
        self.trusted_keys.clear();
    }

    pub fn add_trusted_key(&mut self, key: PublicKey) {
        self.trusted_keys.insert(PubKey::new(key));
    }

    pub fn remove_trusted_key(&mut self, key: PublicKey) {
        self.trusted_keys.remove(&PubKey::new(key));
    }

    pub fn operators(&self) -> &HashSet<PubKey> {
        &self.operators
    }

    pub fn clear_operators(&mut self) {
        self.operators.clear();
    }

    pub fn add_operator(&mut self, key: PublicKey) {
        self.operators.insert(PubKey::new(key));
    }

    pub fn remove_operator(&mut self, key: PublicKey) {
        self.operators.remove(&PubKey::new(key));
    }

    pub fn load_trusted_keys(&mut self) -> Result<(), AuthKeyLoadError> {
        if let Some(loader) = &self.whitelist_loader {
            return loader
                .load()
                .map(|keys| {
                    self.trusted_keys.extend(keys);
                })
                .map_err(|err| AuthKeyLoadError::IOParse(err.to_string()));
        }
        Err(AuthKeyLoadError::NoWhitelist)
    }

    pub fn load_operators(&mut self) -> Result<(), AuthKeyLoadError> {
        if let Some(loader) = &self.oplist_loader {
            return loader
                .load()
                .map(|keys| {
                    self.operators.extend(keys);
                })
                .map_err(|err| AuthKeyLoadError::IOParse(err.to_string()));
        }
        Err(AuthKeyLoadError::NoOplist)
    }

    pub fn save_trusted_keys(&mut self) {}

    pub fn save_operators(&mut self) {}

    pub fn is_op(&self, key: &PublicKey) -> bool {
        matches!(&self.operators, list if list.iter().find(|k| *k == key).is_some())
    }

    pub fn is_trusted(&self, key: &PublicKey) -> bool {
        matches!(&self.trusted_keys, list if list.iter().find(|k| *k == key).is_some())
    }

    pub fn check_bans(&mut self, user: &str, key: &PublicKey) -> bool {
        let mut is_banned = false;

        if !is_banned {
            is_banned = self.banned_usernames.contains(&user.to_string());
        }

        if !is_banned {
            is_banned = self.banned_fingerprints.contains(&key.fingerprint());
        }

        is_banned
    }

    pub fn ban_username(&mut self, username: &str, duration: Duration) {
        self.banned_usernames.insert(username.to_string(), duration)
    }

    pub fn ban_fingerprint(&mut self, fingerprint: &str, duration: Duration) {
        self.banned_fingerprints
            .insert(fingerprint.to_string(), duration)
    }

    pub fn banned(&self) -> (Vec<String>, Vec<String>) {
        let names = self
            .banned_usernames
            .iter()
            .map(|n| n.into())
            .collect::<Vec<String>>();

        let fingerprints = self
            .banned_fingerprints
            .iter()
            .map(|n| n.into())
            .collect::<Vec<String>>();

        (names, fingerprints)
    }
}
