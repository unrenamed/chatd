use std::str::FromStr;
use strum::{EnumCount, EnumIter, EnumProperty};

use crate::chat::user::{Theme, TimestampMode};

use super::command_props::CommandProps;
use super::parse_error::CommandParseError;
use super::whitelist_command::WhitelistCommand;
use super::OplistCommand;

#[derive(Debug, Clone, PartialEq, EnumProperty, EnumIter, EnumCount)]
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
        Args = "<time|datetime|off>",
        Help = "Prefix messages with a UTC timestamp"
    ))]
    Timestamp(TimestampMode),

    #[strum(props(Cmd = "/theme", Args = "<theme>", Help = "Set your color theme"))]
    Theme(Theme),

    #[strum(props(Cmd = "/themes", Help = "List supported color themes"))]
    Themes,

    #[strum(props(Cmd = "/quiet", Help = "Silence room announcements"))]
    Quiet,

    /// Operator commands

    #[strum(props(
        Cmd = "/mute",
        Args = "<user>",
        Help = "Toggle muting user, preventing messages from broadcasting",
        Op = "true"
    ))]
    Mute(String),

    #[strum(props(
        Cmd = "/kick",
        Args = "<user>",
        Help = "Kick user from the server",
        Op = "true"
    ))]
    Kick(String),

    #[strum(props(
        Cmd = "/ban",
        Args = "<query>",
        Help = "Ban user from the server",
        Op = "true"
    ))]
    Ban(String),

    #[strum(props(Cmd = "/banned", Help = "List the current ban conditions", Op = "true"))]
    Banned,

    #[strum(props(
        Cmd = "/motd",
        Args = "[message]",
        Help = "Set a new message of the day, or print the motd if no message",
        Op = "true"
    ))]
    Motd(Option<String>),

    #[strum(props(
        Cmd = "/whitelist",
        Args = "<command> [args...]",
        Help = "Modify the whitelist or whitelist state. See /whitelist help for subcommands",
        Op = "true"
    ))]
    Whitelist(WhitelistCommand),

    #[strum(props(
        Cmd = "/oplist",
        Args = "<command> [args...]",
        Help = "Modify the oplist or oplist state. See /oplist help for subcommands",
        Op = "true"
    ))]
    Oplist(OplistCommand),

    /// Secret commands (just hidden or easter eggs)

    #[strum(props(Cmd = "/me", Args = "[action]"))]
    Me(Option<String>),

    #[strum(props(Cmd = "/slap", Args = "[user]"))]
    Slap(Option<String>),

    #[strum(props(Cmd = "/shrug",))]
    Shrug,

    #[strum(props(Cmd = "/help"))]
    Help,

    #[strum(props(Cmd = "/version"))]
    Version,

    #[strum(props(Cmd = "/uptime"))]
    Uptime,
}

impl FromStr for Command {
    type Err = CommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (cmd, args) = if let Some((cmd, args)) = s.split_once(' ') {
            (cmd, args.trim())
        } else {
            (s, "")
        };

        if !cmd.starts_with("/") {
            return Err(Self::Err::NotRecognizedAsCommand);
        }

        match cmd.as_bytes() {
            b"/exit" => Ok(Command::Exit),
            b"/help" => Ok(Command::Help),
            b"/version" => Ok(Command::Version),
            b"/uptime" => Ok(Command::Uptime),
            b"/back" => Ok(Command::Back),
            b"/users" => Ok(Command::Users),
            b"/shrug" => Ok(Command::Shrug),
            b"/quiet" => Ok(Command::Quiet),
            b"/themes" => Ok(Command::Themes),
            b"/banned" => Ok(Command::Banned),
            b"/away" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!("away reason"))),
                false => Ok(Command::Away(args.to_string())),
            },
            b"/name" => match args.splitn(2, ' ').nth(0) {
                Some(new_name) if !new_name.is_empty() => Ok(Command::Name(new_name.to_string())),
                _ => Err(Self::Err::ArgumentExpected(format!("new name"))),
            },
            b"/ban" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!("ban query"))),
                false => Ok(Command::Ban(args.to_string())),
            },
            b"/whitelist" => match args.parse::<WhitelistCommand>() {
                Ok(sub_cmd) => Ok(Command::Whitelist(sub_cmd)),
                Err(err) => Err(err),
            },
            b"/oplist" => match args.parse::<OplistCommand>() {
                Ok(sub_cmd) => Ok(Command::Oplist(sub_cmd)),
                Err(err) => Err(err),
            },
            b"/motd" => Ok(match args.is_empty() {
                true => Command::Motd(None),
                false => Command::Motd(Some(args.to_string())),
            }),
            b"/me" => match args.is_empty() {
                true => Ok(Command::Me(None)),
                false => Ok(Command::Me(Some(args.to_string()))),
            },
            b"/reply" => match args.is_empty() {
                true => Err(Self::Err::ArgumentExpected(format!("message body"))),
                false => Ok(Command::Reply(args.to_string())),
            },
            b"/whois" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Whois(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/slap" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Slap(None)),
                Some(user) => Ok(Command::Slap(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/timestamp" => match args.splitn(2, ' ').nth(0) {
                Some(mode) => match mode.parse::<TimestampMode>() {
                    Ok(parsed_mode) => Ok(Command::Timestamp(parsed_mode)),
                    Err(_) => Err(Self::Err::Other(format!(
                        "timestamp mode value must be one of: {}",
                        TimestampMode::values().join(", ")
                    ))),
                },
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/theme" => match args.splitn(2, ' ').nth(0) {
                Some(theme) => match theme.parse::<Theme>() {
                    Ok(parsed_theme) => Ok(Command::Theme(parsed_theme)),
                    Err(_) => Err(Self::Err::Other(format!(
                        "theme value must be one of: {}",
                        Theme::values().join(", ")
                    ))),
                },
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/ignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => Ok(Command::Ignore(None)),
                Some(user) => Ok(Command::Ignore(Some(user.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/unignore" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Unignore(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/focus" => match args.splitn(2, ' ').nth(0) {
                Some(users) if users.is_empty() => Ok(Command::Focus(None)),
                Some(users) => Ok(Command::Focus(Some(users.to_string()))),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/mute" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Mute(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/kick" => match args.splitn(2, ' ').nth(0) {
                Some(user) if user.is_empty() => {
                    Err(Self::Err::ArgumentExpected(format!("user name")))
                }
                Some(user) => Ok(Command::Kick(user.to_string())),
                None => unreachable!(), // splitn returns [""] for an empty input
            },
            b"/msg" => {
                let mut iter = args.splitn(2, ' ');
                let user = match iter.next() {
                    Some(user) if !user.is_empty() => user.to_string(),
                    _ => return Err(Self::Err::ArgumentExpected(format!("user name"))),
                };
                let body = match iter.next() {
                    Some(body) if !body.is_empty() => body.trim_start().to_string(),
                    _ => return Err(Self::Err::ArgumentExpected(format!("message body"))),
                };
                Ok(Command::Msg(
                    user.to_string(),
                    body.trim_start().to_string(),
                ))
            }
            _ => Err(Self::Err::UnknownCommand),
        }
    }
}

impl CommandProps for Command {
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
mod should {
    use super::*;

    #[test]
    fn parse_exit_command() {
        assert_eq!("/exit".parse::<Command>().unwrap(), Command::Exit);
    }

    #[test]
    fn parse_help_command() {
        assert_eq!("/help".parse::<Command>().unwrap(), Command::Help);
    }

    #[test]
    fn parse_version_command() {
        assert_eq!("/version".parse::<Command>().unwrap(), Command::Version);
    }

    #[test]
    fn parse_uptime_command() {
        assert_eq!("/uptime".parse::<Command>().unwrap(), Command::Uptime);
    }

    #[test]
    fn parse_back_command() {
        assert_eq!("/back".parse::<Command>().unwrap(), Command::Back);
    }

    #[test]
    fn parse_users_command() {
        assert_eq!("/users".parse::<Command>().unwrap(), Command::Users);
    }

    #[test]
    fn parse_shrug_command() {
        assert_eq!("/shrug".parse::<Command>().unwrap(), Command::Shrug);
    }

    #[test]
    fn parse_quiet_command() {
        assert_eq!("/quiet".parse::<Command>().unwrap(), Command::Quiet);
    }

    #[test]
    fn parse_themes_command() {
        assert_eq!("/themes".parse::<Command>().unwrap(), Command::Themes);
    }

    #[test]
    fn parse_banned_command() {
        assert_eq!("/banned".parse::<Command>().unwrap(), Command::Banned);
    }

    #[test]
    fn parse_away_command_with_args() {
        assert_eq!(
            "/away Out for lunch".parse::<Command>().unwrap(),
            Command::Away("Out for lunch".to_string())
        );
    }

    #[test]
    fn fail_to_parse_away_command_without_args() {
        assert_eq!(
            "/away".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected(
                "away reason".to_string()
            ))
        );
    }

    #[test]
    fn parse_name_command() {
        assert_eq!(
            "/name new_user".parse::<Command>().unwrap(),
            Command::Name("new_user".to_string())
        );
    }

    #[test]
    fn fail_to_parse_name_command_without_args() {
        assert_eq!(
            "/name".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("new name".to_string()))
        );
    }

    #[test]
    fn parse_ban_command() {
        assert_eq!(
            "/ban spammer".parse::<Command>().unwrap(),
            Command::Ban("spammer".to_string())
        );
    }

    #[test]
    fn fail_to_parse_ban_command_without_args() {
        assert_eq!(
            "/ban".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("ban query".to_string()))
        );
    }

    #[test]
    fn parse_whitelist_command() {
        assert_eq!(
            "/whitelist add user".parse::<Command>().unwrap(),
            Command::Whitelist(WhitelistCommand::Add("user".to_string()))
        );
    }

    #[test]
    fn parse_oplist_command() {
        assert_eq!(
            "/oplist add operator".parse::<Command>().unwrap(),
            Command::Oplist(OplistCommand::Add("operator".to_string()))
        );
    }

    #[test]
    fn parse_motd_command_with_args() {
        assert_eq!(
            "/motd Welcome!".parse::<Command>().unwrap(),
            Command::Motd(Some("Welcome!".to_string()))
        );
    }

    #[test]
    fn parse_motd_command_without_args() {
        assert_eq!("/motd".parse::<Command>().unwrap(), Command::Motd(None));
    }

    #[test]
    fn parse_me_command_with_args() {
        assert_eq!(
            "/me is happy".parse::<Command>().unwrap(),
            Command::Me(Some("is happy".to_string()))
        );
    }

    #[test]
    fn parse_me_command_without_args() {
        assert_eq!("/me".parse::<Command>().unwrap(), Command::Me(None));
    }

    #[test]
    fn parse_reply_command() {
        assert_eq!(
            "/reply Sure thing!".parse::<Command>().unwrap(),
            Command::Reply("Sure thing!".to_string())
        );
    }

    #[test]
    fn fail_to_parse_reply_command_without_args() {
        assert_eq!(
            "/reply".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected(
                "message body".to_string()
            ))
        );
    }

    #[test]
    fn parse_whois_command() {
        assert_eq!(
            "/whois username".parse::<Command>().unwrap(),
            Command::Whois("username".to_string())
        );
    }

    #[test]
    fn fail_to_parse_whois_command_without_args() {
        assert_eq!(
            "/whois".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("user name".to_string()))
        );
    }

    #[test]
    fn parse_slap_command_with_args() {
        assert_eq!(
            "/slap user".parse::<Command>().unwrap(),
            Command::Slap(Some("user".to_string()))
        );
    }

    #[test]
    fn parse_slap_command_without_args() {
        assert_eq!("/slap".parse::<Command>().unwrap(), Command::Slap(None));
    }

    #[test]
    fn parse_timestamp_command_with_valid_mode() {
        assert_eq!(
            "/timestamp time".parse::<Command>().unwrap(),
            Command::Timestamp(TimestampMode::Time)
        );
    }

    #[test]
    fn fail_to_parse_timestamp_command_with_invalid_mode() {
        assert_eq!(
            "/timestamp invalid_mode".parse::<Command>(),
            Err(CommandParseError::Other(
                "timestamp mode value must be one of: time, datetime, off".to_string()
            ))
        );
    }

    #[test]
    fn parse_theme_command_with_valid_theme() {
        assert_eq!(
            "/theme colors".parse::<Command>().unwrap(),
            Command::Theme(Theme::Colors)
        );
    }

    #[test]
    fn fail_to_parse_theme_command_with_invalid_theme() {
        assert_eq!(
            "/theme invalid_theme".parse::<Command>(),
            Err(CommandParseError::Other(
                "theme value must be one of: colors, mono, hacker".to_string()
            ))
        );
    }

    #[test]
    fn parse_ignore_command_with_args() {
        assert_eq!(
            "/ignore user".parse::<Command>().unwrap(),
            Command::Ignore(Some("user".to_string()))
        );
    }

    #[test]
    fn parse_ignore_command_without_args() {
        assert_eq!("/ignore".parse::<Command>().unwrap(), Command::Ignore(None));
    }

    #[test]
    fn parse_unignore_command() {
        assert_eq!(
            "/unignore user".parse::<Command>().unwrap(),
            Command::Unignore("user".to_string())
        );
    }

    #[test]
    fn fail_to_parse_unignore_command_without_args() {
        assert_eq!(
            "/unignore".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("user name".to_string()))
        );
    }

    #[test]
    fn parse_focus_command_with_args() {
        assert_eq!(
            "/focus user".parse::<Command>().unwrap(),
            Command::Focus(Some("user".to_string()))
        );
    }

    #[test]
    fn parse_focus_command_without_args() {
        assert_eq!("/focus".parse::<Command>().unwrap(), Command::Focus(None));
    }

    #[test]
    fn parse_mute_command() {
        assert_eq!(
            "/mute user".parse::<Command>().unwrap(),
            Command::Mute("user".to_string())
        );
    }

    #[test]
    fn fail_to_parse_mute_command_without_args() {
        assert_eq!(
            "/mute".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("user name".to_string()))
        );
    }

    #[test]
    fn parse_kick_command() {
        assert_eq!(
            "/kick user".parse::<Command>().unwrap(),
            Command::Kick("user".to_string())
        );
    }

    #[test]
    fn fail_to_parse_kick_command_without_args() {
        assert_eq!(
            "/kick".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("user name".to_string()))
        );
    }

    #[test]
    fn parse_msg_command_with_args() {
        assert_eq!(
            "/msg user Hello!".parse::<Command>().unwrap(),
            Command::Msg("user".to_string(), "Hello!".to_string())
        );
    }

    #[test]
    fn fail_to_parse_msg_command_without_body() {
        assert_eq!(
            "/msg user".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected(
                "message body".to_string()
            ))
        );
    }

    #[test]
    fn fail_to_parse_msg_command_without_args() {
        assert_eq!(
            "/msg".parse::<Command>(),
            Err(CommandParseError::ArgumentExpected("user name".to_string()))
        );
    }

    #[test]
    fn fail_to_parse_invalid_command() {
        assert_eq!(
            "/invalid".parse::<Command>(),
            Err(CommandParseError::UnknownCommand)
        );
    }
}
