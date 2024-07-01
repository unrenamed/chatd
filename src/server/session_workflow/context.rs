use crate::server::room::{Command, User};

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
