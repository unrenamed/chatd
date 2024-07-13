use async_trait::async_trait;
use std::fmt::Write;

use crate::auth::{Auth, BanAttribute, BanQuery};
use crate::chat::message::Message;
use crate::chat::{
    format_commands, message, ChatRoom, Command, CommandProps, OplistCommand, OplistLoadMode,
    Theme, TimestampMode, User, UserStatus, WhitelistCommand, WhitelistLoadMode, CHAT_COMMANDS,
    OPLIST_COMMANDS, WHITELIST_COMMANDS,
};
use crate::terminal::Terminal;
use crate::utils::{self, sanitize};

use super::handler::WorkflowHandler;
use super::WorkflowContext;

#[derive(Default)]
pub struct CommandExecutor {
    next: Option<Box<dyn WorkflowHandler>>,
}

#[async_trait]
impl WorkflowHandler for CommandExecutor {
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        let command = match &context.command {
            Some(command) => command,
            None => return Ok(()),
        };
        let user = context.user.clone();
        let username = &user.username;

        match command {
            Command::Exit => {
                let member = room.find_member(username);
                member.exit()?;
            }
            Command::Away(reason) => {
                let member = room.find_member_mut(username);
                member.user.go_away(reason.to_string());

                let message = message::Emote::new(
                    member.user.clone(),
                    format!("has gone away: \"{}\"", reason),
                );
                room.send_message(message.into()).await?;
            }
            Command::Back => {
                let member = room.find_member_mut(username);
                if let UserStatus::Away {
                    reason: _,
                    since: _,
                } = &member.user.status
                {
                    member.user.return_active();
                    let message = message::Emote::new(member.user.clone(), "is back".to_string());
                    room.send_message(message.into()).await?;
                }
            }
            Command::Name(new_name) => 'label: {
                let member = room.find_member_mut(username);
                let user = member.user.clone();
                let new_name = sanitize::name(&new_name);

                if user.username == *new_name {
                    let message = message::Error::new(
                        user,
                        "new name is the same as the original".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                if let Some(_) = room.try_find_member(&new_name) {
                    let message = message::Error::new(
                        user,
                        format!("\"{}\" name is already taken", new_name),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let message = message::Announce::new(
                    user.clone(),
                    format!("user is now known as {}.", new_name),
                );
                room.send_message(message.into()).await?;

                let new_name = new_name.to_string();
                let old_name = user.username;
                let user_id = user.id;

                let member = room.find_member_mut(username);
                member.user.set_username(new_name.clone());
                terminal.set_prompt(&member.user.display_name);

                let member = member.clone();
                room.add_member(new_name.clone(), member);
                room.remove_member(&old_name);
                room.add_name(user_id, new_name);
            }
            Command::Msg(to, msg) => 'label: {
                let from = room.find_member(username).user.clone();

                match room.try_find_member_mut(&to).map(|a| &mut a.user) {
                    None => {
                        let message =
                            message::Error::new(from.clone(), format!("user is not found"));
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(to) if from.id.eq(&to.id) => {
                        let message = message::Error::new(
                            from.clone(),
                            format!("you can't message yourself"),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(to) => {
                        let status = to.status.clone();
                        let name = to.username.clone();

                        to.set_reply_to(from.id);

                        let message =
                            message::Private::new(from.clone(), to.clone(), msg.to_string());
                        room.send_message(message.into()).await?;

                        match status {
                            UserStatus::Away { reason, since: _ } => {
                                let message = message::System::new(
                                    from.clone(),
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

                if from.reply_to.is_none() {
                    let message =
                        message::Error::new(from.clone(), "no message to reply to".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_id = &from.reply_to.unwrap();
                let target_name = room.try_get_name(&target_id);
                if target_name.is_none() {
                    let message =
                        message::Error::new(from.clone(), "user already left the room".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let member = room.find_member(target_name.unwrap());
                let to = member.user.clone();
                let message = message::Private::new(from, to, (*message_body).to_string());
                room.send_message(message.into()).await?;
            }
            Command::Users => {
                let member = room.find_member(username);
                let user = member.user.clone();

                let mut usernames = room.names().values().collect::<Vec<&String>>();
                usernames.sort_by_key(|a| a.to_lowercase());

                let colorized_names = usernames
                    .iter()
                    .map(|u| user.theme.style_username(u).to_string())
                    .collect::<Vec<String>>();

                let body = format!(
                    "{} connected: {}",
                    room.names().len(),
                    colorized_names.join(", ")
                );

                let message = message::System::new(user, body);
                room.send_message(message.into()).await?;
            }
            Command::Whois(target_name) => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = match room
                    .try_find_member(&target_name)
                    .map(|member| &member.user)
                {
                    Some(target) => message::System::new(user, target.to_string()).into(),
                    None => message::Error::new(user, "user not found".to_string()).into(),
                };
                room.send_message(message).await?;
            }
            Command::Slap(target_name) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                if target_name.is_none() {
                    let message = message::Emote::new(
                        user,
                        "hits himself with a squishy banana.".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_name = target_name.as_deref().unwrap();
                let target = room
                    .try_find_member_mut(&target_name)
                    .map(|member| &member.user);

                let message = if let Some(t) = target {
                    message::Emote::new(user, format!("hits {} with a squishy banana.", t.username))
                        .into()
                } else {
                    message::Error::new(user, "that slippin' monkey not in the room".to_string())
                        .into()
                };
                room.send_message(message).await?;
            }
            Command::Shrug => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::Emote::new(user, "¯\\_(◕‿◕)_/¯".to_string());
                room.send_message(message.into()).await?;
            }
            Command::Me(action) => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::Emote::new(
                    user,
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

                let mut to_display: Vec<&Command> = CHAT_COMMANDS
                    .iter()
                    .filter(|cmd| cmd.is_visible())
                    .collect();

                to_display.sort_by(|a, b| a.cmd().len().cmp(&b.cmd().len()));

                let mut help = format!("Available commands: {}", utils::NEWLINE);
                let noop_commands: Vec<&Command> = to_display
                    .iter()
                    .filter(|c| !c.is_op())
                    .map(|c| *c)
                    .collect();
                help.push_str(&format_commands(noop_commands));

                if user.is_op {
                    let op_commands: Vec<&Command> = to_display
                        .iter()
                        .filter(|c| c.is_op())
                        .map(|c| *c)
                        .collect();
                    help.push_str(&format!(
                        "{}{}Operator commands: {}{}",
                        utils::NEWLINE,
                        utils::NEWLINE,
                        utils::NEWLINE,
                        &format_commands(op_commands)
                    ));
                }

                let message = message::System::new(user.clone(), help);
                room.send_message(message.into()).await?;
            }
            Command::Quiet => {
                let member = room.find_member_mut(username);
                member.user.switch_quiet_mode();
                let message = message::System::new(
                    member.user.clone(),
                    match member.user.quiet {
                        true => "Quiet mode is toggled ON",
                        false => "Quiet mode is toggled OFF",
                    }
                    .to_string(),
                );
                room.send_message(message.into()).await?;
            }
            Command::Timestamp(mode) => {
                let member = room.find_member_mut(username);
                member.user.set_timestamp_mode(*mode);
                let message = message::System::new(
                    member.user.clone(),
                    match member.user.timestamp_mode {
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
                let message = message::System::new(user, format!("Set theme: {}", theme));
                member.user.set_theme((*theme).into());
                terminal.set_prompt(&member.user.display_name);
                room.send_message(message.into()).await?;
            }
            Command::Themes => {
                let member = room.find_member(username);
                let user = member.user.clone();
                let message = message::System::new(
                    user,
                    format!("Supported themes: {}", Theme::values().join(", ")),
                );
                room.send_message(message.into()).await?;
            }
            Command::Ignore(target) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                if target.is_none() {
                    let ignored_usernames: Vec<String> = user
                        .ignored
                        .iter()
                        .filter_map(|id| room.try_get_name(id))
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
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target_username = target.as_deref().unwrap();
                match room
                    .try_find_member(&target_username)
                    .map(|a| a.user.id.clone())
                {
                    Some(target_id) if target_id == user.id => {
                        let message = message::Error::new(
                            user.clone(),
                            "you can't ignore yourself".to_string(),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) if user.ignored.contains(&target_id) => {
                        let message = message::System::new(
                            user.clone(),
                            format!("user already in the ignored list"),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    None => {
                        let message =
                            message::Error::new(user.clone(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) => {
                        room.find_member_mut(username)
                            .user
                            .ignored
                            .insert(target_id);
                        let message =
                            message::System::new(user, format!("Ignoring: {}", target_username));
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Unignore(target_username) => 'label: {
                let member = room.find_member(username);
                let user = member.user.clone();

                match room
                    .try_find_member(&target_username)
                    .map(|a| a.user.id.clone())
                {
                    None => {
                        let message =
                            message::Error::new(user.clone(), "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) if !user.ignored.contains(&target_id) => {
                        let message = message::Error::new(
                            user.clone(),
                            "user not in the ignored list yet".to_string(),
                        );
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target_id) => {
                        room.find_member_mut(username)
                            .user
                            .ignored
                            .remove(&target_id);
                        let message = message::System::new(
                            user,
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
                        .focused
                        .iter()
                        .filter_map(|id| room.try_get_name(id))
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
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let target = target.as_deref().unwrap();
                if target == "$" {
                    room.find_member_mut(username).user.focused.clear();
                    let message =
                        message::System::new(user, "Removed focus from all users".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let mut focused = vec![];
                for target_name in target.split(",") {
                    match room
                        .try_find_member(&target_name.to_string())
                        .map(|a| a.user.id.clone())
                    {
                        None => continue,
                        Some(target_id) if target_id == user.id => continue,
                        Some(target_id) if user.focused.contains(&target_id) => continue,
                        Some(target_id) => {
                            room.find_member_mut(username)
                                .user
                                .focused
                                .insert(target_id);

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
                room.send_message(message.into()).await?;
            }
            Command::Version => {
                let message = message::System::new(user, format!("{}", env!("CARGO_PKG_VERSION")));
                room.send_message(message.into()).await?;
            }
            Command::Uptime => {
                let message = message::System::new(user, room.uptime());
                room.send_message(message.into()).await?;
            }
            Command::Mute(target_username) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                match room
                    .try_find_member_mut(&target_username)
                    .map(|a| &mut a.user)
                {
                    None => {
                        let message = message::Error::new(user, "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(target) if target.id == user.id => {
                        let message =
                            message::Error::new(user, "you can't mute yourself".to_string());
                        room.send_message(message.into()).await?;
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
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Motd(new_motd) => 'label: {
                if new_motd.is_none() {
                    let message = message::System::new(user, room.motd().clone());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                if !user.is_op {
                    let message = message::Error::new(
                        user,
                        "must be an operator to modify the MOTD".to_string(),
                    );
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                room.set_motd(new_motd.as_deref().unwrap().to_string());

                let message = message::Announce::new(
                    user.clone(),
                    format!(
                        "set new message of the day: {}-> {}",
                        utils::NEWLINE,
                        room.motd()
                    ),
                );
                room.send_message(message.into()).await?;
            }
            Command::Kick(target_username) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                match room.try_find_member_mut(&target_username) {
                    None => {
                        let message = message::Error::new(user, "user not found".to_string());
                        room.send_message(message.into()).await?;
                        break 'label;
                    }
                    Some(member) => {
                        let message = message::Announce::new(
                            user,
                            format!("kicked {} from the server", target_username),
                        );
                        member.exit()?;
                        room.send_message(message.into()).await?;
                    }
                }
            }
            Command::Ban(query) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let query = query.parse::<BanQuery>();
                if let Err(err) = query {
                    let message = message::Error::new(user, err.to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                let mut messages: Vec<Message> = vec![];

                match query.unwrap() {
                    BanQuery::Single { name, duration } => {
                        match room
                            .try_find_member(&name)
                            .filter(|member| member.user.public_key.is_some())
                        {
                            Some(member) => {
                                auth.ban_fingerprint(
                                    &member.user.public_key.as_ref().unwrap().fingerprint(),
                                    duration,
                                );
                                let message = message::Announce::new(
                                    user.clone(),
                                    format!("banned {} from the server", member.user.username),
                                );
                                member.exit()?;
                                messages.push(message.into());
                            }
                            None => {
                                let message =
                                    message::Error::new(user, "user not found".to_string());
                                room.send_message(message.into()).await?;
                                break 'label;
                            }
                        }
                    }
                    BanQuery::Multiple(items) => {
                        for item in items {
                            match item.attribute {
                                BanAttribute::Name(name) => {
                                    auth.ban_username(&name, item.duration);

                                    for (_, member) in room.members_iter_mut() {
                                        if member.user.username.eq(&name) {
                                            let message = message::Announce::new(
                                                user.clone(),
                                                format!("banned {} from the server", name),
                                            );
                                            messages.push(message.into());
                                            member.exit()?;
                                        }
                                    }
                                }
                                BanAttribute::Fingerprint(fingerprint) => {
                                    auth.ban_fingerprint(&fingerprint, item.duration);

                                    for (_, member) in room.members_iter_mut() {
                                        if let Some(key) = &member.user.public_key {
                                            if key.fingerprint().eq(&fingerprint) {
                                                let message = message::Announce::new(
                                                    user.clone(),
                                                    format!(
                                                        "banned {} from the server",
                                                        member.user.username
                                                    ),
                                                );
                                                messages.push(message.into());
                                                member.exit()?;
                                            }
                                        }
                                    }
                                }
                                BanAttribute::Ip(_) => todo!(),
                            }
                        }
                    }
                }

                let message = message::System::new(
                    user,
                    "Banning is complete. Offline users were silently banned.".to_string(),
                );
                messages.push(message.into());

                for message in messages {
                    room.send_message(message).await?;
                }
            }
            Command::Banned => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
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

                let message = message::System::new(user, banned);
                room.send_message(message.into()).await?;
            }
            Command::Whitelist(command) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                exec_whitelist_command(command, &user, room, auth).await?;
            }
            Command::Oplist(command) => 'label: {
                if !user.is_op {
                    let message = message::Error::new(user, "must be an operator".to_string());
                    room.send_message(message.into()).await?;
                    break 'label;
                }

                exec_oplist_command(command, &user, room, auth).await?;
            }
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
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
                user.clone(),
                "Server whitelisting is now enabled".to_string(),
            );
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Off => {
            auth.disable_whitelist_mode();
            let message = message::System::new(
                user.clone(),
                "Server whitelisting is now disabled".to_string(),
            );
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Add(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];
            let mut no_key_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.add_trusted_key(pk),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    match room.try_find_member(user).map(|m| &m.user) {
                        Some(user) => match &user.public_key {
                            Some(pk) => auth.add_trusted_key(pk.clone()),
                            None => no_key_users.push(user.to_string()),
                        },
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
            if !no_key_users.is_empty() {
                messages.push(format!(
                    "Users w/o public keys: {}",
                    no_key_users.join(", ")
                ));
            }

            if messages.is_empty() {
                messages.push(format!("Server whitelist is updated!"));
            }

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Remove(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];
            let mut no_key_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.remove_trusted_key(pk),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    match room.try_find_member(user).map(|m| &m.user) {
                        Some(user) => match &user.public_key {
                            Some(pk) => auth.remove_trusted_key(pk.clone()),
                            None => no_key_users.push(user.to_string()),
                        },
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
            if !no_key_users.is_empty() {
                messages.push(format!(
                    "Users w/o public keys: {}",
                    no_key_users.join(", ")
                ));
            }

            if messages.is_empty() {
                messages.push(format!("Server whitelist is updated!"));
            }

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Load(mode) => {
            if *mode == WhitelistLoadMode::Replace {
                auth.clear_trusted_keys();
            }
            let message: Message = match auth.load_trusted_keys() {
                Ok(_) => {
                    let body = "Trusted keys are up-to-date with the whitelist file".to_string();
                    message::System::new(user.clone(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.clone(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        WhitelistCommand::Save => {
            let message: Message = match auth.save_trusted_keys() {
                Ok(_) => {
                    let body = "Whitelist file is up-to-date with the trusted keys".to_string();
                    message::System::new(user.clone(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.clone(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        WhitelistCommand::Reverify => 'label: {
            if !auth.is_whitelist_enabled() {
                let message = message::System::new(
                    user.clone(),
                    "Whitelist is disabled, so nobody will be kicked".to_string(),
                );
                room.send_message(message.into()).await?;
                break 'label;
            }

            let auth = auth;
            let mut kicked = vec![];
            for (_, member) in room.members_iter() {
                if member.user.public_key.is_none()
                    || member
                        .user
                        .public_key
                        .as_ref()
                        .is_some_and(|key| !auth.is_trusted(&key))
                {
                    kicked.push(member.user.clone());
                    member.exit()?;
                }
            }

            for user in kicked {
                let message = message::Announce::new(
                    user,
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

            let mut trusted_online_users = vec![];
            let mut trusted_keys = vec![];

            for key in auth.trusted_keys() {
                if let Some(user) = room
                    .members_iter()
                    .map(|(_, m)| &m.user)
                    .find(|u| u.public_key.as_ref().is_some_and(|k| *key == *k))
                {
                    trusted_online_users.push(user.username.clone());
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

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        WhitelistCommand::Help => {
            let mut to_display: Vec<&WhitelistCommand> = WHITELIST_COMMANDS
                .iter()
                .filter(|cmd| cmd.is_visible())
                .collect();

            to_display.sort_by(|a, b| a.cmd().len().cmp(&b.cmd().len()));

            let mut help = format!("Available commands: {}", utils::NEWLINE);
            help.push_str(&format_commands(to_display));

            let message = message::System::new(user.clone(), help);
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
            let mut no_key_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.add_operator(pk),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    match room.try_find_member(user).map(|m| &m.user) {
                        Some(user) => match &user.public_key {
                            Some(pk) => auth.add_operator(pk.clone()),
                            None => no_key_users.push(user.to_string()),
                        },
                        None => invalid_users.push(user.to_string()),
                    }
                    match room.try_find_member_mut(user).map(|m| &mut m.user) {
                        Some(user) => match &user.public_key {
                            Some(_) => user.is_op = true,
                            None => {}
                        },
                        None => {}
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
            if !no_key_users.is_empty() {
                messages.push(format!(
                    "Users w/o public keys: {}",
                    no_key_users.join(", ")
                ));
            }

            if messages.is_empty() {
                messages.push(format!("Server operators list is updated!"));
            }

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Remove(users_or_keys) => {
            let mut invalid_keys = vec![];
            let mut invalid_users = vec![];
            let mut no_key_users = vec![];

            let mut is_key = false;
            for user_or_key in users_or_keys.split_whitespace() {
                if user_or_key.starts_with("ssh-") {
                    is_key = true;
                    continue;
                }

                if is_key {
                    let key = user_or_key;
                    match russh_keys::parse_public_key_base64(&key) {
                        Ok(pk) => auth.remove_operator(pk),
                        Err(_) => invalid_keys.push(key.to_string()),
                    }
                    is_key = false;
                } else {
                    let user = user_or_key;
                    match room.try_find_member(user).map(|m| &m.user) {
                        Some(user) => match &user.public_key {
                            Some(pk) => auth.remove_operator(pk.clone()),
                            None => no_key_users.push(user.to_string()),
                        },
                        None => invalid_users.push(user.to_string()),
                    }
                    match room.try_find_member_mut(user).map(|m| &mut m.user) {
                        Some(user) => match &user.public_key {
                            Some(_) => user.is_op = false,
                            None => {}
                        },
                        None => {}
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
            if !no_key_users.is_empty() {
                messages.push(format!(
                    "Users w/o public keys: {}",
                    no_key_users.join(", ")
                ));
            }

            if messages.is_empty() {
                messages.push(format!("Server operators list is updated!"));
            }

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Load(mode) => {
            if *mode == OplistLoadMode::Replace {
                auth.clear_operators();
            }
            let message: Message = match auth.load_operators() {
                Ok(_) => {
                    let body = "Operators keys are up-to-date with the oplist file".to_string();
                    message::System::new(user.clone(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.clone(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        OplistCommand::Save => {
            let message: Message = match auth.save_operators() {
                Ok(_) => {
                    let body = "Oplist file is up-to-date with the operators".to_string();
                    message::System::new(user.clone(), body).into()
                }
                Err(err) => {
                    let body = err.to_string();
                    message::Error::new(user.clone(), body).into()
                }
            };
            room.send_message(message).await?;
        }
        OplistCommand::Status => {
            let auth = auth;
            let mut messages: Vec<String> = vec![];
            let mut online_operators = vec![];
            let mut offline_keys = vec![];

            for key in auth.operators() {
                if let Some(user) = room
                    .members_iter()
                    .map(|(_, m)| &m.user)
                    .find(|u| u.public_key.as_ref().is_some_and(|k| *key == *k))
                {
                    online_operators.push(user.username.clone());
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

            let message = message::System::new(user.clone(), messages.join(utils::NEWLINE));
            room.send_message(message.into()).await?;
        }
        OplistCommand::Help => {
            let mut to_display: Vec<&OplistCommand> = OPLIST_COMMANDS
                .iter()
                .filter(|cmd| cmd.is_visible())
                .collect();

            to_display.sort_by(|a, b| a.cmd().len().cmp(&b.cmd().len()));

            let mut help = format!("Available commands: {}", utils::NEWLINE);
            help.push_str(&format_commands(to_display));

            let message = message::System::new(user.clone(), help);
            room.send_message(message.into()).await?;
        }
    }

    Ok(())
}
