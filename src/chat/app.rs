use super::{input::UserInput, session::UserSession, user::User};

/// An individual instance of the chat application for every connected client
/// Contains a local, non-shared state for a particular client
#[derive(Clone, Debug, Default)]
pub struct ChatApp {
    pub user: User,
    pub input: UserInput,
    pub session: UserSession,
}

impl ChatApp {
    pub fn new(username: String) -> Self {
        Self {
            user: User::new(username),
            ..Default::default()
        }
    }
}
