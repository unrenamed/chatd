use async_trait::async_trait;

use crate::server::{terminal::Terminal, ServerRoom};

use super::WorkflowContext;

// The Handler trait declares a method for building the chain of
// handlers. It also declares a method for executing a request.
#[async_trait]
pub trait WorkflowHandler: Send + Sync {
    async fn execute(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ServerRoom,
    ) -> anyhow::Result<()> {
        match self.handle(context, terminal, room).await {
            Ok(_) => match &mut self.next() {
                Some(next) => next.execute(context, terminal, room).await,
                None => Ok(()),
            },
            Err(err) => Err(err),
        }
    }

    async fn handle(
        &mut self,
        context: &mut WorkflowContext,
        terminal: &mut Terminal,
        room: &mut ServerRoom,
    ) -> anyhow::Result<()>;

    fn next(&mut self) -> &mut Option<Box<dyn WorkflowHandler>>;
}

/// Helps to wrap an object into a boxed type.
pub fn into_next(
    handler: impl WorkflowHandler + Sized + 'static,
) -> Option<Box<dyn WorkflowHandler>> {
    Some(Box::new(handler))
}
