use std::io::Write;
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use governor::clock::{Clock, QuantaClock, Reference};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use russh_keys::key::PublicKey;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;

use super::app::App;
use super::command::{Command, CommandParseError};
use super::message::{self, Message};
use super::message_history::MessageHistory;
use super::user::{Theme, TimestampMode, User, UserStatus};

use crate::server::auth::{BanAttribute, BanQuery};
use crate::server::terminal::TerminalHandle;
use crate::server::Auth;
use crate::utils;

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
    names: HashMap<UserId, UserName>,
    apps: HashMap<UserName, App>,
    ratelims: HashMap<UserId, Arc<Mutex<RateLimit>>>,
    history: MessageHistory,
    motd: String,
    created_at: DateTime<Utc>,
    auth: Arc<Mutex<Auth>>,
}

impl ServerRoom {
    pub fn new(motd: &str, auth: Arc<Mutex<Auth>>) -> Self {
        Self {
            auth,
            names: HashMap::new(),
            apps: HashMap::new(),
            ratelims: HashMap::new(),
            history: MessageHistory::new(),
            motd: motd.to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn try_find_app_by_id(&mut self, user_id: UserId) -> Option<&mut App> {
        let name = self.try_find_name(&user_id);
        match name {
            Some(username) => self.try_find_app_mut(&username.clone()),
            None => None,
        }
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        username: UserName,
        is_op: bool,
        key: Option<PublicKey>,
        terminal: TerminalHandle,
        ssh_id: String,
    ) {
        let name = match self.is_room_member(&username) {
            true => User::gen_rand_name(),
            false => username,
        };

        let user = User::new(user_id, name.clone(), ssh_id, key, is_op);
        let mut app = App::new(user.clone(), terminal);

        app.terminal.set_prompt(&app.terminal.get_prompt(&user));

        self.apps.insert(name.clone(), app.clone());
        self.names.insert(user_id, name.clone());
        self.ratelims.insert(
            user_id,
            Arc::new(Mutex::new(RateLimit::direct(MESSAGE_RATE_QUOTA))),
        );

        self.send_motd(&name).await;
        self.feed_history(&name).await;

        let message =
            message::Announce::new(user, format!("joined. (Connected: {})", self.apps.len()));
        self.send_message(message.into()).await;
    }

    pub async fn send_motd(&mut self, username: &UserName) {
        let motd = self.motd.clone();
        let app = self.find_app(username);
        let message = message::System::new(app.user.clone(), format!("{}{}", motd, utils::NEWLINE));
        let _ = app.send_message(message.into()).await;
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

    pub async fn cleanup(&mut self, user_id: &UserId) {
        for (_, app) in &mut self.apps {
            app.user.ignored.remove(user_id);
            app.user.focused.remove(user_id);
        }
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
                    if m.from.is_muted && app.user.id == m.from.id {
                        app.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if app.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if !app.user.focused.is_empty() && !app.user.focused.contains(&m.from.id) {
                        continue;
                    }
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(ref m) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if m.from.is_muted && app.user.id == m.from.id {
                        app.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if app.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(ref m) => {
                self.history.push(msg.clone());
                for (_, app) in self.apps.iter() {
                    if m.from.is_muted && app.user.id == m.from.id {
                        app.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if app.user.quiet {
                        continue;
                    }
                    if app.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if let Err(_) = app.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.find_app(&m.from.username);

                if m.from.is_muted {
                    from.send_user_is_muted_message().await.unwrap();
                    return;
                }

                from.send_message(msg.clone()).await.unwrap();

                let to = self.find_app(&m.to.username);
                if !to.user.ignored.contains(&m.from.id) {
                    to.send_message(msg).await.unwrap();
                }
            }
        }
    }

    pub async fn handle_input(&mut self, user_id: &UserId, data: &[u8]) {
        let username = self.names.get(user_id).unwrap().clone();

        let mut decoder = Decoder::new();
        for byte in data {
            for keycode in decoder.write(*byte) {
                match keycode {
                    KeyCode::Backspace => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.remove_before_cursor();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::CtrlA | KeyCode::CtrlArrowLeft | KeyCode::Home => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.move_cursor_start();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::CtrlE | KeyCode::CtrlArrowRight | KeyCode::End => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.move_cursor_end();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::CtrlD => todo!(),
                    KeyCode::CtrlW => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.remove_last_word_before_cursor();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::CtrlK => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.remove_after_cursor();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::CtrlU => {
                        let app = self.find_app_mut(&username);
                        app.terminal.clear_input().await.unwrap();
                    }
                    KeyCode::CtrlY => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.restore();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::ArrowLeft | KeyCode::CtrlB => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.move_cursor_prev();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::ArrowRight | KeyCode::CtrlF => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.move_cursor_next();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::Char(_) | KeyCode::Space => {
                        let app = self.find_app_mut(&username);
                        app.terminal
                            .input
                            .insert_before_cursor(keycode.bytes().as_slice());
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::ArrowUp => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.set_history_prev();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::ArrowDown => {
                        let app = self.find_app_mut(&username);
                        app.terminal.input.set_history_next();
                        app.terminal.write_prompt().await.unwrap();
                    }
                    KeyCode::Tab => todo!(),
                    KeyCode::Enter => {
                        let (user, input) = {
                            let app = self.find_app(&username);
                            (app.user.clone(), app.terminal.input.clone())
                        };

                        let ratelimit = self.ratelims.get(&user_id).unwrap();
                        let err = ratelimit.lock().await.check().err();

                        if let Some(nu) = err {
                            let now = QuantaClock::default().now();
                            let remaining_nanos = nu.earliest_possible().duration_since(now);
                            let remaining_duration = Duration::from_nanos(remaining_nanos.as_u64());
                            let truncated_remaining_duration =
                                Duration::new(remaining_duration.as_secs(), 0);

                            let body = format!(
                                "rate limit exceeded. Message dropped. Next allowed in {}",
                                humantime::format_duration(truncated_remaining_duration)
                            );
                            let message = message::Error::new(user, body);
                            self.send_message(message.into()).await;
                            return;
                        }

                        let input_str = input.to_string();

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

                        self.handle_command(&username).await;
                    }
                    _ => {}
                }
            }
        }
    }

    async fn handle_command(&mut self, username: &UserName) {
        let app = self.find_app_mut(username);
        let user = app.user.clone();

        let cmd = Command::parse(&app.terminal.input.bytes());
        let input_str = app.terminal.input.to_string();
        app.terminal.clear_input().await.unwrap();

        match cmd {
            Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                let message = message::Public::new(user, input_str);
                self.send_message(message.into()).await;
                return;
            }
            Err(err) => {
                app.terminal.input.push_to_history();

                let message = message::Command::new(user.clone(), input_str);
                self.send_message(message.into()).await;

                let message = message::Error::new(user, format!("{}", err));
                self.send_message(message.into()).await;

                return;
            }
            Ok(_) => {
                let message = message::Command::new(user.clone(), input_str);
                self.send_message(message.into()).await;
            }
        }

        let cmd = cmd.unwrap();
        match cmd {
            Command::Exit => {
                let app = self.find_app(username);
                app.terminal.exit().await;
                return;
            }
            Command::Away(reason) => {
                let app = self.find_app_mut(username);
                app.user.go_away(reason.to_string());

                let message =
                    message::Emote::new(app.user.clone(), format!("has gone away: \"{}\"", reason));
                self.send_message(message.into()).await;
            }
            Command::Back => {
                let app = self.find_app_mut(username);
                if let UserStatus::Away {
                    reason: _,
                    since: _,
                } = &app.user.status
                {
                    app.user.return_active();
                    let message = message::Emote::new(app.user.clone(), "is back".to_string());
                    self.send_message(message.into()).await;
                }
            }
            Command::Name(new_name) => 'label: {
                let app = self.find_app_mut(username);
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

                let app = self.find_app_mut(username);
                app.user.set_new_name(new_name.clone());
                app.terminal.set_prompt(&app.terminal.get_prompt(&app.user));

                let app = app.clone();
                self.apps.insert(new_name.clone(), app);
                self.apps.remove(&old_name);
                self.names.insert(user_id, new_name.clone());
            }
            Command::Msg(to, msg) => 'label: {
                let from = self.find_app(username).user.clone();

                match self.try_find_app_mut(&to).map(|a| &mut a.user) {
                    None => {
                        let message =
                            message::Error::new(from.clone(), format!("user is not found"));
                        self.send_message(message.into()).await;
                        break 'label;
                    }
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

                        to.set_reply_to(from.id);

                        let message =
                            message::Private::new(from.clone(), to.clone(), msg.to_string());
                        self.send_message(message.into()).await;

                        match status {
                            UserStatus::Away { reason, since: _ } => {
                                let message = message::System::new(
                                    from.clone(),
                                    format!(
                                        "Sent PM to {}, but they're away now: {}",
                                        name, reason
                                    ),
                                );
                                self.send_message(message.into()).await;
                            }
                            UserStatus::Active => {}
                        }
                    }
                }
            }
            Command::Reply(message_body) => 'label: {
                let app = self.find_app(username);
                let from = app.user.clone();

                if from.reply_to.is_none() {
                    let message =
                        message::Error::new(from.clone(), "no message to reply to".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let target_id = &from.reply_to.unwrap();
                let target_name = self.names.get(&target_id);
                if target_name.is_none() {
                    let message =
                        message::Error::new(from.clone(), "user already left the room".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let app = self.find_app(target_name.unwrap());
                let to = app.user.clone();
                let message = message::Private::new(from, to, message_body);
                self.send_message(message.into()).await;
            }
            Command::Users => {
                let app = self.find_app(username);
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
                let app = self.find_app(username);
                let user = app.user.clone();
                let message = match self.try_find_app(&target_name).map(|app| &app.user) {
                    Some(target) => message::System::new(user, target.to_string()).into(),
                    None => message::Error::new(user, "user not found".to_string()).into(),
                };
                self.send_message(message).await;
            }
            Command::Slap(target_name) => 'label: {
                let app = self.find_app(username);
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
                    message::Emote::new(user, format!("hits {} with a squishy banana.", t.username))
                        .into()
                } else {
                    message::Error::new(user, "that slippin' monkey not in the room".to_string())
                        .into()
                };
                self.send_message(message).await;
            }
            Command::Shrug => {
                let app = self.find_app(username);
                let user = app.user.clone();
                let message = message::Emote::new(user, "¯\\_(◕‿◕)_/¯".to_string());
                self.send_message(message.into()).await;
            }
            Command::Me(action) => {
                let app = self.find_app(username);
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
                let app = self.find_app(username);
                let user = app.user.clone();

                let message = message::System::new(user.clone(), Command::to_string(user.is_op));
                self.send_message(message.into()).await;
            }
            Command::Quiet => {
                let app = self.find_app_mut(username);
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
                let app = self.find_app_mut(username);
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
                let app = self.find_app_mut(username);
                let user = app.user.clone();
                let theme_clone = theme.clone();
                app.user.theme = theme.into();
                let message = message::System::new(user, format!("Set theme: {}", theme_clone));
                self.send_message(message.into()).await;
            }
            Command::Themes => {
                let app = self.find_app(username);
                let user = app.user.clone();
                let message = message::System::new(
                    user,
                    format!("Supported themes: {}", Theme::all().join(", ")),
                );
                self.send_message(message.into()).await;
            }
            Command::Ignore(target) => 'label: {
                let app = self.find_app(username);
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
                        let message =
                            message::Error::new(user.clone(), "user not found".to_string());
                        self.send_message(message.into()).await;
                        break 'label;
                    }
                    Some(target_id) => {
                        self.find_app_mut(username).user.ignored.insert(target_id);
                        let message =
                            message::System::new(user, format!("Ignoring: {}", target_username));
                        self.send_message(message.into()).await;
                    }
                }
            }
            Command::Unignore(target_username) => 'label: {
                let app = self.find_app(username);
                let user = app.user.clone();

                match self
                    .try_find_app(&target_username)
                    .map(|a| a.user.id.clone())
                {
                    None => {
                        let message =
                            message::Error::new(user.clone(), "user not found".to_string());
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
                        self.find_app_mut(username).user.ignored.remove(&target_id);
                        let message = message::System::new(
                            user,
                            format!("No longer ignoring: {}", target_username),
                        );
                        self.send_message(message.into()).await;
                    }
                }
            }
            Command::Focus(target) => 'label: {
                let app = self.find_app(username);
                let user = app.user.clone();

                if target.is_none() {
                    let focused_usernames: Vec<String> = user
                        .focused
                        .iter()
                        .filter_map(|id| self.names.get(id))
                        .map(|name| user.theme.style_username(name).to_string())
                        .collect();

                    let message_text = match focused_usernames.is_empty() {
                        true => "Focusing no users".to_string(),
                        false => format!(
                            "Focusing on {} users: {}",
                            focused_usernames.len(),
                            focused_usernames.join(", ")
                        ),
                    };

                    let message = message::System::new(user, message_text);
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let target = target.unwrap();
                if target == "$" {
                    self.find_app_mut(username).user.focused.clear();
                    let message =
                        message::System::new(user, "Removed focus from all users".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let mut focused = vec![];
                for target_name in target.split(",") {
                    match self
                        .try_find_app(&target_name.to_string())
                        .map(|a| a.user.id.clone())
                    {
                        None => continue,
                        Some(target_id) if target_id == user.id => continue,
                        Some(target_id) if user.focused.contains(&target_id) => continue,
                        Some(target_id) => {
                            self.find_app_mut(username).user.focused.insert(target_id);

                            focused.push(target_name);
                        }
                    }
                }

                let focused_usernames: Vec<String> = focused
                    .iter()
                    .map(|name| user.theme.style_username(name).to_string())
                    .collect();

                let message_text = match focused_usernames.is_empty() {
                    true => "No online users found to focus".to_string(),
                    false => format!(
                        "Focusing on {} users: {}",
                        focused_usernames.len(),
                        focused_usernames.join(", ")
                    ),
                };

                let message = message::System::new(user, message_text);
                self.send_message(message.into()).await;
            }
            Command::Version => {
                let message = message::System::new(user, format!("{}", env!("CARGO_PKG_VERSION")));
                self.send_message(message.into()).await;
            }
            Command::Uptime => {
                let now = Utc::now();
                let since_created = now.signed_duration_since(self.created_at).num_seconds() as u64;
                let uptime = humantime::format_duration(Duration::from_secs(since_created));
                let message = message::System::new(user, uptime.to_string());
                self.send_message(message.into()).await;
            }
            Command::Mute(target_username) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                match self.try_find_app_mut(&target_username).map(|a| &mut a.user) {
                    None => {
                        let message = message::Error::new(user, "user not found".to_string());
                        self.send_message(message.into()).await;
                        break 'label;
                    }
                    Some(target) if target.id == user.id => {
                        let message =
                            message::Error::new(user, "you can't mute yourself".to_string());
                        self.send_message(message.into()).await;
                        break 'label;
                    }
                    Some(target) => {
                        target.switch_mute_mode();
                        let target = target.clone();
                        let message = message::System::new(
                            user,
                            format!(
                                "{}: {}, id = {}",
                                match target.is_muted {
                                    true => "Muted",
                                    false => "Unmuted",
                                },
                                target.username,
                                target.id
                            ),
                        );
                        self.send_message(message.into()).await;
                    }
                }
            }
            Command::Motd(text) => 'label: {
                if text.is_none() {
                    let message = message::System::new(user, self.motd.clone());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                if !user.is_op {
                    let message = message::Error::new(
                        user,
                        "must be an operator to modify the MOTD".to_string(),
                    );
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let motd = text.unwrap();
                self.motd = motd.clone();

                let message = message::Announce::new(
                    user.clone(),
                    format!("set new message of the day: {}-> {}", utils::NEWLINE, motd),
                );
                self.send_message(message.into()).await;
            }
            Command::Kick(target_username) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                match self.try_find_app_mut(&target_username) {
                    None => {
                        let message = message::Error::new(user, "user not found".to_string());
                        self.send_message(message.into()).await;
                        break 'label;
                    }
                    Some(app) => {
                        app.terminal.exit().await;

                        let message = message::Announce::new(
                            user,
                            format!("kicked {} from the server", target_username),
                        );
                        self.send_message(message.into()).await;
                    }
                }
            }
            Command::Ban(query) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let query = query.parse::<BanQuery>();
                if let Err(err) = query {
                    let message = message::Error::new(user, err.to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let mut messages: Vec<Message> = vec![];

                match query.unwrap() {
                    BanQuery::Single { name, duration } => {
                        match self
                            .try_find_app(&name)
                            .filter(|app| app.user.public_key.is_some())
                        {
                            Some(app) => {
                                self.auth.lock().await.ban_fingerprint(
                                    &app.user.public_key.as_ref().unwrap().fingerprint(),
                                    duration,
                                );
                                app.terminal.exit().await;
                                let message = message::Announce::new(
                                    user.clone(),
                                    format!("banned {} from the server", app.user.username),
                                );
                                messages.push(message.into());
                            }
                            None => {
                                let message =
                                    message::Error::new(user, "user not found".to_string());
                                self.send_message(message.into()).await;
                                break 'label;
                            }
                        }
                    }
                    BanQuery::Multiple(items) => {
                        for item in items {
                            match item.attribute {
                                BanAttribute::Name(name) => {
                                    self.auth.lock().await.ban_username(&name, item.duration);

                                    for (_, app) in self.apps.iter_mut() {
                                        if app.user.username.eq(&name) {
                                            app.terminal.exit().await;
                                            let message = message::Announce::new(
                                                user.clone(),
                                                format!("banned {} from the server", name),
                                            );
                                            messages.push(message.into());
                                        }
                                    }
                                }
                                BanAttribute::Fingerprint(fingerprint) => {
                                    self.auth
                                        .lock()
                                        .await
                                        .ban_fingerprint(&fingerprint, item.duration);

                                    for (_, app) in self.apps.iter_mut() {
                                        if let Some(key) = &app.user.public_key {
                                            if key.fingerprint().eq(&fingerprint) {
                                                app.terminal.exit().await;
                                                let message = message::Announce::new(
                                                    user.clone(),
                                                    format!(
                                                        "banned {} from the server",
                                                        app.user.username
                                                    ),
                                                );
                                                messages.push(message.into());
                                            }
                                        }
                                    }
                                }
                                BanAttribute::Ip(_) => todo!(),
                            }
                        }
                    }
                }

                messages.push(
                    message::System::new(
                        user,
                        "Banning is complete. Offline users were silently banned.".to_string(),
                    )
                    .into(),
                );

                for message in messages {
                    self.send_message(message).await;
                }
            }
            Command::Banned => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    self.send_message(message.into()).await;
                    break 'label;
                }

                let (names, fingerprints) = self.auth.lock().await.banned();
                let mut buf = Vec::new();
                write!(buf, "Banned:").unwrap();

                for name in names {
                    write!(buf, "{} \"name={}\"", utils::NEWLINE, name).unwrap();
                }

                for fingerprint in fingerprints {
                    write!(buf, "{} \"fingerprint={}\"", utils::NEWLINE, fingerprint).unwrap();
                }

                let message = message::System::new(user, String::from_utf8(buf).unwrap());
                self.send_message(message.into()).await;
            }
        }
    }

    pub fn handle_window_resize(&mut self, user_id: &UserId, (width, height): (u16, u16)) {
        let username = self.try_find_name(user_id).unwrap().clone();
        let app = self.find_app_mut(&username);
        app.terminal.set_size(width, height);
    }

    fn is_room_member(&self, username: &str) -> bool {
        self.apps.contains_key(username)
    }

    fn find_app(&self, username: &str) -> &App {
        self.apps
            .get(username)
            .expect(format!("User {username} MUST have an app within a server room").as_str())
    }

    fn find_app_mut(&mut self, username: &str) -> &mut App {
        self.apps
            .get_mut(username)
            .expect(format!("User {username} MUST have an app within a server room").as_str())
    }

    fn try_find_app(&self, username: &str) -> Option<&App> {
        self.apps.get(username)
    }

    fn try_find_app_mut(&mut self, username: &str) -> Option<&mut App> {
        self.apps.get_mut(username)
    }

    fn try_find_name(&self, user_id: &UserId) -> Option<&UserName> {
        self.names.get(user_id)
    }
}
