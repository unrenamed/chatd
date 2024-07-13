use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq)]
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
