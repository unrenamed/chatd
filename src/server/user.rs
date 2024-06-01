use chrono::{DateTime, Utc};
use rand::seq::SliceRandom;
use rand::Rng;
use std::{collections::BTreeSet, fmt::Display, time::Duration};
use strum::EnumString;

use crate::utils;

use super::theme::UserTheme;

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

#[derive(Debug, Clone, PartialEq, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum TimestampMode {
    Time,
    DateTime,
    Off,
}

impl Default for TimestampMode {
    fn default() -> Self {
        Self::Off
    }
}

impl TimestampMode {
    pub fn format(&self) -> Option<&str> {
        match self {
            TimestampMode::Time => Some("%H:%M"),
            TimestampMode::DateTime => Some("%Y-%m-%d %H:%M:%S"),
            TimestampMode::Off => None,
        }
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
    pub theme: UserTheme,
    pub quiet: bool,
    pub timestamp_mode: TimestampMode,
    pub ignored: BTreeSet<usize>,
}

impl User {
    pub fn new(id: usize, username: String, ssh_client: String, fingerprint: String) -> Self {
        Self {
            id,
            username,
            ssh_client,
            fingerprint,
            joined_at: Utc::now(),
            reply_to: None,
            quiet: false,
            ..Default::default()
        }
    }

    pub fn switch_quiet_mode(&mut self) {
        self.quiet = !self.quiet;
    }

    pub fn set_timestamp_mode(&mut self, mode: TimestampMode) {
        self.timestamp_mode = mode;
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

    pub fn gen_rand_name() -> String {
        let adjectives = [
            "Cool", "Mighty", "Brave", "Clever", "Happy", "Calm", "Eager", "Gentle", "Kind",
            "Jolly", "Swift", "Bold", "Fierce", "Wise", "Valiant", "Bright", "Noble", "Zany",
            "Epic",
        ];
        let nouns = [
            "Tiger", "Eagle", "Panda", "Shark", "Lion", "Wolf", "Dragon", "Phoenix", "Hawk",
            "Bear", "Falcon", "Panther", "Griffin", "Lynx", "Orca", "Cobra", "Jaguar", "Kraken",
            "Pegasus", "Stallion",
        ];

        let mut rng = rand::thread_rng();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let noun = nouns.choose(&mut rng).unwrap();
        let number: u16 = rng.gen_range(1..=9999);

        format!("{}{}{}", adjective, noun, number)
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
