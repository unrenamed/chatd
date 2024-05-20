#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Exit,
    Away,
    Back,
    Nick,
}

impl Command {
    pub fn is_command(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0x2f])
    }

    pub fn parse(bytes: &[u8]) -> Result<Command, String> {
        match bytes {
            b"/exit" => Ok(Command::Exit),
            b"/away" => Ok(Command::Away),
            b"/back" => Ok(Command::Back),
            b"/nick" => Ok(Command::Nick),
            // Add other byte slice matches here
            _ => Err("Unknown command".to_string()),
        }
    }
}
