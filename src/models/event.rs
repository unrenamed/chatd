pub enum ClientEvent {
    Connected(ConnectedEvent),
    Left(LeftEvent),
    GoAway(GoAwayEvent),
    ReturnBack(ReturnBackEvent),
    SendMessage(SendMessageEvent),
}

pub struct ConnectedEvent {
    pub username: String,
    pub total_connected: usize,
}

pub struct LeftEvent {
    pub username: String,
    pub session_duration: i64,
}

pub struct SendMessageEvent {
    pub username: String,
    pub message: String,
}

pub struct GoAwayEvent {
    pub username: String,
    pub reason: String,
}

pub struct ReturnBackEvent {
    pub username: String,
}
