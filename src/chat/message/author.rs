use crate::chat::{User, UserName};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Author {
    id: usize,
    username: UserName,
    is_muted: bool,
}

pub type Recipient = Author;

impl Author {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn username(&self) -> &UserName {
        &self.username
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }
}

impl From<User> for Author {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            is_muted: user.is_muted,
        }
    }
}

impl From<&User> for Author {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            username: user.username.clone(),
            is_muted: user.is_muted,
        }
    }
}
