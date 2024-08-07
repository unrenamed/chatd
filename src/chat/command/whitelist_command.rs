use std::fmt;
use std::str::FromStr;
use strum::{EnumCount, EnumIter, EnumProperty, EnumString, IntoEnumIterator};

use super::command_props::CommandProps;
use super::parse_error::CommandParseError;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum WhitelistLoadMode {
    Merge,
    Replace,
}

impl Default for WhitelistLoadMode {
    fn default() -> Self {
        Self::Merge
    }
}

impl WhitelistLoadMode {
    pub fn from_prefix(prefix: &str) -> Option<WhitelistLoadMode> {
        WhitelistLoadMode::iter().find(|mode| mode.to_string().starts_with(prefix))
    }

    pub fn values() -> Vec<String> {
        WhitelistLoadMode::iter().map(|m| m.to_string()).collect()
    }
}

impl fmt::Display for WhitelistLoadMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WhitelistLoadMode::Merge => "merge",
                WhitelistLoadMode::Replace => "replace",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, EnumProperty, EnumIter, EnumCount)]
pub enum WhitelistCommand {
    #[strum(props(
        Cmd = "on",
        Help = "Enable whitelist mode (applies to new connections only)"
    ))]
    On,

    #[strum(props(
        Cmd = "off",
        Help = "Disable whitelist mode (applies to new connections only)"
    ))]
    Off,

    #[strum(props(
        Cmd = "add",
        Args = "<user | key>...",
        Help = "Add users or keys to the trusted keys"
    ))]
    Add(String),

    #[strum(props(
        Cmd = "remove",
        Args = "<user | key>...",
        Help = "Remove users or keys from the trusted keys"
    ))]
    Remove(String),

    #[strum(props(
        Cmd = "load",
        Args = "merge | replace",
        Help = "Load public keys from whitelist file and merge it with or replace the in-memory data"
    ))]
    Load(WhitelistLoadMode),

    #[strum(props(
        Cmd = "save",
        Help = "Export public keys to the whitelist file, overwriting the existing file content"
    ))]
    Save,

    #[strum(props(Cmd = "reverify", Help = "Kick all users not in the whitelist"))]
    Reverify,

    #[strum(props(Cmd = "status", Help = "Show status information"))]
    Status,

    #[strum(props(Cmd = "help"))]
    Help,
}

impl FromStr for WhitelistCommand {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Self::Err::ArgumentExpected(format!("whitelist command")));
        };

        let (cmd, args) = if let Some((cmd, args)) = s.split_once(' ') {
            (cmd, args.trim())
        } else {
            (s, "")
        };
        match cmd.as_bytes() {
            b"on" => Ok(Self::On),
            b"off" => Ok(Self::Off),
            b"save" => Ok(Self::Save),
            b"reverify" => Ok(Self::Reverify),
            b"status" => Ok(Self::Status),
            b"help" => Ok(Self::Help),
            b"add" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!(
                    "list of users or keys"
                ))),
                false => Ok(Self::Add(args.to_string())),
            },
            b"remove" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!(
                    "list of users or keys"
                ))),
                false => Ok(Self::Remove(args.to_string())),
            },
            b"load" => match args.parse::<WhitelistLoadMode>() {
                Ok(mode) => Ok(Self::Load(mode)),
                Err(_) => Err(Self::Err::Other(format!(
                    "load mode value must be one of: {}",
                    WhitelistLoadMode::values().join(", ")
                ))),
            },
            _ => Err(Self::Err::UnknownCommand),
        }
    }
}

impl Default for WhitelistCommand {
    fn default() -> Self {
        Self::Help
    }
}

impl CommandProps for WhitelistCommand {
    fn cmd(&self) -> &str {
        self.get_str("Cmd").unwrap_or_default()
    }

    fn args(&self) -> &str {
        self.get_str("Args").unwrap_or_default()
    }

    fn help(&self) -> &str {
        self.get_str("Help").unwrap_or_default()
    }

    fn is_op(&self) -> bool {
        self.get_str("Op").unwrap_or_default() == "true"
    }
}

#[cfg(test)]
mod whitelist_load_mode_should {
    use super::*;

    #[test]
    fn recognize_merge() {
        assert_eq!(
            WhitelistLoadMode::from_prefix("mer"),
            Some(WhitelistLoadMode::Merge)
        );
    }

    #[test]
    fn recognize_replace() {
        assert_eq!(
            WhitelistLoadMode::from_prefix("rep"),
            Some(WhitelistLoadMode::Replace)
        );
    }

    #[test]
    fn return_none_for_unknown_prefix() {
        assert_eq!(WhitelistLoadMode::from_prefix("unknown"), None);
    }

    #[test]
    fn return_list_of_modes() {
        assert_eq!(
            WhitelistLoadMode::values(),
            vec!["merge".to_string(), "replace".to_string()]
        );
    }

    #[test]
    fn display_modes() {
        assert_eq!(WhitelistLoadMode::Merge.to_string(), "merge");
        assert_eq!(WhitelistLoadMode::Replace.to_string(), "replace");
    }
}

#[cfg(test)]
mod whitelist_command_should {
    use super::*;

    #[test]
    fn parse_on_off_commands() {
        assert_eq!("on".parse::<WhitelistCommand>(), Ok(WhitelistCommand::On));
        assert_eq!("off".parse::<WhitelistCommand>(), Ok(WhitelistCommand::Off));
    }

    #[test]
    fn parse_add_command() {
        let command = "add user1 user2";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Add("user1 user2".to_string()))
        );
    }

    #[test]
    fn parse_remove_command() {
        let command = "remove user1 user2";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Remove("user1 user2".to_string()))
        );
    }

    #[test]
    fn parse_load_command_with_merge() {
        let command = "load merge";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Load(WhitelistLoadMode::Merge))
        );
    }

    #[test]
    fn parse_load_command_with_replace() {
        let command = "load replace";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Load(WhitelistLoadMode::Replace))
        );
    }

    #[test]
    fn parse_save_command() {
        let command = "save";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Save)
        );
    }

    #[test]
    fn parse_status_command() {
        let command = "status";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Status)
        );
    }

    #[test]
    fn parse_help_command() {
        let command = "help";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Help)
        );
    }

    #[test]
    fn parse_reverify_command() {
        let command = "reverify";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Ok(WhitelistCommand::Reverify)
        );
    }

    #[test]
    fn fail_for_unknown_command() {
        let command = "unknown";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Err(CommandParseError::UnknownCommand)
        );
    }

    #[test]
    fn fail_for_add_command_without_args() {
        let command = "add";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Err(CommandParseError::ArgumentExpected(
                "list of users or keys".to_string()
            ))
        );
    }

    #[test]
    fn fail_for_remove_command_without_args() {
        let command = "remove";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Err(CommandParseError::ArgumentExpected(
                "list of users or keys".to_string()
            ))
        );
    }

    #[test]
    fn fail_for_load_command_with_invalid_mode() {
        let command = "load invalid";
        assert_eq!(
            command.parse::<WhitelistCommand>(),
            Err(CommandParseError::Other(
                "load mode value must be one of: merge, replace".to_string()
            ))
        );
    }
}
