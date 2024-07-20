use std::fmt::Debug;
use std::sync::Arc;

use log::{error, info, trace, warn};
use terminal_keycode::KeyCode;
use tokio::spawn;
use tokio::sync::mpsc::{self, Receiver};
use tokio::sync::{watch, Mutex};

use crate::auth::Auth;
use crate::chat::ChatRoom;
use crate::pubkey::PubKey;
use crate::server::session_workflow::{self, WorkflowContext, WorkflowHandler};
use crate::terminal::{keyboard_decoder, Terminal, TerminalHandle};

type SessionId = usize;
type SessionSshId = String;
type SessionConnectUsername = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionEvent {
    Data(Vec<u8>),
    Disconnect,
    WindowResize(u16, u16),
    Env(String, String),
}

pub enum SessionRepositoryEvent {
    NewSession(
        SessionId,
        SessionSshId,
        SessionConnectUsername,
        PubKey,
        TerminalHandle,
        Receiver<SessionEvent>,
    ),
}

impl Debug for SessionRepositoryEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NewSession(arg0, arg1, arg2, arg3, _arg4, _arg5) => f
                .debug_tuple("NewSession")
                .field(arg0)
                .field(arg1)
                .field(arg2)
                .field(arg3)
                .finish(),
        }
    }
}

pub struct SessionRepository {
    repo_event_receiver: Receiver<SessionRepositoryEvent>,
}

impl SessionRepository {
    pub fn new(repo_event_receiver: Receiver<SessionRepositoryEvent>) -> Self {
        Self {
            repo_event_receiver,
        }
    }

    pub async fn wait_for_sessions(&mut self, room: Arc<Mutex<ChatRoom>>, auth: Arc<Mutex<Auth>>) {
        while let Some(event) = self.repo_event_receiver.recv().await {
            match event {
                SessionRepositoryEvent::NewSession(id, ssh_id, username, pk, handle, event_rx) => {
                    let room = room.clone();
                    let auth = auth.clone();

                    let mut terminal = Terminal::new(handle);
                    let (message_tx, message_rx) = mpsc::channel(100);
                    let (exit_tx, exit_rx) = watch::channel(());

                    spawn(async move {
                        {
                            let mut room = room.lock().await;
                            let join_result = room
                                .join(id, username, pk, ssh_id, message_tx, exit_tx)
                                .await;
                            if let Ok(user) = join_result {
                                terminal.set_prompt(&user.config().display_name());
                            }
                        }
                        Self::handle_session(
                            id, room, auth, terminal, event_rx, message_rx, exit_rx,
                        )
                        .await;
                    });
                }
            }
        }
    }

    async fn handle_session(
        id: SessionId,
        room: Arc<Mutex<ChatRoom>>,
        auth: Arc<Mutex<Auth>>,
        terminal: Terminal<TerminalHandle>,
        event_rx: Receiver<SessionEvent>,
        message_rx: Receiver<String>,
        exit_rx: watch::Receiver<()>,
    ) {
        let terminal = Arc::new(Mutex::new(terminal));
        let (disconnect_tx, disconnect_rx) = watch::channel(());

        let session_handle = spawn(Self::process_session_events(
            id,
            room.clone(),
            auth,
            terminal.clone(),
            event_rx,
            disconnect_tx,
        ));

        let room_handle = spawn(Self::process_room_events(
            id,
            room.clone(),
            terminal,
            message_rx,
            exit_rx,
            disconnect_rx,
        ));

        let _ = session_handle.await;
        let _ = room_handle.await;

        trace!("Fell through the session tasks, indicating disconnection on session. Threads are closed");
    }

    async fn process_session_events(
        id: SessionId,
        room: Arc<Mutex<ChatRoom>>,
        auth: Arc<Mutex<Auth>>,
        terminal: Arc<Mutex<Terminal<TerminalHandle>>>,
        mut event_rx: Receiver<SessionEvent>,
        disconnect_tx: watch::Sender<()>,
    ) {
        info!("Session events processing task for id={id} is started");

        while let Some(event) = event_rx.recv().await {
            match event {
                SessionEvent::Data(data) => {
                    let mut room = room.lock().await;
                    let mut auth = auth.lock().await;
                    let mut term = terminal.lock().await;

                    let user = room.find_member_by_id(id).user.clone();
                    let mut ctx = WorkflowContext::new(user);

                    let mut text_bytes = vec![];
                    let codes = keyboard_decoder::decode_bytes_to_codes(&data);
                    for code in codes {
                        if let Err(err) = match code {
                            KeyCode::Char(_) | KeyCode::Space => {
                                text_bytes = [text_bytes, code.bytes()].concat();
                                Ok(())
                            }
                            KeyCode::Tab => {
                                session_workflow::autocomplete()
                                    .execute(&mut ctx, &mut term, &mut room, &mut auth)
                                    .await
                            }
                            KeyCode::Enter => {
                                session_workflow::input_submit()
                                    .execute(&mut ctx, &mut term, &mut room, &mut auth)
                                    .await
                            }
                            _ => {
                                session_workflow::emacs_key(code)
                                    .execute(&mut ctx, &mut term, &mut room, &mut auth)
                                    .await
                            }
                        } {
                            error!("Failed to execute workflow for user {}: {}", id, err);
                        }
                    }

                    if !text_bytes.is_empty() {
                        term.input.insert_before_cursor(&text_bytes);
                        if let Err(err) = term.print_input_line() {
                            error!("Failed to execute workflow for user {}: {}", id, err);
                        }
                    }
                }
                SessionEvent::Env(name, value) => {
                    let mut room = room.lock().await;
                    let mut auth = auth.lock().await;
                    let mut term = terminal.lock().await;

                    let user = room.find_member_by_id(id).user.clone();
                    let mut ctx = WorkflowContext::new(user);

                    if let Err(err) = session_workflow::env(name, value)
                        .execute(&mut ctx, &mut term, &mut room, &mut auth)
                        .await
                    {
                        error!("Failed to execute env workflow for user {}: {}", id, err);
                    }
                }
                SessionEvent::Disconnect => {
                    let _ = disconnect_tx.send(());
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

    async fn process_room_events(
        id: SessionId,
        room: Arc<Mutex<ChatRoom>>,
        terminal: Arc<Mutex<Terminal<TerminalHandle>>>,
        mut message_rx: Receiver<String>,
        mut exit_rx: watch::Receiver<()>,
        mut disconnect_rx: watch::Receiver<()>,
    ) {
        info!("Render task for id={id} is started");

        tokio::select! {
            _ = exit_rx.changed() => {
                terminal.lock().await.exit();
                if let Err(err) = room.lock().await.leave(&id).await {
                    error!("Failed to exit the server by user {}: {}", id, err);
                }
                info!("Render task for id={id} aborted because session is closed by a user");
                return;
            }
            _ = disconnect_rx.changed() => {
                if let Err(err) = room.lock().await.leave(&id).await {
                    error!("Failed to disconnect user {} from the server: {}", id, err);
                }
                info!("Render task for id={id} aborted because session is disconnected");
                return;
            }
            _ = async {
                while let Some(msg) = message_rx.recv().await {
                    let _ = terminal.lock().await.print_message(&msg);
                }
            } => {
                // Warning: This situation is uncommon and should not occur under normal circumstances.
                warn!("Render task for id={id} finished its work");
            }
        }
    }
}
