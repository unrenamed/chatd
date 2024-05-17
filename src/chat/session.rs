#[derive(Clone, Debug)]
pub struct UserSession {
    pub started_at: i64,
}

impl UserSession {
    pub fn secs_since_start(&self) -> i64 {
        let curr_timestamp = chrono::offset::Utc::now().timestamp();
        curr_timestamp - self.started_at
    }
}

impl Default for UserSession {
    fn default() -> Self {
        Self {
            started_at: chrono::offset::Utc::now().timestamp(),
        }
    }
}
