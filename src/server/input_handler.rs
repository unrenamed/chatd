use log::info;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::chat::app::ChatApp;
use crate::chat::user::{self, User};
use crate::server::command::CommandParseError;
use crate::server::message;
use crate::utils;

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
                    let mut input_iter = std::str::from_utf8(&self.app.input.bytes)
                        .expect("Input must be a valid UTF-8 string")
                        .split_whitespace()
                        .into_iter();

                    let command_msg = message::CommandMessage {
                        from: self.app.user.clone(),
                        cmd: input_iter.nth(0).unwrap().to_string(),
                        args: input_iter.collect::<Vec<_>>().join(" "),
                    };

                    let cmd = Command::parse(&self.app.input.bytes);

                    match cmd {
                        Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                            messages.push(Message::Public(message::PublicMessage {
                                from: self.app.user.clone(),
                                body: self.app.input.to_str(),
                            }));
                            self.app.input.clear();
                            return InputCallbackAction::NoAction;
                        }
                        Err(err) => {
                            messages.push(Message::Command(command_msg));
                            messages.push(Message::System(message::SystemMessage {
                                from: self.app.user.clone(),
                                body: format!("Error: {}", err),
                            }));
                            self.app.input.clear();
                            return InputCallbackAction::NoAction;
                        }
                        Ok(_) => messages.push(Message::Command(command_msg)),
                    }

                    match cmd.unwrap() {
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
                        Command::Away(reason) => {
                            self.app.user.go_away(reason.clone());
                            messages.push(Message::Emote(message::EmoteMessage {
                                from: self.app.user.clone(),
                                body: format!("has gone away: \"{}\"", reason),
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
                        Command::Name(new_name) => {
                            messages.push(Message::Announce(message::AnnounceMessage {
                                from: self.app.user.clone(),
                                body: format!("user is now known as {}.", new_name),
                            }));

                            self.app.user.set_new_name(new_name.to_string());
                        }
                        Command::Msg(user, body) => {
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
                        Command::Users => {
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
                        Command::Whois(target) => {
                            let user = self.users.iter().find(|u| u.username.eq(&target));
                            let body = match user {
                                Some(u) => u.to_string(),
                                None => format!("Error: User is not found"),
                            };
                            messages.push(Message::System(message::SystemMessage {
                                from: self.app.user.clone(),
                                body,
                            }));
                        }
                        Command::Slap(target) => {
                            if target.is_none() {
                                messages.push(Message::Emote(message::EmoteMessage {
                                    from: self.app.user.clone(),
                                    body: format!("hits himself with a squishy banana."),
                                }));
                                return InputCallbackAction::NoAction;
                            }

                            let target = target.unwrap();
                            let user = self.users.iter().find(|u| u.username.eq(&target));

                            if let Some(u) = user {
                                messages.push(Message::Emote(message::EmoteMessage {
                                    from: self.app.user.clone(),
                                    body: format!("hits {} with a squishy banana.", u.username),
                                }));
                            } else {
                                messages.push(Message::System(message::SystemMessage {
                                    from: self.app.user.clone(),
                                    body: format!("Error: That slippin' monkey is not in the room"),
                                }))
                            }
                        }
                        Command::Shrug => {
                            messages.push(Message::Emote(message::EmoteMessage {
                                from: self.app.user.clone(),
                                body: "¯\\_(ツ)_/¯".to_string(),
                            }));
                        }
                        Command::Me(action) => {
                            messages.push(Message::Emote(message::EmoteMessage {
                                from: self.app.user.clone(),
                                body: match action {
                                    Some(s) => format!("{}", s),
                                    None => format!("is at a loss for words."),
                                },
                            }));
                        }
                        Command::Help => messages.push(Message::System(message::SystemMessage {
                            from: self.app.user.clone(),
                            body: format!(
                                "Available commands: {}{}",
                                utils::NEWLINE,
                                Command::to_string()
                            ),
                        })),
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
