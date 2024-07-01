use async_trait::async_trait;

use crate::server::{terminal::Terminal, ServerRoom};

use super::context::ControlContext;

#[async_trait]
pub trait ControlHandler: Send {
    async fn handle<'a>(
        &'a self,
        context: &'a mut ControlContext,
        terminal: &'a mut Terminal,
        room: &'a mut ServerRoom,
    ) -> Option<Box<dyn ControlHandler>>;
}
