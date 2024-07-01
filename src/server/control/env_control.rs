use async_trait::async_trait;

use crate::server::env::Env;
use crate::server::room::Command;
use crate::server::terminal::Terminal;
use crate::server::ServerRoom;

use super::command_control::CommandControl;
use super::control_handler::ControlHandler;

pub struct EnvControl;

#[allow(unused_variables)]
#[async_trait]
impl ControlHandler for EnvControl {
    async fn handle<'a>(
        &'a self,
        context: &'a mut super::context::ControlContext,
        terminal: &'a mut Terminal,
        room: &'a mut ServerRoom,
    ) -> Option<Box<dyn ControlHandler>> {
        if context.env.is_none() {
            return None;
        }

        let user = match &context.user {
            Some(user) => user.clone(),
            None => match room.try_find_member_by_id(context.user_id) {
                Some(m) => m.user.clone(),
                None => return None,
            },
        };

        let env = context.env.as_ref().unwrap();
        let cmd_str = match env {
            Env::Theme(theme) => format!("/theme {}", theme),
            Env::Timestamp(mode) => format!("/timestamp {}", mode),
        };

        let cmd = cmd_str.parse::<Command>();

        match cmd {
            Ok(cmd) => {
                context.command = Some(cmd);
                context.user = Some(user);
                Some(Box::new(CommandControl) as Box<dyn ControlHandler>)
            }
            Err(_) => None,
        }
    }
}
