use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use super::super::message::Message;

#[derive(Debug, Clone)]
pub struct MessageChannel {
    sender: mpsc::Sender<Message>,
    receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
}

impl MessageChannel {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub async fn send(&self, msg: Message) -> Result<(), mpsc::error::SendError<Message>> {
        self.sender.send(msg).await
    }

    pub async fn receive(&self) -> Result<Message, mpsc::error::TryRecvError> {
        self.receiver.lock().await.try_recv()
    }
}
