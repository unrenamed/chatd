use russh_keys::key::PublicKey;
use std::collections::HashSet;
use std::time::Duration;

use crate::utils::TimedHashSet;

use super::pk::PubKey;

#[derive(Clone)]
pub struct Auth {
    operators: Option<HashSet<PubKey>>,
    trusted_keys: Option<HashSet<PubKey>>,
    is_whitelist_enabled: bool,
    banned_usernames: TimedHashSet<String>,
    banned_fingerprints: TimedHashSet<String>,
}

impl Auth {
    pub fn new(operators: Option<HashSet<PubKey>>, trusted_keys: Option<HashSet<PubKey>>) -> Self {
        Self {
            operators,
            trusted_keys: trusted_keys.clone(),
            is_whitelist_enabled: trusted_keys.is_some_and(|set| !set.is_empty()),
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

    pub fn trusted_keys(&self) -> &Option<HashSet<PubKey>> {
        &self.trusted_keys
    }

    pub fn add_trusted_key(&mut self, key: PublicKey) {
        if let Some(keys) = &mut self.trusted_keys {
            keys.insert(PubKey::new(key));
        }
    }

    pub fn remove_trusted_key(&mut self, key: PublicKey) {
        if let Some(keys) = &mut self.trusted_keys {
            keys.remove(&PubKey::new(key));
        }
    }

    pub fn is_op(&self, key: &PublicKey) -> bool {
        matches!(&self.operators, Some(list) if list.iter().find(|k| *k == key).is_some())
    }

    pub fn is_trusted(&self, key: &PublicKey) -> bool {
        matches!(&self.trusted_keys, Some(list) if list.iter().find(|k| *k == key).is_some())
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
