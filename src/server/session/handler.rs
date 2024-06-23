use std::sync::Arc;

use anyhow::Ok;
use log::info;
use russh::server::Auth;
use russh::server::Handler;
use russh::server::Msg;
use russh::server::Response;
use russh::server::Session;
use russh::Channel;
use russh::ChannelId;
use russh::MethodSet;
use russh::Pty;
use russh_keys::key::PublicKey;
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::server::auth;
use crate::server::terminal::TerminalHandle;

use super::SessionEvent;
use super::SessionRepositoryEvent;

/// Server handler. Each client will have their own handler.
pub struct ThinHandler {
    id: usize,
    connect_username: String,
    public_key: Option<PublicKey>,
    auth: Arc<Mutex<auth::Auth>>,
    repo_event_sender: Sender<SessionRepositoryEvent>,
    session_event_sender: Option<Sender<SessionEvent>>,
}

impl ThinHandler {
    pub fn new(
        id: usize,
        auth: Arc<Mutex<auth::Auth>>,
        repo_event_sender: Sender<SessionRepositoryEvent>,
    ) -> ThinHandler {
        ThinHandler {
            id,
            connect_username: String::new(),
            public_key: None,
            auth,
            repo_event_sender,
            session_event_sender: None,
        }
    }
}

#[async_trait::async_trait]
impl Handler for ThinHandler {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        info!("Starting a new session id={}", self.id);

        let id = self.id;
        let connect_username = self.connect_username.clone();
        let ssh_id = String::from_utf8_lossy(session.remote_sshid()).to_string();
        let key = self.public_key.clone();

        let terminal_handle = TerminalHandle {
            handle: session.handle(),
            sink: Vec::new(),
            channel_id: channel.id(),
        };

        let sender = self.repo_event_sender.clone();
        let (session_event_tx, session_event_rx) = tokio::sync::mpsc::channel(100);
        self.session_event_sender = Some(session_event_tx);

        let auth = self.auth.lock().await;
        let is_op = match &self.public_key {
            Some(key) => auth.is_op(key),
            None => false,
        };

        spawn(async move {
            sender
                .send(SessionRepositoryEvent::NewSession(
                    id,
                    ssh_id,
                    connect_username,
                    is_op,
                    key,
                    terminal_handle,
                    session_event_rx,
                ))
                .await
                .unwrap();
        });

        Ok(true)
    }

    #[allow(unused_variables)]
    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        pk: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        info!("Public key offered auth request for user {}", user);

        let mut auth = self.auth.lock().await;
        if auth.is_trusted(pk) && !auth.check_bans(&user, &pk) {
            return Ok(Auth::Accept);
        }

        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY | MethodSet::NONE),
        })
    }

    async fn auth_publickey(&mut self, user: &str, pk: &PublicKey) -> Result<Auth, Self::Error> {
        info!(
            "Public key auth request for user {} using key {:?}",
            user, pk
        );
        self.connect_username = String::from(user);
        self.public_key = Some(pk.clone());
        Ok(Auth::Accept)
    }

    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        info!("None auth request for user {}", user);

        let auth = self.auth.lock().await;
        if auth.has_operators() {
            return Ok(Auth::Reject {
                proceed_with_methods: Some(MethodSet::PUBLICKEY),
            });
        }

        self.connect_username = String::from(user);
        self.public_key = None;
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        info!(
            "Password auth request for user {} using credentials {}",
            user, password
        );
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY | MethodSet::NONE),
        })
    }

    #[allow(unused_variables)]
    async fn auth_keyboard_interactive(
        &mut self,
        user: &str,
        submethods: &str,
        response: Option<Response<'async_trait>>,
    ) -> Result<Auth, Self::Error> {
        info!("Keyboard interactive auth request for user {}", user);
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY | MethodSet::NONE),
        })
    }

    async fn data(
        &mut self,
        _: ChannelId,
        data: &[u8],
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let data = data.to_vec();
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            sender.send(SessionEvent::Data(data)).await.unwrap();
        });

        Ok(())
    }

    async fn pty_request(
        &mut self,
        _: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            sender
                .send(SessionEvent::WindowResize(
                    col_width as u16,
                    row_height as u16,
                ))
                .await
                .unwrap();
        });

        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            sender
                .send(SessionEvent::WindowResize(
                    col_width as u16,
                    row_height as u16,
                ))
                .await
                .unwrap();
        });

        Ok(())
    }
}

/// Handles cleanup when the connection ends.
///
/// This implementation of `Drop` is primarily designed to gracefully manage unexpected disconnects,
/// such as when a client abruptly kills the connection without sending a disconnect signal.
///
/// Upon dropping the `ThinHandler`, it will check if there is an associated `session_event_sender`.
/// This ensures that the disconnection event is properly handled even if the connectionc termination
/// was before the session is opened (e.g. one of the authenticated methods rejected the connection)
impl Drop for ThinHandler {
    fn drop(&mut self) {
        if let Some(sender) = &self.session_event_sender {
            info!("Clean up from disconnected session id={}", self.id);
            let sender = sender.clone();
            tokio::spawn(async move {
                sender.send(SessionEvent::Disconnect).await.unwrap();
            });
        }
    }
}
