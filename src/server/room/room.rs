use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use governor::Quota;
use nonzero_ext::nonzero;
use russh_keys::key::PublicKey;
use tokio::sync::{mpsc, Mutex};

use super::member::RoomMember;
use super::message;
use super::message::Message;
use super::message_history::MessageHistory;
use super::user::User;
use super::CommandCollection;

use crate::server::ratelimit::RateLimit;
use crate::server::Auth;
use crate::utils;

type UserId = usize;
type UserName = String;

const MESSAGE_MAX_BURST: std::num::NonZeroU32 = nonzero!(10u32);
const MESSAGE_RATE_QUOTA: Quota = Quota::per_second(MESSAGE_MAX_BURST);

pub struct ServerRoom {
    pub names: HashMap<UserId, UserName>,
    pub members: HashMap<UserName, RoomMember>,
    pub ratelims: HashMap<UserId, RateLimit>,
    pub history: MessageHistory,
    pub commands: CommandCollection,
    pub motd: String,
    pub created_at: DateTime<Utc>,
    pub auth: Arc<Mutex<Auth>>,
}

impl ServerRoom {
    pub fn new(motd: &str, auth: Arc<Mutex<Auth>>) -> Self {
        Self {
            auth,
            names: HashMap::new(),
            members: HashMap::new(),
            ratelims: HashMap::new(),
            history: MessageHistory::new(),
            commands: CommandCollection::new(),
            motd: motd.to_string(),
            created_at: Utc::now(),
        }
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        username: UserName,
        is_op: bool,
        key: Option<PublicKey>,
        ssh_id: String,
        tx: mpsc::Sender<String>,
    ) -> User {
        let name = match self.is_room_member(&username) {
            true => User::gen_rand_name(),
            false => username,
        };

        let user = User::new(user_id, name.clone(), ssh_id, key, is_op);
        let member = RoomMember::new(user.clone(), tx);
        self.members.insert(name.clone(), member.clone());
        self.names.insert(user_id, name.clone());
        self.ratelims
            .insert(user_id, RateLimit::direct(MESSAGE_RATE_QUOTA));

        self.send_motd(&name).await;
        self.feed_history(&name).await;

        let message = message::Announce::new(
            user.clone(),
            format!("joined. (Connected: {})", self.members.len()),
        );
        self.send_message(message.into()).await;

        user
    }

    pub async fn send_motd(&mut self, username: &UserName) {
        let motd = self.motd.clone();
        let member = self.find_member(username);
        let message =
            message::System::new(member.user.clone(), format!("{}{}", motd, utils::NEWLINE));
        let _ = member.send_message(message.into()).await;
    }

    pub async fn feed_history(&mut self, username: &UserName) {
        let member = self.find_member(username);
        for msg in self.history.iter() {
            if let Err(_) = member.send_message(msg.to_owned()).await {
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
        let user = self.find_member(&username).user.clone();

        let duration = humantime::format_duration(user.joined_duration());
        let message = message::Announce::new(user, format!("left: (After {})", duration));
        self.send_message(message.into()).await;

        self.members.remove(&username);
        self.names.remove(user_id);
        self.ratelims.remove(user_id);

        for (_, member) in &mut self.members {
            member.user.ignored.remove(user_id);
            member.user.focused.remove(user_id);
        }
    }

    pub async fn send_message(&mut self, msg: Message) {
        match msg {
            Message::System(ref m) => {
                let member = self.find_member(&m.from.username);
                member.send_message(msg).await.unwrap();
            }
            Message::Command(ref m) => {
                let member = self.find_member(&m.from.username);
                member.send_message(msg).await.unwrap();
            }
            Message::Error(ref m) => {
                let member = self.find_member(&m.from.username);
                member.send_message(msg).await.unwrap();
            }
            Message::Public(ref m) => {
                self.history.push(msg.clone());
                for (_, member) in self.members.iter() {
                    if m.from.is_muted && member.user.id == m.from.id {
                        member.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if member.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if !member.user.focused.is_empty() && !member.user.focused.contains(&m.from.id)
                    {
                        continue;
                    }
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Emote(ref m) => {
                self.history.push(msg.clone());
                for (_, member) in self.members.iter() {
                    if m.from.is_muted && member.user.id == m.from.id {
                        member.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if member.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Announce(ref m) => {
                self.history.push(msg.clone());
                for (_, member) in self.members.iter() {
                    if m.from.is_muted && member.user.id == m.from.id {
                        member.send_user_is_muted_message().await.unwrap();
                    }
                    if m.from.is_muted {
                        continue;
                    }
                    if member.user.quiet {
                        continue;
                    }
                    if member.user.ignored.contains(&m.from.id) {
                        continue;
                    }
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.find_member(&m.from.username);

                if m.from.is_muted {
                    from.send_user_is_muted_message().await.unwrap();
                    return;
                }

                from.send_message(msg.clone()).await.unwrap();

                let to = self.find_member(&m.to.username);
                if !to.user.ignored.contains(&m.from.id) {
                    to.send_message(msg).await.unwrap();
                }
            }
        }
    }

    pub fn find_name_by_prefix(&self, prefix: &str, skip: &str) -> Option<String> {
        let mut members = vec![];
        for member in self.members.values() {
            if member.user.username.starts_with(prefix) {
                members.push(member.clone());
            }
        }

        // Sort in descending order (recently active first)
        members.sort_by(|a, b| b.last_sent_at.cmp(&a.last_sent_at));

        let names: Vec<&String> = members.iter().map(|m| &m.user.username).collect();
        if names.is_empty() {
            return None;
        } else if names[0] != skip {
            return Some(names[0].clone());
        } else if names.len() > 1 {
            return Some(names[1].clone());
        }
        None
    }

    pub fn find_member(&self, username: &str) -> &RoomMember {
        self.members
            .get(username)
            .expect(format!("User {username} MUST have an member within a server room").as_str())
    }

    pub fn find_member_mut(&mut self, username: &str) -> &mut RoomMember {
        self.members
            .get_mut(username)
            .expect(format!("User {username} MUST have an member within a server room").as_str())
    }

    pub fn is_room_member(&self, username: &str) -> bool {
        self.members.contains_key(username)
    }

    pub fn try_find_member_by_id(&mut self, user_id: UserId) -> Option<&RoomMember> {
        let name = self.try_find_name(&user_id);
        match name {
            Some(username) => self.try_find_member(username),
            None => None,
        }
    }

    pub fn try_find_member(&self, username: &str) -> Option<&RoomMember> {
        self.members.get(username)
    }

    pub fn try_find_member_mut(&mut self, username: &str) -> Option<&mut RoomMember> {
        self.members.get_mut(username)
    }

    pub fn try_find_name(&self, user_id: &UserId) -> Option<&UserName> {
        self.names.get(user_id)
    }
}
