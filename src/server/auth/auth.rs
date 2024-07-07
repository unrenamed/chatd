use russh_keys::key::PublicKey;
use std::collections::HashSet;
use std::time::Duration;

use crate::utils::TimedHashSet;

use super::pubkey::PubKey;
use super::{pubkey_file_manager, PubKeyFileManager};

#[derive(Debug)]
pub enum AuthError {
    NoWhitelist,
    NoOplist,
    LoadKeysError(pubkey_file_manager::LoadError),
    SaveKeysError(pubkey_file_manager::SaveError),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::NoWhitelist => write!(f, "no whitelist file in the server configuration"),
            AuthError::NoOplist => write!(f, "no oplist file in the server configuration"),
            AuthError::LoadKeysError(err) => write!(f, "{}", err),
            AuthError::SaveKeysError(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Clone, Default)]
pub struct Auth {
    is_whitelist_enabled: bool,
    oplist_file_manager: Option<PubKeyFileManager>,
    whitelist_file_manager: Option<PubKeyFileManager>,
    operators: HashSet<PubKey>,
    trusted_keys: HashSet<PubKey>,
    banned_usernames: TimedHashSet<String>,
    banned_fingerprints: TimedHashSet<String>,
}

impl Auth {
    pub fn set_oplist(&mut self, oplist_file_manager: PubKeyFileManager) {
        self.oplist_file_manager = Some(oplist_file_manager);
    }

    pub fn set_whitelist(&mut self, whitelist_file_manager: PubKeyFileManager) {
        self.whitelist_file_manager = Some(whitelist_file_manager);
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
        self.trusted_keys.insert(key.into());
    }

    pub fn remove_trusted_key(&mut self, key: PublicKey) {
        self.trusted_keys.remove(&key.into());
    }

    pub fn operators(&self) -> &HashSet<PubKey> {
        &self.operators
    }

    pub fn clear_operators(&mut self) {
        self.operators.clear();
    }

    pub fn add_operator(&mut self, key: PublicKey) {
        self.operators.insert(key.into());
    }

    pub fn remove_operator(&mut self, key: PublicKey) {
        self.operators.remove(&key.into());
    }

    pub fn load_trusted_keys(&mut self) -> Result<(), AuthError> {
        if let Some(loader) = &self.whitelist_file_manager {
            return loader
                .load_keys()
                .map(|keys| {
                    self.trusted_keys.extend(keys);
                })
                .map_err(AuthError::LoadKeysError);
        }
        Err(AuthError::NoWhitelist)
    }

    pub fn load_operators(&mut self) -> Result<(), AuthError> {
        if let Some(loader) = &self.oplist_file_manager {
            return loader
                .load_keys()
                .map(|keys| {
                    self.operators.extend(keys);
                })
                .map_err(AuthError::LoadKeysError);
        }
        Err(AuthError::NoOplist)
    }

    pub fn save_trusted_keys(&mut self) -> Result<(), AuthError> {
        if let Some(loader) = &self.whitelist_file_manager {
            return loader
                .save_keys(&self.trusted_keys)
                .map_err(AuthError::SaveKeysError);
        }
        Err(AuthError::NoWhitelist)
    }

    pub fn save_operators(&mut self) -> Result<(), AuthError> {
        if let Some(loader) = &self.oplist_file_manager {
            return loader
                .save_keys(&self.operators)
                .map_err(AuthError::SaveKeysError);
        }
        Err(AuthError::NoOplist)
    }

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
