use chrono::{DateTime, Utc};
use russh_keys::key::PublicKey;
use std::collections::BTreeSet;
use std::fmt::Display;
use std::time::Duration;

use crate::utils;

use super::config::UserConfig;
use super::status::UserStatus;
use super::{UserName, UserTheme};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct User {
    pub id: usize,
    pub username: UserName,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
    pub ssh_client: String,
    pub public_key: Option<PublicKey>,
    pub reply_to: Option<usize>,
    pub config: UserConfig,
    pub is_op: bool,
    pub is_muted: bool,
    pub ignored: BTreeSet<usize>,
    pub focused: BTreeSet<usize>,
}

impl User {
    pub fn new(
        id: usize,
        username: UserName,
        ssh_client: String,
        public_key: Option<PublicKey>,
        is_op: bool,
    ) -> Self {
        let mut user = Self {
            id,
            ssh_client,
            is_op,
            public_key,
            joined_at: Utc::now(),
            ..Default::default()
        };
        user.set_username(username);
        user
    }

    pub fn switch_mute_mode(&mut self) {
        self.is_muted = !self.is_muted;
    }

    pub fn go_away(&mut self, reason: String) {
        self.status = UserStatus::Away {
            reason,
            since: Utc::now(),
        };
    }

    pub fn return_active(&mut self) {
        self.status = UserStatus::Active;
    }

    pub fn set_username(&mut self, username: UserName) {
        self.username = username;
        self.update_display_name();
        self.update_highlight();
    }

    pub fn set_theme(&mut self, theme: UserTheme) {
        self.config.set_theme(theme);
        self.update_display_name();
        self.update_highlight();
    }

    pub fn joined_duration(&self) -> Duration {
        let now = Utc::now();
        let secs = now.signed_duration_since(self.joined_at).num_seconds() as u64;
        Duration::from_secs(secs)
    }

    pub fn set_reply_to(&mut self, reply_to: usize) {
        self.reply_to = Some(reply_to);
    }

    fn update_display_name(&mut self) {
        self.config.set_display_name(&self.username);
    }

    fn update_highlight(&mut self) {
        self.config.set_highlight(&format!("@{}", self.username));
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fingerprint = match &self.public_key {
            Some(key) => format!("SHA256: {}", key.fingerprint()),
            None => "(no public key)".to_string(),
        };

        write!(
            f,
            "name: {}{} > fingerprint: {}{} > client: {}{} > joined: {} ago",
            self.username,
            utils::NEWLINE,
            fingerprint,
            utils::NEWLINE,
            self.ssh_client,
            utils::NEWLINE,
            humantime::format_duration(self.joined_duration()),
        )?;

        match &self.status {
            UserStatus::Active => Ok(()),
            UserStatus::Away { reason, since } => {
                let now = Utc::now();
                let secs = now.signed_duration_since(since).num_seconds() as u64;
                write!(
                    f,
                    "{} > away ({} ago) {}",
                    utils::NEWLINE,
                    humantime::format_duration(Duration::from_secs(secs)),
                    reason
                )
            }
        }
    }
}
