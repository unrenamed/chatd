use async_trait::async_trait;
use terminal_keycode::KeyCode;

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::terminal::Terminal;

use super::handler::WorkflowHandler;
use super::WorkflowContext;

pub struct TerminalKeyMapper {
    key: KeyCode,
    next: Option<Box<dyn WorkflowHandler>>,
}

impl TerminalKeyMapper {
    pub fn new(key: KeyCode) -> Self {
        Self { key, next: None }
    }
}

#[async_trait]
impl WorkflowHandler for TerminalKeyMapper {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        match self.key {
            KeyCode::Backspace => {
                terminal.input.remove_before_cursor();
                terminal.print_input_line()?;
            }
            KeyCode::CtrlA | KeyCode::CtrlArrowLeft | KeyCode::Home => {
                terminal.input.move_cursor_start();
                terminal.print_input_line()?;
            }
            KeyCode::CtrlE | KeyCode::CtrlArrowRight | KeyCode::End => {
                terminal.input.move_cursor_end();
                terminal.print_input_line()?;
            }
            KeyCode::CtrlD => todo!(),
            KeyCode::CtrlW => {
                terminal.input.remove_last_word_before_cursor();
                terminal.print_input_line()?;
            }
            KeyCode::CtrlK => {
                terminal.input.remove_after_cursor();
                terminal.print_input_line()?;
            }
            KeyCode::CtrlU => {
                terminal.clear_input()?;
            }
            KeyCode::CtrlY => {
                terminal.input.restore();
                terminal.print_input_line()?;
            }
            KeyCode::ArrowLeft | KeyCode::CtrlB => {
                terminal.input.move_cursor_prev();
                terminal.print_input_line()?;
            }
            KeyCode::ArrowRight | KeyCode::CtrlF => {
                terminal.input.move_cursor_next();
                terminal.print_input_line()?;
            }
            KeyCode::ArrowUp => {
                terminal.input.set_history_prev();
                terminal.print_input_line()?;
            }
            KeyCode::ArrowDown => {
                terminal.input.set_history_next();
                terminal.print_input_line()?;
            }
            KeyCode::Char(_) | KeyCode::Space => {
                terminal.input.insert_before_cursor(&self.key.bytes());
                terminal.print_input_line()?;
            }
            _ => {}
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
