use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::utils::{self, kmp::KMP};

pub enum ClientEvent {
    Connected(ConnectedEvent),
    Left(LeftEvent),
    GoAway(GoAwayEvent),
    ReturnBack(ReturnBackEvent),
    SendMessage(SendMessageEvent),
}

impl ClientEvent {
    pub fn format_line(&self, current_username: &str) -> Line {
        match self {
            ClientEvent::Connected(event) => event.format_line(current_username),
            ClientEvent::Left(event) => event.format_line(current_username),
            ClientEvent::SendMessage(event) => event.format_line(current_username),
            ClientEvent::GoAway(event) => event.format_line(current_username),
            ClientEvent::ReturnBack(event) => event.format_line(current_username),
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

pub trait Displayable {
    fn format_line(&self, current_username: &str) -> Line;
}

impl Displayable for ConnectedEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!(
                " * {} joined. (Connected: {})",
                self.username, self.total_connected
            ),
            Style::default().fg(Color::DarkGray),
        )])
    }
}

impl Displayable for LeftEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!(
                " * {} left. ({})",
                self.username,
                utils::datetime::format_distance_to_now(self.session_duration)
            ),
            Style::default().fg(Color::DarkGray),
        )])
    }
}

impl Displayable for SendMessageEvent {
    fn format_line(&self, current_username: &str) -> Line {
        let (r, g, b) = utils::rgb::gen_rgb(&self.username);
        let username_span = Span::styled(
            format!("{}: ", self.username),
            Style::default().fg(Color::Rgb(r, g, b)),
        );

        let pattern = format!("@{}", current_username);
        let matches = KMP::new(&pattern).search(&self.message);
        let mut message_spans =
            utils::message::split_by_indices(&self.message, &matches, pattern.len());

        let mut spans = vec![username_span];
        spans.append(&mut message_spans);

        Line::from(spans)
    }
}

impl Displayable for GoAwayEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!("** {} has gone away: {}", self.username, self.reason),
            Style::default().fg(Color::White),
        )])
    }
}

impl Displayable for ReturnBackEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!("** {} is back.", self.username),
            Style::default().fg(Color::White),
        )])
    }
}
