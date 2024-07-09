use async_trait::async_trait;

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::server::env::Env;
use crate::terminal::Terminal;

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

pub struct EnvParser {
    name: String,
    value: String,
    next: Option<Box<dyn WorkflowHandler>>,
}

impl EnvParser {
    pub fn new(name: String, value: String, next: impl WorkflowHandler + 'static) -> Self {
        Self {
            name,
            value,
            next: into_next(next),
        }
    }
}

#[async_trait]
impl WorkflowHandler for EnvParser {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
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

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
