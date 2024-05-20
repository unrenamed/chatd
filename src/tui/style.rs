pub enum TextPart {
    Info(String),
    InfoDimmed(String),
    Message(String),
    MessageHighlighted(String),
    Username { name: String, display_name: String },
}

pub struct StyledText {
    pub parts: Vec<TextPart>,
}

impl StyledText {
    pub fn new(parts: Vec<TextPart>) -> Self {
        Self { parts }
    }
}
