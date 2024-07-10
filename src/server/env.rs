use std::str::FromStr;

const ENV_PREFIX: &str = "CHATD_";

#[derive(Debug)]
pub enum Env {
    Theme(String),
    Timestamp(String),
}

impl FromStr for Env {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s
            .split_once('=')
            .ok_or("Malformed environment variable format. Expected format: 'NAME=value'")?;

        if value.is_empty() {
            return Err("Environment variable value is empty");
        }

        let theme_var = format!("{}THEME", ENV_PREFIX);
        let timestamp_var = format!("{}TIMESTAMP", ENV_PREFIX);

        match name {
            _ if name == theme_var => Ok(Env::Theme(value.to_string())),
            _ if name == timestamp_var => Ok(Env::Timestamp(value.to_string())),
            _ => Err("Unknown environment variable type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_theme_env_var() {
        let env_var = "CHATD_THEME=colors";
        let env = Env::from_str(env_var).unwrap();
        if let Env::Theme(value) = env {
            assert_eq!(value, "colors");
        } else {
            panic!("Expected Env::Theme variant");
        }
    }

    #[test]
    fn test_valid_timestamp_env_var() {
        let env_var = "CHATD_TIMESTAMP=datetime";
        let env = Env::from_str(env_var).unwrap();
        if let Env::Timestamp(value) = env {
            assert_eq!(value, "datetime");
        } else {
            panic!("Expected Env::Timestamp variant");
        }
    }

    #[test]
    fn test_unknown_env_var_type() {
        let env_var = "CHATD_UNKNOWN=value";
        let err = Env::from_str(env_var).unwrap_err();
        assert_eq!(err, "Unknown environment variable type");
    }

    #[test]
    fn test_malformed_env_var() {
        let env_var = "CHATD_THEMEdark";
        let err = Env::from_str(env_var).unwrap_err();
        assert_eq!(
            err,
            "Malformed environment variable format. Expected format: 'NAME=value'"
        );
    }

    #[test]
    fn test_empty_value() {
        let env_var = "CHATD_THEME=";
        let err = Env::from_str(env_var).unwrap_err();
        assert_eq!(err, "Environment variable value is empty");
    }
}
