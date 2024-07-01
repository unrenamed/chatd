use async_trait::async_trait;

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

use crate::server::room::message;
use crate::server::terminal::Terminal;
use crate::server::ServerRoom;

const INPUT_MAX_LEN: usize = 1024;

#[derive(Default)]
pub struct InputValidator {
    next: Option<Box<dyn WorkflowHandler>>,
}

impl InputValidator {
    pub fn new(next: impl WorkflowHandler + 'static) -> Self {
        Self {
            next: into_next(next),
        }
    }
}

#[async_trait]
impl WorkflowHandler for InputValidator {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ServerRoom,
    ) {
        let input_str = terminal.input.to_string();
        if input_str.trim().is_empty() {
            self.next = None;
        }

        if input_str.len() > INPUT_MAX_LEN {
            let message = message::Error::new(
                context.user.clone(),
                "message dropped. Input is too long".to_string(),
            );
            room.send_message(message.into()).await;
            self.next = None;
        }

        context.command_str = Some(input_str);
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
