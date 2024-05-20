use std::fmt::Display;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Exit,
    Away,
    Back,
    ChangeName,
    SendPrivateMessage,
    GetAllUsers,
    Whois,
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
            b"/name" => Ok(Command::ChangeName),
            b"/msg" => Ok(Command::SendPrivateMessage),
            b"/users" => Ok(Command::GetAllUsers),
            b"/whois" => Ok(Command::Whois),
            // Add other byte slice matches here
            _ => Err("Unknown command".to_string()),
        }
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Command::Exit => "/exit",
                Command::Away => "/away",
                Command::Back => "/back",
                Command::ChangeName => "/name",
                Command::SendPrivateMessage => "/msg",
                Command::GetAllUsers => "/users",
                Command::Whois => "/whois",
            }
        )
    }
}
