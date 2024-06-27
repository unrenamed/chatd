use crate::server::{terminal::Terminal, ServerRoom};
use std::future::Future;
use std::pin::Pin;

use super::context::ControlContext;

pub trait ControlHandler: Send {
    fn handle<'a>(
        &'a self,
        context: &'a mut ControlContext,
        terminal: &'a mut Terminal,
        room: &'a mut ServerRoom,
    ) -> Pin<Box<dyn Future<Output = Option<Box<dyn ControlHandler>>> + Send + 'a>>;
}
