use std::sync::Arc;

use log::{error, info};
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
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::auth;
use crate::terminal::TerminalHandle;

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

    #[cfg(test)]
    pub fn connect_username(&self) -> &String {
        &self.connect_username
    }

    #[cfg(test)]
    pub fn public_key(&self) -> &Option<PublicKey> {
        &self.public_key
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
        let terminal_handle = TerminalHandle::new(channel.id(), session.handle());

        let sender = self.repo_event_sender.clone();
        let (session_event_tx, session_event_rx) = tokio::sync::mpsc::channel(100);
        self.session_event_sender = Some(session_event_tx);

        let auth = self.auth.lock().await;
        let is_op = match &self.public_key {
            Some(key) => auth.is_op(key),
            None => false,
        };

        tokio::spawn(async move {
            let event = SessionRepositoryEvent::NewSession(
                id,
                ssh_id,
                connect_username,
                is_op,
                key,
                terminal_handle,
                session_event_rx,
            );
            if let Err(err) = sender.send(event).await {
                error!("Failed to send NewSession event for channel {id}: {err}");
            }
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
        if !auth.is_whitelist_enabled() {
            return Ok(Auth::Accept);
        }

        if auth.is_trusted(pk) && !auth.check_bans(&user, &pk) {
            return Ok(Auth::Accept);
        }

        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY),
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

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        info!(
            "Password auth request for user {} using credentials {}",
            user, password
        );
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY),
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
            proceed_with_methods: Some(MethodSet::PUBLICKEY),
        })
    }

    #[allow(unused_variables)]
    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let data = data.to_vec();
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            if let Err(err) = sender.send(SessionEvent::Data(data)).await {
                error!("Failed to send Data event for channel {channel}: {err}");
            }
        });

        Ok(())
    }

    #[allow(unused_variables)]
    async fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        modes: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            let event = SessionEvent::WindowResize(col_width as u16, row_height as u16);
            if let Err(err) = sender.send(event).await {
                error!("Failed to send WindowResize event for channel {channel}: {err}");
            }
        });

        Ok(())
    }

    #[allow(unused_variables)]
    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            let event = SessionEvent::WindowResize(col_width as u16, row_height as u16);
            if let Err(err) = sender.send(event).await {
                error!("Failed to send WindowResize event for channel {channel}: {err}");
            }
        });

        Ok(())
    }

    #[allow(unused_variables)]
    async fn env_request(
        &mut self,
        channel: ChannelId,
        variable_name: &str,
        variable_value: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let name = variable_name.to_string();
        let value = variable_value.to_string();

        let sender = self
            .session_event_sender
            .clone()
            .expect("Session event sender to be initialized during session creation");

        tokio::spawn(async move {
            let event = SessionEvent::Env(name, value);
            if let Err(err) = sender.send(event).await {
                error!("Failed to send Env event for channel {channel}: {err}");
            }
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
            let channel = self.id;
            tokio::spawn(async move {
                if let Err(err) = sender.send(SessionEvent::Disconnect).await {
                    error!("Failed to send Disconnect event for channel {channel}: {err}");
                }
            });
        }
    }
}
