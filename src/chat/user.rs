#[derive(Clone, Debug)]
pub enum UserStatus {
    Active,
    Away { reason: String },
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Clone, Debug, Default)]
pub struct User {
    pub username: String,
    pub status: UserStatus,
}

impl User {
    pub fn new(username: String) -> Self {
        Self {
            username,
            status: UserStatus::Active,
        }
    }

    pub fn go_away(&mut self, reason: String) {
        self.status = UserStatus::Away { reason };
    }

    pub fn return_active(&mut self) {
        self.status = UserStatus::Active;
    }
}
