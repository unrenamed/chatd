use std::pin::Pin;

use terminal_keycode::KeyCode;

use crate::server::terminal::Terminal;

use super::autocomplete_control::AutocompleteControl;
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
            if context.code.is_none() {
                return None;
            }

            let code = context.code.as_ref().unwrap();
            match code {
                KeyCode::Backspace => {
                    terminal.input.remove_before_cursor();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::CtrlA | KeyCode::CtrlArrowLeft | KeyCode::Home => {
                    terminal.input.move_cursor_start();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::CtrlE | KeyCode::CtrlArrowRight | KeyCode::End => {
                    terminal.input.move_cursor_end();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::CtrlD => todo!(),
                KeyCode::CtrlW => {
                    terminal.input.remove_last_word_before_cursor();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::CtrlK => {
                    terminal.input.remove_after_cursor();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::CtrlU => {
                    terminal.clear_input().unwrap();
                }
                KeyCode::CtrlY => {
                    terminal.input.restore();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::ArrowLeft | KeyCode::CtrlB => {
                    terminal.input.move_cursor_prev();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::ArrowRight | KeyCode::CtrlF => {
                    terminal.input.move_cursor_next();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::ArrowUp => {
                    terminal.input.set_history_prev();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::ArrowDown => {
                    terminal.input.set_history_next();
                    terminal.print_input_line().unwrap();
                }
                KeyCode::Char(_) | KeyCode::Space => {
                    terminal.input.insert_before_cursor(&code.bytes());
                    terminal.print_input_line().unwrap();
                }
                KeyCode::Tab => {
                    return Some(Box::new(AutocompleteControl) as Box<dyn ControlHandler>)
                }
                KeyCode::Enter => return Some(Box::new(InputControl) as Box<dyn ControlHandler>),
                _ => {}
            }

            None
        })
    }
}