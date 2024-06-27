use std::fmt::Debug;
use std::sync::Arc;

use log::{debug, info, warn};
use russh_keys::key::PublicKey;
use tokio::spawn;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::{watch, Mutex};

use crate::server::control::{run_control_chain, ControlContext};
use crate::server::terminal::{keyboard_decoder, Terminal, TerminalHandle};
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
                    username,
                    is_op,
                    pk,
                    handle,
                    event_rx,
                ) => {
                    let room = room.clone();
                    let mut terminal = Terminal::new(handle);
                    let (message_tx, message_rx) = mpsc::channel(100);

                    spawn(async move {
                        {
                            let mut room = room.lock().await;
                            let user = room.join(id, username, is_op, pk, ssh_id, message_tx).await;
                            terminal.set_prompt(&terminal.get_prompt(&user));
                        }
                        Self::handle_session(id, room, terminal, event_rx, message_rx).await;
                    });
                }
            }
        }
    }

    async fn handle_session(
        id: SessionId,
        room: Arc<Mutex<ServerRoom>>,
        terminal: Terminal,
        event_rx: Receiver<SessionEvent>,
        message_rx: Receiver<String>,
    ) {
        let (exit_tx, exit_rx) = watch::channel(());
        let terminal = Arc::new(Mutex::new(terminal));

        let session_handle = spawn(Self::process_session_events(
            id,
            room.clone(),
            terminal.clone(),
            event_rx,
            exit_tx.clone(),
        ));

        let message_handle = spawn(Self::process_message_events(
            id,
            terminal,
            message_rx,
            exit_rx.clone(),
        ));

        let _ = session_handle.await;
        let _ = message_handle.await;

        debug!("Fell through the session tasks, indicating disconnection on session. Threads are closed");
    }

    async fn process_session_events(
        id: SessionId,
        room: Arc<Mutex<ServerRoom>>,
        terminal: Arc<Mutex<Terminal>>,
        mut event_rx: Receiver<SessionEvent>,
        exit_tx: watch::Sender<()>,
    ) {
        info!("Session events processing task for id={id} is started");

        while let Some(event) = event_rx.recv().await {
            match event {
                SessionEvent::Data(data) => {
                    let mut room = room.lock().await;
                    let mut terminal = terminal.lock().await;
                    let codes = keyboard_decoder::decode_bytes_to_codes(&data);
                    for code in codes {
                        let mut context = ControlContext::new(id, code);
                        run_control_chain(&mut context, &mut terminal, &mut room).await;
                    }
                }
                SessionEvent::Disconnect => {
                    let mut room = room.lock().await;
                    room.leave(&id).await;
                    let _ = exit_tx.send(());
                    info!("Session events processing task for id={id} is finished");
                    return;
                }
                SessionEvent::WindowResize(width, height) => {
                    let mut terminal = terminal.lock().await;
                    terminal.set_size(width, height);
                }
            }
        }
    }

    async fn process_message_events(
        id: SessionId,
        terminal: Arc<Mutex<Terminal>>,
        mut message_rx: Receiver<String>,
        mut exit_rx: watch::Receiver<()>,
    ) {
        info!("Render task for id={id} is started");

        tokio::select! {
            _ = exit_rx.changed() => {
                info!("Render task for id={id} aborted because session is disconnected");
                return;
            }
            _ = async {
                while let Some(msg) = message_rx.recv().await {
                    let _ = terminal.lock().await.write_message(&msg);
                }
            } => {
                // Warning: This situation is uncommon and should not occur under normal circumstances.
                warn!("Render task for id={id} finished its work");
            }
        }
    }
}
