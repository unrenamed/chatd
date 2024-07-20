use async_trait::async_trait;
use std::io::Write;

use crate::auth::{Auth, BanAttribute, BanQuery};
use crate::chat::message::Message;
use crate::chat::{
    format_commands, message, ChatRoom, Command, OplistCommand, OplistLoadMode, Theme,
    TimestampMode, User, UserName, UserStatus, WhitelistCommand, WhitelistLoadMode,
    VISIBLE_NOOP_CHAT_COMMANDS, VISIBLE_OPLIST_COMMANDS, VISIBLE_OP_CHAT_COMMANDS,
    VISIBLE_WHITELIST_COMMANDS,
};
use crate::terminal::{CloseHandle, Terminal};
use crate::utils::{self, sanitize};

use super::handler::WorkflowHandler;
use super::WorkflowContext;

pub struct CommandExecutor<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> CommandExecutor<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    pub fn new() -> Self {
        Self { next: None }
    }
}

#[async_trait]
impl<H> WorkflowHandler<H> for CommandExecutor<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal<H>,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        let command = match &context.command {
            Some(command) => command,
            None => return Ok(()),
        };
        let user = context.user.clone();
        let username = &user.username();

        match command {
            Command::Exit => {
                let member = room.find_member(username);
                member.exit()?;
            }
            Command::Away(reason) => {
                let member = room.find_member_mut(username);
                member.user.go_away(reason.to_string());

                let message = message::Emote::new(
                    member.user.clone().into(),
                    format!("has gone away: \"{}\"", reason),
                );
                room.send_message(message.into()).await?;
            }
            Command::Back => {
                let member = room.find_member_mut(username);
                if let UserStatus::Away {
                    reason: _,
                    since: _,
                } = &member.user.status()
                {
                    member.user.return_active();
                    let message =
                        message::Emote::new(member.user.clone().into(), "is back".to_string());
                    room.send_message(message.into()).await?;
                }
            }
            Command::Name(new_name) => 'label: {
                let member = room.find_member_mut(username);
                let user = member.user.clone();
                let new_name = sanitize::name(&new_name);
                let new_username = UserName::from(&new_name);

                if user.username() == &new_username {
                    let message = message::Error::new(
                        user.into(),
                        "new name is the same as the original".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                if let Some(_) = room.try_find_member(&new_username) {
                    let message = message::Error::new(
                        user.into(),
                        format!("\"{}\" name is already taken", new_username),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let message = message::Announce::new(
                    user.clone().into(),
                    format!("user is now known as {}.", new_username),
                );
                room.send_message(message.into()).await?;

                let old_name = user.username();
                let user_id = user.id();

                let member = room.find_member_mut(username);
                member.user.set_username(new_username.clone());
                terminal.set_prompt(&member.user.config().display_name());

                let member = member.clone();
                room.add_member(new_username.clone(), member);
                room.remove_member(&old_name);
                room.add_name(user_id, new_username);
            }
            Command::Msg(to_username, msg) => 'label: {
                let from = room.find_member(username).user.clone();
                let to_username = UserName::from(to_username);

                match room.try_find_member_mut(&to_username).map(|a| &mut a.user) {
                    None => {
                        let message =
                            message::Error::new(from.into(), format!("user is not found"));
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(to) if from.id().eq(&to.id()) => {
                        let message =
                            message::Error::new(from.into(), format!("you can't message yourself"));
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(to) => {
                        let status = to.status().clone();
                        let name = to.username().clone();

                        to.set_reply_to(from.id());

                        let message = message::Private::new(
                            from.clone().into(),
                            to.clone().into(),
                            msg.to_string(),
                        );
                        room.send_message(message.into()).await?;

                        match status {
                            UserStatus::Away { reason, since: _ } => {
                                let message = message::System::new(
                                    from.into(),
                                    format!(
                                        "Sent PM to {}, but they're away now: {}",
                                        name, reason
                                    ),
                                );
                                room.send_message(message.into()).await?;
                            }
                            UserStatus::Active => {}
                        }
                    }
                }
            }
            Command::Reply(message_body) => 'label: {
                let member = room.find_member(username);
                let from = member.user.clone();

                if from.reply_to().is_none() {
                    let message =
                        message::Error::new(from.into(), "no message to reply to".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_id = &from.reply_to().unwrap();
                let target_name = room.try_get_name(&target_id);
                if target_name.is_none() {
                    let message =
                        message::Error::new(from.into(), "user already left the room".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let member = room.find_member(target_name.unwrap());
                let to = member.user.clone();
                let message =
                    message::Private::new(from.into(), to.into(), (*message_body).to_string());
                room.send_message(message.into()).await?;
            }
            Command::Users => {
                let member = room.find_member(username);
                let user = member.user.clone();

                let mut usernames = room.names().values().collect::<Vec<&UserName>>();
                usernames.sort_by_key(|a| a.to_lowercase());

                let colorized_names = usernames
                    .iter()
                    .map(|u| user.config().theme().style_username(u).to_string())
                    .collect::<Vec<String>>();

                let body = format!(
                    "{} connected: {}",
                    room.names().len(),
                    colorized_names.join(", ")
                );

                let message = message::System::new(user.into(), body);
                room.send_message(message.into()).await?;
            }
            Command::Whois(target_username) => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let target_username = UserName::from(target_username);
                let message = match room
                    .try_find_member(&target_username)
                    .map(|member| &member.user)
                {
                    Some(target) => message::System::new(user.into(), target.to_string()).into(),
                    None => message::Error::new(user.into(), "user not found".to_string()).into(),
                };
                room.send_message(message).await?;
            }
            Command::Slap(target_username) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                if target_username.is_none() {
                    let message = message::Emote::new(
                        user.into(),
                        "hits himself with a squishy banana.".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_username = UserName::from(target_username.as_deref().unwrap());
                let target = room
                    .try_find_member_mut(&target_username)
                    .map(|member| &member.user);

                let message = if let Some(t) = target {
                    message::Emote::new(
                        user.into(),
                        format!("hits {} with a squishy banana.", t.username()),
                    )
                    .into()
                } else {
                    message::Error::new(
                        user.into(),
                        "that slippin' monkey not in the room".to_string(),
                    )
                    .into()
                };
                room.send_message(message).await?;
            }
            Command::Shrug => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::Emote::new(user.into(), "¯\\_(◕‿◕)_/¯".to_string());
                room.send_message(message.into()).await?;
            }
            Command::Me(action) => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::Emote::new(
                    user.into(),
                    match action {
                        Some(s) => format!("{}", s),
                        None => format!("is at a loss for words."),
                    },
                );
                room.send_message(message.into()).await?;
            }
            Command::Help => {
                let member = room.find_member(username);
                let user = member.user.clone();

                let mut help = format!("Available commands: {}", utils::NEWLINE);
                help.push_str(&format_commands(&VISIBLE_NOOP_CHAT_COMMANDS));

                if auth.is_op(&user.public_key().clone().into()) {
                    help.push_str(&format!(
                        "{}{}Operator commands: {}{}",
                        utils::NEWLINE,
                        utils::NEWLINE,
                        utils::NEWLINE,
                        &format_commands(&VISIBLE_OP_CHAT_COMMANDS)
                    ));
                }

                let message = message::System::new(user.into(), help);
                room.send_message(message.into()).await?;
            }
            Command::Quiet => {
                let member = room.find_member_mut(username);
                member.user.config_mut().switch_quiet_mode();
                let message = message::System::new(
                    member.user.clone().into(),
                    match member.user.config().quiet() {
                        true => "Quiet mode is toggled ON",
                        false => "Quiet mode is toggled OFF",
                    }
                    .to_string(),
                );
                room.send_message(message.into()).await?;
            }
            Command::Timestamp(mode) => {
                let member = room.find_member_mut(username);
                member.user.config_mut().set_timestamp_mode(*mode);
                let message = message::System::new(
                    member.user.clone().into(),
                    match member.user.config().timestamp_mode() {
                        TimestampMode::Time | TimestampMode::DateTime => {
                            "Timestamp is toggled ON, timezone is UTC"
                        }
                        TimestampMode::Off => "Timestamp is toggled OFF",
                    }
                    .to_string(),
                );
                room.send_message(message.into()).await?;
            }
            Command::Theme(theme) => {
                let member = room.find_member_mut(username);
                let message = message::System::new(user.into(), format!("Set theme: {}", theme));
                member.user.set_theme((*theme).into());
                terminal.set_prompt(&member.user.config().display_name());
                room.send_message(message.into()).await?;
            }
            Command::Themes => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::System::new(
                    user.into(),
                    format!("Supported themes: {}", Theme::values().join(", ")),
                );
                room.send_message(message.into()).await?;
            }
            Command::Ignore(target) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                if target.is_none() {
                    let ignored_usernames: Vec<String> = user
                        .ignored()
                        .iter()
                        .filter_map(|id| room.try_get_name(id))
                        .map(|name| user.config().theme().style_username(name).to_string())
                        .collect();

                    let message_text = match ignored_usernames.is_empty() {
                        true => "0 users ignored".to_string(),
                        false => format!(
                            "{} users ignored: {}",
                            ignored_usernames.len(),
                            ignored_usernames.join(", ")
                        ),
                    };

                    let message = message::System::new(user.into(), message_text);
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_username = UserName::from(target.as_deref().unwrap());
                match room
                    .try_find_member(&target_username)
                    .map(|a| a.user.id().clone())
                {
                    Some(target_id) if target_id == user.id() => {
                        let message = message::Error::new(
                            user.into(),
                            "you can't ignore yourself".to_string(),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) if user.ignored().contains(&target_id) => {
                        let message = message::System::new(
                            user.into(),
                            format!("user already in the ignored list"),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    None => {
                        let message =
                            message::Error::new(user.into(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) => {
                        room.find_member_mut(username).user.ignore(target_id);
                        let message = message::System::new(
                            user.into(),
                            format!("Ignoring: {}", target_username),
                        );
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Unignore(target_username) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                let target_username = UserName::from(target_username);
                match room
                    .try_find_member(&target_username)
                    .map(|a| a.user.id().clone())
                {
                    None => {
                        let message =
                            message::Error::new(user.into(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) if !user.ignored().contains(&target_id) => {
                        let message = message::Error::new(
                            user.into(),
                            "user not in the ignored list yet".to_string(),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) => {
                        room.find_member_mut(username).user.unignore(&target_id);
                        let message = message::System::new(
                            user.into(),
                            format!("No longer ignoring: {}", target_username),
                        );
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Focus(target) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                if target.is_none() {
                    let focused_usernames: Vec<String> = user
                        .focused()
                        .iter()
                        .filter_map(|id| room.try_get_name(id))
                        .map(|name| user.config().theme().style_username(name).to_string())
                        .collect();

                    let message_text = match focused_usernames.is_empty() {
                        true => "Focusing no users".to_string(),
                        false => format!(
                            "Focusing on {} users: {}",
                            focused_usernames.len(),
                            focused_usernames.join(", ")
                        ),
                    };

                    let message = message::System::new(user.into(), message_text);
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target = target.as_deref().unwrap();
                if target == "$" {
                    room.find_member_mut(username).user.unfocus_all();
                    let message = message::System::new(
                        user.into(),
                        "Removed focus from all users".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let mut focused = vec![];
                for target_username in target.split(",") {
                    let target_username = UserName::from(target_username);
                    match room
                        .try_find_member(&target_username)
                        .map(|a| a.user.id().clone())
                    {
                        None => continue,
                        Some(target_id) if target_id == user.id() => continue,
                        Some(target_id) if user.focused().contains(&target_id) => continue,
                        Some(target_id) => {
                            room.find_member_mut(username).user.focus(target_id);
                            focused.push(target_username);
                        }
                    }
                }

                let focused_usernames: Vec<String> = focused
                    .iter()
                    .map(|name| user.config().theme().style_username(name).to_string())
                    .collect();

                let message_text = match focused_usernames.is_empty() {
                    true => "No online users found to focus".to_string(),
                    false => format!(
                        "Focusing on {} users: {}",
                        focused_usernames.len(),
                        focused_usernames.join(", ")
                    ),
                };

                let message = message::System::new(user.into(), message_text);
                room.send_message(message.into()).await?;
            }
            Command::Version => {
                let message =
                    message::System::new(user.into(), format!("{}", env!("CARGO_PKG_VERSION")));
                room.send_message(message.into()).await?;
            }
            Command::Uptime => {
                let message = message::System::new(user.into(), room.uptime());
                room.send_message(message.into()).await?;
            }
            Command::Mute(target_username) => 'label: {
                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_username = UserName::from(target_username);
                match room
                    .try_find_member_mut(&target_username)
                    .map(|a| &mut a.user)
                {
                    None => {
                        let message =
                            message::Error::new(user.into(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target) if target.id() == user.id() => {
                        let message =
                            message::Error::new(user.into(), "you can't mute yourself".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target) => {
                        target.switch_mute_mode();
                        let target = target.clone();
                        let message = message::System::new(
                            user.into(),
                            format!(
                                "{}: {}, id = {}",
                                match target.is_muted() {
                                    true => "Muted",
                                    false => "Unmuted",
                                },
                                target.username(),
                                target.id()
                            ),
                        );
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Motd(new_motd) => 'label: {
                if new_motd.is_none() {
                    let message = message::System::new(user.into(), room.motd().clone());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                if !auth.is_op(&user.public_key().clone().into()) {
                    let message = message::Error::new(
                        user.into(),
                        "must be an operator to modify the MOTD".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                room.set_motd(new_motd.as_deref().unwrap().to_string());

                let message = message::Announce::new(
                    user.into(),
                    format!(
                        "set new message of the day: {}-> {}",
                        utils::NEWLINE,
                        room.motd()
                    ),
                );
                room.send_message(message.into()).await?;
            }
            Command::Kick(target_username) => 'label: {
                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_username = UserName::from(target_username);
                match room.try_find_member_mut(&target_username) {
                    None => {
                        let message =
                            message::Error::new(user.into(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(member) => {
                        let message = message::Announce::new(
                            user.into(),
                            format!("kicked {} from the server", target_username),
                        );
                        member.exit()?;
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Ban(query) => 'label: {
                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let query = query.parse::<BanQuery>();
                if let Err(err) = query {
                    let message = message::Error::new(user.into(), err.to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let mut messages: Vec<Message> = vec![];

                match query.unwrap() {
                    BanQuery::Single { name, duration } => {
                        let target_username = UserName::from(&name);
                        match room.try_find_member(&target_username) {
                            Some(member) => {
                                auth.ban_fingerprint(
                                    &member.user.public_key().fingerprint(),
                                    duration,
                                );
                                let message = message::Announce::new(
                                    user.clone().into(),
                                    format!("banned {} from the server", member.user.username()),
                                );
                                member.exit()?;
                                messages.push(message.into());
                            }
                            None => {
                                let message =
                                    message::Error::new(user.into(), "user not found".to_string());
                                room.send_message(message.into()).await?;
                                break 'label;
                            }
                        }
                    }
                    BanQuery::Multiple(items) => {
                        for item in items {
                            match item.attribute {
                                BanAttribute::Name(name) => {
                                    let username = UserName::from(&name);
                                    auth.ban_username(&username, item.duration);

                                    for (_, member) in room.members_iter_mut() {
                                        if member.user.username().eq(&username) {
                                            let message = message::Announce::new(
                                                user.clone().into(),
                                                format!("banned {} from the server", username),
                                            );
                                            messages.push(message.into());
                                            member.exit()?;
                                        }
                                    }
                                }
                                BanAttribute::Fingerprint(fingerprint) => {
                                    auth.ban_fingerprint(&fingerprint, item.duration);

                                    for (_, member) in room.members_iter_mut() {
                                        if member.user.public_key().fingerprint().eq(&fingerprint) {
                                            let message = message::Announce::new(
                                                user.clone().into(),
                                                format!(
                                                    "banned {} from the server",
                                                    member.user.username()
                                                ),
                                            );
                                            messages.push(message.into());
                                            member.exit()?;
                                        }
                                    }
                                }
                                BanAttribute::Ip(_) => todo!(),
                            }
                        }
                    }
                }

                let message = message::System::new(
                    user.into(),
                    "Banning is complete. Offline users were silently banned.".to_string(),
                );
                messages.push(message.into());

                for message in messages {
                    room.send_message(message).await?;
                }
            }
            Command::Banned => 'label: {
                use std::fmt::Write;

                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let (names, fingerprints) = auth.banned();
                let mut banned = String::new();
                write!(banned, "Banned:").expect("Failed to write banned members to string");

                for name in names {
                    write!(banned, "{} \"name={}\"", utils::NEWLINE, name)
                        .expect("Failed to write banned members to string");
                }

                for fingerprint in fingerprints {
                    write!(banned, "{} \"fingerprint={}\"", utils::NEWLINE, fingerprint)
                        .expect("Failed to write banned members to string");
                }

                let message = message::System::new(user.into(), banned);
                room.send_message(message.into()).await?;
            }
            Command::Whitelist(command) => 'label: {
                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                exec_whitelist_command(command, &user, room, auth).await?;
            }
            Command::Oplist(command) => 'label: {
                if !auth.is_op(&user.public_key().clone().into()) {
                    let message =
                        message::Error::new(user.into(), "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                exec_oplist_command(command, &user, room, auth).await?;
            }
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}

async fn exec_whitelist_command(
    command: &WhitelistCommand,
    user: &User,
    room: &mut ChatRoom,
    auth: &mut Auth,
) -> anyhow::Result<()> {
    match command {
        WhitelistCommand::On => {
            auth.enable_whitelist_mode();
            let message = message::System::new(
                user.into(),
                "Server whitelisting is now enabled".to_string(),
            );
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Off => {
            auth.disable_whitelist_mode();
            let message = message::System::new(
                user.into(),
                "Server whitelisting is now disabled".to_string(),
            );
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Add(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.add_trusted_key(pk.into()),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    let username = UserName::from(user);
                    match room.try_find_member(&username).map(|m| &m.user) {
                        Some(user) => auth.add_trusted_key(user.public_key().clone().into()),
                        None => invalid_users.push(user.to_string()),
                    }
                }
            }

            let mut messages = vec![];
            if !invalid_keys.is_empty() {
                messages.push(format!("Invalid keys: {}", invalid_keys.join(", ")));
            }
            if !invalid_users.is_empty() {
                messages.push(format!("Invalid users: {}", invalid_users.join(", ")));
            }
            if messages.is_empty() {
                messages.push(format!("Server whitelist is updated!"));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Remove(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.remove_trusted_key(pk.into()),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    let username = UserName::from(user);
                    match room.try_find_member(&username).map(|m| &m.user) {
                        Some(user) => auth.remove_trusted_key(user.public_key().clone()),
                        None => invalid_users.push(user.to_string()),
                    }
                }
            }

            let mut messages = vec![];
            if !invalid_keys.is_empty() {
                messages.push(format!("Invalid keys: {}", invalid_keys.join(", ")));
            }
            if !invalid_users.is_empty() {
                messages.push(format!("Invalid users: {}", invalid_users.join(", ")));
            }
            if messages.is_empty() {
                messages.push(format!("Server whitelist is updated!"));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Load(mode) => {
            if *mode == WhitelistLoadMode::Replace {
                auth.clear_trusted_keys();
            }
            let message: Message = match auth.load_trusted_keys() {
                Ok(_) => {
                    let body = "Trusted keys are up-to-date with the whitelist file".to_string();
                    message::System::new(user.into(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.into(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        WhitelistCommand::Save => {
            let message: Message = match auth.save_trusted_keys() {
                Ok(_) => {
                    let body = "Whitelist file is up-to-date with the trusted keys".to_string();
                    message::System::new(user.into(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.into(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        WhitelistCommand::Reverify => 'label: {
            if !auth.is_whitelist_enabled() {
                let message = message::System::new(
                    user.into(),
                    "Whitelist is disabled, so nobody will be kicked".to_string(),
                );
                room.send_message(message.into()).await?;
                break 'label;
            }

            let auth = auth;
            let mut kicked = vec![];
            for (_, member) in room.members_iter() {
                if !auth.is_trusted(&member.user.public_key()) {
                    kicked.push(member.user.clone());
                    member.exit()?;
                }
            }

            for user in kicked {
                let message = message::Announce::new(
                    user.into(),
                    "was kicked during pubkey reverification".to_string(),
                );
                room.send_message(message.into()).await?;
            }
        }
        WhitelistCommand::Status => {
            let auth = auth;
            let mut messages: Vec<String> = vec![];

            messages.push(
                String::from("Server whitelisting is ")
                    + match auth.is_whitelist_enabled() {
                        true => "enabled",
                        false => "disabled",
                    },
            );

            let mut trusted_online_users: Vec<String> = vec![];
            let mut trusted_keys = vec![];

            for key in auth.trusted_keys() {
                if let Some(user) = room
                    .members_iter()
                    .map(|(_, m)| &m.user)
                    .find(|u| u.public_key() == key)
                {
                    trusted_online_users.push(user.username().clone().into());
                } else {
                    trusted_keys.push(key.fingerprint());
                }
            }

            if !trusted_online_users.is_empty() {
                messages.push(format!(
                    "Trusted online users: {}",
                    trusted_online_users.join(", ")
                ));
            }

            if !trusted_keys.is_empty() {
                messages.push(format!("Trusted offline keys: {}", trusted_keys.join(", ")));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Help => {
            let mut help = format!("Available commands: {}", utils::NEWLINE);
            help.push_str(&format_commands(&VISIBLE_WHITELIST_COMMANDS));

            let message = message::System::new(user.into(), help);
            room.send_message(message.into()).await?;
        }
    }

    Ok(())
}

async fn exec_oplist_command(
    command: &OplistCommand,
    user: &User,
    room: &mut ChatRoom,
    auth: &mut Auth,
) -> anyhow::Result<()> {
    match command {
        OplistCommand::Add(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.add_operator(pk.into()),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    let username = UserName::from(user);
                    match room.try_find_member(&username).map(|m| &m.user) {
                        Some(user) => auth.add_operator(user.public_key().clone()),
                        None => invalid_users.push(user.to_string()),
                    }
                }
            }

            let mut messages = vec![];
            if !invalid_keys.is_empty() {
                messages.push(format!("Invalid keys: {}", invalid_keys.join(", ")));
            }
            if !invalid_users.is_empty() {
                messages.push(format!("Invalid users: {}", invalid_users.join(", ")));
            }

            if messages.is_empty() {
                messages.push(format!("Server operators list is updated!"));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Remove(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.remove_operator(pk.into()),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    let username = UserName::from(user);
                    match room.try_find_member(&username).map(|m| &m.user) {
                        Some(user) => auth.remove_operator(user.public_key().clone()),
                        None => invalid_users.push(user.to_string()),
                    }
                }
            }

            let mut messages = vec![];
            if !invalid_keys.is_empty() {
                messages.push(format!("Invalid keys: {}", invalid_keys.join(", ")));
            }
            if !invalid_users.is_empty() {
                messages.push(format!("Invalid users: {}", invalid_users.join(", ")));
            }
            if messages.is_empty() {
                messages.push(format!("Server operators list is updated!"));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Load(mode) => {
            if *mode == OplistLoadMode::Replace {
                auth.clear_operators();
            }
            let message: Message = match auth.load_operators() {
                Ok(_) => {
                    let body = "Operators keys are up-to-date with the oplist file".to_string();
                    message::System::new(user.into(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.into(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        OplistCommand::Save => {
            let message: Message = match auth.save_operators() {
                Ok(_) => {
                    let body = "Oplist file is up-to-date with the operators".to_string();
                    message::System::new(user.into(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.into(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        OplistCommand::Status => {
            let auth = auth;
            let mut messages: Vec<String> = vec![];
            let mut online_operators: Vec<String> = vec![];
            let mut offline_keys = vec![];

            for key in auth.operators() {
                if let Some(user) = room
                    .members_iter()
                    .map(|(_, m)| &m.user)
                    .find(|u| u.public_key() == key)
                {
                    online_operators.push(user.username().clone().into());
                } else {
                    offline_keys.push(key.fingerprint());
                }
            }

            if !online_operators.is_empty() {
                messages.push(format!("Online operators: {}", online_operators.join(", ")));
            }

            if !offline_keys.is_empty() {
                messages.push(format!(
                    "Operators offline keys: {}",
                    offline_keys.join(", ")
                ));
            }

            let message = message::System::new(user.into(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Help => {
            let mut help = format!("Available commands: {}", utils::NEWLINE);
            help.push_str(&format_commands(&VISIBLE_OPLIST_COMMANDS));

            let message = message::System::new(user.into(), help);
            room.send_message(message.into()).await?;
        }
    }

    Ok(())
}
