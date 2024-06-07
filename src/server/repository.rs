use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use log::info;
use tokio::spawn;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::room::ServerRoom;
use super::terminal::TerminalHandle;

pub type SessionId = usize;
pub type SessionSshId = String;
pub type SessionConnectUsername = String;
pub type SessionFingerprint = String;
pub type SessionIsOp = bool;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionEvent {
    Data(Vec<u8>),
    Disconnect,
}

pub enum SessionRepositoryEvent {
    NewSession(
        SessionId,
        SessionSshId,
        SessionConnectUsername,
        SessionFingerprint,
        SessionIsOp,
        TerminalHandle,
        Receiver<SessionEvent>,
    ),
}

impl Debug for SessionRepositoryEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewSession(arg0, arg1, arg2, arg3, arg4, _arg5, _arg6) => f
                .debug_tuple("NewSession")
                .field(arg0)
                .field(arg1)
                .field(arg2)
                .field(arg3)
                .field(arg4)
                .finish(),
        }
    }
}

pub struct SessionRepository {
    pub repo_event_receiver: Receiver<SessionRepositoryEvent>,
}

impl SessionRepository {
    pub fn new(repo_event_receiver: Receiver<SessionRepositoryEvent>) -> Self {
        Self {
            repo_event_receiver,
        }
    }

    pub async fn wait_for_sessions(&mut self, room: Arc<Mutex<ServerRoom>>) {
        loop {
            let event = self.repo_event_receiver.recv().await;
            if let Some(e) = event {
                match e {
                    SessionRepositoryEvent::NewSession(
                        id,
                        ssh_id,
                        connect_username,
                        fingerprint,
                        is_op,
                        handle,
                        event_receiver,
                    ) => {
                        let room = room.clone();
                        spawn(async move {
                            room.lock()
                                .await
                                .join(id, connect_username, fingerprint, is_op, handle, ssh_id)
                                .await;

                            Self::handle_session(id, room, event_receiver).await;
                        });
                    }
                }
            }
        }
    }

    async fn handle_session(
        id: SessionId,
        room: Arc<Mutex<ServerRoom>>,
        mut event_rx: Receiver<SessionEvent>,
    ) {
        info!("Handling new session id={}", id);

        spawn(async move {
            loop {
                sleep(Duration::from_millis(1)).await;
                while let Ok(event) = event_rx.try_recv() {
                    match event {
                        SessionEvent::Data(data) => {
                            room.lock().await.handle_input(&id, data.as_slice()).await;
                        }
                        SessionEvent::Disconnect => {
                            room.lock().await.leave(&id).await;
                            room.lock().await.cleanup(&id).await;
                        }
                    }
                }
            }
        });
    }
}
