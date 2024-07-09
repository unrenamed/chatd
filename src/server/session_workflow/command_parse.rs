use async_trait::async_trait;
use chrono::Utc;

use crate::auth::Auth;
use crate::chat::{message, Command, CommandParseError, ChatRoom};
use crate::terminal::Terminal;

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

#[derive(Default)]
pub struct CommandParser {
    next: Option<Box<dyn WorkflowHandler>>,
}

impl CommandParser {
    pub fn new(next: impl WorkflowHandler + 'static) -> Self {
        Self {
            next: into_next(next),
        }
    }
}

#[async_trait]
impl WorkflowHandler for CommandParser {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
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
                room.find_member_mut(&user.username)
                    .update_last_sent_time(Utc::now());
                let message = message::Public::new(user, input_str);
                room.send_message(message.into()).await?;
            }
            Err(err) => {
                terminal.input.push_to_history();
                terminal.clear_input()?;
                let message = message::Command::new(user.clone(), input_str);
                room.send_message(message.into()).await?;
                let message = message::Error::new(user, format!("{}", err));
                room.send_message(message.into()).await?;
            }
            Ok(command) => {
                terminal.input.push_to_history();
                terminal.clear_input()?;
                let message = message::Command::new(user.clone(), input_str);
                room.send_message(message.into()).await?;
                context.command = Some(command);
            }
        }
        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
