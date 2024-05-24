use std::{collections::HashMap, sync::Arc};

use log::info;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::{
    server::{
        command::{Command, CommandParseError},
        user,
    },
    utils,
};

use super::{
    app::{self, MessageChannel},
    message::{self, Message},
    motd::Motd,
    state::UserState,
    terminal::TerminalHandle,
    user::User,
};

type UserId = usize;

type UserName = String;

#[derive(Clone)]
pub struct ServerRoom {
    names: HashMap<UserId, UserName>,
    members: HashMap<UserName, app::App>,
    motd: Motd,
}

impl ServerRoom {
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            members: HashMap::new(),
            motd: Default::default(),
        }
    }

    pub fn members_mut(&mut self) -> &mut HashMap<UserName, app::App> {
        &mut self.members
    }

    pub fn motd(&self) -> &String {
        &self.motd.get()
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        user_name: UserName,
        fingerpint: String,
        terminal: TerminalHandle,
        ssh_id: &[u8],
    ) {
        let name = match self.is_member(&user_name).await {
            true => User::gen_rand_name(),
            false => user_name,
        };

        let user = User::new(
            user_id,
            name.clone(),
            String::from_utf8_lossy(ssh_id).to_string(),
            fingerpint,
        );

        let member = app::App {
            user: user.clone(),
            state: UserState::new(),
            terminal: Arc::new(Mutex::new(terminal)),
            channel: MessageChannel::new(),
        };

        self.members.insert(name.clone(), member);
        self.names.insert(user_id, name);

        let join_msg_body = format!("joined. (Connected: {})", self.members.len());
        self.send_message(message::Announce::new(user, join_msg_body).into())
            .await;
    }

    pub async fn send_message(&mut self, msg: Message) {
        match msg {
            Message::System(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Command(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Error(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg).await.unwrap();
            }
            Message::Public(_) => {
                for (_, member) in self.members.iter() {
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(_) => {
                for (_, member) in self.members.iter() {
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(_) => {
                for (_, member) in self.members.iter() {
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.members.get(&m.from.username).unwrap();
                from.send_message(msg.clone()).await.unwrap();

                let to = self.members.get(&m.to.username).unwrap();
                to.send_message(msg).await.unwrap();
            }
        }
    }

    pub async fn handle_input(&mut self, user_id: &UserId, data: &[u8]) {
        let mut user_name = self.names.get(user_id).unwrap().clone();

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
                    let cmd = {
                        let member = self.members.get_mut(&user_name).unwrap();
                        Command::parse(&member.state.input.bytes())
                    };

                    match cmd {
                        Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                            let message = {
                                let member = self.members.get_mut(&user_name).unwrap();
                                message::Public::new(
                                    member.user.clone(),
                                    member.state.input.to_str(),
                                )
                                .into()
                            };
                            self.send_message(message).await;

                            let member = self.members.get_mut(&user_name).unwrap();
                            member.state.input.clear();

                            return;
                        }
                        Err(err) => {
                            let message = {
                                let member = self.members.get_mut(&user_name).unwrap();
                                let mut input_iter =
                                    std::str::from_utf8(&member.state.input.bytes())
                                        .expect("Input must be a valid UTF-8 string")
                                        .split_whitespace()
                                        .into_iter();
                                message::Command::new(
                                    member.user.clone(),
                                    input_iter.nth(0).unwrap().to_string(),
                                    input_iter.collect::<Vec<_>>().join(" "),
                                )
                                .into()
                            };
                            self.send_message(message).await;

                            let message = {
                                let member = self.members.get_mut(&user_name).unwrap();
                                message::Error::new(member.user.clone(), format!("{}", err)).into()
                            };
                            self.send_message(message).await;

                            let member = self.members.get_mut(&user_name).unwrap();
                            member.state.input.clear();

                            return;
                        }
                        Ok(_) => {
                            let message = {
                                let member = self.members.get_mut(&user_name).unwrap();
                                let mut input_iter =
                                    std::str::from_utf8(&member.state.input.bytes())
                                        .expect("Input must be a valid UTF-8 string")
                                        .split_whitespace()
                                        .into_iter();
                                message::Command::new(
                                    member.user.clone(),
                                    input_iter.nth(0).unwrap().to_string(),
                                    input_iter.collect::<Vec<_>>().join(" "),
                                )
                                .into()
                            };
                            self.send_message(message).await;
                        }
                    }

                    match cmd.unwrap() {
                        Command::Exit => {
                            let from = self.members.get_mut(&user_name).unwrap().clone();
                            let user_id = from.user.id;

                            let duration = humantime::format_duration(from.user.joined_duration());
                            let message = message::Announce::new(
                                from.user.clone(),
                                format!("left: (After {})", duration),
                            );
                            self.send_message(message.into()).await;
                            self.members.remove(&user_name);
                            self.names.remove(&user_id);
                            return;
                        }
                        Command::Away(reason) => {
                            let from = self.members.get_mut(&user_name).unwrap();
                            from.user.go_away(reason.to_string());

                            let message = message::Emote::new(
                                from.user.clone(),
                                format!("has gone away: \"{}\"", reason),
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Back => {
                            let from = self.members.get_mut(&user_name).unwrap();
                            match &from.user.status {
                                user::UserStatus::Active => {}
                                user::UserStatus::Away {
                                    reason: _,
                                    since: _,
                                } => {
                                    from.user.return_active();
                                    let message = message::Emote::new(
                                        from.user.clone(),
                                        "is back".to_string(),
                                    );
                                    self.send_message(message.into()).await;
                                }
                            }
                        }
                        Command::Name(new_name) => 'label: {
                            let from = self.members.get_mut(&user_name).unwrap();
                            let user = from.user.clone();

                            if user.username == new_name {
                                let message = message::Error::new(
                                    user.clone(),
                                    "new name is the same as the original".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            if let Some(_) = self.members.get(&new_name) {
                                let message = message::Error::new(
                                    user.clone(),
                                    format!("\"{}\" name is already taken", new_name),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let message = message::Announce::new(
                                user.clone(),
                                format!("user is now known as {}.", new_name),
                            );
                            self.send_message(message.into()).await;

                            let new_name = new_name.to_string();
                            let old_name = user.username;
                            let user_id = user.id;

                            let from = self.members.get_mut(&user_name).unwrap();
                            from.user.set_new_name(new_name.clone());

                            let app = from.clone();
                            self.members.insert(new_name.clone(), app);
                            self.members.remove(&old_name);
                            self.names.insert(user_id, new_name.clone());
                            user_name = new_name
                        }
                        Command::Msg(to, msg) => {
                            let from = self.members.get_mut(&user_name).unwrap().clone();

                            match self.members.get(&to) {
                                Some(member) if from.user.id.eq(&member.user.id) => {
                                    self.send_message(
                                        message::Error::new(
                                            from.user.clone(),
                                            format!("you can't message yourself"),
                                        )
                                        .into(),
                                    )
                                    .await;
                                }
                                Some(member) => {
                                    let target_status = member.user.status.clone();
                                    let target_name = member.user.username.clone();

                                    self.send_message(
                                        message::Private::new(
                                            from.user.clone(),
                                            member.user.clone(),
                                            msg.to_string(),
                                        )
                                        .into(),
                                    )
                                    .await;

                                    match target_status {
                                        user::UserStatus::Away { reason, since: _ } => {
                                            self.send_message(
                                                message::System::new(
                                                    from.user.clone(),
                                                    format!(
                                                        "Sent PM to {}, but they're away now: {}",
                                                        target_name, reason
                                                    ),
                                                )
                                                .into(),
                                            )
                                            .await;
                                        }
                                        user::UserStatus::Active => {}
                                    }
                                }
                                None => {
                                    self.send_message(
                                        message::Error::new(
                                            from.user.clone(),
                                            format!("user is not found"),
                                        )
                                        .into(),
                                    )
                                    .await;
                                }
                            }

                            if let Some(to) = self.members.get_mut(&to) {
                                if !from.user.id.eq(&to.user.id) {
                                    to.user.set_reply_to(from.user.id);
                                }
                            }
                        }
                        Command::Reply(body) => 'label: {
                            let from = self.members.get(&user_name).unwrap().clone();
                            if from.user.reply_to.is_none() {
                                let message = message::Error::new(
                                    from.user.clone(),
                                    "there is no message to reply to".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target_id = &from.user.reply_to.unwrap();
                            let target_name = self.names.get(&target_id);
                            if target_name.is_none() {
                                let message = message::Error::new(
                                    from.user.clone(),
                                    "user already left the room".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let to = self.members.get(target_name.unwrap()).unwrap().clone();
                            let message =
                                message::Private::new(from.user.clone(), to.user.clone(), body);
                            self.send_message(message.into()).await;
                        }
                        Command::Users => {
                            let from = self.members.get(&user_name).unwrap().clone();
                            let mut usernames = self.names.values().collect::<Vec<&String>>();
                            usernames.sort_by_key(|a| a.to_lowercase());

                            let colorized_names = usernames
                                .iter()
                                .map(|u| from.user.theme.style_username(u).to_string())
                                .collect::<Vec<String>>();

                            let body = format!(
                                "{} connected: {}",
                                self.names.len(),
                                colorized_names.join(", ")
                            );

                            self.send_message(message::System::new(from.user.clone(), body).into())
                                .await;
                        }
                        Command::Whois(target) => {
                            let from = self.members.get(&user_name).unwrap().clone();
                            let message = match self.members.get(&target) {
                                Some(member) => {
                                    message::System::new(from.user.clone(), member.user.to_string())
                                        .into()
                                }
                                None => message::Error::new(
                                    from.user.clone(),
                                    "user is not found".to_string(),
                                )
                                .into(),
                            };
                            self.send_message(message).await;
                        }
                        Command::Slap(target) => 'label: {
                            let from = self.members.get_mut(&user_name).unwrap().clone();
                            if target.is_none() {
                                let message = message::Emote::new(
                                    from.user.clone(),
                                    "hits himself with a squishy banana.".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target = target.unwrap();
                            let target = self.members.get_mut(&target).map(|app| &mut app.user);

                            let message = if let Some(u) = target {
                                message::Emote::new(
                                    from.user.clone(),
                                    format!("hits {} with a squishy banana.", u.username),
                                )
                                .into()
                            } else {
                                message::Error::new(
                                    from.user.clone(),
                                    "that slippin' monkey is not in the room".to_string(),
                                )
                                .into()
                            };
                            self.send_message(message).await;
                        }
                        Command::Shrug => {
                            let from = self.members.get_mut(&user_name).unwrap().clone();
                            self.send_message(
                                message::Emote::new(from.user.clone(), "¯\\_(ツ)_/¯".to_string())
                                    .into(),
                            )
                            .await;
                        }
                        Command::Me(action) => {
                            let from = self.members.get_mut(&user_name).unwrap().clone();
                            self.send_message(
                                message::Emote::new(
                                    from.user.clone(),
                                    match action {
                                        Some(s) => format!("{}", s),
                                        None => format!("is at a loss for words."),
                                    },
                                )
                                .into(),
                            )
                            .await;
                        }
                        Command::Help => {
                            let from = self.members.get_mut(&user_name).unwrap().clone();
                            self.send_message(
                                message::System::new(
                                    from.user.clone(),
                                    format!(
                                        "Available commands: {}{}",
                                        utils::NEWLINE,
                                        Command::to_string()
                                    ),
                                )
                                .into(),
                            )
                            .await;
                        }
                    }

                    let member = self.members.get_mut(&user_name).unwrap();
                    member.state.input.clear();
                }
                KeyCode::Backspace => {
                    let member = self.members.get_mut(&user_name).unwrap();
                    member.state.input.pop();
                }
                KeyCode::CtrlW => {
                    let member = self.members.get_mut(&user_name).unwrap();
                    member.state.input.remove_last_word();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    let member = self.members.get_mut(&user_name).unwrap();
                    member.state.input.extend(data);
                }
                _ => {}
            }
        }
    }

    async fn is_member(&self, username: &UserName) -> bool {
        self.members.contains_key(username)
    }
}
