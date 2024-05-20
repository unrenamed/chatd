use std::io::Write;

use crossterm::cursor::{self};
use crossterm::terminal::{self};
use crossterm::{queue, style};

use crate::chat::app::ChatApp;
use crate::utils;

use super::message::{ChatMessageFormatter, Message};
use super::terminal::TerminalHandle;

pub fn render(
    term: &mut TerminalHandle,
    app: &ChatApp,
    messages: &Vec<Message>,
    motd: &str,
) -> Result<(), anyhow::Error> {
    queue!(
        term,
        cursor::SavePosition,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::FromCursorDown),
    )?;

    queue!(
        term,
        style::Print(utils::NEWLINE),
        style::Print(format!("{}{}", motd, utils::NEWLINE)),
        style::Print(utils::NEWLINE),
        style::SetAttribute(style::Attribute::Reset)
    )?;

    for (idx, message) in messages.iter().enumerate() {
        let show_message = idx >= app.history_start_idx
            && match message {
                Message::Public(_) => true,
                Message::Announce(_) => true,
                Message::Emote(_) => true,
                Message::System(msg) => msg.from.id.eq(&app.user.id),
                Message::Command(msg) => msg.from.id.eq(&app.user.id),
                Message::Private(msg) => msg.from.id.eq(&app.user.id) || msg.to.id.eq(&app.user.id),
            };
        if !show_message {
            continue;
        }
        queue!(
            term,
            style::Print(message.format(&app)),
            style::Print(utils::NEWLINE)
        )?
    }

    queue!(
        term,
        style::Print(format!(
            "[{}] {}",
            app.theme.style_username(&app.user.username),
            app.input.to_str()
        )),
    )?;

    queue!(term, cursor::RestorePosition)?;

    term.flush()?;

    Ok(())
}
