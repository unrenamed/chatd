use enum_dispatch::enum_dispatch;

use crate::utils::kmp::KMP;

use super::user::User;

/// Messages:
/// - public: sent from a user to everyone, anyone can see it
/// nazar: hello all
/// - private: sent from a user to another user, not shown to anyone else
/// [PM from nazar] psss... what's up?
/// - emote: sent from a user to everyone, like a away or back event
/// ** user has gone away: will be back in 10 minutes
/// - announce: sent from the server to everyone, like a join or leave event
/// * user joined (Connected 14)
/// - system: sent from the server directly to a user, not shown to anyone else. Usually in response to something, like /help.
/// -> Error: This username is already taken

#[enum_dispatch]
#[derive(Clone)]
pub enum Message {
    Public,
    Private,
    Emote,
    Announce,
    System,
    Error,
    Command,
}

// The trait for formatting a message within the context of a chat application
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
