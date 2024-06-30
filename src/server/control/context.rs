use terminal_keycode::KeyCode;

use crate::server::{
    env::Env,
    room::{Command, User},
};

pub struct ControlContext {
    pub user_id: usize,
    pub code: Option<KeyCode>,
    pub user: Option<User>,
    pub command: Option<Command>,
    pub env: Option<Env>,
}

impl ControlContext {
    pub fn new(user_id: usize) -> Self {
        Self {
            user_id,
            code: None,
            user: None,
            command: None,
            env: None,
        }
    }
}
