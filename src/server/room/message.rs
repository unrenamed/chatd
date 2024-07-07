use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;
use regex::{escape, Regex};

use crate::server::user::User;

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
    fn get_created_at(&self) -> DateTime<Utc>;

    fn format_with_timestamp(&self, user: &User, format: &str) -> String {
        let timestamp = self.get_created_at().format(format);
        format!(
            "{} {}",
            user.theme.style_system_text(&timestamp.to_string()),
            self.format(user)
        )
    }
}

#[derive(Clone, Debug)]
pub struct Public {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl Public {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for Public {
    fn format(&self, user: &User) -> String {
        let pattern = format!("@{}", user.username);
        let escaped_pattern = escape(&pattern);
        let mut message = self.body.clone();

        if let Ok(re) = Regex::new(&escaped_pattern) {
            let replacement = user.theme.style_tagged_username(&pattern).to_string();
            message = re.replace_all(&self.body, replacement).to_string();
        }

        let username = user.theme.style_username(&self.from.username);
        format!("{}: {}", username, message)
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct Private {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub to: User,
    pub body: String,
}

impl Private {
    pub fn new(from: User, to: User, body: String) -> Self {
        Self {
            from,
            to,
            body,
            created_at: Utc::now(),
        }
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

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct Emote {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl Emote {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for Emote {
    fn format(&self, user: &User) -> String {
        let text = format!(" ** {} {}", &self.from.username, &self.body);
        user.theme.style_text(&text).to_string()
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct Announce {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl Announce {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for Announce {
    fn format(&self, user: &User) -> String {
        let text = format!(" * {} {}", &self.from.username, &self.body);
        user.theme.style_system_text(&text).to_string()
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct System {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl System {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for System {
    fn format(&self, user: &User) -> String {
        let text = format!("-> {}", &self.body);
        user.theme.style_system_text(&text).to_string()
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl Error {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for Error {
    fn format(&self, user: &User) -> String {
        let text = format!("-> Error: {}", &self.body);
        user.theme.style_system_text(&text).to_string()
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug)]
pub struct Command {
    pub created_at: DateTime<Utc>,
    pub from: User,
    pub body: String,
}

impl Command {
    pub fn new(from: User, body: String) -> Self {
        Self {
            from,
            body,
            created_at: Utc::now(),
        }
    }
}

impl MessageFormatter for Command {
    fn format(&self, user: &User) -> String {
        format!(
            "[{}] {}",
            user.theme.style_username(&self.from.username),
            user.theme.style_text(&self.body),
        )
    }

    fn get_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
