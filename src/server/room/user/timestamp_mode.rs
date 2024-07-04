use std::fmt::Display;

use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, EnumString)]
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

    pub fn from_prefix(prefix: &str) -> Option<TimestampMode> {
        TimestampMode::iter().find(|mode| mode.to_string().starts_with(prefix))
    }
}

impl Display for TimestampMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TimestampMode::Time => "time",
                TimestampMode::DateTime => "datetime",
                TimestampMode::Off => "off",
            }
        )
    }
}
