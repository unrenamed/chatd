#[derive(Debug, PartialEq)]
pub enum CommandParseError {
    NotRecognizedAsCommand,
    UnknownCommand,
    ArgumentExpected(String),
    Other(String),
}

impl std::fmt::Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CommandParseError::NotRecognizedAsCommand => write!(f, "not a command"),
            CommandParseError::UnknownCommand => write!(f, "unknown command"),
            CommandParseError::ArgumentExpected(arg) => write!(f, "{} is expected", arg),
            CommandParseError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for CommandParseError {}
