use async_trait::async_trait;
use std::io::Write;

use crate::auth::Auth;
use crate::chat::{message, ratelimit, ChatRoom};
use crate::terminal::{CloseHandle, Terminal};

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

#[derive(Default)]
pub struct InputRateChecker<H>
where
    H: Clone + Write + CloseHandle + Send,
{
    next: Option<Box<dyn WorkflowHandler<H>>>,
}

impl<H> InputRateChecker<H>
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
impl<H> WorkflowHandler<H> for InputRateChecker<H>
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
        let no_ratelim_error_msg = format!(
            "User {} should have its own rate-limit in the server room",
            context.user.username
        );

        let rl = room
            .get_ratelimit(context.user.id)
            .expect(no_ratelim_error_msg.as_str());

        if let Err(remaining) = ratelimit::check(rl) {
            let body = format!(
                "rate limit exceeded. Message dropped. Next allowed in {}",
                humantime::format_duration(remaining)
            );
            let message = message::Error::new(context.user.clone().into(), body);
            room.send_message(message.into()).await?;
            self.next = None;
        }

        Ok(())
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>> {
        &mut self.next
    }
}
