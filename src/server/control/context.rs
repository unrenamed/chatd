use terminal_keycode::KeyCode;

use crate::server::room::{Command, User};

pub struct ControlContext {
    pub user_id: usize,
    pub code: KeyCode,
    pub user: Option<User>,
    pub command: Option<Command>,
}

impl ControlContext {
    pub fn new(user_id: usize, code: KeyCode) -> Self {
        Self {
            user_id,
            code,
            user: None,
            command: None,
        }
    }
}
