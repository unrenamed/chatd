use control_handler::ControlHandler;
use terminal_control::TerminalControl;

use super::{terminal::Terminal, ServerRoom};

mod autocomplete_control;
mod command_control;
mod context;
mod control_handler;
mod input_control;
mod terminal_control;

pub use context::ControlContext;

pub async fn run_control_chain(
    context: &mut ControlContext,
    terminal: &mut Terminal,
    room: &mut ServerRoom,
) {
    let mut handler: Box<dyn ControlHandler> = Box::new(TerminalControl);
    loop {
        match handler.handle(context, terminal, room).await {
            Some(next_handler) => handler = next_handler,
            None => break,
        }
    }
}
