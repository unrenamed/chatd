use std::pin::Pin;

use terminal_keycode::KeyCode;

use crate::server::terminal::Terminal;

use super::context::ControlContext;
use super::control_handler::ControlHandler;
use super::input_control::InputControl;

pub struct TerminalControl;

impl ControlHandler for TerminalControl {
    fn handle<'a>(
        &'a self,
        context: &'a mut ControlContext,
        terminal: &'a mut Terminal,
        _: &'a mut crate::server::ServerRoom,
    ) -> Pin<Box<dyn futures::Future<Output = Option<Box<dyn ControlHandler>>> + Send + 'a>> {
        Box::pin(async move {
            match context.code {
                KeyCode::Backspace => {
                    terminal.input.remove_before_cursor();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::CtrlA | KeyCode::CtrlArrowLeft | KeyCode::Home => {
                    terminal.input.move_cursor_start();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::CtrlE | KeyCode::CtrlArrowRight | KeyCode::End => {
                    terminal.input.move_cursor_end();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::CtrlD => todo!(),
                KeyCode::CtrlW => {
                    terminal.input.remove_last_word_before_cursor();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::CtrlK => {
                    terminal.input.remove_after_cursor();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::CtrlU => {
                    terminal.clear_input().unwrap();
                }
                KeyCode::CtrlY => {
                    terminal.input.restore();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::ArrowLeft | KeyCode::CtrlB => {
                    terminal.input.move_cursor_prev();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::ArrowRight | KeyCode::CtrlF => {
                    terminal.input.move_cursor_next();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    terminal.input.insert_before_cursor(&context.code.bytes());
                    terminal.write_prompt().unwrap();
                }
                KeyCode::ArrowUp => {
                    terminal.input.set_history_prev();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::ArrowDown => {
                    terminal.input.set_history_next();
                    terminal.write_prompt().unwrap();
                }
                KeyCode::Tab => todo!(),
                KeyCode::Enter => return Some(Box::new(InputControl) as Box<dyn ControlHandler>),
                _ => {}
            }

            None
        })
    }
}
