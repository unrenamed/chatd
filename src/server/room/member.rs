use tokio::sync::mpsc;

use crate::server::room::message;
use crate::server::room::message::Message;
use crate::server::room::user::User;

use super::message::MessageFormatter;

#[derive(Clone)]
pub struct RoomMember {
    pub user: User,
    pub message_tx: mpsc::Sender<String>,
}

impl RoomMember {
    pub fn new(user: User, message_tx: mpsc::Sender<String>) -> Self {
        Self { user, message_tx }
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
