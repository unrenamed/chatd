use std::pin::Pin;

use super::context::ControlContext;
use super::control_handler::ControlHandler;

use crate::server::terminal::Terminal;
use crate::server::ServerRoom;

pub struct AutocompleteControl;

impl ControlHandler for AutocompleteControl {
    fn handle<'a>(
        &'a self,
        context: &'a mut ControlContext,
        terminal: &'a mut Terminal,
        room: &'a mut ServerRoom,
    ) -> Pin<Box<dyn futures::Future<Output = Option<Box<dyn ControlHandler>>> + Send + 'a>> {
        Box::pin(async move {
            let input_str = terminal.input.to_string();
            if input_str.trim().is_empty() {
                return None;
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
            } else if !name.is_empty() && cursor_pos > cmd_end_pos + 1 && cursor_pos <= name_end_pos
            {
                let user = match &context.user {
                    Some(user) => user.clone(),
                    None => match room.try_find_member_by_id(context.user_id) {
                        Some(m) => m.user.clone(),
                        None => return None,
                    },
                };
                let completed = room.find_name_by_prefix(name, &user.username);
                if let Some(username) = completed {
                    let name_bytes = username.as_bytes();
                    terminal.input.move_cursor_to(name_end_pos);
                    terminal.input.remove_last_word_before_cursor();
                    terminal.input.insert_before_cursor(name_bytes);
                    terminal.print_input_line().unwrap();
                }
            }

            None
        })
    }
}
