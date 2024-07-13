use std::sync::Arc;

use log::{error, info};
use russh::server::{Auth, Handler, Msg, Response, Session};
use russh::{Channel, ChannelId, MethodSet, Pty};
use russh_keys::key::PublicKey;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::auth;
use crate::terminal::TerminalHandle;

use super::{SessionEvent, SessionRepositoryEvent};

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
/// This implementation of `Drop` is primarily designed to gracefully
/// manage unexpected disconnects, such as when a client abruptly
/// kills the connection without sending a disconnect signal.
///
/// Upon dropping the `ThinHandler`, it will check if there is an
/// associated `session_event_sender`. This ensures that the
/// disconnection event is properly handled even if the connectionc
/// termination was before the session is opened (e.g. one of the
/// authenticated methods rejected the connection)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth;

    use async_trait::async_trait;
    use client::Config;
    use futures::Future;
    use russh::{client, server, MethodSet};
    use russh_keys::key::{KeyPair, PublicKey};
    use server::{Auth, Config as ServerConfig, Handler};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc::Receiver;
    use tokio::sync::Mutex;
    use tokio::time;

    fn create_public_key() -> PublicKey {
        let key_pair = russh_keys::key::KeyPair::generate_ed25519().unwrap();
        key_pair.clone_public_key().unwrap()
    }

    async fn receive_event<T>(
        receiver: &mut Receiver<T>,
        timeout_duration: Duration,
    ) -> Result<T, &'static str> {
        match time::timeout(timeout_duration, receiver.recv()).await {
            Ok(Some(event)) => Ok(event),
            Ok(None) => Err("Channel closed"),
            Err(_) => Err("Failed to receive event before timeout"),
        }
    }

    async fn test_session<RC, CH, SH, F>(client_handler: CH, server_handler: SH, run_client: RC)
    where
        RC: FnOnce(client::Handle<CH>) -> F + Send + Sync + 'static,
        F: Future<Output = client::Handle<CH>> + Send + Sync + 'static,
        CH: client::Handler + Send + Sync + 'static,
        SH: server::Handler + Send + Sync + 'static,
    {
        // Client configuration
        let client_config = Arc::new(Config::default());
        let client_key = KeyPair::generate_ed25519().unwrap();

        // Server configuration
        let server_key = KeyPair::generate_ed25519().unwrap();
        let mut config = ServerConfig::default();
        config.inactivity_timeout = None;
        config.auth_rejection_time = std::time::Duration::from_secs(3);
        config.keys.push(server_key);
        let server_config = Arc::new(config);
        let server_socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        // Start the mock server
        let server_join = tokio::spawn(async move {
            let (socket, _) = server_socket.accept().await.unwrap();
            server::run_stream(server_config, socket, server_handler)
                .await
                .map_err(|_| ())
                .unwrap()
        });

        // Connect the client to the mock server
        let client_join = tokio::spawn(async move {
            let mut session = client::connect(client_config, server_addr, client_handler)
                .await
                .map_err(|_| ())
                .unwrap();
            let authenticated = session
                .authenticate_publickey("user".to_owned(), Arc::new(client_key))
                .await
                .unwrap();
            assert!(authenticated);
            session
        });

        let (_, client_session) = tokio::join!(server_join, client_join);
        run_client(client_session.unwrap()).await;
    }

    #[tokio::test]
    async fn test_channel_open_session() {
        #[derive(Debug)]
        struct Client {}

        #[async_trait]
        impl client::Handler for Client {
            type Error = russh::Error;

            async fn check_server_key(
                &mut self,
                _server_public_key: &russh_keys::key::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let auth = auth::Auth::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        test_session(Client {}, handler, |c| async move {
            let _ = c.channel_open_session().await;

            let timeout_duration = Duration::from_secs(1);
            match receive_event(&mut rx, timeout_duration).await {
                Ok(event) => assert!(matches!(
                    event,
                    SessionRepositoryEvent::NewSession(
                        id, _, username, is_op, _, _, _
                    ) if id == 1 && username == "user".to_string() && !is_op
                )),
                Err(err) => panic!("{}", err),
            }
            c
        })
        .await;
    }

    #[tokio::test]
    async fn test_env_request() {
        #[derive(Debug)]
        struct Client {}

        #[async_trait]
        impl client::Handler for Client {
            type Error = russh::Error;

            async fn check_server_key(
                &mut self,
                _server_public_key: &russh_keys::key::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let auth = auth::Auth::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        test_session(
            Client {},
            handler,
            |c| async move {
                let channel = c.channel_open_session().await.unwrap();
                let env_vars = vec![
                    ("THEME".to_string(), "mono".to_string()),
                    ("TIMESTAMP".to_string(), "datetime".to_string()),
                ];
                for (name, value) in env_vars {
                    channel.set_env( false, name.as_str(), value.as_str()).await.unwrap();
                }

                let timeout_duration = Duration::from_secs(1);
                match receive_event(&mut rx, timeout_duration).await {
                    Ok(event) => match event {
                        SessionRepositoryEvent::NewSession(_, _, _, _, _, _, mut event_rx) => {
                            match receive_event(&mut event_rx, timeout_duration).await {
                                Ok(event) => assert!(matches!(event, SessionEvent::Env(name, value) if name == "THEME" && value == "mono")),
                                Err(err) => panic!("{}", err),
                            }

                            match receive_event(&mut event_rx, timeout_duration).await {
                                Ok(event) => assert!(matches!(event, SessionEvent::Env(name, value) if name == "TIMESTAMP" && value == "datetime")),
                                Err(err) => panic!("{}", err),
                            }
                        }
                    },
                    Err(err) => panic!("{}", err),
                }
                c
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_pty_request() {
        #[derive(Debug)]
        struct Client {}

        #[async_trait]
        impl client::Handler for Client {
            type Error = russh::Error;

            async fn check_server_key(
                &mut self,
                _server_public_key: &russh_keys::key::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let auth = auth::Auth::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        test_session(
            Client {},
            handler,
            |c| async move {
                let channel = c.channel_open_session().await.unwrap();
                channel.request_pty( false, "xterm", 100, 50, 1, 1, &[]).await.unwrap();

                let timeout_duration = Duration::from_secs(1);
                match receive_event(&mut rx, timeout_duration).await {
                    Ok(event) => match event {
                        SessionRepositoryEvent::NewSession(_, _, _, _, _, _, mut event_rx) => {
                            match receive_event(&mut event_rx, timeout_duration).await {
                                Ok(event) => assert!(matches!(event, SessionEvent::WindowResize(cw, rh) if cw == 100 && rh == 50)),
                                Err(err) => panic!("{}", err),
                            }
                        }
                    },
                    Err(err) => panic!("{}", err),
                }
                c
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_window_change_request() {
        #[derive(Debug)]
        struct Client {}

        #[async_trait]
        impl client::Handler for Client {
            type Error = russh::Error;

            async fn check_server_key(
                &mut self,
                _server_public_key: &russh_keys::key::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let auth = auth::Auth::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        test_session(
            Client {},
            handler,
            |c| async move {
                let channel = c.channel_open_session().await.unwrap();
                channel.window_change(100, 50, 1, 1).await.unwrap();

                let timeout_duration = Duration::from_secs(1);
                match receive_event(&mut rx, timeout_duration).await {
                    Ok(event) => match event {
                        SessionRepositoryEvent::NewSession(_, _, _, _, _, _, mut event_rx) => {
                            match receive_event(&mut event_rx, timeout_duration).await {
                                Ok(event) => assert!(matches!(event, SessionEvent::WindowResize(cw, rh) if cw == 100 && rh == 50)),
                                Err(err) => panic!("{}", err),
                            }
                        }
                    },
                    Err(err) => panic!("{}", err),
                }
                c
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_data_request() {
        #[derive(Debug)]
        struct Client {}

        #[async_trait]
        impl client::Handler for Client {
            type Error = russh::Error;

            async fn check_server_key(
                &mut self,
                _server_public_key: &russh_keys::key::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let auth = auth::Auth::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        test_session(Client {}, handler, |c| async move {
            let channel = c.channel_open_session().await.unwrap();
            let data = &b"Hello, world!"[..];
            channel.data(data).await.unwrap();

            let timeout_duration = Duration::from_secs(1);
            match receive_event(&mut rx, timeout_duration).await {
                Ok(event) => match event {
                    SessionRepositoryEvent::NewSession(_, _, _, _, _, _, mut event_rx) => {
                        match receive_event(&mut event_rx, timeout_duration).await {
                            Ok(event) => {
                                assert!(matches!(event, SessionEvent::Data(bytes) if bytes == data))
                            }
                            Err(err) => panic!("{}", err),
                        }
                    }
                },
                Err(err) => panic!("{}", err),
            }
            c
        })
        .await;
    }

    #[tokio::test]
    async fn test_auth_keyboard_interactive() {
        let auth = auth::Auth::default();
        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());
        let response = handler.auth_keyboard_interactive("user", "", None).await;
        assert!(
            matches!(response, Ok(Auth::Reject { proceed_with_methods }) if proceed_with_methods == Some(MethodSet::PUBLICKEY))
        );
    }

    #[tokio::test]
    async fn test_auth_password() {
        let auth = auth::Auth::default();
        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());
        let response = handler.auth_password("user", "password").await;
        assert!(
            matches!(response, Ok(Auth::Reject { proceed_with_methods }) if proceed_with_methods == Some(MethodSet::PUBLICKEY))
        );
    }

    #[tokio::test]
    async fn test_auth_publickey() {
        let auth = auth::Auth::default();
        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        let pubkey = create_public_key();
        let response = handler.auth_publickey("user", &pubkey).await;

        assert!(matches!(response, Ok(Auth::Accept)));
        assert_eq!(handler.connect_username(), "user");
        assert_eq!(handler.public_key(), &Some(pubkey));
    }

    #[tokio::test]
    async fn test_auth_publickey_offered_when_whitelist_disabled() {
        let mut auth = auth::Auth::default();
        auth.disable_whitelist_mode();

        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());
        let response = handler
            .auth_publickey_offered("user", &create_public_key())
            .await;

        // Any user is allowed when whitelist is disabled
        assert!(matches!(response, Ok(Auth::Accept)));
    }

    #[tokio::test]
    async fn test_auth_publickey_offered_when_whitelist_enabled() {
        let user_pk = create_public_key();
        let guest_pk = create_public_key();
        let banned_user_pk = create_public_key();

        let mut auth = auth::Auth::default();
        auth.enable_whitelist_mode();
        auth.add_trusted_key(user_pk.clone());
        auth.add_trusted_key(banned_user_pk.clone());
        auth.ban_fingerprint(&banned_user_pk.fingerprint(), Duration::from_secs(60));

        let (tx, _) = tokio::sync::mpsc::channel(1);
        let mut handler = ThinHandler::new(1, Arc::new(Mutex::new(auth.clone())), tx.clone());

        // Any trusted and not banned user is allowed
        let response = handler.auth_publickey_offered("user", &user_pk).await;
        assert!(matches!(response, Ok(Auth::Accept)));

        // Any user not in the whitelist is not allowed
        let response = handler.auth_publickey_offered("guest", &guest_pk).await;
        assert!(
            matches!(response, Ok(Auth::Reject { proceed_with_methods }) if proceed_with_methods == Some(MethodSet::PUBLICKEY))
        );

        // Any trusted user whose name or fingerprint is banned is not allowed
        let response = handler
            .auth_publickey_offered("banned_user", &guest_pk)
            .await;
        assert!(
            matches!(response, Ok(Auth::Reject { proceed_with_methods }) if proceed_with_methods == Some(MethodSet::PUBLICKEY))
        );
    }
}
