use std::fmt;
use std::str::FromStr;
use strum::{EnumCount, EnumIter, EnumProperty, EnumString, IntoEnumIterator};

use super::{command_props::CommandProps, parse_error::CommandParseError};

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum OplistLoadMode {
    Merge,
    Replace,
}

impl Default for OplistLoadMode {
    fn default() -> Self {
        Self::Merge
    }
}

impl OplistLoadMode {
    pub fn from_prefix(prefix: &str) -> Option<OplistLoadMode> {
        OplistLoadMode::iter().find(|mode| mode.to_string().starts_with(prefix))
    }

    pub fn values() -> Vec<String> {
        OplistLoadMode::iter().map(|m| m.to_string()).collect()
    }
}

impl fmt::Display for OplistLoadMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OplistLoadMode::Merge => "merge",
                OplistLoadMode::Replace => "replace",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, EnumProperty, EnumIter, EnumCount)]
pub enum OplistCommand {
    #[strum(props(
        Cmd = "add",
        Args = "<user | key>...",
        Help = "Add users or keys to the operators list"
    ))]
    Add(String),

    #[strum(props(
        Cmd = "remove",
        Args = "<user | key>...",
        Help = "Remove users or keys from the operators list"
    ))]
    Remove(String),

    #[strum(props(
        Cmd = "load",
        Args = "merge | replace",
        Help = "Load public keys from oplist file and merge it with or replace the in-memory data"
    ))]
    Load(OplistLoadMode),

    #[strum(props(
        Cmd = "save",
        Help = "Export public keys to the oplist file, overwriting the existing file content"
    ))]
    Save,

    #[strum(props(Cmd = "status", Help = "Show status information"))]
    Status,

    #[strum(props(Cmd = "help"))]
    Help,
}

impl FromStr for OplistCommand {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Self::Err::ArgumentExpected(format!("oplist command")));
        };

        let (cmd, args) = if let Some((cmd, args)) = s.split_once(' ') {
            (cmd, args.trim_start())
        } else {
            (s, "")
        };
        match cmd.as_bytes() {
            b"save" => Ok(Self::Save),
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
            b"load" => match args.parse::<OplistLoadMode>() {
                Ok(mode) => Ok(Self::Load(mode)),
                Err(_) => Err(Self::Err::Other(format!(
                    "load mode value must be one of: {}",
                    OplistLoadMode::values().join(", ")
                ))),
            },
            _ => Err(Self::Err::UnknownCommand),
        }
    }
}

impl Default for OplistCommand {
    fn default() -> Self {
        Self::Help
    }
}

impl CommandProps for OplistCommand {
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
