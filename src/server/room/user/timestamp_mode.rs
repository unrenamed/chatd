use strum::EnumString;

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
