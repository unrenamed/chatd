use control_handler::ControlHandler;

use super::{terminal::Terminal, ServerRoom};

mod context;
mod control_handler;

pub mod autocomplete_control;
pub mod command_control;
pub mod env_control;
pub mod input_control;
pub mod terminal_control;

pub use context::ControlContext;

pub async fn run_control_chain(
    starting_control: Box<dyn ControlHandler>,
    context: &mut ControlContext,
    terminal: &mut Terminal,
    room: &mut ServerRoom,
) {
    let mut handler: Box<dyn ControlHandler> = starting_control;
    loop {
        match handler.handle(context, terminal, room).await {
            Some(next_handler) => handler = next_handler,
            None => break,
        }
    }
}
