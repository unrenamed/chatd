use ratatui::{backend::CrosstermBackend, Terminal};
use russh::{server::Session, ChannelId};
use std::collections::HashMap;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::{
    chat::{app::ChatApp, user::UserStatus},
    server::{GoAwayEvent, LeftEvent, ReturnBackEvent, SendMessageEvent},
};

use super::{command::Command, terminal::TerminalHandle, ClientEvent};

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

pub struct InputHandler<'a> {
    client_id: &'a usize,
    clients: &'a Mutex<HashMap<usize, (SshTerminal, ChatApp)>>,
    events: &'a Mutex<Vec<ClientEvent>>,
}

impl<'a> InputHandler<'a> {
    pub fn new(
        client_id: &'a usize,
        clients: &'a Mutex<HashMap<usize, (SshTerminal, ChatApp)>>,
        events: &'a Mutex<Vec<ClientEvent>>,
    ) -> Self {
        InputHandler {
            client_id,
            clients,
            events,
        }
    }

    pub async fn handle_data(
        &self,
        channel: ChannelId,
        session: &mut Session,
        data: &[u8],
    ) -> Result<(), String> {
        let mut clients = self.clients.lock().await;
        let mut events = self.events.lock().await;
        let (_, app) = clients.get_mut(self.client_id).unwrap();
        let username = app.user.username.clone();

        let mut decoder = Decoder::new();
        for keycode in decoder.write(data[0]) {
            print![
                "code={:?} bytes={:?} printable={:?}\r\n",
                keycode,
                keycode.bytes(),
                keycode.printable()
            ];

            match keycode {
                KeyCode::Enter => {
                    let (cmd, args) = split_at_first_space(&app.input.bytes);

                    if !Command::is_command(cmd) {
                        events.push(ClientEvent::SendMessage(SendMessageEvent {
                            username: app.user.username.clone(),
                            message: app.input.to_str(),
                        }));
                    } else if let Ok(cmd) = Command::parse(&cmd) {
                        match cmd {
                            Command::Exit => {
                                events.push(ClientEvent::Left(LeftEvent {
                                    username: username.clone(),
                                    session_duration: app.session.secs_since_start(),
                                }));
                                clients.remove(self.client_id);
                                session.close(channel);
                                return Ok(());
                            }
                            Command::Away => {
                                let reason = match std::str::from_utf8(args) {
                                    Ok(v) => String::from(v),
                                    Err(_) => String::new(),
                                };
                                app.user.go_away(reason.clone());
                                events.push(ClientEvent::GoAway(GoAwayEvent {
                                    username: username.clone(),
                                    reason,
                                }));
                            }
                            Command::Back => match &app.user.status {
                                UserStatus::Active => {}
                                UserStatus::Away { reason: _ } => {
                                    app.user.return_active();
                                    events.push(ClientEvent::ReturnBack(ReturnBackEvent {
                                        username: username.clone(),
                                    }));
                                }
                            },
                        }
                    }

                    app.input.clear();
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Char(_) | KeyCode::Tab | KeyCode::Space => {
                    app.input.extend(data);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn split_at_first_space(bytes: &[u8]) -> (&[u8], &[u8]) {
    // Find the position of the first space
    if let Some(pos) = bytes.iter().position(|&b| b == b' ') {
        // Split the slice at the position of the first space
        let (first, rest) = bytes.split_at(pos);
        // Skip the space in the rest slice
        (first, &rest[1..])
    } else {
        // If there's no space, return the original slice
        (bytes, &[])
    }
}
