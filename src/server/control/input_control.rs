use std::pin::Pin;

use super::command_control::CommandControl;
use super::context::ControlContext;
use super::control_handler::ControlHandler;

use crate::server::room::{message, Command, CommandParseError};
use crate::server::terminal::Terminal;
use crate::server::{ratelimit, ServerRoom};

pub struct InputControl;

const INPUT_MAX_LEN: usize = 1024;

impl ControlHandler for InputControl {
    fn handle<'a>(
        &'a self,
        context: &'a mut ControlContext,
        terminal: &'a mut Terminal,
        room: &'a mut ServerRoom,
    ) -> Pin<Box<dyn futures::Future<Output = Option<Box<dyn ControlHandler>>> + Send + 'a>> {
        Box::pin(async move {
            let user_id = context.user_id;

            let username = match room.try_find_name(&user_id) {
                Some(name) => name,
                None => return None,
            };

            let member = room.find_member(&username);
            let user = member.user.clone();

            let rl = room.ratelims.get(&user_id).unwrap();
            if let Err(remaining) = ratelimit::check(rl) {
                let body = format!(
                    "rate limit exceeded. Message dropped. Next allowed in {}",
                    humantime::format_duration(remaining)
                );
                let message = message::Error::new(user, body);
                room.send_message(message.into()).await;
                return None;
            }

            let input_str = terminal.input.to_string();
            if input_str.trim().is_empty() {
                return None;
            }

            if input_str.len() > INPUT_MAX_LEN {
                let message =
                    message::Error::new(user, "message dropped. Input is too long".to_string());
                room.send_message(message.into()).await;
                return None;
            }

            let cmd = input_str.parse::<Command>();
            match cmd {
                Err(err) if err == CommandParseError::NotRecognizedAsCommand => {
                    terminal.clear_input().unwrap();

                    let message = message::Public::new(user, input_str);
                    room.send_message(message.into()).await;

                    return None;
                }
                Err(err) => {
                    terminal.input.push_to_history();
                    terminal.clear_input().unwrap();

                    let message = message::Command::new(user.clone(), input_str);
                    room.send_message(message.into()).await;
                    let message = message::Error::new(user, format!("{}", err));
                    room.send_message(message.into()).await;

                    return None;
                }
                Ok(_) => {
                    terminal.input.push_to_history();
                    terminal.clear_input().unwrap();

                    let message = message::Command::new(user.clone(), input_str);
                    room.send_message(message.into()).await;

                    context.user = Some(user);
                    context.command = Some(cmd.unwrap());
                    return Some(Box::new(CommandControl) as Box<dyn ControlHandler>);
                }
            }
        })
    }
}
