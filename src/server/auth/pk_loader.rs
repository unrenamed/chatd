use std::collections::HashSet;

use crate::utils;

use super::PubKey;

#[derive(Debug, Clone)]
pub struct PublicKeyLoader {
    path: String,
}

impl PublicKeyLoader {
    pub fn new(path: &str) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> anyhow::Result<HashSet<PubKey>> {
        let lines = utils::fs::read_file_lines(&self.path)?;
        let keys: HashSet<PubKey> = lines
            .iter()
            .filter_map(|line| utils::ssh::split_ssh_key(line))
            .filter_map(|(_, key, _)| russh_keys::parse_public_key_base64(&key).ok())
            .map(|key| PubKey::new(key))
            .collect();
        Ok(keys)
    }
}
