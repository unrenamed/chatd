use russh_keys::key::PublicKey;
use std::time::Duration;

use super::set::TimedHashSet;

#[derive(Clone)]
pub struct Auth {
    operators: Option<Vec<PublicKey>>,
    trusted_keys: Option<Vec<PublicKey>>,
    banned_usernames: TimedHashSet<String>,
    banned_fingerprints: TimedHashSet<String>,
}

impl Auth {
    pub fn new(operators: Option<Vec<PublicKey>>, trusted_keys: Option<Vec<PublicKey>>) -> Self {
        Self {
            operators,
            trusted_keys,
            banned_fingerprints: TimedHashSet::new(),
            banned_usernames: TimedHashSet::new(),
        }
    }

    pub fn has_operators(&self) -> bool {
        self.operators.is_some()
    }

    pub fn is_op(&self, key: &PublicKey) -> bool {
        match &self.operators {
            Some(list) => list.iter().find(|k| (*k).eq(key)).is_some(),
            None => false,
        }
    }

    pub fn is_trusted(&self, key: &PublicKey) -> bool {
        match &self.trusted_keys {
            Some(list) => list.iter().find(|k| (*k).eq(key)).is_some(),
            None => false,
        }
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
