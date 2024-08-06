use std::collections::hash_map::{Iter, IterMut};
use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use governor::Quota;
use nonzero_ext::nonzero;
use tokio::sync::{mpsc, watch};

use super::member::RoomMember;

use crate::chat::message::{self, Message, MessageHistory};
use crate::chat::ratelimit::RateLimit;
use crate::chat::user::{User, UserName};
use crate::pubkey::PubKey;
use crate::utils::{self, sanitize};

type UserId = usize;

const MESSAGE_MAX_BURST: std::num::NonZeroU32 = nonzero!(10u32);
const MESSAGE_RATE_QUOTA: Quota = Quota::per_second(MESSAGE_MAX_BURST);

pub struct ChatRoom {
    names: HashMap<UserId, UserName>,
    members: HashMap<UserName, RoomMember>,
    ratelims: HashMap<UserId, RateLimit>,
    history: MessageHistory<20>,
    motd: String,
    created_at: DateTime<Utc>,
}

impl ChatRoom {
    pub fn new(motd: &str) -> Self {
        Self {
            names: HashMap::new(),
            members: HashMap::new(),
            ratelims: HashMap::new(),
            history: MessageHistory::new(),
            motd: motd.to_string(),
            created_at: Utc::now(),
        }
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

    pub fn members_iter(&self) -> Iter<UserName, RoomMember> {
        self.members.iter()
    }

    pub fn names(&self) -> &HashMap<UserId, UserName> {
        &self.names
    }

    pub async fn join(
        &mut self,
        user_id: UserId,
        username: String,
        key: PubKey,
        ssh_id: String,
        message_tx: mpsc::Sender<String>,
        exit_tx: watch::Sender<()>,
    ) -> anyhow::Result<User> {
        let username = match self.is_room_member(&username) {
            true => rand::random::<UserName>(),
            false if username.trim().is_empty() => rand::random::<UserName>(),
            false => sanitize::name(&username).into(),
        };

        let user = User::new(user_id, username.clone(), ssh_id, key);
        let member = RoomMember::new(user.clone(), message_tx, exit_tx);

        self.members.insert(username.clone(), member);
        self.names.insert(user_id, username.clone());
        self.ratelims
            .insert(user_id, RateLimit::direct(MESSAGE_RATE_QUOTA));

        self.send_motd(&username).await;
        self.feed_history(&username).await;

        let message = message::Announce::new(
            user.clone().into(),
            format!("joined. (Connected: {})", self.members.len()),
        );
        self.send_message(message.into()).await?;

        Ok(user)
    }

    pub async fn send_motd(&mut self, username: &UserName) {
        let motd = self.motd.clone();
        let member = self.find_member(username);
        let message = message::System::new(
            member.user.clone().into(),
            format!("{}{}", motd, utils::NEWLINE),
        );
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

    pub async fn leave(&mut self, user_id: &UserId) -> anyhow::Result<()> {
        let username = match self.try_get_name(user_id) {
            Some(name) => name.clone(),
            None => return Ok(()),
        };

        let member = self.find_member(&username);
        let user = &member.user;
        let duration = humantime::format_duration(user.joined_duration());
        let message =
            message::Announce::new(user.clone().into(), format!("left: (After {})", duration));
        self.send_message(message.into()).await?;

        self.members.remove(&username);
        self.names.remove(user_id);
        self.ratelims.remove(user_id);

        for (_, member) in &mut self.members {
            member.user.unignore(user_id);
            member.user.unfocus(user_id);
        }

        Ok(())
    }

    pub async fn send_message(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::System(ref m) => {
                let member = self.find_member(&m.from().username());
                member.send_message(msg).await?;
            }
            Message::Command(ref m) => {
                let member = self.find_member(&m.from().username());
                member.send_message(msg).await?;
            }
            Message::Error(ref m) => {
                let member = self.find_member(&m.from().username());
                member.send_message(msg).await?;
            }
            Message::Public(ref m) => {
                self.history.push(msg.clone());
                for (_, member) in self.members.iter() {
                    if m.from().is_muted() && member.user.id() == m.from().id() {
                        member.send_user_is_muted_message().await?;
                    }
                    if m.from().is_muted() {
                        continue;
                    }
                    if member.user.ignored().contains(&m.from().id()) {
                        continue;
                    }
                    if !member.user.focused().is_empty()
                        && !member.user.focused().contains(&m.from().id())
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
                    if m.from().is_muted() && member.user.id() == m.from().id() {
                        member.send_user_is_muted_message().await?;
                    }
                    if m.from().is_muted() {
                        continue;
                    }
                    if member.user.ignored().contains(&m.from().id()) {
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
                    if m.from().is_muted() && member.user.id() == m.from().id() {
                        member.send_user_is_muted_message().await?;
                    }
                    if m.from().is_muted() {
                        continue;
                    }
                    if member.user.config().quiet() {
                        continue;
                    }
                    if member.user.ignored().contains(&m.from().id()) {
                        continue;
                    }
                    if let Err(_) = member.send_message(msg.clone()).await {
                        continue;
                    }
                }
            }
            Message::Private(ref m) => {
                let from = self.find_member(&m.from().username());

                if m.from().is_muted() {
                    from.send_user_is_muted_message().await?;
                    return Ok(());
                }

                let to = self.find_member(&m.to().username());
                if !to.user.ignored().contains(&m.from().id()) {
                    to.send_message(msg).await?;
                }
            }
        }

        Ok(())
    }

    pub fn find_name_by_prefix(&self, prefix: &str, skip: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }

        let mut members = vec![];
        for member in self.members.values() {
            if member.user.username().starts_with(prefix) {
                members.push(member.clone());
            }
        }

        // Sort in descending order (recently active first)
        members.sort_by(|a, b| b.last_sent_time().cmp(&a.last_sent_time()));

        let names: Vec<&UserName> = members.iter().map(|m| m.user.username()).collect();
        if names.is_empty() {
            return None;
        } else if names[0] != skip {
            return Some(names[0].as_ref().into());
        } else if names.len() > 1 {
            return Some(names[1].as_ref().into());
        }
        None
    }

    pub fn is_room_member(&self, username: &str) -> bool {
        self.members.contains_key(&username.into())
    }

    pub fn find_member(&self, username: &UserName) -> &RoomMember {
        self.members
            .get(&username)
            .expect(format!("User {username} should be a member of the server room").as_str())
    }

    pub fn find_member_mut(&mut self, username: &UserName) -> &mut RoomMember {
        self.members
            .get_mut(&username)
            .expect(format!("User {username} should be a member of the server room").as_str())
    }

    pub fn find_member_by_id(&mut self, user_id: UserId) -> &RoomMember {
        self.try_get_name(&user_id)
            .and_then(|name| self.try_find_member(&name))
            .expect(format!("User {user_id} should be a member of the server room").as_str())
    }

    pub fn try_find_member(&self, username: &UserName) -> Option<&RoomMember> {
        self.members.get(&username)
    }

    pub fn try_find_member_mut(&mut self, username: &UserName) -> Option<&mut RoomMember> {
        self.members.get_mut(&username)
    }

    pub fn try_get_name(&self, user_id: &UserId) -> Option<&UserName> {
        self.names.get(user_id)
    }
}

#[cfg(test)]
mod should {
    use std::usize;

    use super::*;
    use crate::chat::user::{User, UserName};
    use crate::pubkey::PubKey;
    use message::Author;
    use tokio::sync::{mpsc, watch};

    pub struct MockChannel {
        tx: mpsc::Sender<String>,
        rx: mpsc::Receiver<String>,
        messages: Vec<String>,
    }

    impl MockChannel {
        pub fn new(buffer: usize) -> Self {
            let (tx, rx) = mpsc::channel(buffer);
            let messages = Vec::new();
            Self { tx, rx, messages }
        }
    }

    #[tokio::test]
    async fn create_chat_room() {
        let chat_room = ChatRoom::new("Welcome to the chat room!");

        assert_eq!(chat_room.motd(), "Welcome to the chat room!");
        assert_eq!(chat_room.uptime(), "0s");
        assert!(chat_room.names().is_empty());
        assert!(chat_room.members_iter().count() == 0);
    }

    #[tokio::test]
    async fn set_and_get_motd() {
        let mut chat_room = ChatRoom::new("Welcome!");
        chat_room.set_motd("New MOTD".to_string());

        assert_eq!(chat_room.motd(), "New MOTD");
    }

    #[tokio::test]
    async fn add_and_remove_member() {
        let (message_tx, _message_rx) = mpsc::channel(1);
        let (_exit_tx, _exit_rx) = watch::channel(());
        let user = User::default();
        let username = UserName::from("alice");
        let mut chat_room = ChatRoom::new("Welcome!");

        let member = RoomMember::new(user.clone(), message_tx, _exit_tx);
        chat_room.add_member(username.clone(), member);

        assert!(chat_room.is_room_member(&username));
        chat_room.remove_member(&username);
        assert!(!chat_room.is_room_member(&username));
    }

    #[tokio::test]
    async fn join_chat_room() {
        let mut channel = MockChannel::new(5);
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        // Populate chat history
        let history_message =
            message::Public::new(Author::from(User::default()), "Hi all!".to_string());
        assert!(chat_room.send_message(history_message.into()).await.is_ok());

        // Join user
        let user = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx,
            )
            .await;

        assert!(user.is_ok());
        assert!(chat_room.is_room_member("alice"));
        assert_eq!(chat_room.names().get(&1).unwrap(), "alice");

        // Receive exactly 3 messages
        for _ in 0..3 {
            match channel.rx.try_recv() {
                Ok(msg) => channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(channel.messages[0].contains("Welcome!"),);
        assert!(channel.messages[1].contains("Hi all!"),);
        assert!(channel.messages[2].contains("alice joined. (Connected: 1)"),);
    }

    #[tokio::test]
    async fn leave_chat_room() {
        let mut channel = MockChannel::new(5);
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        // Join user
        let _ = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx,
            )
            .await;

        chat_room.leave(&1).await.unwrap();
        assert!(!chat_room.is_room_member(&"alice"));
        assert!(chat_room.names().get(&1).is_none());

        // Receive exactly 3 messages
        for _ in 0..3 {
            match channel.rx.try_recv() {
                Ok(msg) => channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(channel.messages[0].contains("Welcome!"));
        assert!(channel.messages[1].contains("alice joined. (Connected: 1)"));
        assert!(channel.messages[2].contains("alice left: (After 0s)"));
    }

    #[tokio::test]
    async fn send_system_messages() {
        let mut channel = MockChannel::new(5);
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let user = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let author: Author = user.into();
        let system = message::System::new(author.clone(), "welcome to the chat".to_string());
        let error = message::Error::new(author.clone(), "unknown command".to_string());
        let command = message::Command::new(author.clone(), "/help".to_string());
        assert!(chat_room.send_message(system.into()).await.is_ok());
        assert!(chat_room.send_message(error.into()).await.is_ok());
        assert!(chat_room.send_message(command.into()).await.is_ok());

        // Receive exactly 5 messages
        for _ in 0..5 {
            match channel.rx.try_recv() {
                Ok(msg) => channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 5 messages
        match channel.rx.try_recv() {
            Ok(_) => panic!("More than 5 messages were received"),
            Err(_) => {}
        }

        assert!(channel.messages[2].contains("-> welcome to the chat"));
        assert!(channel.messages[3].contains("Error: unknown command"));
        assert!(channel.messages[4].contains("/help"));
    }

    #[tokio::test]
    async fn send_public_message() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Public::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 4 messages
        for _ in 0..4 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[3].contains("Hello, World!"));
    }

    #[tokio::test]
    async fn not_send_public_message_if_author_is_muted() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let mut author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        author.switch_mute_mode();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Public::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("You are muted and cannot send messages."));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn not_send_public_message_if_author_is_ignored_by_recipient() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        chat_room
            .find_member_mut(recipient.username())
            .user
            .ignore(author.id());

        let msg = message::Public::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn not_send_public_message_if_author_is_not_in_focus_by_recipient() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        chat_room
            .find_member_mut(recipient.username())
            .user
            .focus(usize::MAX); // focus at least one user, any user, except the message author

        let msg = message::Public::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn send_emote_message() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Emote::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 4 messages
        for _ in 0..4 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[3].contains("Hello, World!"));
    }

    #[tokio::test]
    async fn not_send_emote_message_if_author_is_muted() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let mut author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        author.switch_mute_mode();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Emote::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("You are muted and cannot send messages."));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn not_send_emote_message_if_author_is_ignored_by_recipient() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        chat_room
            .find_member_mut(recipient.username())
            .user
            .ignore(author.id());

        let msg = message::Emote::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn send_announce_message() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Announce::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 4 messages
        for _ in 0..4 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[3].contains("Hello, World!"));
    }

    #[tokio::test]
    async fn not_send_announce_message_if_author_is_muted() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let mut author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        author.switch_mute_mode();

        let mut recipient_channel = MockChannel::new(5);
        let _ = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Announce::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("You are muted and cannot send messages."));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn not_send_announce_message_if_author_is_ignored_by_recipient() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        chat_room
            .find_member_mut(recipient.username())
            .user
            .ignore(author.id());

        let msg = message::Announce::new(author.into(), "Hello, World!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("Hello, World!"));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn send_private_message() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Private::new(author.into(), recipient.into(), "Hello, Bob!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 3 messages
        for _ in 0..3 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 4 messages
        for _ in 0..4 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[2].contains("bob joined"));
        assert!(recipient_channel.messages[3].contains("Hello, Bob!"));
    }

    #[tokio::test]
    async fn not_send_private_message_from_muted_author() {
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let mut author_channel = MockChannel::new(5);
        let mut author = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                author_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        author.switch_mute_mode();

        let mut recipient_channel = MockChannel::new(5);
        let recipient = chat_room
            .join(
                2,
                "bob".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                recipient_channel.tx.clone(),
                exit_tx.clone(),
            )
            .await
            .unwrap();

        let msg = message::Private::new(author.into(), recipient.into(), "Hello, Bob!".to_string());
        assert!(chat_room.send_message(msg.into()).await.is_ok());

        // Receive exactly 4 messages
        for _ in 0..4 {
            match author_channel.rx.try_recv() {
                Ok(msg) => author_channel.messages.push(msg),
                Err(_) => panic!("Expected 4 messages but received less"),
            }
        }

        // Check if there are more than 4 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 4 messages were received"),
            Err(_) => {}
        }

        // Receive exactly 3 messages
        for _ in 0..3 {
            match recipient_channel.rx.try_recv() {
                Ok(msg) => recipient_channel.messages.push(msg),
                Err(_) => panic!("Expected 3 messages but received less"),
            }
        }

        // Check if there are more than 3 messages
        match author_channel.rx.try_recv() {
            Ok(_) => panic!("More than 3 messages were received"),
            Err(_) => {}
        }

        assert!(author_channel.messages[3].contains("You are muted and cannot send messages."));
        assert!(recipient_channel.messages[2].contains("bob joined"));
    }

    #[tokio::test]
    async fn find_name_by_prefix() {
        let channel = MockChannel::new(10);
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        assert_eq!(chat_room.find_name_by_prefix("", ""), None);
        assert_eq!(chat_room.find_name_by_prefix("jo", ""), None);

        let _ = chat_room
            .join(
                1,
                "john".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx.clone(),
            )
            .await;

        assert_eq!(chat_room.find_name_by_prefix("jo", ""), Some("john".into()));
        assert_eq!(chat_room.find_name_by_prefix("jo", "john"), None);

        let _ = chat_room
            .join(
                2,
                "johnathan".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx,
            )
            .await;

        assert_eq!(
            chat_room.find_name_by_prefix("jo", "john"),
            Some("johnathan".into())
        );
    }

    #[tokio::test]
    async fn try_get_name() {
        let channel = MockChannel::new(5);
        let (exit_tx, _exit_rx) = watch::channel(());
        let mut chat_room = ChatRoom::new("Welcome!");

        let _ = chat_room
            .join(
                1,
                "alice".to_string(),
                PubKey::default(),
                "ssh".to_string(),
                channel.tx.clone(),
                exit_tx,
            )
            .await;

        assert_eq!(chat_room.try_get_name(&1).unwrap(), "alice");
    }
}
