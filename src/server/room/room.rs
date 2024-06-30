use std::collections::hash_map::IterMut;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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
    names: HashMap<UserId, UserName>,
    members: HashMap<UserName, RoomMember>,
    ratelims: HashMap<UserId, RateLimit>,
    history: MessageHistory,
    commands: CommandCollection,
    motd: String,
    created_at: DateTime<Utc>,
    auth: Arc<Mutex<Auth>>,
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

    pub fn commands(&self) -> &CommandCollection {
        &self.commands
    }

    pub fn motd(&self) -> &String {
        &self.motd
    }

    pub fn set_motd(&mut self, motd: String) {
        self.motd = motd;
    }

    pub fn uptime(&self) -> String {
        let now = Utc::now();
        let since_created = now.signed_duration_since(self.created_at).num_seconds() as u64;
        humantime::format_duration(Duration::from_secs(since_created)).to_string()
    }

    pub fn auth(&self) -> &Arc<Mutex<Auth>> {
        &self.auth
    }

    pub fn get_ratelimit(&self, user_id: UserId) -> Option<&RateLimit> {
        self.ratelims.get(&user_id)
    }

    pub fn add_member(&mut self, name: UserName, member: RoomMember) {
        self.members.insert(name, member);
    }

    pub fn remove_member(&mut self, username: &UserName) {
        self.members.remove(username);
    }

    pub fn add_name(&mut self, id: UserId, name: UserName) {
        self.names.insert(id, name);
    }

    pub fn members_iter_mut(&mut self) -> IterMut<UserName, RoomMember> {
        self.members.iter_mut()
    }

    pub fn names(&self) -> &HashMap<UserId, UserName> {
        &self.names
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

        self.members.insert(name.clone(), member);
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
        let name = self.try_get_name(user_id);
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
        members.sort_by(|a, b| b.last_sent_time().cmp(&a.last_sent_time()));

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
        let name = self.try_get_name(&user_id);
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

    pub fn try_get_name(&self, user_id: &UserId) -> Option<&UserName> {
        self.names.get(user_id)
    }
}
