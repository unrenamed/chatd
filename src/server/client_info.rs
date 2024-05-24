#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: usize,
    pub connect_username: String,
    pub fingerprint: String,
}

impl ClientInfo {
    pub fn new() -> Self {
        Self {
            id: 0,
            connect_username: String::new(),
            fingerprint: String::new(),
        }
    }
}
