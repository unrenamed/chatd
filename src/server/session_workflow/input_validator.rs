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

#[cfg(test)]
mod should {
    use crate::chat::User;
    use crate::server::session_workflow::input_rate_checker::InputRateChecker;
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
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);

        assert!(parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn return_next_handler() {
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);
        assert!(parser.next().is_some());
    }

    #[tokio::test]
    async fn unset_next_handler_when_input_is_empty() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);

        terminal.input.clear();

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert!(parser.next().is_none());
    }

    #[tokio::test]
    async fn unset_next_handler_when_input_is_whitespace_only() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);

        terminal.input.clear();
        terminal.input.insert_before_cursor(b"     ");

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert!(parser.next().is_none());
    }

    #[tokio::test]
    async fn unset_next_handler_when_input_is_too_long() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);

        terminal.input.clear();
        terminal
            .input
            .insert_before_cursor("".repeat(1025).as_bytes());

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;
        assert!(parser.next().is_none());
    }

    #[tokio::test]
    async fn add_command_to_context_when_input_is_valid() {
        let (mut auth, mut terminal, mut room, mut context) = setup!();
        let checker: InputRateChecker<MockHandle> = InputRateChecker::default();
        let mut parser = InputValidator::new(checker);

        terminal.input.clear();
        terminal.input.insert_before_cursor(b"/help");

        let _ = parser
            .handle(&mut context, &mut terminal, &mut room, &mut auth)
            .await;

        assert_eq!(context.command_str, Some("/help".into()));
        assert!(parser.next().is_some());
    }
}
