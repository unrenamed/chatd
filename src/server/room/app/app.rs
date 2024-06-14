use std::io::Write;
use std::sync::Arc;

use crossterm::cursor;
use crossterm::terminal;
use crossterm::{queue, style};
use tokio::sync::{mpsc, Mutex};

use crate::server::room::message;
use crate::server::room::message::Message;
use crate::server::room::message::MessageFormatter;
use crate::server::room::user::User;
use crate::server::terminal::TerminalHandle;
use crate::utils;

use super::message_channel::MessageChannel;
use super::state::UserState;

type Terminal = Arc<Mutex<TerminalHandle>>;

#[derive(Clone)]
pub struct App {
    pub user: User,
    pub state: UserState,
    pub channel: MessageChannel,
    pub terminal: Terminal,
}

impl App {
    pub fn new(user: User, terminal: Terminal) -> Self {
        Self {
            user,
            terminal,
            state: UserState::new(),
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

    pub async fn render(&mut self) -> Result<(), anyhow::Error> {
        queue!(self.terminal.lock().await, cursor::Show)?;

        // On the first render we CAN NOT restore cursor position because the entire
        // screen will be cleared
        if !self.state.first_render {
            // After saving the cursor position at the start of the user prompt,
            // we can restore it to clear the screen part for new messages and prompt
            queue!(
                self.terminal.lock().await,
                cursor::RestorePosition,
                terminal::Clear(terminal::ClearType::FromCursorDown)
            )?;
        }

        while let Ok(msg) = self.channel.receive().await {
            queue!(
                self.terminal.lock().await,
                style::Print(match self.user.timestamp_mode.format() {
                    Some(fmt) => msg.format_with_timestamp(&self.user, fmt),
                    None => msg.format(&self.user),
                }),
                style::Print(utils::NEWLINE)
            )?;
        }

        let user_input = format!(
            "[{}] {}",
            self.user.theme.style_username(&self.user.username),
            self.state.input.to_string()
        );

        let prefix_len = 3 + self.user.username.len() as u16; // 3 is the length of "[] " wrapping

        queue!(
            self.terminal.lock().await,
            cursor::SavePosition,
            style::Print(user_input),
            cursor::MoveToColumn(prefix_len + *self.state.input.char_cursor_pos() as u16),
        )?;

        self.terminal.lock().await.flush()?;

        if self.state.first_render {
            self.state.first_render = false;
        }

        Ok(())
    }
}
