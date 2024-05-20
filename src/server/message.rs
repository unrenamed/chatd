use crate::{
    chat::{app::ChatApp, user::User},
    utils::kmp::KMP,
};

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
pub enum Message {
    Public(PublicMessage),
    Private(PrivateMessage),
    Emote(EmoteMessage),
    Announce(AnnounceMessage),
    System(SystemMessage),
    Command(CommandMessage),
}

// The trait for formatting a message within the context of a chat application
pub trait ChatMessageFormatter {
    fn format(&self, app: &ChatApp) -> String;
}

impl ChatMessageFormatter for Message {
    fn format(&self, app: &ChatApp) -> String {
        match self {
            Message::Public(msg) => msg.format(app),
            Message::Private(msg) => msg.format(app),
            Message::Emote(msg) => msg.format(app),
            Message::Announce(msg) => msg.format(app),
            Message::System(msg) => msg.format(app),
            Message::Command(msg) => msg.format(app),
        }
    }
}

pub struct PublicMessage {
    pub from: User,
    pub body: String,
}

impl ChatMessageFormatter for PublicMessage {
    fn format(&self, app: &ChatApp) -> String {
        let pattern = format!("@{}", app.user.username);
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
                    app.theme
                        .style_text(&self.body[prev_index..index])
                        .to_string(),
                );
            }
            if index + pattern_len <= self.body.len() {
                body_parts.push(
                    app.theme
                        .style_tagged_username(&self.body[index..index + pattern_len])
                        .to_string(),
                );
                prev_index = index + pattern_len;
            }
        }
        if prev_index < self.body.len() {
            body_parts.push(app.theme.style_text(&self.body[prev_index..]).to_string());
        }

        format!(
            "{}: {}",
            app.theme.style_username(&self.from.username),
            body_parts.join("")
        )
    }
}

pub struct PrivateMessage {
    pub from: User,
    pub to: User,
    pub body: String,
}

impl ChatMessageFormatter for PrivateMessage {
    fn format(&self, app: &ChatApp) -> String {
        if app.user.username.eq(&self.from.username) {
            format!(
                "[PM to {}] {}",
                app.theme.style_username(&self.to.username),
                app.theme.style_text(&self.body)
            )
        } else {
            format!(
                "[PM from {}] {}",
                app.theme.style_username(&self.from.username),
                app.theme.style_text(&self.body)
            )
        }
    }
}

pub struct EmoteMessage {
    pub from: User,
    pub body: String,
}

impl ChatMessageFormatter for EmoteMessage {
    fn format(&self, app: &ChatApp) -> String {
        let text = format!(" ** {} {}", &self.from.username, &self.body);
        app.theme.style_text(&text).to_string()
    }
}

pub struct AnnounceMessage {
    pub from: User,
    pub body: String,
}

impl ChatMessageFormatter for AnnounceMessage {
    fn format(&self, app: &ChatApp) -> String {
        let text = format!(" * {} {}", &self.from.username, &self.body);
        app.theme.style_system_text(&text).to_string()
    }
}

pub struct SystemMessage {
    pub from: User,
    pub body: String,
}

impl ChatMessageFormatter for SystemMessage {
    fn format(&self, app: &ChatApp) -> String {
        let text = format!("-> {}", &self.body);
        app.theme.style_system_text(&text).to_string()
    }
}

pub struct CommandMessage {
    pub from: User,
    pub cmd: String,
    pub args: String,
}

impl ChatMessageFormatter for CommandMessage {
    fn format(&self, app: &ChatApp) -> String {
        format!(
            "[{}] {} {}",
            app.theme.style_username(&self.from.username),
            app.theme.style_text(&self.cmd),
            app.theme.style_text(&self.args)
        )
    }
}
