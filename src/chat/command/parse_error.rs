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

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn display_not_recognized_as_command_error() {
        let error = CommandParseError::NotRecognizedAsCommand;
        assert_eq!(format!("{}", error), "not a command");
    }

    #[test]
    fn display_unknown_command_error() {
        let error = CommandParseError::UnknownCommand;
        assert_eq!(format!("{}", error), "unknown command");
    }

    #[test]
    fn display_argument_expected_error() {
        let error = CommandParseError::ArgumentExpected("argument".to_string());
        assert_eq!(format!("{}", error), "argument is expected");
    }

    #[test]
    fn display_other_error() {
        let error = CommandParseError::Other("other error".to_string());
        assert_eq!(format!("{}", error), "other error");
    }
}
