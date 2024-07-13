use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, watch};

use crate::chat::message::{self, Message, MessageFormatter};
use crate::chat::user::User;

#[derive(Clone)]
pub struct RoomMember {
    pub user: User,
    message_tx: mpsc::Sender<String>,
    exit_tx: watch::Sender<()>,
    last_sent_at: Option<DateTime<Utc>>,
}

impl RoomMember {
    pub fn new(user: User, message_tx: mpsc::Sender<String>, exit_tx: watch::Sender<()>) -> Self {
        Self {
            user,
            message_tx,
            exit_tx,
            last_sent_at: None,
        }
    }

    pub fn last_sent_time(&self) -> &Option<DateTime<Utc>> {
        &self.last_sent_at
    }

    pub fn update_last_sent_time(&mut self, time: DateTime<Utc>) {
        self.last_sent_at = Some(time);
    }

    pub fn exit(&self) -> Result<(), watch::error::SendError<()>> {
        self.exit_tx.send(())
    }

    pub async fn send_message(&self, msg: Message) -> Result<(), mpsc::error::SendError<String>> {
        let message = match self.user.timestamp_mode.format() {
            Some(fmt) => msg.format_with_timestamp(&self.user, fmt),
            None => msg.format(&self.user),
        };
        self.message_tx.send(message).await
    }

    pub async fn send_user_is_muted_message(&self) -> Result<(), mpsc::error::SendError<String>> {
        let msg = message::Error::new(
            self.user.clone(),
            "You are muted and cannot send messages.".to_string(),
        );
        self.send_message(msg.into()).await
    }
}
