use async_trait::async_trait;
use chrono::Utc;
use std::io::Write;

use crate::auth::Auth;
use crate::chat::{message, ChatRoom, Command, CommandParseError};
use crate::terminal::{CloseHandle, Terminal};

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

#[derive(Default)]
pub struct CommandParser<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> CommandParser<H>
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
impl<H> WorkflowHandler<H> for CommandParser<H>
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
        let user = context.user.clone();

        let command_str = if let Some(str) = &context.command_str {
            str
        } else {
            return Ok(());
        };

        let input_str = terminal.input.to_string();

        match command_str.parse::<Command>() {
            Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                terminal.clear_input()?;
                room.find_member_mut(&user.username())
                    .update_last_sent_time(Utc::now());
                let message = message::Public::new(user.clone().into(), input_str);
                room.send_message(message.into()).await?;
            }
            Err(err) => {
                terminal.input.push_to_history();
                terminal.clear_input()?;
                let message = message::Command::new(user.clone().into(), input_str);
                room.send_message(message.into()).await?;
                let message = message::Error::new(user.clone().into(), format!("{}", err));
                room.send_message(message.into()).await?;
            }
            Ok(command) => {
                terminal.input.push_to_history();
                terminal.clear_input()?;
                let message = message::Command::new(user.clone().into(), input_str);
                room.send_message(message.into()).await?;
                context.command = Some(command);
            }
        }
        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}
