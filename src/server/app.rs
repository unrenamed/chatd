use std::io::Write;
use std::sync::Arc;

use crossterm::cursor::{self};
use crossterm::terminal::{self};
use crossterm::{queue, style};
use tokio::sync::{mpsc, Mutex};

use crate::server::message::MessageFormatter;
use crate::server::terminal::TerminalHandle;
use crate::utils;

use super::message::Message;
use super::{state::UserState, user::User};

type Terminal = Arc<Mutex<TerminalHandle>>;

#[derive(Clone)]
pub struct MessageChannel {
    sender: mpsc::Sender<Message>,
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl MessageChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.sender.send(msg).await
    }

    pub async fn receive(&self) -> Result<Message, mpsc::error::TryRecvError> {
        self.receiver.lock().await.try_recv()
    }
}

#[derive(Clone)]
pub struct App {
    pub user: User,
    pub state: UserState,
    pub channel: MessageChannel,
    pub terminal: Terminal,
}

impl App {
    pub async fn send_message(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.channel.send(msg).await
    }

    pub async fn render_motd(&mut self, motd: &str) -> Result<(), anyhow::Error> {
        if self.state.render_motd {
            queue!(
                self.terminal.lock().await,
                style::Print(utils::NEWLINE),
                style::Print(format!("{}{}", motd, utils::NEWLINE)),
                style::Print(utils::NEWLINE),
                style::SetAttribute(style::Attribute::Reset)
            )?;

            self.state.render_motd = false;
        }

        Ok(())
    }

    pub async fn render(&mut self) -> Result<(), anyhow::Error> {
        queue!(self.terminal.lock().await, cursor::Show)?;

        // on the first render we CAN NOT restore cursor position for the entire screen will be cleared
        if !self.state.first_render {
            // now, after we saved the cursor position at the beginning of the user prompt, we CAN restore
            // the cursor position to clear everything down to prepare the screen for rendering new messages
            // and new user prompt
            queue!(
                self.terminal.lock().await,
                cursor::RestorePosition,
                terminal::Clear(terminal::ClearType::FromCursorDown)
            )?;
        }

        if let Ok(msg) = self.channel.receive().await {
            queue!(
                self.terminal.lock().await,
                style::Print(msg.format(&self.user)),
                style::Print(utils::NEWLINE)
            )?;
        }

        queue!(
            self.terminal.lock().await,
            cursor::SavePosition,
            style::Print(format!(
                "[{}] {}",
                self.user.theme.style_username(&self.user.username),
                self.state.input.to_str()
            )),
        )?;

        self.terminal.lock().await.flush()?;

        if self.state.first_render {
            self.state.first_render = false;
        }

        Ok(())
    }
}
