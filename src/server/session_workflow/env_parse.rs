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

#[cfg(test)]
mod should {
    use crate::chat::User;
    use crate::server::session_workflow::command_exec::CommandExecutor;
    use mockall::mock;

    use super::*;

    mock! {
        pub Handle {}

        impl Write for Handle {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
            fn flush(&mut self) -> std::io::Result<()>;
        }

        impl Clone for Handle {
            fn clone(&self) -> Self;
        }

        impl CloseHandle for Handle {
            fn close(&mut self) {}
        }
    }

    macro_rules! setup {
        () => {{
            let user = User::default();
            let auth = Auth::default();
            let terminal = Terminal::new(MockHandle::new());
            let room = ChatRoom::new("Hello Chatters!");
            let context = WorkflowContext::new(user);
            (auth, terminal, room, context)
        }};
    }

    #[tokio::test]
    async fn return_ok() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let executor: CommandExecutor<MockHandle> = CommandExecutor::new();
        let mut parser = EnvParser::new("CHATD_THEME".to_string(), "mono".to_string(), executor);

        assert!(parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn return_next_handler() {
        let executor: CommandExecutor<MockHandle> = CommandExecutor::new();
        let mut parser = EnvParser::new("CHATD_THEME".to_string(), "mono".to_string(), executor);
        assert!(parser.next().is_some());
    }

    #[tokio::test]
    async fn add_theme_command_to_context() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let executor: CommandExecutor<MockHandle> = CommandExecutor::new();
        let mut parser = EnvParser::new("CHATD_THEME".to_string(), "mono".to_string(), executor);

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;

        assert_eq!(context.command_str, Some("/theme mono".to_string()));
    }

    #[tokio::test]
    async fn add_timestamp_command_to_context() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let executor: CommandExecutor<MockHandle> = CommandExecutor::new();
        let mut parser = EnvParser::new(
            "CHATD_TIMESTAMP".to_string(),
            "datetime".to_string(),
            executor,
        );

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;

        assert_eq!(context.command_str, Some("/timestamp datetime".to_string()));
    }

    #[tokio::test]
    async fn not_add_command_to_context_when_env_is_not_recognized() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let executor: CommandExecutor<MockHandle> = CommandExecutor::new();
        let mut parser = EnvParser::new("CHATD_INVALID".to_string(), "value".to_string(), executor);

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;

        assert_eq!(context.command_str, None);
    }
}
