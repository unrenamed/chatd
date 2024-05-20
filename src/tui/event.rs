use crate::{
    models::event::*,
    tui::style::{StyledText, TextPart},
    utils::{self, kmp::KMP},
};

impl ClientEvent {
    pub fn format(&self, current_username: &str) -> StyledText {
        match self {
            ClientEvent::SendMessage(event) => event.format(current_username),
            ClientEvent::Left(event) => event.format(),
            ClientEvent::GoAway(event) => event.format(),
            ClientEvent::Connected(event) => event.format(),
            ClientEvent::ReturnBack(event) => event.format(),
        }
    }
}

trait FormattableEvent {
    fn format(&self) -> StyledText;
}

trait FormattableEventWithUser {
    fn format(&self, current_username: &str) -> StyledText;
}

impl FormattableEvent for ConnectedEvent {
    fn format(&self) -> StyledText {
        let text = format!(
            " * {} joined. (Connected: {})",
            self.username, self.total_connected
        );
        StyledText::new(vec![TextPart::InfoDimmed(text)])
    }
}

impl FormattableEvent for LeftEvent {
    fn format(&self) -> StyledText {
        let text = format!(
            " * {} left. ({})",
            self.username,
            utils::datetime::format_distance_to_now(self.session_duration)
        );
        StyledText::new(vec![TextPart::InfoDimmed(text)])
    }
}

impl FormattableEvent for GoAwayEvent {
    fn format(&self) -> StyledText {
        let text = format!("** {} has gone away: {}", self.username, self.reason);
        StyledText::new(vec![TextPart::Info(text)])
    }
}

impl FormattableEvent for ReturnBackEvent {
    fn format(&self) -> StyledText {
        let text = format!("** {} is back.", self.username);
        StyledText::new(vec![TextPart::Info(text)])
    }
}

impl FormattableEventWithUser for SendMessageEvent {
    fn format(&self, current_username: &str) -> StyledText {
        let mut parts = vec![TextPart::Username {
            name: self.username.clone(),
            display_name: format!("{}: ", self.username),
        }];

        let pattern = format!("@{}", current_username);
        let matches = KMP::new(&pattern).search(&self.message);
        let mut message_spans = split_message_by_indices(&self.message, &matches, pattern.len());

        parts.append(&mut message_spans);
        StyledText::new(parts)
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
