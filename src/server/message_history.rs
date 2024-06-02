use circular_buffer::CircularBuffer;

use super::message::Message;

const MESSAGE_HISTORY_LEN: usize = 20;

#[derive(Clone)]
pub struct MessageHistory {
    buf: CircularBuffer<MESSAGE_HISTORY_LEN, Message>,
}

impl MessageHistory {
    pub fn new() -> Self {
        Self {
            buf: CircularBuffer::new(),
        }
    }

    pub fn push(&mut self, message: Message) {
        self.buf.push_back(message)
    }

    pub fn iter(&self) -> circular_buffer::Iter<Message> {
        self.buf.iter()
    }
}
