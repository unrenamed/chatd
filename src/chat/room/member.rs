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
        let message = match self.user.config().timestamp_mode().format() {
            Some(fmt) => msg.format_with_timestamp(&self.user.config(), fmt),
            None => msg.format(&self.user.config()),
        };
        self.message_tx.send(message).await
    }

    pub async fn send_user_is_muted_message(&self) -> Result<(), mpsc::error::SendError<String>> {
        let msg = message::Error::new(
            self.user.clone().into(),
            "You are muted and cannot send messages.".to_string(),
        );
        self.send_message(msg.into()).await
    }
}

#[cfg(test)]
mod should {
    use chrono::Utc;
    use message::MessageBaseOps;
    use tokio::sync::{mpsc, watch};

    use super::*;
    use crate::chat::user::User;
    use crate::chat::TimestampMode;

    #[tokio::test]
    async fn create_room_member() {
        let (message_tx, _message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let user = User::default();

        let room_member = RoomMember::new(user.clone(), message_tx, _exit_tx);

        assert_eq!(room_member.user, user);
        assert!(room_member.last_sent_time().is_none());
    }

    #[tokio::test]
    async fn update_last_sent_time() {
        let (message_tx, _message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let user = User::default();
        let mut room_member = RoomMember::new(user.clone(), message_tx, _exit_tx);

        let now = Utc::now();
        room_member.update_last_sent_time(now);

        assert_eq!(*room_member.last_sent_time(), Some(now));
    }

    #[tokio::test]
    async fn send_message() {
        let (message_tx, mut message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let user = User::default();
        let room_member = RoomMember::new(user.clone(), message_tx, _exit_tx);

        let msg = message::System::new(User::default().into(), "Hello".to_string());
        let result = room_member.send_message(msg.into()).await;

        assert!(result.is_ok());
        let received_message = message_rx.recv().await.unwrap();
        assert!(received_message.contains("Hello"));
    }

    #[tokio::test]
    async fn send_message_with_timestamp() {
        let (message_tx, mut message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let mut user = User::default();
        user.config_mut().set_timestamp_mode(TimestampMode::Time);
        let room_member = RoomMember::new(user.clone(), message_tx, _exit_tx);

        let msg = message::System::new(User::default().into(), "Hello".to_string());
        let timestamp = msg.message_created_at();
        let timestamp_format = user.config().timestamp_mode().format().unwrap();

        let result = room_member.send_message(msg.into()).await;
        assert!(result.is_ok());
        let received_message = message_rx.recv().await.unwrap();
        assert!(received_message.contains(&timestamp.format(timestamp_format).to_string()));
        assert!(received_message.contains("Hello"));
    }

    #[tokio::test]
    async fn send_user_is_muted_message() {
        let (message_tx, mut message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let user = User::default();
        let room_member = RoomMember::new(user.clone(), message_tx, _exit_tx);

        let result = room_member.send_user_is_muted_message().await;

        assert!(result.is_ok());
        let received_message = message_rx.recv().await.unwrap();
        assert!(received_message.contains("You are muted and cannot send messages."));
    }

    #[tokio::test]
    async fn exit() {
        let (_message_tx, _message_rx) = mpsc::channel(1);
        let (exit_tx, exit_rx) = watch::channel(());
        let user = User::default();
        let room_member = RoomMember::new(user.clone(), _message_tx, exit_tx);

        let result = room_member.exit();

        assert!(result.is_ok());
        assert_eq!(*exit_rx.borrow(), ());
    }
}
