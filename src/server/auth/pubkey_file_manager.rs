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

#[derive(Debug, Clone)]
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
            .filter_map(|(_, key, _)| russh_keys::parse_public_key_base64(&key).ok())
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
