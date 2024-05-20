use std::io::Write;

use crate::chat::app::ChatApp;
use crate::models::event::ClientEvent;
use crate::models::terminal::TerminalHandle;
use crossterm::cursor::{self};
use crossterm::terminal::{self};
use crossterm::{queue, style};

use super::rgb;
use super::style::TextPart;

pub fn render(term: &mut TerminalHandle, app: &ChatApp, events: &Vec<ClientEvent>, motd: &str) {
    queue!(
        term,
        cursor::SavePosition,
        cursor::Hide,
        terminal::Clear(terminal::ClearType::FromCursorDown),
    )
    .unwrap();

    queue!(
        term,
        style::Print("\n\r"),
        style::Print(format!("{}\n\r", motd)),
        style::Print("\n\r"),
        style::SetAttribute(style::Attribute::Reset)
    )
    .unwrap();

    for event in events.iter() {
        let text = event.format(&app.user.username);
        for part in text.parts.iter() {
            match part {
                TextPart::Info(text) => {
                    queue!(term, style::Print(text)).unwrap();
                }
                TextPart::InfoDimmed(text) => {
                    queue!(
                        term,
                        style::SetForegroundColor(style::Color::DarkGrey),
                        style::Print(text),
                        style::ResetColor
                    )
                    .unwrap();
                }
                TextPart::Message(msg) => {
                    queue!(term, style::Print(msg),).unwrap();
                }
                TextPart::MessageHighlighted(msg) => {
                    queue!(
                        term,
                        style::SetBackgroundColor(style::Color::Rgb {
                            r: 254,
                            g: 246,
                            b: 120
                        }),
                        style::Print(msg),
                        style::ResetColor
                    )
                    .unwrap();
                }
                TextPart::Username { name, display_name } => {
                    let (r, g, b) = rgb::gen_rgb(&name);
                    queue!(
                        term,
                        style::SetForegroundColor(style::Color::Rgb { r, g, b }),
                        style::Print(display_name),
                        style::ResetColor
                    )
                    .unwrap();
                }
            }
        }
        queue!(term, style::Print("\n\r"),).unwrap();
    }

    let (r, g, b) = rgb::gen_rgb(&app.user.username);
    queue!(
        term,
        style::SetForegroundColor(style::Color::Rgb { r, g, b }),
        style::Print(format!("[{}]: ", app.user.username)),
        style::ResetColor,
        style::Print(app.input.to_str())
    )
    .unwrap();

    queue!(term, cursor::RestorePosition).unwrap();

    term.flush().unwrap();
}
