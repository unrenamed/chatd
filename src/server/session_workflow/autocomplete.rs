use async_trait::async_trait;

use super::handler::WorkflowHandler;
use super::WorkflowContext;

use crate::server::room::{Command, CommandProps, Theme, TimestampMode, CHAT_COMMANDS};
use crate::server::terminal::Terminal;
use crate::server::ServerRoom;

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
        room: &mut ServerRoom,
    ) -> anyhow::Result<()> {
        let input_str = terminal.input.to_string();
        if input_str.trim().is_empty() {
            return Ok(());
        }

        let mut iter = input_str.splitn(3, ' ');
        let cmd = iter.next().unwrap_or(&input_str);
        let arg1 = iter.next().unwrap_or("").trim();

        let completed_cmd = match CHAT_COMMANDS.iter().find(|c| c.has_prefix(cmd)) {
            Some(cmd) => cmd,
            None => return Ok(()),
        };

        let cmd_end_pos = cmd.len();
        let arg1_end_pos = cmd_end_pos + arg1.len() + 1;
        let cursor_pos = terminal.input.cursor_byte_pos();

        if cursor_pos > 0 && cursor_pos <= cmd_end_pos {
            let cmd_bytes = completed_cmd.cmd().as_bytes();
            terminal.input.move_cursor_to(cmd_end_pos);
            terminal.input.remove_last_word_before_cursor();
            terminal.input.insert_before_cursor(cmd_bytes);
            terminal.print_input_line()?;
        } else if !arg1.is_empty() && cursor_pos > cmd_end_pos + 1 && cursor_pos <= arg1_end_pos {
            let completed_arg = match completed_cmd {
                Command::Timestamp(_) => TimestampMode::from_prefix(arg1).map(|m| m.to_string()),
                Command::Theme(_) => Theme::from_prefix(arg1).map(|t| t.to_string()),
                cmd if cmd.args().starts_with("<user>") || cmd.args().starts_with("[user]") => {
                    room.find_name_by_prefix(arg1, &context.user.username)
                }
                _ => None,
            };

            if let Some(arg) = completed_arg {
                terminal.input.move_cursor_to(arg1_end_pos);
                terminal.input.remove_last_word_before_cursor();
                terminal.input.insert_before_cursor(arg.as_bytes());
                terminal.print_input_line()?;
            }
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
