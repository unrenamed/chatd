use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io;

use crate::utils;

use super::PubKey;

#[derive(Debug)]
pub enum LoadError {
    IoError(io::Error),
    NoKeysError,
}

impl From<io::Error> for LoadError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::IoError(err) => write!(f, "I/O error: {}", err),
            LoadError::NoKeysError => write!(f, "file has no keys"),
        }
    }
}

#[derive(Debug)]
pub enum SaveError {
    IoError(io::Error),
    EncodeError(russh_keys::Error),
    NoKeysError,
}

impl From<io::Error> for SaveError {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<russh_keys::Error> for SaveError {
    fn from(value: russh_keys::Error) -> Self {
        Self::EncodeError(value)
    }
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::IoError(err) => write!(f, "I/O error: {}", err),
            SaveError::EncodeError(err) => write!(f, "failed to encode key to base64: {}", err),
            SaveError::NoKeysError => write!(f, "file has no keys"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PubKeyFileManager {
    file_path: String,
}

impl PubKeyFileManager {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }

    pub fn load_keys(&self) -> Result<HashSet<PubKey>, LoadError> {
        let lines = utils::fs::read_file_lines(&self.file_path)?;

        let keys: HashSet<PubKey> = lines
            .iter()
            .filter_map(|line| utils::ssh::split_ssh_key(line))
            .filter_map(|(_, key)| russh_keys::parse_public_key_base64(&key).ok())
            .map(|key| key.into())
            .collect();

        if keys.is_empty() {
            Err(LoadError::NoKeysError)
        } else {
            Ok(keys)
        }
    }

    pub fn save_keys(&self, keys: &HashSet<PubKey>) -> Result<(), SaveError> {
        if keys.is_empty() {
            return Err(SaveError::NoKeysError);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.file_path)?;

        for key in keys {
            let key = key.clone();
            russh_keys::write_public_key_base64(&mut file, &key.into())?;
        }

        Ok(())
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
    fn test_load_keys_success() {
        let file_path = "test_keys_load_success.txt";
        let pubkey = create_test_pubkey();
        let (_dir, full_path) = setup_test_file(&file_path, &pubkey.long());

        let manager = PubKeyFileManager::new(&full_path);
        let keys = manager.load_keys().unwrap();
        assert!(keys.contains(&pubkey));
    }

    #[test]
    fn test_load_keys_no_keys_error() {
        let file_path = "test_keys_no_keys.txt";
        let (_dir, full_path) = setup_test_file(&file_path, "");

        let manager = PubKeyFileManager::new(&full_path);
        let result = manager.load_keys();
        assert!(matches!(result, Err(LoadError::NoKeysError)));
    }

    #[test]
    fn test_load_keys_io_error() {
        let file_path = "non_existent_file.txt";
        let manager = PubKeyFileManager::new(file_path);
        let result = manager.load_keys();
        assert!(matches!(result, Err(LoadError::IoError(_))));
    }

    #[test]
    fn test_save_keys_success() {
        let file_path = "test_keys_save_success.txt";
        let (_dir, full_path) = setup_empty_test_file(file_path);

        let pubkey = create_test_pubkey();
        let keys = HashSet::from([pubkey.clone()]);

        let manager = PubKeyFileManager::new(&full_path);
        manager.save_keys(&keys).unwrap();

        let saved_content = fs::read_to_string(full_path).unwrap();
        assert!(saved_content.contains(&pubkey.long()));
    }

    #[test]
    fn test_save_keys_content_truncate() {
        let file_path = "test_keys_save_content_truncate.txt";
        let old_key = create_test_pubkey();
        let (_dir, full_path) = setup_test_file(file_path, &old_key.long());

        let new_key = create_test_pubkey();
        let keys = HashSet::from([new_key.clone()]);

        let manager = PubKeyFileManager::new(&full_path);
        manager.save_keys(&keys).unwrap();

        let saved_content = fs::read_to_string(full_path).unwrap();
        assert!(!saved_content.contains(&old_key.long()));
        assert!(saved_content.contains(&new_key.long()));
    }

    #[test]
    fn test_save_keys_no_keys_error() {
        let file_path = "test_keys_save_no_keys.txt";
        let (_dir, full_path) = setup_test_file(file_path, "");
        let keys: HashSet<PubKey> = HashSet::new();

        let manager = PubKeyFileManager::new(&full_path);
        let result = manager.save_keys(&keys);

        assert!(matches!(result, Err(SaveError::NoKeysError)));
    }

    #[test]
    fn test_save_keys_io_error() {
        let file_path = "test_keys_save_io_error.txt"; // Assuming file does not exist

        let pubkey = create_test_pubkey();
        let keys = HashSet::from([pubkey]);

        let manager = PubKeyFileManager::new(file_path);
        let result = manager.save_keys(&keys);

        assert!(matches!(result, Err(SaveError::IoError(_))));
    }

    #[test]
    fn test_save_keys_encode_error() {
        // russh_keys::write_public_key_base64 can only fail with IO
        // error Since the IO error is already handled when
        // opening the file, we cannot mock
        // russh_keys::write_public_key_base64 to fail.
    }

    #[test]
    fn test_save_error_display_io_error() {
        let io_error = io::Error::from(io::ErrorKind::NotFound);
        let save_error = SaveError::IoError(io_error);
        assert_eq!(format!("{}", save_error), "I/O error: entity not found");
    }

    #[test]
    fn test_save_error_display_encode_error() {
        let encode_error = russh_keys::Error::KeyIsCorrupt;
        let save_error = SaveError::EncodeError(encode_error);
        assert_eq!(
            format!("{}", save_error),
            "failed to encode key to base64: The key is corrupt"
        );
    }

    #[test]
    fn test_save_error_display_no_keys_error() {
        let save_error = SaveError::NoKeysError;
        assert_eq!(format!("{}", save_error), "file has no keys");
    }

    #[test]
    fn test_load_error_display_io_error() {
        let io_error = io::Error::from(io::ErrorKind::NotFound);
        let load_error = LoadError::IoError(io_error);
        assert_eq!(format!("{}", load_error), "I/O error: entity not found");
    }

    #[test]
    fn test_load_error_display_no_keys_error() {
        let load_error = LoadError::NoKeysError;
        assert_eq!(format!("{}", load_error), "file has no keys");
    }
}
