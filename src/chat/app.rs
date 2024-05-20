use super::{input::UserInput, theme::Theme, user::User};

/// An individual instance of the chat application for every connected client
/// Contains a local, non-shared state for a particular client
#[derive(Clone, Debug, Default)]
pub struct ChatApp {
    pub user: User,
    pub input: UserInput,
    pub theme: Theme,
    pub history_start_idx: usize,
}

impl ChatApp {
    pub fn new(id: usize, username: String, ssh_client: String, fingerprint: String) -> Self {
        Self {
            user: User::new(id, username, ssh_client, fingerprint),
            ..Default::default()
        }
    }
}
