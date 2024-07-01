use async_trait::async_trait;

use super::handler::WorkflowHandler;
use super::WorkflowContext;

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
    ) {
        let input_str = terminal.input.to_string();
        if input_str.trim().is_empty() {
            return;
        }

        let mut iter = input_str.splitn(3, ' ');
        let cmd = iter.next().unwrap_or(&input_str);
        let name = iter.next().unwrap_or("").trim();

        let cmd_end_pos = cmd.len();
        let name_end_pos = cmd_end_pos + name.len() + 1;
        let cursor_pos = terminal.input.cursor_byte_pos();

        if cmd.starts_with("/") && cursor_pos > 0 && cursor_pos <= cmd_end_pos {
            let completed = room.commands().from_prefix(&cmd);
            if let Some(command) = completed {
                let cmd_bytes = command.cmd().as_bytes();
                terminal.input.move_cursor_to(cmd_end_pos);
                terminal.input.remove_last_word_before_cursor();
                terminal.input.insert_before_cursor(cmd_bytes);
                terminal.print_input_line().unwrap();
            }
        } else if !name.is_empty() && cursor_pos > cmd_end_pos + 1 && cursor_pos <= name_end_pos {
            let completed = room.find_name_by_prefix(name, &context.user.username);
            if let Some(username) = completed {
                let name_bytes = username.as_bytes();
                terminal.input.move_cursor_to(name_end_pos);
                terminal.input.remove_last_word_before_cursor();
                terminal.input.insert_before_cursor(name_bytes);
                terminal.print_input_line().unwrap();
            }
        }
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
