use crate::utils::{self, kmp::KMP};

pub enum TextPart {
    Info(String),
    InfoDimmed(String),
    Message(String),
    MessageHighlighted(String),
    Username(String),
}

pub struct StyledText {
    pub parts: Vec<TextPart>,
}

impl StyledText {
    fn new(parts: Vec<TextPart>) -> Self {
        Self { parts }
    }
}

pub enum ClientEvent {
    Connected(ConnectedEvent),
    Left(LeftEvent),
    GoAway(GoAwayEvent),
    ReturnBack(ReturnBackEvent),
    SendMessage(SendMessageEvent),
}

impl ClientEvent {
    pub fn format(&self, current_username: &str) -> StyledText {
        match self {
            ClientEvent::Connected(event) => event.format(current_username),
            ClientEvent::Left(event) => event.format(current_username),
            ClientEvent::SendMessage(event) => event.format(current_username),
            ClientEvent::GoAway(event) => event.format(current_username),
            ClientEvent::ReturnBack(event) => event.format(current_username),
        }
    }
}

pub struct ConnectedEvent {
    pub username: String,
    pub total_connected: usize,
}

pub struct LeftEvent {
    pub username: String,
    pub session_duration: i64,
}

pub struct SendMessageEvent {
    pub username: String,
    pub message: String,
}

pub struct GoAwayEvent {
    pub username: String,
    pub reason: String,
}

pub struct ReturnBackEvent {
    pub username: String,
}

pub trait Display {
    fn format(&self, current_username: &str) -> StyledText;
}

impl Display for ConnectedEvent {
    fn format(&self, _: &str) -> StyledText {
        let text = format!(
            " * {} joined. (Connected: {})",
            self.username, self.total_connected
        );
        StyledText::new(vec![TextPart::InfoDimmed(text)])
    }
}

impl Display for LeftEvent {
    fn format(&self, _: &str) -> StyledText {
        let text = format!(
            " * {} left. ({})",
            self.username,
            utils::datetime::format_distance_to_now(self.session_duration)
        );
        StyledText::new(vec![TextPart::InfoDimmed(text)])
    }
}

impl Display for SendMessageEvent {
    fn format(&self, current_username: &str) -> StyledText {
        let mut parts = vec![TextPart::Username(format!("{}: ", self.username))];

        let pattern = format!("@{}", current_username);
        let matches = KMP::new(&pattern).search(&self.message);
        let mut message_spans = split_message_by_indices(&self.message, &matches, pattern.len());

        parts.append(&mut message_spans);
        StyledText::new(parts)
    }
}

impl Display for GoAwayEvent {
    fn format(&self, _: &str) -> StyledText {
        let text = format!("** {} has gone away: {}", self.username, self.reason);
        StyledText::new(vec![TextPart::Info(text)])
    }
}

impl Display for ReturnBackEvent {
    fn format(&self, _: &str) -> StyledText {
        let text = format!("** {} is back.", self.username);
        StyledText::new(vec![TextPart::Info(text)])
    }
}

fn split_message_by_indices<'a>(
    text: &'a str,
    indices: &[usize],
    substr_len: usize,
) -> Vec<TextPart> {
    let mut spans = Vec::new();
    let mut prev_index = 0;

    for &index in indices {
        if index >= text.len() {
            break;
        }

        if prev_index < index {
            spans.push(TextPart::Message(String::from(&text[prev_index..index])));
        }

        if index + substr_len <= text.len() {
            spans.push(TextPart::MessageHighlighted(String::from(
                &text[index..index + substr_len],
            )));
            prev_index = index + substr_len;
        }
    }

    if prev_index < text.len() {
        spans.push(TextPart::Message(String::from(&text[prev_index..])));
    }

    spans
}
