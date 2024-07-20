use async_trait::async_trait;
use std::io::Write;

use super::handler::WorkflowHandler;
use super::WorkflowContext;

use crate::auth::Auth;
use crate::chat::{
    ChatRoom, Command, CommandProps, OplistCommand, OplistLoadMode, Theme, TimestampMode,
    WhitelistCommand, WhitelistLoadMode, CHAT_COMMANDS, NOOP_CHAT_COMMANDS, OPLIST_COMMANDS,
    WHITELIST_COMMANDS,
};
use crate::terminal::{CloseHandle, Terminal};

pub struct Autocomplete<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> Autocomplete<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    pub fn new() -> Self {
        Self { next: None }
    }
}

#[async_trait]
impl<H> WorkflowHandler<H> for Autocomplete<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal<H>,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        let input_str = terminal.input.to_string();
        if input_str.trim().is_empty() {
            return Ok(());
        }

        let cursor_pos = terminal.input.cursor_byte_pos();
        if cursor_pos == 0 {
            return Ok(());
        }

        let re = regex::Regex::new(r"\S+\s*|\s+").unwrap();
        let mut words_iter = re.find_iter(&input_str).map(|mat| mat.as_str());

        let cmd = words_iter.next().unwrap_or(&input_str);
        let (cmd_prefix, cmd_end_pos, cmd_prefix_end_pos) = get_argument_details(cmd, 0);

        let commands = match auth.is_op(&context.user.public_key()) {
            true => CHAT_COMMANDS.clone(),
            false => NOOP_CHAT_COMMANDS.clone(),
        };
        let complete_cmd = match commands.iter().find(|c| c.has_prefix(&cmd_prefix)) {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        if cursor_pos > 0 && cursor_pos <= cmd_prefix_end_pos {
            paste_complete_text(terminal, cmd_end_pos, &complete_cmd.cmd())?;
            return Ok(());
        }

        match complete_cmd {
            Command::Whitelist(_) => 'label: {
                let subcmd = words_iter.next().unwrap_or_default();
                let (subcmd_prefix, subcmd_end_pos, subcmd_prefix_end_pos) =
                    get_argument_details(subcmd, cmd_end_pos);

                let complete_subcmd = match WHITELIST_COMMANDS
                    .iter()
                    .find(|c| c.has_prefix(&subcmd_prefix))
                {
                    Some(cmd) => cmd,
                    None => break 'label,
                };

                if cursor_pos > cmd_end_pos && cursor_pos <= subcmd_prefix_end_pos {
                    paste_complete_text(terminal, subcmd_end_pos, &complete_subcmd.cmd())?;
                    break 'label;
                }

                match complete_subcmd {
                    WhitelistCommand::Add(_) | WhitelistCommand::Remove(_) => {
                        let mut prev_name_end_pos = subcmd_end_pos;
                        while let Some(name) = words_iter.next() {
                            let new_name_end_pos = prev_name_end_pos + name.len();
                            complete_argument(name, prev_name_end_pos, terminal, |prefix| {
                                room.find_name_by_prefix(prefix, context.user.username().as_ref())
                            })?;
                            prev_name_end_pos = new_name_end_pos;
                        }
                    }
                    WhitelistCommand::Load(_) => {
                        let mode = words_iter.next().unwrap_or_default();
                        complete_argument(mode, subcmd_end_pos, terminal, |prefix| {
                            WhitelistLoadMode::from_prefix(prefix)
                        })?;
                    }
                    _ => break 'label,
                }
            }
            Command::Oplist(_) => 'label: {
                let subcmd = words_iter.next().unwrap_or_default();
                let (subcmd_prefix, subcmd_end_pos, subcmd_prefix_end_pos) =
                    get_argument_details(subcmd, cmd_end_pos);

                let complete_subcmd = match OPLIST_COMMANDS
                    .iter()
                    .find(|c| c.has_prefix(&subcmd_prefix))
                {
                    Some(cmd) => cmd,
                    None => break 'label,
                };

                if cursor_pos > cmd_end_pos && cursor_pos <= subcmd_prefix_end_pos {
                    paste_complete_text(terminal, subcmd_end_pos, &complete_subcmd.cmd())?;
                    break 'label;
                }

                match complete_subcmd {
                    OplistCommand::Add(_) | OplistCommand::Remove(_) => {
                        let mut prev_name_end_pos = subcmd_end_pos;
                        while let Some(name) = words_iter.next() {
                            let new_name_end_pos = prev_name_end_pos + name.len();
                            complete_argument(name, prev_name_end_pos, terminal, |prefix| {
                                room.find_name_by_prefix(prefix, context.user.username().as_ref())
                            })?;
                            prev_name_end_pos = new_name_end_pos;
                        }
                    }
                    OplistCommand::Load(_) => {
                        let mode = words_iter.next().unwrap_or_default();
                        complete_argument(mode, subcmd_end_pos, terminal, |prefix| {
                            OplistLoadMode::from_prefix(prefix)
                        })?;
                    }
                    _ => {}
                }
            }
            Command::Timestamp(_) => {
                let mode = words_iter.next().unwrap_or_default();
                complete_argument(mode, cmd_end_pos, terminal, |prefix| {
                    TimestampMode::from_prefix(prefix)
                })?;
            }
            Command::Theme(_) => {
                let theme = words_iter.next().unwrap_or_default();
                complete_argument(theme, cmd_end_pos, terminal, |prefix| {
                    Theme::from_prefix(prefix)
                })?;
            }
            cmd if cmd.args().starts_with("<user>") || cmd.args().starts_with("[user]") => {
                let user = words_iter.next().unwrap_or_default();
                complete_argument(user, cmd_end_pos, terminal, |prefix| {
                    room.find_name_by_prefix(prefix, context.user.username().as_ref())
                })?;
            }
            _ => {}
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}

fn complete_argument<'a, F, T, H: Clone + Write + CloseHandle + Send>(
    arg: &str,
    prev_arg_end_pos: usize,
    terminal: &mut Terminal<H>,
    get_completion: F,
) -> anyhow::Result<()>
where
    F: Fn(&str) -> Option<T>,
    T: ToString,
{
    let cursor_pos = terminal.input.cursor_byte_pos();
    let (arg_prefix, arg_end_pos, arg_prefix_end_pos) = get_argument_details(arg, prev_arg_end_pos);
    if cursor_pos > prev_arg_end_pos && cursor_pos <= arg_prefix_end_pos {
        if let Some(complete) = get_completion(&arg_prefix).map(|c| c.to_string()) {
            paste_complete_text(terminal, arg_end_pos, &complete)?;
        }
    }

    Ok(())
}

fn get_argument_details(arg: &str, prev_arg_end_pos: usize) -> (String, usize, usize) {
    let arg_prefix = arg.trim().to_string();
    let arg_end_pos = prev_arg_end_pos + arg.len();
    let whitespace_count = arg.chars().filter(|&c| c.is_whitespace()).count();
    let arg_prefix_end_pos = arg_end_pos - whitespace_count;
    (arg_prefix, arg_end_pos, arg_prefix_end_pos)
}

fn paste_complete_text<H: Clone + Write + CloseHandle + Send>(
    terminal: &mut Terminal<H>,
    end_pos: usize,
    text: &str,
) -> anyhow::Result<()> {
    terminal.input.move_cursor_to(end_pos);
    terminal.input.remove_last_word_before_cursor();
    terminal.input.insert_before_cursor(text.as_bytes());
    terminal.input.insert_before_cursor(" ".as_bytes());
    terminal.print_input_line()?;
    Ok(())
}

#[cfg(test)]
mod should {
    use crate::{chat::User, pubkey::PubKey};
    use mockall::mock;
    use tokio::sync::mpsc;
    use tokio::sync::watch;

    use super::*;

    mock! {
        pub Handle {}

        impl Write for Handle {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
            fn flush(&mut self) -> std::io::Result<()>;
        }

        impl Clone for Handle {
            fn clone(&self) -> Self;
        }

        impl CloseHandle for Handle {
            fn close(&mut self) {}
        }
    }

    macro_rules! setup {
        () => {{
            let user = User::default();
            let auth = Auth::default();
            let terminal = Terminal::new(MockHandle::new());
            let room = ChatRoom::new("Hello Chatters!");
            let context = WorkflowContext::new(user);
            let autocomplete: Autocomplete<MockHandle> = Autocomplete::new();
            (auth, terminal, room, context, autocomplete)
        }};
    }

    #[tokio::test]
    async fn return_ok() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        assert!(autocomplete
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn complete_all_noop_commands() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let prefix_command_map = vec![
            ("/ms", "/msg"),
            ("/ex", "/exit"),
            ("/aw", "/away"),
            ("/ba", "/back"),
            ("/na", "/name"),
            ("/re", "/reply"),
            ("/fo", "/focus"),
            ("/us", "/users"),
            ("/wh", "/whois"),
            ("/th", "/theme"),
            ("/qu", "/quiet"),
            ("/ig", "/ignore"),
            ("/un", "/unignore"),
            ("/ti", "/timestamp"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(14)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal.input.insert_before_cursor(prefix.as_bytes());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("{command} "));
        }
    }

    #[tokio::test]
    async fn complete_all_op_commands() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let prefix_command_map = vec![
            ("/ba", "/ban"),
            ("/mu", "/mute"),
            ("/ki", "/kick"),
            ("/mo", "/motd"),
            ("/bann", "/banned"),
            ("/op", "/oplist"),
            ("/whi", "/whitelist"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(7)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal.input.insert_before_cursor(prefix.as_bytes());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("{command} "));
        }
    }

    #[tokio::test]
    async fn not_complete_op_commands_when_user_is_not_operator() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let prefix_command_map = vec![
            ("/ba", "/ban"),
            ("/mu", "/mute"),
            ("/ki", "/kick"),
            ("/mo", "/motd"),
            ("/bann", "/banned"),
            ("/op", "/oplist"),
            ("/whi", "/whitelist"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(1)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal.input.insert_before_cursor(prefix.as_bytes());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_ne!(terminal.input.to_string(), format!("{command} "));
        }
    }

    #[tokio::test]
    async fn complete_hidden_commands() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let prefix_command_map = vec![
            ("/m", "/me"),
            ("/sl", "/slap"),
            ("/sh", "/shrug"),
            ("/he", "/help"),
            ("/ve", "/version"),
            ("/up", "/uptime"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(6)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal.input.insert_before_cursor(prefix.as_bytes());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("{command} "));
        }
    }

    #[tokio::test]
    async fn complete_theme_argument() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let prefix_full_map = vec![("mo", "mono"), ("co", "colors"), ("ha", "hacker")];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(3)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_full_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/theme ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/theme {command} "));
        }
    }

    #[tokio::test]
    async fn complete_timestamp_argument() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let prefix_full_map = vec![("ti", "time"), ("da", "datetime")];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_full_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/timestamp ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/timestamp {command} "));
        }
    }

    #[tokio::test]
    async fn complete_username_argument() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();

        let mut alice = User::default();
        alice.set_username("alice".into());
        let mut bob = User::default();
        bob.set_username("bob".into());

        let (alice_msg_tx, _) = mpsc::channel(1);
        let (alice_exit_tx, _) = watch::channel(());
        room.join(
            1,
            alice.username().clone().into(),
            PubKey::default(),
            String::default(),
            alice_msg_tx,
            alice_exit_tx,
        )
        .await
        .unwrap();

        let (bob_msg_tx, _) = mpsc::channel(1);
        let (bob_exit_tx, _) = watch::channel(());
        room.join(
            2,
            bob.username().clone().into(),
            PubKey::default(),
            String::default(),
            bob_msg_tx,
            bob_exit_tx,
        )
        .await
        .unwrap();

        let prefix_full_map = vec![("al", "alice"), ("b", "bob")];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        for (prefix, name) in prefix_full_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/msg ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/msg {name} "));
        }
    }

    #[tokio::test]
    async fn complete_whitelist_subcommand() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let prefix_command_map = vec![
            ("o", "on"),
            ("of", "off"),
            ("a", "add"),
            ("re", "remove"),
            ("rev", "reverify"),
            ("s", "save"),
            ("l", "load"),
            ("st", "status"),
            ("h", "help"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(9)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/whitelist ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/whitelist {command} "));
        }
    }

    #[tokio::test]
    async fn complete_oplist_subcommand() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let prefix_command_map = vec![
            ("a", "add"),
            ("re", "remove"),
            ("s", "save"),
            ("l", "load"),
            ("st", "status"),
            ("h", "help"),
        ];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(6)
            .returning(|| Ok(()));

        for (prefix, command) in prefix_command_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/oplist ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/oplist {command} "));
        }
    }

    #[tokio::test]
    async fn complete_whiltelist_load_command_arguments() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let prefix_arg_map = vec![("me", "merge"), ("re", "replace")];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        for (prefix, arg) in prefix_arg_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/whitelist load ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(
                terminal.input.to_string(),
                format!("/whitelist load {arg} ")
            );
        }
    }

    #[tokio::test]
    async fn complete_oplist_load_command_arguments() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let prefix_arg_map = vec![("me", "merge"), ("re", "replace")];

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        for (prefix, arg) in prefix_arg_map {
            terminal.input.clear();
            terminal
                .input
                .insert_before_cursor(&[b"/oplist load ", prefix.as_bytes()].concat());

            let _ = autocomplete
                .handle(&mut context, &mut terminal, &mut room, &mut auth)
                .await;

            assert_eq!(terminal.input.to_string(), format!("/oplist load {arg} "));
        }
    }

    #[tokio::test]
    async fn complete_whitelist_add_command_arguments() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let mut alice = User::default();
        alice.set_username("alice".into());
        let mut bob = User::default();
        bob.set_username("bob".into());

        let (alice_msg_tx, _) = mpsc::channel(1);
        let (alice_exit_tx, _) = watch::channel(());
        room.join(
            1,
            alice.username().clone().into(),
            PubKey::default(),
            String::default(),
            alice_msg_tx,
            alice_exit_tx,
        )
        .await
        .unwrap();

        let (bob_msg_tx, _) = mpsc::channel(1);
        let (bob_exit_tx, _) = watch::channel(());
        room.join(
            2,
            bob.username().clone().into(),
            PubKey::default(),
            String::default(),
            bob_msg_tx,
            bob_exit_tx,
        )
        .await
        .unwrap();

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        terminal.input.clear();
        terminal.input.insert_before_cursor(b"/whitelist add al");
        let _ = autocomplete
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert_eq!(terminal.input.to_string(), format!("/whitelist add alice "));

        terminal.input.insert_before_cursor(b"bo");
        let _ = autocomplete
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert_eq!(
            terminal.input.to_string(),
            format!("/whitelist add alice bob ")
        );
    }

    #[tokio::test]
    async fn complete_oplist_add_command_arguments() {
        let (mut auth, mut terminal, mut room, mut context, mut autocomplete) = setup!();
        auth.add_operator(context.user.public_key().clone());

        let mut alice = User::default();
        alice.set_username("alice".into());
        let mut bob = User::default();
        bob.set_username("bob".into());

        let (alice_msg_tx, _) = mpsc::channel(1);
        let (alice_exit_tx, _) = watch::channel(());
        room.join(
            1,
            alice.username().clone().into(),
            PubKey::default(),
            String::default(),
            alice_msg_tx,
            alice_exit_tx,
        )
        .await
        .unwrap();

        let (bob_msg_tx, _) = mpsc::channel(1);
        let (bob_exit_tx, _) = watch::channel(());
        room.join(
            2,
            bob.username().clone().into(),
            PubKey::default(),
            String::default(),
            bob_msg_tx,
            bob_exit_tx,
        )
        .await
        .unwrap();

        terminal
            .handle()
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .expect_flush()
            .times(2)
            .returning(|| Ok(()));

        terminal.input.clear();
        terminal.input.insert_before_cursor(b"/oplist add al");
        let _ = autocomplete
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert_eq!(terminal.input.to_string(), format!("/oplist add alice "));

        terminal.input.insert_before_cursor(b"bo");
        let _ = autocomplete
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert_eq!(
            terminal.input.to_string(),
            format!("/oplist add alice bob ")
        );
    }
}
