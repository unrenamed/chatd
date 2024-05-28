use enum_dispatch::enum_dispatch;

use crate::utils::kmp::KMP;

use super::user::User;

#[enum_dispatch]
#[derive(Clone)]
pub enum Message {
    // Sent by a user to everyone; visible to all
    Public,
    // Sent by a user to everyone; shares an action or visible emotion
    Emote,
    // Sent by the server to everyone; announces server actions
    Announce,
    // Sent by a user to another user; private, not shown to others
    Private,
    // Sent by the server to a caller; usually in response to commands
    System,
    // Sent by the server to a caller; indicates an error occurred during a command
    Error,
    // Sent by the server to a caller; reminder about the called command
    Command,
}

/// Trait for formatting a message within the context of a chat user
#[enum_dispatch(Message)]
pub trait MessageFormatter: Clone {
    fn format(&self, user: &User) -> String;
}

#[derive(Clone, Debug)]
pub struct Public {
    pub from: User,
    pub body: String,
}

impl Public {
    pub fn new(from: User, body: String) -> Self {
        Self { from, body }
    }
}

impl MessageFormatter for Public {
    fn format(&self, user: &User) -> String {
        let pattern = format!("@{}", user.username);
        let pattern_len = pattern.len();
        let matches = KMP::new(&pattern).search(&self.body);

        let mut body_parts = Vec::new();
        let mut prev_index = 0;

        for &index in &matches {
            if index >= self.body.len() {
                break;
            }
            if prev_index < index {
                body_parts.push(
                    user.theme
                        .style_text(&self.body[prev_index..index])
                        .to_string(),
                );
            }
            if index + pattern_len <= self.body.len() {
                body_parts.push(
                    user.theme
                        .style_tagged_username(&self.body[index..index + pattern_len])
                        .to_string(),
                );
                prev_index = index + pattern_len;
            }
        }
        if prev_index < self.body.len() {
            body_parts.push(user.theme.style_text(&self.body[prev_index..]).to_string());
        }

        format!(
            "{}: {}",
            user.theme.style_username(&self.from.username),
            body_parts.join("")
        )
    }
}

#[derive(Clone, Debug)]
pub struct Private {
    pub from: User,
    pub to: User,
    pub body: String,
}

impl Private {
    pub fn new(from: User, to: User, body: String) -> Self {
        Self { from, to, body }
    }
}

impl MessageFormatter for Private {
    fn format(&self, user: &User) -> String {
        if user.username.eq(&self.from.username) {
            format!(
                "[PM to {}] {}",
                user.theme.style_username(&self.to.username),
                user.theme.style_text(&self.body)
            )
        } else {
            format!(
                "[PM from {}] {}",
                user.theme.style_username(&self.from.username),
                user.theme.style_text(&self.body)
            )
        }
    }
}

#[derive(Clone, Debug)]
pub struct Emote {
    pub from: User,
    pub body: String,
}

impl Emote {
    pub fn new(from: User, body: String) -> Self {
        Self { from, body }
    }
}

impl MessageFormatter for Emote {
    fn format(&self, user: &User) -> String {
        let text = format!(" ** {} {}", &self.from.username, &self.body);
        user.theme.style_text(&text).to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Announce {
    pub from: User,
    pub body: String,
}

impl Announce {
    pub fn new(from: User, body: String) -> Self {
        Self { from, body }
    }
}

impl MessageFormatter for Announce {
    fn format(&self, user: &User) -> String {
        let text = format!(" * {} {}", &self.from.username, &self.body);
        user.theme.style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug)]
pub struct System {
    pub from: User,
    pub body: String,
}

impl System {
    pub fn new(from: User, body: String) -> Self {
        Self { from, body }
    }
}

impl MessageFormatter for System {
    fn format(&self, user: &User) -> String {
        let text = format!("-> {}", &self.body);
        user.theme.style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub from: User,
    pub body: String,
}

impl Error {
    pub fn new(from: User, body: String) -> Self {
        Self { from, body }
    }
}

impl MessageFormatter for Error {
    fn format(&self, user: &User) -> String {
        let text = format!("-> Error: {}", &self.body);
        user.theme.style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub from: User,
    pub cmd: String,
    pub args: String,
}

impl Command {
    pub fn new(from: User, cmd: String, args: String) -> Self {
        Self { from, cmd, args }
    }
}

impl MessageFormatter for Command {
    fn format(&self, user: &User) -> String {
        format!(
            "[{}] {} {}",
            user.theme.style_username(&self.from.username),
            user.theme.style_text(&self.cmd),
            user.theme.style_text(&self.args)
        )
    }
}
