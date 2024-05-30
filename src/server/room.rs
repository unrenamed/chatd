use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use governor::clock::{Clock, Reference};
use governor::{Quota, RateLimiter};
use log::info;
use nonzero_ext::nonzero;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use crate::server::command::{Command, CommandParseError};
use crate::server::user;
use crate::utils;

use super::{
    app::{self, MessageChannel},
    history::MessageHistory,
    message::{self, Message},
    motd::Motd,
    state::UserState,
    terminal::TerminalHandle,
    user::User,
};

type UserId = usize;
type UserName = String;
type RateLimit = RateLimiter<
    governor::state::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
    governor::middleware::NoOpMiddleware,
>;

const INPUT_MAX_LEN: usize = 1024;
const MESSAGE_MAX_BURST: std::num::NonZeroU32 = nonzero!(10u32);
const MESSAGE_RATE_QUOTA: Quota = Quota::per_second(MESSAGE_MAX_BURST);

#[derive(Clone)]
pub struct ServerRoom {
    pub names: HashMap<UserId, UserName>,
    apps: HashMap<UserName, app::App>,
    ratelims: HashMap<UserId, Arc<Mutex<RateLimit>>>,
    history: MessageHistory,
    motd: Motd,
}

impl ServerRoom {
    pub fn new() -> Self {
        Self {
            names: HashMap::new(),
            apps: HashMap::new(),
            ratelims: HashMap::new(),
            history: MessageHistory::new(),
            motd: Default::default(),
        }
    }

    pub fn apps_mut(&mut self) -> &mut HashMap<UserName, app::App> {
        &mut self.apps
    }

    pub fn motd(&self) -> &String {
        &self.motd.get()
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        username: UserName,
        fingerpint: String,
        terminal: TerminalHandle,
        ssh_id: String,
    ) {
        let name = match self.is_room_member(&username) {
            true => User::gen_rand_name(),
            false => username,
        };

        let user = User::new(user_id, name.clone(), ssh_id, fingerpint);

        let app = app::App {
            user: user.clone(),
            state: UserState::new(),
            terminal: Arc::new(Mutex::new(terminal)),
            channel: MessageChannel::new(),
        };

        self.apps.insert(name.clone(), app.clone());
        self.names.insert(user_id, name.clone());
        self.ratelims.insert(
            user_id,
            Arc::new(Mutex::new(RateLimit::direct(MESSAGE_RATE_QUOTA))),
        );

        self.feed_history(&name).await;

        let message =
            message::Announce::new(user, format!("joined. (Connected: {})", self.apps.len()));
        self.send_message(message.into()).await;
    }

    pub async fn feed_history(&mut self, username: &UserName) {
        let app = self.find_app(username);
        for msg in self.history.iter() {
            if let Err(_) = app.send_message(msg.to_owned()).await {
                continue;
            }
        }
    }

    pub async fn leave(&mut self, user_id: &UserId) {
        let name = self.try_find_name(user_id);
        if let None = name {
            info!("No username found for {}", user_id);
            return;
        }

        let username = name.unwrap().clone();
        let user = self.find_app(&username).user.clone();

        let duration = humantime::format_duration(user.joined_duration());
        let message = message::Announce::new(user, format!("left: (After {})", duration));
        self.send_message(message.into()).await;

        self.apps.remove(&username);
        self.names.remove(user_id);
        self.ratelims.remove(user_id);
    }

    pub async fn send_message(&mut self, msg: Message) {
        match msg {
            Message::System(ref m) => {
                let app = self.find_app(&m.from.username);
                app.send_message(msg).await.unwrap();
            }
            Message::Command(ref m) => {
                let app = self.find_app(&m.from.username);
                app.send_message(msg).await.unwrap();
            }
            Message::Error(ref m) => {
                let app = self.find_app(&m.from.username);
                app.send_message(msg).await.unwrap();
            }
            Message::Public(_) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(_) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(_) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if app.user.quiet {
                        continue;
                    }
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.find_app(&m.from.username);
                from.send_message(msg.clone()).await.unwrap();

                let to = self.find_app(&m.to.username);
                to.send_message(msg).await.unwrap();
            }
        }
    }

    pub async fn handle_input(&mut self, user_id: &UserId, data: &[u8]) {
        let mut username = self.names.get(user_id).unwrap().clone();

        let mut decoder = Decoder::new();
        for keycode in decoder.write(data[0]) {
            match keycode {
                KeyCode::Enter => {
                    let app = self.find_app(&username);
                    let user = app.user.clone();

                    let ratelimit = self.ratelims.get(&user_id).unwrap();
                    let err = ratelimit.lock().await.check().err();

                    if let Some(not_until) = err {
                        let now = governor::clock::QuantaClock::default().now();
                        let next_allowed_nanos =
                            not_until.earliest_possible().duration_since(now).as_u64();
                        let next_allowed_secs = Duration::from_nanos(next_allowed_nanos).as_secs();
                        let next_allowed_truncated = Duration::new(next_allowed_secs, 0);

                        let body = format!(
                            "rate limit exceeded. Message dropped. Next allowed in {}",
                            humantime::format_duration(next_allowed_truncated)
                        );
                        let message = message::Error::new(user, body);
                        self.send_message(message.into()).await;
                        return;
                    }

                    let input_str = std::str::from_utf8(&app.state.input.bytes())
                        .expect("Input must be a valid UTF-8 string");

                    if input_str.trim().is_empty() {
                        return;
                    }

                    if input_str.len() > INPUT_MAX_LEN {
                        let message = message::Error::new(
                            user,
                            "Message dropped. Input is too long".to_string(),
                        );
                        self.send_message(message.into()).await;
                        return;
                    }

                    let cmd = Command::parse(&app.state.input.bytes());

                    match cmd {
                        Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                            let message = message::Public::new(user, app.state.input.to_str());
                            self.send_message(message.into()).await;

                            let app = self.find_app_mut(&username);
                            app.state.input.clear();

                            return;
                        }
                        Err(err) => {
                            let message =
                                message::Command::new(user.clone(), input_str.to_string());
                            self.send_message(message.into()).await;

                            let message = message::Error::new(user, format!("{}", err));
                            self.send_message(message.into()).await;

                            let app = self.find_app_mut(&username);
                            app.state.input.clear();

                            return;
                        }
                        Ok(_) => {
                            let message =
                                message::Command::new(user.clone(), input_str.to_string());
                            self.send_message(message.into()).await;
                        }
                    }

                    match cmd.unwrap() {
                        Command::Exit => {
                            let app = self.find_app(&username);
                            app.terminal.lock().await.close();
                            return;
                        }
                        Command::Away(reason) => {
                            let app = self.find_app_mut(&username);
                            app.user.go_away(reason.to_string());

                            let message = message::Emote::new(
                                app.user.clone(),
                                format!("has gone away: \"{}\"", reason),
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Back => {
                            let app = self.find_app_mut(&username);
                            if let user::UserStatus::Away {
                                reason: _,
                                since: _,
                            } = &app.user.status
                            {
                                app.user.return_active();
                                let message =
                                    message::Emote::new(app.user.clone(), "is back".to_string());
                                self.send_message(message.into()).await;
                            }
                        }
                        Command::Name(new_name) => 'label: {
                            let app = self.find_app_mut(&username);
                            let user = app.user.clone();

                            if user.username == new_name {
                                let message = message::Error::new(
                                    user,
                                    "New name is the same as the original".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            if let Some(_) = self.try_find_app(&new_name) {
                                let message = message::Error::new(
                                    user,
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

                            let app = self.find_app_mut(&username);
                            app.user.set_new_name(new_name.clone());

                            let app = app.clone();
                            self.apps.insert(new_name.clone(), app);
                            self.apps.remove(&old_name);
                            self.names.insert(user_id, new_name.clone());
                            username = new_name
                        }
                        Command::Msg(to, msg) => {
                            let app = self.find_app(&username);
                            let from = app.user.clone();

                            match self.try_find_app(&to).map(|a| &a.user) {
                                Some(to) if from.id.eq(&to.id) => {
                                    let message = message::Error::new(
                                        from.clone(),
                                        format!("You can't message yourself"),
                                    );
                                    self.send_message(message.into()).await;
                                }
                                Some(to) => {
                                    let status = to.status.clone();
                                    let name = to.username.clone();

                                    let message = message::Private::new(
                                        from.clone(),
                                        to.clone(),
                                        msg.to_string(),
                                    );
                                    self.send_message(message.into()).await;

                                    match status {
                                        user::UserStatus::Away { reason, since: _ } => {
                                            let message = message::System::new(
                                                from.clone(),
                                                format!(
                                                    "Sent PM to {}, but they're away now: {}",
                                                    name, reason
                                                ),
                                            );
                                            self.send_message(message.into()).await;
                                        }
                                        user::UserStatus::Active => {}
                                    }
                                }
                                None => {
                                    let message = message::Error::new(
                                        from.clone(),
                                        format!("User is not found"),
                                    );
                                    self.send_message(message.into()).await;
                                }
                            }

                            if let Some(to) = self.try_find_app_mut(&to).map(|a| &mut a.user) {
                                if !from.id.eq(&to.id) {
                                    to.set_reply_to(from.id);
                                }
                            }
                        }
                        Command::Reply(message_body) => 'label: {
                            let app = self.find_app(&username);
                            let from = app.user.clone();

                            if from.reply_to.is_none() {
                                let message = message::Error::new(
                                    from.clone(),
                                    "There is no message to reply to".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target_id = &from.reply_to.unwrap();
                            let target_name = self.names.get(&target_id);
                            if target_name.is_none() {
                                let message = message::Error::new(
                                    from.clone(),
                                    "User already left the room".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let app = self.find_app(target_name.unwrap());
                            let to = app.user.clone();
                            let message = message::Private::new(from, to, message_body);
                            self.send_message(message.into()).await;
                        }
                        Command::Users => {
                            let app = self.find_app(&username);
                            let user = app.user.clone();

                            let mut usernames = self.names.values().collect::<Vec<&String>>();
                            usernames.sort_by_key(|a| a.to_lowercase());

                            let colorized_names = usernames
                                .iter()
                                .map(|u| user.theme.style_username(u).to_string())
                                .collect::<Vec<String>>();

                            let body = format!(
                                "{} connected: {}",
                                self.names.len(),
                                colorized_names.join(", ")
                            );

                            let message = message::System::new(user, body);
                            self.send_message(message.into()).await;
                        }
                        Command::Whois(target_name) => {
                            let app = self.find_app(&username);
                            let user = app.user.clone();
                            let message = match self.try_find_app(&target_name).map(|app| &app.user)
                            {
                                Some(target) => {
                                    message::System::new(user, target.to_string()).into()
                                }
                                None => message::Error::new(user, "User is not found".to_string())
                                    .into(),
                            };
                            self.send_message(message).await;
                        }
                        Command::Slap(target_name) => 'label: {
                            let app = self.find_app(&username);
                            let user = app.user.clone();

                            if target_name.is_none() {
                                let message = message::Emote::new(
                                    user,
                                    "hits himself with a squishy banana.".to_string(),
                                );
                                self.send_message(message.into()).await;
                                break 'label;
                            }

                            let target_name = target_name.unwrap();
                            let target = self.try_find_app_mut(&target_name).map(|app| &app.user);

                            let message = if let Some(t) = target {
                                message::Emote::new(
                                    user,
                                    format!("hits {} with a squishy banana.", t.username),
                                )
                                .into()
                            } else {
                                message::Error::new(
                                    user,
                                    "That slippin' monkey is not in the room".to_string(),
                                )
                                .into()
                            };
                            self.send_message(message).await;
                        }
                        Command::Shrug => {
                            let app = self.find_app(&username);
                            let user = app.user.clone();
                            let message = message::Emote::new(user, "¯\\_(ツ)_/¯".to_string());
                            self.send_message(message.into()).await;
                        }
                        Command::Me(action) => {
                            let app = self.find_app(&username);
                            let user = app.user.clone();
                            let message = message::Emote::new(
                                user,
                                match action {
                                    Some(s) => format!("{}", s),
                                    None => format!("is at a loss for words."),
                                },
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Help => {
                            let app = self.find_app(&username);
                            let user = app.user.clone();
                            let message = message::System::new(
                                user,
                                format!(
                                    "Available commands: {}{}",
                                    utils::NEWLINE,
                                    Command::to_string()
                                ),
                            );
                            self.send_message(message.into()).await;
                        }
                        Command::Quiet => {
                            let app = self.find_app_mut(&username);
                            app.user.switch_quiet_mode();
                            let message = message::System::new(
                                app.user.clone(),
                                match app.user.quiet {
                                    true => "Quiet mode is toggled ON",
                                    false => "Quiet mode is toggled OFF",
                                }
                                .to_string(),
                            );
                            self.send_message(message.into()).await;
                        }
                    }

                    let app = self.find_app_mut(&username);
                    app.state.input.clear();
                }
                KeyCode::Backspace => {
                    let app = self.find_app_mut(&username);
                    app.state.input.pop();
                }
                KeyCode::CtrlW => {
                    let app = self.find_app_mut(&username);
                    app.state.input.remove_last_word();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    let app = self.find_app_mut(&username);
                    app.state.input.extend(data);
                }
                _ => {}
            }
        }
    }

    fn is_room_member(&self, username: &UserName) -> bool {
        self.apps.contains_key(username)
    }

    fn find_app(&self, username: &UserName) -> &app::App {
        self.apps
            .get(username)
            .expect("User MUST have an app within a server room")
    }

    fn find_app_mut(&mut self, username: &UserName) -> &mut app::App {
        self.apps
            .get_mut(username)
            .expect("User MUST have an app within a server room")
    }

    fn try_find_app(&self, username: &UserName) -> Option<&app::App> {
        self.apps.get(username)
    }

    fn try_find_app_mut(&mut self, username: &UserName) -> Option<&mut app::App> {
        self.apps.get_mut(username)
    }

    fn try_find_name(&self, user_id: &UserId) -> Option<&UserName> {
        self.names.get(user_id)
    }
}
