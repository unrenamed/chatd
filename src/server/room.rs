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

use super::theme::Theme;
use super::user::TimestampMode;
use super::{
    app::{self, MessageChannel},
    message::{self, Message},
    message_history::MessageHistory,
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
            Message::Public(ref m) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if app.user.ignored.contains(&m.from.id) {
                        continue;
                    } else if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(ref m) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if app.user.ignored.contains(&m.from.id) {
                        continue;
                    } else if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(ref m) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if app.user.quiet {
                        continue;
                    } else if app.user.ignored.contains(&m.from.id) {
                        continue;
                    } else if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.find_app(&m.from.username);
                from.send_message(msg.clone()).await.unwrap();

                let to = self.find_app(&m.to.username);
                if !to.user.ignored.contains(&m.from.id) {
                    to.send_message(msg).await.unwrap();
                }
            }
        }
    }

    pub async fn handle_input(&mut self, user_id: &UserId, data: &[u8]) {
        let mut username = self.names.get(user_id).unwrap().clone();

        let mut decoder = Decoder::new();
        for byte in data {
            for keycode in decoder.write(*byte) {
                match keycode {
                    KeyCode::Backspace => {
                        let app = self.find_app_mut(&username);
                        app.state.input.remove_before_cursor();
                    }
                    KeyCode::CtrlA => {
                        let app = self.find_app_mut(&username);
                        app.state.input.move_cursor_start();
                    }
                    KeyCode::CtrlE => {
                        let app = self.find_app_mut(&username);
                        app.state.input.move_cursor_end();
                    }
                    KeyCode::CtrlW => {
                        let app = self.find_app_mut(&username);
                        app.state.input.remove_last_word_before_cursor();
                    }
                    KeyCode::CtrlK => {
                        let app = self.find_app_mut(&username);
                        app.state.input.remove_after_cursor();
                    }
                    KeyCode::CtrlU => {
                        let app = self.find_app_mut(&username);
                        app.state.input.clear();
                    }
                    KeyCode::CtrlY => {
                        let app = self.find_app_mut(&username);
                        app.state.input.restore();
                    }
                    KeyCode::ArrowLeft | KeyCode::CtrlB => {
                        let app = self.find_app_mut(&username);
                        app.state.input.move_cursor_prev();
                    }
                    KeyCode::ArrowRight | KeyCode::CtrlF => {
                        let app = self.find_app_mut(&username);
                        app.state.input.move_cursor_next();
                    }
                    KeyCode::Char(_) | KeyCode::Space => {
                        let app = self.find_app_mut(&username);
                        app.state
                            .input
                            .insert_before_cursor(keycode.bytes().as_slice());
                    }
                    KeyCode::ArrowUp => {
                        let app = self.find_app_mut(&username);
                        app.state.input.set_history_prev();
                    }
                    KeyCode::ArrowDown => {
                        let app = self.find_app_mut(&username);
                        app.state.input.set_history_next();
                    }
                    KeyCode::Enter => {
                        let app = self.find_app(&username);
                        let user = app.user.clone();

                        let ratelimit = self.ratelims.get(&user_id).unwrap();
                        let err = ratelimit.lock().await.check().err();

                        if let Some(not_until) = err {
                            let now = governor::clock::QuantaClock::default().now();
                            let next_allowed_nanos =
                                not_until.earliest_possible().duration_since(now).as_u64();
                            let next_allowed_secs =
                                Duration::from_nanos(next_allowed_nanos).as_secs();
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
                                "message dropped. Input is too long".to_string(),
                            );
                            self.send_message(message.into()).await;
                            return;
                        }

                        let cmd = Command::parse(&app.state.input.bytes());

                        match cmd {
                            Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                                let message =
                                    message::Public::new(user, app.state.input.to_string());
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
                                app.state.input.push_to_history();
                                app.state.input.clear();

                                return;
                            }
                            Ok(_) => {
                                let message =
                                    message::Command::new(user.clone(), input_str.to_string());
                                self.send_message(message.into()).await;

                                let app = self.find_app_mut(&username);
                                app.state.input.push_to_history();
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
                                    let message = message::Emote::new(
                                        app.user.clone(),
                                        "is back".to_string(),
                                    );
                                    self.send_message(message.into()).await;
                                }
                            }
                            Command::Name(new_name) => 'label: {
                                let app = self.find_app_mut(&username);
                                let user = app.user.clone();

                                if user.username == new_name {
                                    let message = message::Error::new(
                                        user,
                                        "new name is the same as the original".to_string(),
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
                            Command::Msg(to, msg) => 'label: {
                                let app = self.find_app(&username);
                                let from = app.user.clone();

                                match self.try_find_app(&to).map(|a| &a.user) {
                                    Some(to) if from.id.eq(&to.id) => {
                                        let message = message::Error::new(
                                            from.clone(),
                                            format!("you can't message yourself"),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
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
                                            format!("user is not found"),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                }

                                if let Some(to) = self.try_find_app_mut(&to).map(|a| &mut a.user) {
                                    to.set_reply_to(from.id);
                                }
                            }
                            Command::Reply(message_body) => 'label: {
                                let app = self.find_app(&username);
                                let from = app.user.clone();

                                if from.reply_to.is_none() {
                                    let message = message::Error::new(
                                        from.clone(),
                                        "no message to reply to".to_string(),
                                    );
                                    self.send_message(message.into()).await;
                                    break 'label;
                                }

                                let target_id = &from.reply_to.unwrap();
                                let target_name = self.names.get(&target_id);
                                if target_name.is_none() {
                                    let message = message::Error::new(
                                        from.clone(),
                                        "user already left the room".to_string(),
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
                                let message = match self
                                    .try_find_app(&target_name)
                                    .map(|app| &app.user)
                                {
                                    Some(target) => {
                                        message::System::new(user, target.to_string()).into()
                                    }
                                    None => message::Error::new(user, "user not found".to_string())
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
                                let target =
                                    self.try_find_app_mut(&target_name).map(|app| &app.user);

                                let message = if let Some(t) = target {
                                    message::Emote::new(
                                        user,
                                        format!("hits {} with a squishy banana.", t.username),
                                    )
                                    .into()
                                } else {
                                    message::Error::new(
                                        user,
                                        "that slippin' monkey not in the room".to_string(),
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
                            Command::Timestamp(mode) => {
                                let app = self.find_app_mut(&username);
                                app.user.set_timestamp_mode(mode);
                                let message = message::System::new(
                                    app.user.clone(),
                                    match app.user.timestamp_mode {
                                        TimestampMode::Time | TimestampMode::DateTime => {
                                            "Timestamp is toggled ON, timezone is UTC"
                                        }
                                        TimestampMode::Off => "Timestamp is toggled OFF",
                                    }
                                    .to_string(),
                                );
                                self.send_message(message.into()).await;
                            }
                            Command::Theme(theme) => {
                                let app = self.find_app_mut(&username);
                                let user = app.user.clone();
                                let theme_clone = theme.clone();
                                app.user.theme = theme.into();
                                let message = message::System::new(
                                    user,
                                    format!("Set theme: {}", theme_clone),
                                );
                                self.send_message(message.into()).await;
                            }
                            Command::Themes => {
                                let app = self.find_app(&username);
                                let user = app.user.clone();
                                let message = message::System::new(
                                    user,
                                    format!("Supported themes: {}", Theme::all().join(", ")),
                                );
                                self.send_message(message.into()).await;
                            }
                            Command::Ignore(target) => 'label: {
                                let app = self.find_app(&username);
                                let user = app.user.clone();

                                if target.is_none() {
                                    let ignored_usernames: Vec<String> = user
                                        .ignored
                                        .iter()
                                        .filter_map(|id| self.names.get(id))
                                        .map(|name| user.theme.style_username(name).to_string())
                                        .collect();

                                    let message_text = match ignored_usernames.is_empty() {
                                        true => "0 users ignored".to_string(),
                                        false => format!(
                                            "{} users ignored: {}",
                                            ignored_usernames.len(),
                                            ignored_usernames.join(", ")
                                        ),
                                    };

                                    let message = message::System::new(user, message_text);
                                    self.send_message(message.into()).await;
                                    break 'label;
                                }

                                let target_username = target.unwrap();
                                match self
                                    .try_find_app(&target_username)
                                    .map(|a| a.user.id.clone())
                                {
                                    Some(target_id) if target_id == user.id => {
                                        let message = message::Error::new(
                                            user.clone(),
                                            "you can't ignore yourself".to_string(),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                    Some(target_id) if user.ignored.contains(&target_id) => {
                                        let message = message::System::new(
                                            user.clone(),
                                            format!("user already in the ignored list"),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                    None => {
                                        let message = message::Error::new(
                                            user.clone(),
                                            "user not found".to_string(),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                    Some(target_id) => {
                                        self.find_app_mut(&username).user.ignored.insert(target_id);
                                        let message = message::System::new(
                                            user,
                                            format!("Ignoring: {}", target_username),
                                        );
                                        self.send_message(message.into()).await;
                                    }
                                }
                            }
                            Command::Unignore(target_username) => 'label: {
                                let app = self.find_app(&username);
                                let user = app.user.clone();

                                match self
                                    .try_find_app(&target_username)
                                    .map(|a| a.user.id.clone())
                                {
                                    None => {
                                        let message = message::Error::new(
                                            user.clone(),
                                            "user not found".to_string(),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                    Some(target_id) if !user.ignored.contains(&target_id) => {
                                        let message = message::Error::new(
                                            user.clone(),
                                            "user not in the ignored list yet".to_string(),
                                        );
                                        self.send_message(message.into()).await;
                                        break 'label;
                                    }
                                    Some(target_id) => {
                                        self.find_app_mut(&username)
                                            .user
                                            .ignored
                                            .remove(&target_id);
                                        let message = message::System::new(
                                            user,
                                            format!("No longer ignoring: {}", target_username),
                                        );
                                        self.send_message(message.into()).await;
                                    }
                                }
                            }
                        }

                        let app = self.find_app_mut(&username);
                        app.state.input.clear();
                    }
                    _ => {}
                }
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
