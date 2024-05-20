pub enum ClientEvent {
    Connected(ConnectedEvent),
    Left(LeftEvent),
    GoAway(GoAwayEvent),
    ReturnBack(ReturnBackEvent),
    SendMessage(SendMessageEvent),
    ChangedName(ChangedNameEvent),
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

pub struct ChangedNameEvent {
    pub old_username: String,
    pub new_username: String,
}
