use async_trait::async_trait;

use crate::server::room::message;
use crate::server::terminal::Terminal;
use crate::server::{ratelimit, ServerRoom};

use super::handler::{into_next, WorkflowHandler};
use super::WorkflowContext;

#[derive(Default)]
pub struct InputRateChecker {
    next: Option<Box<dyn WorkflowHandler>>,
}

impl InputRateChecker {
    pub fn new(next: impl WorkflowHandler + 'static) -> Self {
        Self {
            next: into_next(next),
        }
    }
}

#[async_trait]
impl WorkflowHandler for InputRateChecker {
    #[allow(unused_variables)]
    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ServerRoom,
    ) {
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
            let message = message::Error::new(context.user.clone(), body);
            room.send_message(message.into()).await;
            self.next = None;
        }
    }

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>> {
        &mut self.next
    }
}
