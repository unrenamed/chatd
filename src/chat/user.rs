use std::{fmt::Display, time::Duration};

use chrono::{DateTime, Utc};

use crate::utils;

#[derive(Clone, Debug)]
pub enum UserStatus {
    Active,
    Away {
        reason: String,
        since: DateTime<Utc>,
    },
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Clone, Debug, Default)]
pub struct User {
    pub id: usize,
    pub username: String,
    pub status: UserStatus,
    pub joined_at: DateTime<Utc>,
    pub ssh_client: String,
    pub fingerprint: String,
    pub reply_to: Option<usize>,
}

impl User {
    pub fn new(id: usize, username: String, ssh_client: String, fingerprint: String) -> Self {
        Self {
            id,
            username,
            ssh_client,
            fingerprint,
            status: UserStatus::Active,
            joined_at: Utc::now(),
            reply_to: None,
        }
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

    pub fn set_new_name(&mut self, username: String) {
        self.username = username;
    }

    pub fn joined_duration(&self) -> Duration {
        let now = Utc::now();
        let secs = now.signed_duration_since(self.joined_at).num_seconds() as u64;
        Duration::from_secs(secs)
    }

    pub fn set_reply_to(&mut self, reply_to: usize) {
        self.reply_to = Some(reply_to);
    }
}

impl Display for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}{} > fingerprint: {}{} > client: {}{} > joined: {} ago",
            self.username,
            utils::NEWLINE,
            self.fingerprint,
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
