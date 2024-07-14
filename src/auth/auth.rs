use std::collections::HashSet;
use std::time::Duration;

use crate::pubkey::PubKey;

use super::set::TimedHashSet;
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

    pub fn add_trusted_key(&mut self, key: PubKey) {
        self.trusted_keys.insert(key.into());
    }

    pub fn remove_trusted_key(&mut self, key: PubKey) {
        self.trusted_keys.remove(&key.into());
    }

    pub fn operators(&self) -> &HashSet<PubKey> {
        &self.operators
    }

    pub fn clear_operators(&mut self) {
        self.operators.clear();
    }

    pub fn add_operator(&mut self, key: PubKey) {
        self.operators.insert(key.into());
    }

    pub fn remove_operator(&mut self, key: PubKey) {
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

    pub fn is_op(&self, key: &PubKey) -> bool {
        matches!(&self.operators, list if list.iter().find(|k| *k == key).is_some())
    }

    pub fn is_trusted(&self, key: &PubKey) -> bool {
        matches!(&self.trusted_keys, list if list.iter().find(|k| *k == key).is_some())
    }

    pub fn check_bans(&mut self, user: &str, key: &PubKey) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::fixture::{FileWriteBin, PathChild};
    use assert_fs::TempDir;
    use std::fs::{self};

    fn create_test_pubkey() -> PubKey {
        let key_pair = russh_keys::key::KeyPair::generate_ed25519().unwrap();
        PubKey::from(key_pair.clone_public_key().unwrap())
    }

    fn setup_test_file(file_path: &str, content: &str) -> (TempDir, String) {
        let temp = TempDir::new().unwrap();
        temp.child(file_path)
            .write_binary(content.as_bytes())
            .unwrap();
        let path = format!("{}/{file_path}", temp.path().display());
        (temp, path)
    }

    fn setup_empty_test_file(file_path: &str) -> (TempDir, String) {
        setup_test_file(file_path, "")
    }

    #[test]
    fn test_set_oplist() {
        let file_path = "test_oplist.txt";
        let manager = PubKeyFileManager::new(file_path);
        let mut auth = Auth::default();

        auth.set_oplist(manager.clone());
        assert_eq!(auth.oplist_file_manager, Some(manager));
    }

    #[test]
    fn test_set_whitelist() {
        let file_path = "test_whitelist.txt";
        let manager = PubKeyFileManager::new(file_path);
        let mut auth = Auth::default();

        auth.set_whitelist(manager.clone());
        assert_eq!(auth.whitelist_file_manager, Some(manager));
    }

    #[test]
    fn test_enable_disable_whitelist_mode() {
        let mut auth = Auth::default();

        auth.enable_whitelist_mode();
        assert!(auth.is_whitelist_enabled());

        auth.disable_whitelist_mode();
        assert!(!auth.is_whitelist_enabled());
    }

    #[test]
    fn test_load_trusted_keys_success() {
        let file_path = "test_trusted_keys_load_success.txt";
        let pubkey = create_test_pubkey();
        let (_dir, full_path) = setup_test_file(file_path, &pubkey.long());

        let mut auth = Auth::default();
        let manager = PubKeyFileManager::new(&full_path);
        auth.set_whitelist(manager);

        auth.load_trusted_keys().unwrap();
        assert!(auth.is_trusted(&pubkey.into()));
    }

    #[test]
    fn test_load_trusted_keys_no_whitelist() {
        let mut auth = Auth::default();
        let result = auth.load_trusted_keys();

        assert!(matches!(result, Err(AuthError::NoWhitelist)));
    }

    #[test]
    fn test_load_operators_success() {
        let file_path = "test_operators_load_success.txt";
        let pubkey = create_test_pubkey();
        let (_dir, full_path) = setup_test_file(file_path, &pubkey.long());

        let mut auth = Auth::default();
        let manager = PubKeyFileManager::new(&full_path);
        auth.set_oplist(manager);

        auth.load_operators().unwrap();
        assert!(auth.is_op(&pubkey.into()));
    }

    #[test]
    fn test_load_operators_no_oplist() {
        let mut auth = Auth::default();
        let result = auth.load_operators();

        assert!(matches!(result, Err(AuthError::NoOplist)));
    }

    #[test]
    fn test_save_trusted_keys_success() {
        let file_path = "test_trusted_keys_save_success.txt";
        let (_dir, full_path) = setup_empty_test_file(file_path);

        let mut auth = Auth::default();
        let manager = PubKeyFileManager::new(&full_path);
        auth.set_whitelist(manager);

        let pubkey = create_test_pubkey();
        auth.add_trusted_key(pubkey.clone().into());
        auth.save_trusted_keys().unwrap();

        let saved_content = fs::read_to_string(&full_path).unwrap();
        assert!(saved_content.contains(&pubkey.long()));
    }

    #[test]
    fn test_save_trusted_keys_no_whitelist() {
        let mut auth = Auth::default();
        let result = auth.save_trusted_keys();

        assert!(matches!(result, Err(AuthError::NoWhitelist)));
    }

    #[test]
    fn test_save_operators_success() {
        let file_path = "test_operators_save_success.txt";
        let (_dir, full_path) = setup_empty_test_file(file_path);

        let mut auth = Auth::default();
        let manager = PubKeyFileManager::new(&full_path);
        auth.set_oplist(manager);

        let pubkey = create_test_pubkey();
        auth.operators.insert(pubkey.clone());
        auth.save_operators().unwrap();

        let saved_content = fs::read_to_string(&full_path).unwrap();
        assert!(saved_content.contains(&pubkey.long()));
    }

    #[test]
    fn test_save_operators_no_oplist() {
        let mut auth = Auth::default();
        let result = auth.save_operators();

        assert!(matches!(result, Err(AuthError::NoOplist)));
    }

    #[test]
    fn test_is_op() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_operator(pubkey.clone().into());

        assert!(auth.is_op(&pubkey.into()));
    }

    #[test]
    fn test_is_trusted() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_trusted_key(pubkey.clone().into());

        assert!(auth.is_trusted(&pubkey.into()));
    }

    #[test]
    fn test_check_bans() {
        let mut auth = Auth::default();
        let username = "alice";
        let pubkey = create_test_pubkey();
        let fingerprint = pubkey.fingerprint();

        auth.ban_username(username, Duration::from_secs(60));
        auth.ban_fingerprint(&fingerprint, Duration::from_secs(60));

        assert!(auth.check_bans(username, &pubkey.clone().into()));
        assert!(auth.check_bans("", &pubkey.into())); // Check if fingerprint banning works
    }

    #[test]
    fn test_banned() {
        let mut auth = Auth::default();
        let username = "alice";
        let pubkey = create_test_pubkey();
        let fingerprint = pubkey.fingerprint();

        auth.ban_username(username, Duration::from_secs(60));
        auth.ban_fingerprint(&fingerprint, Duration::from_secs(60));

        let (banned_users, banned_fingerprints) = auth.banned();

        assert_eq!(banned_users, vec![username.to_string()]);
        assert_eq!(banned_fingerprints, vec![fingerprint.to_string()]);
    }

    #[test]
    fn test_add_remove_operator() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();

        auth.add_operator(pubkey.clone().into());
        assert!(auth.is_op(&pubkey.clone().into()));

        auth.remove_operator(pubkey.clone().into());
        assert!(!auth.is_op(&pubkey.into()));
    }

    #[test]
    fn test_add_remove_trusted_keys() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();

        auth.add_trusted_key(pubkey.clone().into());
        assert!(auth.is_trusted(&pubkey.clone().into()));

        auth.remove_trusted_key(pubkey.clone().into());
        assert!(!auth.is_trusted(&pubkey.into()));
    }

    #[test]
    fn test_clear_operators() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_operator(pubkey.clone().into());
        auth.clear_operators();
        assert!(auth.operators.is_empty());
    }

    #[test]
    fn test_clear_trusted_keys() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_trusted_key(pubkey.clone().into());
        auth.clear_trusted_keys();
        assert!(auth.trusted_keys.is_empty());
    }

    #[test]
    fn test_get_operators() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_operator(pubkey.clone().into());
        assert_eq!(&auth.operators, auth.operators());
    }

    #[test]
    fn test_get_trusted_keys() {
        let mut auth = Auth::default();
        let pubkey = create_test_pubkey();
        auth.add_trusted_key(pubkey.clone().into());
        assert_eq!(&auth.trusted_keys, auth.trusted_keys());
    }

    #[test]
    fn test_display_no_whitelist() {
        let error = AuthError::NoWhitelist;
        assert_eq!(
            format!("{}", error),
            "no whitelist file in the server configuration"
        );
    }

    #[test]
    fn test_display_no_oplist() {
        let error = AuthError::NoOplist;
        assert_eq!(
            format!("{}", error),
            "no oplist file in the server configuration"
        );
    }

    #[test]
    fn test_display_load_keys_error() {
        let load_error = pubkey_file_manager::LoadError::IoError(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        ));
        let error = AuthError::LoadKeysError(load_error);
        assert_eq!(format!("{}", error), "I/O error: entity not found");
    }

    #[test]
    fn test_display_save_keys_error() {
        let save_error = pubkey_file_manager::SaveError::IoError(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        ));
        let error = AuthError::SaveKeysError(save_error);
        assert_eq!(format!("{}", error), "I/O error: permission denied");
    }
}
