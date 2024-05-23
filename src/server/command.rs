use crate::utils;

use fmt::Write;
use std::fmt;
use strum::{EnumCount, EnumIter, EnumProperty, IntoEnumIterator};

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

    #[strum(props(Cmd = "/users", Help = "List users who are connected"))]
    Users,

    #[strum(props(Cmd = "/whois", Args = "<user>", Help = "Information about a user"))]
    Whois(String),

    #[strum(props(Cmd = "/slap", Args = "[user]", Help = "Show who is the boss here"))]
    Slap(Option<String>),

    #[strum(props(
        Cmd = "/shrug",
        Help = "Express either doubt or just deep indifference"
    ))]
    Shrug,

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
}

impl fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandParseError::NotRecognizedAsCommand => write!(f, "given input is not a command"),
            CommandParseError::UnknownCommand => write!(f, "unknown command"),
            CommandParseError::ArgumentExpected(arg) => write!(f, "{} is expected", arg),
        }
    }
}

impl Command {
    pub fn parse(bytes: &[u8]) -> Result<Command, CommandParseError> {
        let (cmd, args) = split_at_first_space(bytes);
        if !cmd.starts_with(&[0x2f]) {
            return Err(CommandParseError::NotRecognizedAsCommand);
        }

        let args = std::str::from_utf8(args).expect("Command arguments to be a valid UTF-8 string");
        let mut args_iter = args.split_whitespace().into_iter();

        match cmd {
            b"/exit" => Ok(Command::Exit),
            b"/away" => match args.is_empty() {
                true => Err(CommandParseError::ArgumentExpected(format!("away reason"))),
                false => Ok(Command::Away(args.to_string())),
            },
            b"/back" => Ok(Command::Back),
            b"/name" => match args_iter.nth(0) {
                Some(new_name) => Ok(Command::Name(new_name.to_string())),
                None => Err(CommandParseError::ArgumentExpected(format!("new name"))),
            },
            b"/msg" => {
                let user = args_iter.nth(0);
                if user.is_none() {
                    return Err(CommandParseError::ArgumentExpected(format!("new name")));
                }
                let body = args_iter.collect::<Vec<_>>().join(" ");
                if body.is_empty() {
                    return Err(CommandParseError::ArgumentExpected(format!("message body")));
                };
                Ok(Command::Msg(user.unwrap().to_string(), body))
            }
            b"/users" => Ok(Command::Users),
            b"/whois" => match args_iter.nth(0) {
                Some(user) => Ok(Command::Whois(user.to_string())),
                None => Err(CommandParseError::ArgumentExpected(format!("user name"))),
            },
            b"/slap" => match args_iter.nth(0) {
                Some(user) => Ok(Command::Slap(Some(user.to_string()))),
                None => Ok(Command::Slap(None)),
            },
            b"/shrug" => Ok(Command::Shrug),
            b"/me" => match args.is_empty() {
                true => Ok(Command::Me(None)),
                false => Ok(Command::Me(Some(args.to_string()))),
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
