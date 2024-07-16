use async_trait::async_trait;
use std::io::Write;

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

use crate::auth::Auth;
use crate::chat::{message, ChatRoom};
use crate::terminal::{CloseHandle, Terminal};

const INPUT_MAX_LEN: usize = 1024;

#[derive(Default)]
pub struct InputValidator<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> InputValidator<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    pub fn new(next: impl WorkflowHandler<H> + 'static) -> Self {
        Self {
            next: into_next(next),
        }
    }
}

#[async_trait]
impl<H> WorkflowHandler<H> for InputValidator<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal<H>,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        let input_str = terminal.input.to_string();
        if input_str.trim().is_empty() {
            self.next = None;
            return Ok(());
        }

        if input_str.len() > INPUT_MAX_LEN {
            let message = message::Error::new(
                context.user.clone().into(),
                "message dropped. Input is too long".to_string(),
            );
            room.send_message(message.into()).await?;
            self.next = None;
            return Ok(());
        }

        context.command_str = Some(input_str);
        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}
