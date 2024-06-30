use std::str::FromStr;

const ENV_PREFIX: &str = "CHATD_";

pub enum Env {
    Theme(String),
    Timestamp(String),
}

impl FromStr for Env {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((name, value)) = s.split_once('=') {
            match name {
                name if name == ENV_PREFIX.to_owned() + "THEME" => {
                    Ok(Env::Theme(value.to_string()))
                }
                name if name == ENV_PREFIX.to_owned() + "TIMESTAMP" => {
                    Ok(Env::Timestamp(value.to_string()))
                }
                _ => Err("Unknown environment variable type"),
            }
        } else {
            Err("Malformed environment variable format. Expected format: 'NAME=value'")
        }
    }
}
