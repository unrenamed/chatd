use std::sync::Arc;

use async_trait::async_trait;
use log::{error, info};
use room::ServerRoom;
use russh::{server::*, MethodSet};
use russh::{Channel, ChannelId};
use russh_keys::key::PublicKey;
use tokio::sync::Mutex;

use crate::utils;

use self::client_info::ClientInfo;
use self::terminal::TerminalHandle;

mod app;
mod client_info;
mod command;
mod history;
mod input;
mod message;
mod motd;
mod room;
mod state;
mod terminal;
mod theme;
mod user;

static WHITELIST_FILEPATH: &'static str = "./whitelist";

#[derive(Clone)]
pub struct AppServer {
    client_info: ClientInfo,
    room: Arc<Mutex<ServerRoom>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            client_info: ClientInfo::new(),
            room: Arc::new(Mutex::new(ServerRoom::new())),
            whitelist: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        self.init_whitelist();

        let room = self.room.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let motd = room.lock().await.motd().clone();
                for (_, member) in room.lock().await.apps_mut().iter_mut() {
                    member
                        .render_motd(&motd)
                        .await
                        .unwrap_or_else(|error| error!("{}", error));
                    member
                        .render()
                        .await
                        .unwrap_or_else(|error| error!("{}", error));
                }
            }
        });

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![russh_keys::key::KeyPair::generate_ed25519().unwrap()],
            ..Default::default()
        };

        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }

    fn init_whitelist(&mut self) {
        let raw_whitelist = utils::fs::read_lines(WHITELIST_FILEPATH)
            .expect("Should have been able to read the whitelist file");

        let whitelist = raw_whitelist
            .iter()
            .filter_map(|line| utils::ssh::split_ssh_key(line))
            .filter_map(|(_, key, _)| russh_keys::parse_public_key_base64(&key).ok())
            .collect::<Vec<PublicKey>>();

        self.whitelist = Arc::new(Mutex::new(whitelist));
    }
}

/// Trait used to create new handlers when clients connect
impl Server for AppServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self::Handler {
        let s = self.clone();
        self.client_info.id += 1;
        s
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as Handler>::Error) {
        error!("{:?}", _error);
    }
}

/// Server handler. Each client will have their own handler.
#[async_trait]
impl Handler for AppServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let terminal_handle = TerminalHandle {
            handle: session.handle(),
            sink: Vec::new(),
            channel_id: channel.id(),
        };

        self.room
            .lock()
            .await
            .join(
                self.client_info.id,
                self.client_info.connect_username.clone(),
                self.client_info.fingerprint.clone(),
                terminal_handle,
                session.remote_sshid(),
            )
            .await;

        Ok(true)
    }

    #[allow(unused_variables)]
    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        pk: &PublicKey,
    ) -> Result<Auth, Self::Error> {
        info!("auth_publickey_offered: user: {}", user);
        let whitelist = self.whitelist.lock().await;
        if whitelist.iter().any(|key| key.eq(pk)) {
            return Ok(Auth::Accept);
        }
        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::PUBLICKEY | MethodSet::NONE),
        })
    }

    async fn auth_publickey(&mut self, user: &str, pk: &PublicKey) -> Result<Auth, Self::Error> {
        info!("auth_publickey: user: {}", user);
        self.client_info.connect_username = String::from(user);
        self.client_info.fingerprint = format!("SHA256:{}", pk.fingerprint());
        Ok(Auth::Accept)
    }

    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        info!("auth_none: user: {user}");
        self.client_info.connect_username = String::from(user);
        self.client_info.fingerprint = format!("(no public key)");
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        info!("auth_password: credentials: {}, {}", user, password);
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
        info!("auth_keyboard_interactive: user: {user}");
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
        self.room
            .lock()
            .await
            .handle_input(&self.client_info.id, data)
            .await;

        Ok(())
    }
}
