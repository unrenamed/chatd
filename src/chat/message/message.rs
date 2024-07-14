use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;

use crate::chat::UserConfig;

use super::{Author, Recipient};

#[enum_dispatch]
#[derive(Debug, Clone, PartialEq)]
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
pub trait MessageFormatter: Clone + MessageBaseOps {
    fn format(&self, cfg: &UserConfig) -> String;

    fn format_with_timestamp(&self, cfg: &UserConfig, format: &str) -> String {
        let timestamp = self.message_created_at().format(format);
        format!(
            "{} {}",
            cfg.theme().style_system_text(&timestamp.to_string()),
            self.format(cfg)
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct MessageBase {
    body: String,
    created_at: DateTime<Utc>,
}

#[enum_dispatch(Message)]
pub trait MessageBaseOps {
    fn message_body(&self) -> &String;
    fn message_created_at(&self) -> DateTime<Utc>;
}

impl MessageBaseOps for MessageBase {
    fn message_body(&self) -> &String {
        &self.body
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Public {
    base: MessageBase,
    from: Author,
}

impl Public {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for Public {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Public {
    fn format(&self, cfg: &UserConfig) -> String {
        let mut message = self.message_body().to_string();

        if let Some(re) = cfg.highlight() {
            if let Some(matched) = re.find(&message) {
                let replacement = cfg.theme().style_tagged_username(matched).to_string();
                message = re.replace_all(&message, &replacement).to_string();
            }
        }

        let username = cfg.theme().style_username(self.from.username().as_ref());
        format!("{}: {}", username, message)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Private {
    base: MessageBase,
    from: Author,
    to: Recipient,
}

impl Private {
    pub fn new(from: Author, to: Recipient, body: String) -> Self {
        Self {
            from,
            to,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }

    pub fn to(&self) -> &Recipient {
        &self.to
    }
}

impl MessageBaseOps for Private {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Private {
    fn format(&self, cfg: &UserConfig) -> String {
        format!(
            "[PM from {}] {}",
            cfg.theme().style_username(self.from.username().as_ref()),
            cfg.theme().style_text(&self.message_body())
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Emote {
    base: MessageBase,
    from: Author,
}

impl Emote {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for Emote {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Emote {
    fn format(&self, cfg: &UserConfig) -> String {
        let text = format!(" ** {} {}", self.from.username(), &self.message_body());
        cfg.theme().style_text(&text).to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Announce {
    base: MessageBase,
    from: Author,
}

impl Announce {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for Announce {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Announce {
    fn format(&self, cfg: &UserConfig) -> String {
        let text = format!(" * {} {}", self.from.username(), &self.message_body());
        cfg.theme().style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct System {
    base: MessageBase,
    from: Author,
}

impl System {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for System {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for System {
    fn format(&self, cfg: &UserConfig) -> String {
        let text = format!("-> {}", &self.message_body());
        cfg.theme().style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    base: MessageBase,
    from: Author,
}

impl Error {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for Error {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Error {
    fn format(&self, cfg: &UserConfig) -> String {
        let text = format!("-> Error: {}", &self.message_body());
        cfg.theme().style_system_text(&text).to_string()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    base: MessageBase,
    from: Author,
}

impl Command {
    pub fn new(from: Author, body: String) -> Self {
        Self {
            from,
            base: MessageBase {
                body,
                created_at: Utc::now(),
            },
        }
    }

    pub fn from(&self) -> &Author {
        &self.from
    }
}

impl MessageBaseOps for Command {
    fn message_body(&self) -> &String {
        self.base.message_body()
    }

    fn message_created_at(&self) -> DateTime<Utc> {
        self.base.message_created_at()
    }
}

impl MessageFormatter for Command {
    fn format(&self, cfg: &UserConfig) -> String {
        format!(
            "[{}] {}",
            cfg.theme().style_username(self.from.username().as_ref()),
            cfg.theme().style_text(&self.message_body()),
        )
    }
}
