use async_trait::async_trait;
use std::io::Write;

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::server::env::Env;
use crate::terminal::{CloseHandle, Terminal};

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

pub struct EnvParser<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    name: String,
    value: String,
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> EnvParser<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    pub fn new(name: String, value: String, next: impl WorkflowHandler<H> + 'static) -> Self {
        Self {
            name,
            value,
            next: into_next(next),
        }
    }
}

#[async_trait]
impl<H> WorkflowHandler<H> for EnvParser<H>
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
        let env = match format!("{}={}", self.name, self.value).parse::<Env>() {
            Ok(env) => Some(env),
            Err(_) => None,
        };

        if let Some(env) = env {
            let command_str = match env {
                Env::Theme(theme) => format!("/theme {}", theme),
                Env::Timestamp(mode) => format!("/timestamp {}", mode),
            };
            context.command_str = Some(command_str);
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}
