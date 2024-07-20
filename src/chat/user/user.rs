use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use std::fmt::Display;
use std::time::Duration;

use crate::pubkey::PubKey;
use crate::utils;

use super::config::UserConfig;
use super::status::UserStatus;
use super::{UserName, UserTheme};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct User {
    id: usize,
    username: UserName,
    config: UserConfig,
    status: UserStatus,

    public_key: PubKey,

    reply_to: Option<usize>,
    is_muted: bool,

    ignored: BTreeSet<usize>,
    focused: BTreeSet<usize>,

    joined_at: DateTime<Utc>,
    ssh_client: String,
}

impl User {
    pub fn new(id: usize, username: UserName, ssh_client: String, public_key: PubKey) -> Self {
        let mut user = Self {
            id,
            ssh_client,
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

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn username(&self) -> &UserName {
        &self.username
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn config(&self) -> &UserConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut UserConfig {
        &mut self.config
    }

    pub fn status(&self) -> &UserStatus {
        &self.status
    }

    pub fn reply_to(&self) -> &Option<usize> {
        &self.reply_to
    }

    pub fn public_key(&self) -> &PubKey {
        &self.public_key
    }

    pub fn ignored(&self) -> &BTreeSet<usize> {
        &self.ignored
    }

    pub fn focused(&self) -> &BTreeSet<usize> {
        &self.focused
    }

    pub fn unignore(&mut self, id: &usize) {
        self.ignored.remove(id);
    }

    pub fn unfocus(&mut self, id: &usize) {
        self.focused.remove(id);
    }

    pub fn ignore(&mut self, id: usize) {
        self.ignored.insert(id);
    }

    pub fn focus(&mut self, id: usize) {
        self.focused.insert(id);
    }

    pub fn unfocus_all(&mut self) {
        self.focused.clear();
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
        write!(
            f,
            "name: {}{} > fingerprint: {}{} > client: {}{} > joined: {} ago",
            self.username,
            utils::NEWLINE,
            format!("SHA256: {}", self.public_key.fingerprint()),
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
