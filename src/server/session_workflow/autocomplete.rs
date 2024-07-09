use async_trait::async_trait;

use super::handler::WorkflowHandler;
use super::WorkflowContext;

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::chat::{
    Command, CommandProps, OplistCommand, OplistLoadMode, WhitelistCommand, WhitelistLoadMode,
    CHAT_COMMANDS, OPLIST_COMMANDS, WHITELIST_COMMANDS,
};
use crate::chat::{Theme, TimestampMode};
use crate::terminal::Terminal;

#[derive(Default)]
pub struct Autocomplete {
    next: Option<Box<dyn WorkflowHandler>>,
}

#[async_trait]
impl WorkflowHandler for Autocomplete {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
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
        let complete_cmd = match CHAT_COMMANDS.iter().find(|c| c.has_prefix(&cmd_prefix)) {
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
                                room.find_name_by_prefix(prefix, &context.user.username)
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
                                room.find_name_by_prefix(prefix, &context.user.username)
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
                    room.find_name_by_prefix(prefix, &context.user.username)
                })?;
            }
            _ => {}
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}

fn complete_argument<'a, F, T>(
    arg: &str,
    prev_arg_end_pos: usize,
    terminal: &mut Terminal,
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

fn paste_complete_text(terminal: &mut Terminal, end_pos: usize, text: &str) -> anyhow::Result<()> {
    terminal.input.move_cursor_to(end_pos);
    terminal.input.remove_last_word_before_cursor();
    terminal.input.insert_before_cursor(text.as_bytes());
    terminal.input.insert_before_cursor(" ".as_bytes());
    terminal.print_input_line()?;
    Ok(())
}
