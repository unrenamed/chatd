use crate::utils;

use fmt::Write;
use std::{fmt, str::FromStr};
use strum::{EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

use super::{theme::Theme, user::TimestampMode};

#[derive(Debug, PartialEq, EnumProperty, EnumIter, EnumCount)]
pub enum Command {
    #[strum(props(Cmd = "/exit", Help = "Exit the chat application"))]
    Exit,

    #[strum(props(
        Cmd = "/away",
        Args = "<reason>",
        Help = "Let the room know you can't make it and why"
    ))]
    Away(String),

    #[strum(props(Cmd = "/back", Help = "Clear away status"))]
    Back,

    #[strum(props(Cmd = "/name", Args = "<name>", Help = "Rename yourself"))]
    Name(String),

    #[strum(props(
        Cmd = "/msg",
        Args = "<user> <message>",
        Help = "Send a private message to a user"
    ))]
    Msg(String, String),

    #[strum(props(
        Cmd = "/reply",
        Args = "<message>",
        Help = "Reply to the previous private message"
    ))]
    Reply(String),

    #[strum(props(Cmd = "/ignore", Args = "[user]", Help = "Hide messages from a user"))]
    Ignore(Option<String>),

    #[strum(props(
        Cmd = "/unignore",
        Args = "<user>",
        Help = "Stop hidding messages from a user"
    ))]
    Unignore(String),

    #[strum(props(
        Cmd = "/focus",
        Args = "[user]",
        Help = "Only show messages from focused users. $ to reset"
    ))]
    Focus(Option<String>),

    #[strum(props(Cmd = "/users", Help = "List users who are connected"))]
    Users,

    #[strum(props(Cmd = "/whois", Args = "<user>", Help = "Information about a user"))]
    Whois(String),

    #[strum(props(
        Cmd = "/timestamp",
        Args = "<time|datetime>",
        Help = "Prefix messages with a UTC timestamp"
    ))]
    Timestamp(TimestampMode),

    #[strum(props(Cmd = "/theme", Args = "<theme>", Help = "Set your color theme"))]
    Theme(Theme),

    #[strum(props(Cmd = "/themes", Help = "List supported color themes"))]
    Themes,

    #[strum(props(Cmd = "/slap", Args = "[user]", Help = "Show who is the boss here"))]
    Slap(Option<String>),

    #[strum(props(
        Cmd = "/shrug",
        Help = "Express either doubt or just deep indifference"
    ))]
    Shrug,

    #[strum(props(Cmd = "/quiet", Help = "Silence room announcements"))]
    Quiet,

    #[strum(props(
        Cmd = "/me",
        Args = "[action]",
        Help = "Show an action or visible emotion of yours. E.g. '/me looks upset'"
    ))]
    Me(Option<String>),

    #[strum(props(Cmd = "/help", Help = "Show this help message"))]
    Help,
}

#[derive(Debug, PartialEq)]
pub enum CommandParseError {
    NotRecognizedAsCommand,
    UnknownCommand,
    ArgumentExpected(String),
    Custom(String),
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandParseError::NotRecognizedAsCommand => write!(f, "given input is not a command"),
            CommandParseError::UnknownCommand => write!(f, "unknown command"),
            CommandParseError::ArgumentExpected(arg) => write!(f, "{} is expected", arg),
            CommandParseError::Custom(s) => write!(f, "{}", s),
        }
    }
}

impl Command {
    pub fn parse(bytes: &[u8]) -> Result<Command, CommandParseError> {
        let (cmd, args) = split_at_first_space(bytes);
        if !cmd.starts_with(b"/") {
            return Err(CommandParseError::NotRecognizedAsCommand);
        }

        let args = std::str::from_utf8(args)
            .expect("Command arguments to be a valid UTF-8 string")
            .trim_start();

        match cmd {
            b"/exit" => Ok(Command::Exit),
            b"/away" => match args.is_empty() {
                true => Err(CommandParseError::ArgumentExpected(format!("away reason"))),
                false => Ok(Command::Away(args.to_string())),
            },
            b"/back" => Ok(Command::Back),
            b"/name" => match args.splitn(2, ' ').nth(0) {
                Some(new_name) => Ok(Command::Name(new_name.to_string())),
                None => Err(CommandParseError::ArgumentExpected(format!("new name"))),
            },
            b"/msg" => {
                let mut iter = args.splitn(2, ' ');
                let user = iter.next();
                if user.is_none() || user.unwrap().is_empty() {
                    return Err(CommandParseError::ArgumentExpected(format!("user name")));
                }
                let body = iter.next();
                if body.is_none() || body.unwrap().is_empty() {
                    return Err(CommandParseError::ArgumentExpected(format!("message body")));
                };
                Ok(Command::Msg(
                    user.unwrap().to_string(),
                    body.unwrap().trim_start().to_string(),
                ))
            }
            b"/reply" => {
                if args.is_empty() {
                    return Err(CommandParseError::ArgumentExpected(format!("message body")));
                };
                Ok(Command::Reply(args.to_string()))
            }
            b"/users" => Ok(Command::Users),
            b"/whois" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(CommandParseError::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Whois(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/slap" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Slap(None)),
                Some(user) => Ok(Command::Slap(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/shrug" => Ok(Command::Shrug),
            b"/quiet" => Ok(Command::Quiet),
            b"/me" => match args.is_empty() {
                true => Ok(Command::Me(None)),
                false => Ok(Command::Me(Some(args.to_string()))),
            },
            b"/timestamp" => match args.splitn(2, ' ').nth(0) {
                Some(mode) if mode.is_empty() => Err(CommandParseError::Custom(
                    "timestamp value must be one of: time, datetime, off".to_string(),
                )),
                Some(mode) => match mode {
                    "time" | "datetime" | "off" => {
                        Ok(Command::Timestamp(TimestampMode::from_str(mode).unwrap()))
                    }
                    _ => Err(CommandParseError::Custom(
                        "timestamp value must be one of: time, datetime, off".to_string(),
                    )),
                },
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/theme" => match args.splitn(2, ' ').nth(0) {
                Some(theme) if theme.is_empty() => Err(CommandParseError::Custom(format!(
                    "theme value must be one of: {}",
                    Theme::all().join(", ")
                ))),
                Some(theme) => {
                    let supported_themes = Theme::all();
                    if supported_themes.contains(&theme.to_string()) {
                        Ok(Command::Theme(Theme::from_str(theme).unwrap()))
                    } else {
                        Err(CommandParseError::Custom(format!(
                            "theme value must be one of: {}",
                            Theme::all().join(", ")
                        )))
                    }
                }
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/themes" => Ok(Command::Themes),
            b"/ignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Ignore(None)),
                Some(user) => Ok(Command::Ignore(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/unignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(CommandParseError::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Unignore(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/focus" => match args.splitn(2, ' ').nth(0) {
                Some(users) if users.is_empty() => Ok(Command::Focus(None)),
                Some(users) => Ok(Command::Focus(Some(users.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/help" => Ok(Command::Help),
            _ => Err(CommandParseError::UnknownCommand),
        }
    }

    pub fn to_string() -> String {
        let mut result = String::new();
        for (idx, cmd) in Command::iter().enumerate() {
            write!(
                result,
                "{:<10} {:<20} {}{}",
                cmd.get_str("Cmd").unwrap_or_default(),
                cmd.get_str("Args").unwrap_or_default(),
                cmd.get_str("Help").unwrap_or_default(),
                if idx == Command::COUNT - 1 {
                    ""
                } else {
                    utils::NEWLINE
                }
            )
            .unwrap();
        }
        result
    }
}

fn split_at_first_space(bytes: &[u8]) -> (&[u8], &[u8]) {
    // Find the position of the first space
    if let Some(pos) = bytes.iter().position(|&b| b == b' ') {
        // Split the slice at the position of the first space
        let (first, rest) = bytes.split_at(pos);
        // Skip the space in the rest slice
        (first, &rest[1..])
    } else {
        // If there's no space, return the original slice
        (bytes, &[])
    }
}
