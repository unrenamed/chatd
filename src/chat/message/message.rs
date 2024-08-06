use chrono::{DateTime, Utc};
use enum_dispatch::enum_dispatch;

use crate::chat::UserConfig;
use crate::utils::{BEL, NULL};

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
            "[PM from {}] {}{}",
            cfg.theme().style_username(self.from.username().as_ref()),
            cfg.theme().style_text(&self.message_body()),
            if cfg.bell() {
                BEL // emit bell sound in recipient's terminal
            } else {
                NULL
            }
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

#[cfg(test)]
mod should {
    use chrono::TimeZone;

    use super::*;
    use crate::chat::{User, UserConfig};

    // Helper function to create a mock UserConfig
    fn mock_user_config() -> UserConfig {
        UserConfig::default()
    }

    fn mock_user_config_with_highlight() -> UserConfig {
        let mut user = User::default();
        user.set_username("alice".into());
        user.config().clone()
    }

    fn mock_author() -> Author {
        let mut user = User::default();
        user.set_username("alice".into());
        user.into()
    }

    fn mock_recipient() -> Recipient {
        let mut user = User::default();
        user.set_username("bob".into());
        user.into()
    }

    #[test]
    fn format_public_message_correctly() {
        let author = mock_author();
        let msg = Public::new(author.clone(), "hello world".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "\u{1b}[38;2;104;128;66malice\u{1b}[39m: hello world"
        );
    }

    #[test]
    fn format_public_message_with_highlight_correctly() {
        let author = mock_author();
        let msg = Public::new(author.clone(), "hello @alice".to_string());
        let cfg = mock_user_config_with_highlight();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "\u{1b}[38;2;104;128;66malice\u{1b}[39m: hello \u{1b}[48;5;3m\u{1b}[38;5;0m\u{1b}[1m@alice\u{1b}[0m"
        );
    }

    #[test]
    fn format_private_message_correctly() {
        let author = mock_author();
        let recipient = mock_recipient();
        let msg = Private::new(author.clone(), recipient.clone(), "hello".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "[PM from \u{1b}[38;2;104;128;66malice\u{1b}[39m] \u{1b}[38;5;15mhello\u{1b}[39m\u{7}"
        );
    }

    #[test]
    fn format_emote_message_correctly() {
        let author = mock_author();
        let msg = Emote::new(author.clone(), "waves".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(formatted_msg, "\u{1b}[38;5;15m ** alice waves\u{1b}[39m");
    }

    #[test]
    fn format_announce_message_correctly() {
        let author = mock_author();
        let msg = Announce::new(author.clone(), "announcement".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "\u{1b}[38;5;8m * alice announcement\u{1b}[39m"
        );
    }

    #[test]
    fn format_system_message_correctly() {
        let author = mock_author();
        let msg = System::new(author.clone(), "system message".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(formatted_msg, "\u{1b}[38;5;8m-> system message\u{1b}[39m");
    }

    #[test]
    fn format_error_message_correctly() {
        let author = mock_author();
        let msg = Error::new(author.clone(), "error occurred".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "\u{1b}[38;5;8m-> Error: error occurred\u{1b}[39m"
        );
    }

    #[test]
    fn format_command_message_correctly() {
        let author = mock_author();
        let msg = Command::new(author.clone(), "command executed".to_string());
        let cfg = mock_user_config();
        let formatted_msg = msg.format(&cfg);
        assert_eq!(
            formatted_msg,
            "[\u{1b}[38;2;104;128;66malice\u{1b}[39m] \u{1b}[38;5;15mcommand executed\u{1b}[39m"
        );
    }

    #[test]
    fn format_message_with_timestamp() {
        let author = mock_author();
        let timestamp = Utc.with_ymd_and_hms(2024, 7, 19, 12, 34, 56).unwrap();

        let mut msg = Public::new(author.clone(), "hello world".to_string());
        msg.base.created_at = timestamp;

        let cfg = mock_user_config();
        let formatted_msg = msg.format_with_timestamp(&cfg, "%Y-%m-%d %H:%M:%S");
        assert_eq!(formatted_msg, "\u{1b}[38;5;8m2024-07-19 12:34:56\u{1b}[39m \u{1b}[38;2;104;128;66malice\u{1b}[39m: hello world");
    }
}
