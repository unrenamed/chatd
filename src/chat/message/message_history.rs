use circular_buffer::CircularBuffer;

use super::message::Message;

#[derive(Clone)]
pub struct MessageHistory<const SIZE: usize> {
    buf: CircularBuffer<SIZE, Message>,
}

impl<const SIZE: usize> MessageHistory<SIZE> {
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

#[cfg(test)]
mod tests {
    use crate::chat::message::{self, MessageBaseOps};
    use crate::chat::User;

    use super::*;

    fn get_test_system_message(text: &str) -> message::System {
        let user = User::default();
        message::System::new(user.into(), text.to_string())
    }

    #[test]
    fn test_new_message_history() {
        let history = MessageHistory::<1>::new();
        assert_eq!(
            history.buf.len(),
            0,
            "Buffer should be empty on initialization"
        );
    }

    #[test]
    fn test_push_message() {
        let mut history = MessageHistory::<1>::new();
        let message = get_test_system_message("Hello, World!");

        history.push(message.clone().into());
        assert_eq!(history.buf.len(), 1, "Buffer should contain one message");

        let stored_message = history.buf.get(0).unwrap();
        match stored_message {
            Message::System(msg) if msg == &message => (),
            _ => panic!("Stored message should match the pushed message"),
        }
    }

    #[test]
    fn test_push_message_overflow() {
        let mut history = MessageHistory::<5>::new();
        for i in 0..5 {
            history.push(get_test_system_message(&format!("Message {}", i)).into());
        }
        assert_eq!(history.buf.len(), 5, "Buffer should be full");

        history.push(get_test_system_message("Overflow Message").into());
        assert_eq!(
            history.buf.len(),
            5,
            "Buffer should still be full after overflow"
        );

        let first_message = history.buf.get(0).unwrap();
        match first_message {
            Message::System(msg) if msg.message_body() == "Message 1" => (),
            _ => panic!("First message should be the second inserted message after overflow"),
        }
    }

    #[test]
    fn test_iterate_messages() {
        let mut history = MessageHistory::<5>::new();
        let messages: Vec<Message> = (0..5)
            .map(|i| get_test_system_message(&format!("Message {}", i)).into())
            .collect();

        for message in &messages {
            history.push(message.clone());
        }

        let messages_refs: Vec<&Message> = messages.iter().collect();
        let iterated_messages: Vec<&Message> = history.iter().collect();

        assert_eq!(
            iterated_messages, messages_refs,
            "Iterated messages should match pushed messages"
        );
    }

    #[test]
    fn test_iterate_empty() {
        let history = MessageHistory::<1>::new();
        let iterated_messages: Vec<_> = history.iter().collect();
        assert_eq!(
            iterated_messages.len(),
            0,
            "Iterated messages should be empty for a new MessageHistory"
        );
    }
}
