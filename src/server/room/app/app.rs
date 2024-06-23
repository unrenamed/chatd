use tokio::sync::mpsc;

use crate::server::room::message::Message;
use crate::server::room::message::{self, MessageFormatter};
use crate::server::room::user::User;
use crate::server::terminal::TerminalHandle;

use super::message_channel::MessageChannel;
use super::terminal::Terminal;

#[derive(Clone)]
pub struct App {
    pub user: User,
    // FIXME: the next fields MUST NOT be public. Other sessions
    // should NOT have access to these fields on the architecture
    // level
    pub channel: MessageChannel,
    pub terminal: Terminal,
}

impl App {
    pub fn new(user: User, handle: TerminalHandle) -> Self {
        Self {
            user,
            terminal: Terminal::new(handle),
            channel: MessageChannel::new(),
        }
    }

    pub async fn send_message(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.channel.send(msg).await
    }

    pub async fn send_user_is_muted_message(&self) -> Result<(), mpsc::error::SendError<Message>> {
        let msg = message::Error::new(
            self.user.clone(),
            "You are muted and cannot send messages.".to_string(),
        );
        self.send_message(msg.into()).await
    }

    pub async fn wait_for_messages(&mut self) {
        while let Ok(msg) = self.channel.receive().await {
            let message = match self.user.timestamp_mode.format() {
                Some(fmt) => msg.format_with_timestamp(&self.user, fmt),
                None => msg.format(&self.user),
            };
            self.terminal.write_message(&message).await.unwrap();
        }
    }
}
