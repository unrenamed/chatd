use log::info;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::chat::app::ChatApp;
use crate::chat::user::{self, User};
use crate::server::message;

use super::{command::Command, message::Message};

pub enum InputCallbackAction {
    NoAction,
    CloseClientSession,
}

pub struct InputHandler<'a> {
    app: &'a mut ChatApp,
    users: &'a Vec<User>,
    messages: &'a Mutex<Vec<Message>>,
}

impl<'a> InputHandler<'a> {
    pub fn new(
        app: &'a mut ChatApp,
        users: &'a Vec<User>,
        messages: &'a Mutex<Vec<Message>>,
    ) -> Self {
        InputHandler {
            app,
            users,
            messages,
        }
    }

    pub async fn handle_data(&mut self, data: &[u8]) -> InputCallbackAction {
        let mut messages = self.messages.lock().await;

        let mut decoder = Decoder::new();
        for keycode in decoder.write(data[0]) {
            info!(
                "code={:?} bytes={:?} printable={:?}",
                keycode,
                keycode.bytes(),
                keycode.printable()
            );

            match keycode {
                KeyCode::Enter => {
                    let (cmd, args) = split_at_first_space(&self.app.input.bytes);
                    if !Command::is_command(cmd) {
                        messages.push(Message::Public(message::PublicMessage {
                            from: self.app.user.clone(),
                            body: self.app.input.to_str(),
                        }));
                        self.app.input.clear();
                        return InputCallbackAction::NoAction;
                    }

                    let args: &str = std::str::from_utf8(args)
                        .expect("Command arguments to be a valid UTF-8 string");
                    let command: &str =
                        std::str::from_utf8(cmd).expect("Command to be a valid UTF-8 string");

                    messages.push(Message::Command(message::CommandMessage {
                        from: self.app.user.clone(),
                        cmd: command.to_string(),
                        args: args.to_string(),
                    }));

                    let cmd = match Command::parse(cmd) {
                        Ok(c) => c,
                        Err(err) => {
                            messages.push(Message::System(message::SystemMessage {
                                from: self.app.user.clone(),
                                body: format!("Error: {}", err),
                            }));
                            self.app.input.clear();
                            return InputCallbackAction::NoAction;
                        }
                    };

                    match cmd {
                        Command::Exit => {
                            messages.push(Message::Announce(message::AnnounceMessage {
                                from: self.app.user.clone(),
                                body: format!(
                                    "left: (After {})",
                                    humantime::format_duration(self.app.user.joined_duration())
                                ),
                            }));
                            return InputCallbackAction::CloseClientSession;
                        }
                        Command::Away => {
                            self.app.user.go_away(args.to_string());
                            messages.push(Message::Emote(message::EmoteMessage {
                                from: self.app.user.clone(),
                                body: format!("has gone away: \"{}\"", args),
                            }));
                        }
                        Command::Back => match &self.app.user.status {
                            user::UserStatus::Active => {}
                            user::UserStatus::Away {
                                reason: _,
                                since: _,
                            } => {
                                self.app.user.return_active();
                                messages.push(Message::Emote(message::EmoteMessage {
                                    from: self.app.user.clone(),
                                    body: format!("is back."),
                                }));
                            }
                        },
                        Command::ChangeName => {
                            let parts: Vec<&str> = args.split_whitespace().collect();
                            let new_username = parts[0].to_string();

                            messages.push(Message::Announce(message::AnnounceMessage {
                                from: self.app.user.clone(),
                                body: format!("user is now known as {}.", new_username),
                            }));

                            self.app.user.set_new_name(new_username);
                        }
                        Command::SendPrivateMessage => {
                            let parts: Vec<&str> = args.split_whitespace().collect();
                            let user = parts[0].to_string();
                            let body = parts[1..].join(" ");

                            let sender = self.app.user.clone();
                            let target = self.users.iter().find(|u| u.username.eq(&user));

                            match target {
                                Some(u) if sender.id.eq(&u.id) => {
                                    messages.push(Message::System(message::SystemMessage {
                                        from: sender,
                                        body: format!("Error: You can't message yourself"),
                                    }))
                                }
                                Some(u) => {
                                    messages.push(Message::Private(message::PrivateMessage {
                                        from: sender,
                                        to: u.clone(),
                                        body,
                                    }))
                                }
                                None => messages.push(Message::System(message::SystemMessage {
                                    from: sender,
                                    body: format!("Error: User is not found"),
                                })),
                            }
                        }
                        Command::GetAllUsers => {
                            let from = self.app.user.clone();

                            let mut usernames: Vec<String> =
                                self.users.iter().map(|u| (*u).username.clone()).collect();
                            usernames.sort_by_key(|a| a.to_lowercase());

                            let colorized_names = usernames
                                .iter()
                                .map(|u| self.app.theme.style_username(u).to_string())
                                .collect::<Vec<String>>();

                            let body = format!(
                                "{} connected: {}",
                                self.users.len(),
                                colorized_names.join(", ")
                            );

                            messages.push(Message::System(message::SystemMessage { from, body }));
                        }
                        Command::Whois => {
                            let parts: Vec<&str> = args.split_whitespace().collect();
                            let target = parts[0].to_string();
                            let user = self.users.iter().find(|u| u.username.eq(&target));
                            if let Some(u) = user {
                                messages.push(Message::System(message::SystemMessage {
                                    from: self.app.user.clone(),
                                    body: u.to_string(),
                                }));
                            }
                        }
                    }

                    self.app.input.clear();
                }
                KeyCode::Backspace => {
                    self.app.input.pop();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    self.app.input.extend(data);
                }
                _ => {}
            }
        }

        InputCallbackAction::NoAction
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
