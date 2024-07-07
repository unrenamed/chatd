use crate::server::user::User;
use crate::server::command::Command;

pub struct WorkflowContext {
    pub user: User,
    pub command_str: Option<String>,
    pub command: Option<Command>,
}

impl WorkflowContext {
    pub fn new(user: User) -> Self {
        Self {
            user,
            command_str: None,
            command: None,
        }
    }
}
