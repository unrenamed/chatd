use std::io::Write;

use async_trait::async_trait;

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::terminal::{CloseHandle, Terminal};

use super::WorkflowContext;

// The Handler trait declares a method for building the chain of
// handlers. It also declares a method for executing a request.
#[async_trait]
pub trait WorkflowHandler<H>: Send + Sync
where
    H: Clone + Write + CloseHandle + Send,
{
    async fn execute(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal<H>,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()> {
        match self.handle(context, terminal, room, auth).await {
            Ok(_) => match &mut self.next() {
                Some(next) => next.execute(context, terminal, room, auth).await,
                None => Ok(()),
            },
            Err(err) => Err(err),
        }
    }

    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal<H>,
        room: &mut ChatRoom,
        auth: &mut Auth,
    ) -> anyhow::Result<()>;

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler<H>>>;
}

/// Helps to wrap an object into a boxed type.
pub fn into_next<H: Clone + Write + CloseHandle + Send>(
    handler: impl WorkflowHandler<H> + Sized + 'static,
) -> Option<Box<dyn WorkflowHandler<H>>> {
    Some(Box::new(handler))
}
