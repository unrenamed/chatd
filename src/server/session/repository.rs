use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use log::{debug, info, warn};
use russh_keys::key::PublicKey;
use tokio::spawn;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{watch, Mutex};
use tokio::time::sleep;

use crate::server::terminal::TerminalHandle;
use crate::server::ServerRoom;

type SessionId = usize;
type SessionSshId = String;
type SessionConnectUsername = String;
type SessionIsOp = bool;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionEvent {
    Data(Vec<u8>),
    Disconnect,
    WindowResize(u16, u16),
}

pub enum SessionRepositoryEvent {
    NewSession(
        SessionId,
        SessionSshId,
        SessionConnectUsername,
        SessionIsOp,
        Option<PublicKey>,
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
        while let Some(event) = self.repo_event_receiver.recv().await {
            match event {
                SessionRepositoryEvent::NewSession(
                    id,
                    ssh_id,
                    connect_username,
                    is_op,
                    key,
                    handle,
                    event_receiver,
                ) => {
                    let room = room.clone();
                    spawn(async move {
                        room.lock()
                            .await
                            .join(id, connect_username, is_op, key, handle, ssh_id)
                            .await;

                        Self::handle_session(id, room, event_receiver).await;
                    });
                }
            }
        }
    }

    async fn handle_session(
        id: SessionId,
        room: Arc<Mutex<ServerRoom>>,
        event_rx: Receiver<SessionEvent>,
    ) {
        let (exit_tx, exit_rx) = watch::channel(());

        let session_handle = spawn(Self::process_session_events(
            id,
            room.clone(),
            event_rx,
            exit_tx.clone(),
        ));

        let render_handle = spawn(Self::render(id, room, exit_rx.clone()));

        let _ = session_handle.await;
        let _ = render_handle.await;

        debug!("Fell through the session tasks, indicating disconnection on session. Threads are closed");
    }

    async fn process_session_events(
        id: SessionId,
        room: Arc<Mutex<ServerRoom>>,
        mut event_rx: Receiver<SessionEvent>,
        exit_tx: watch::Sender<()>,
    ) {
        info!("Session events processing task for id={id} is started");

        while let Some(event) = event_rx.recv().await {
            match event {
                SessionEvent::Data(data) => {
                    room.lock().await.handle_input(&id, data.as_slice()).await;
                }
                SessionEvent::Disconnect => {
                    room.lock().await.leave(&id).await;
                    room.lock().await.cleanup(&id).await;
                    let _ = exit_tx.send(());
                    info!("Session events processing task for id={id} is finished");
                    return;
                }
                SessionEvent::WindowResize(width, height) => {
                    room.lock().await.handle_window_resize(&id, (width, height));
                }
            }
        }
    }

    async fn render(id: SessionId, room: Arc<Mutex<ServerRoom>>, mut exit_rx: watch::Receiver<()>) {
        info!("Render task for id={id} is started");

        tokio::select! {
            _ = exit_rx.changed() => {
                info!("Render task for id={id} aborted because session is disconnected");
                return;
            }
            _ = async {
                loop {
                    sleep(Duration::from_millis(10)).await;

                    if let Some(app) = room.lock().await.try_find_app_by_id(id) {
                        app.wait_for_messages().await;
                    }
                }
            } => {
                // Warning: This situation is uncommon and should not occur under normal circumstances.
                warn!("Render task for id={id} finished its work");
            }
        }
    }
}
