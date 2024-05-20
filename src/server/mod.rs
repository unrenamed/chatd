use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use log::{error, info};
use russh::{server::*, MethodSet};
use russh::{Channel, ChannelId};
use russh_keys::key::PublicKey;
use tokio::sync::Mutex;

use crate::chat::app::ChatApp;
use crate::chat::user::User;
use crate::utils;

use self::client_info::ClientInfo;
use self::input_handler::{InputCallbackAction, InputHandler};
use self::message::Message;
use self::terminal::TerminalHandle;

mod client_info;
mod command;
mod input_handler;
mod message;
mod terminal;
mod tui;

static HISTORY_MAX_LEN: usize = 20;
static MOTD_FILEPATH: &'static str = "./motd.ans";
static WHITELIST_FILEPATH: &'static str = "./whitelist";

#[derive(Clone)]
pub struct AppServer {
    // per-client connection data
    client_info: ClientInfo,
    // shared server state (these aren't copied, only the pointers are)
    clients: Arc<Mutex<HashMap<usize, (TerminalHandle, ChatApp)>>>,
    messages: Arc<Mutex<Vec<Message>>>,
    chat_members: Arc<Mutex<Vec<User>>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
    motd: Arc<Mutex<String>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            client_info: ClientInfo::new(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            chat_members: Arc::new(Mutex::new(Vec::new())),
            whitelist: Arc::new(Mutex::new(Vec::new())),
            motd: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        self.init_motd();
        self.init_whitelist();

        let clients = self.clients.clone();
        let messages = self.messages.clone();
        let motd = self.motd.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let messages_iter: tokio::sync::MutexGuard<Vec<Message>> = messages.lock().await;
                let motd_content = motd.lock().await;

                for (_, (terminal, app)) in clients.lock().await.iter_mut() {
                    tui::render(terminal, app, &messages_iter, &motd_content)
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

    fn init_motd(&mut self) {
        let motd_bytes = std::fs::read(Path::new(MOTD_FILEPATH))
            .expect("Should have been able to read the motd file");

        // hack to normalize line endings into \r\n
        let motd_content = String::from_utf8_lossy(&motd_bytes)
            .replace("\r\n", "\n")
            .replace("\n", "\n\r");

        self.motd = Arc::new(Mutex::new(motd_content));
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
        {
            let client_id = self.client_info.id;
            let mut clients = self.clients.lock().await;
            let mut chat_members = self.chat_members.lock().await;
            let mut messages = self.messages.lock().await;

            let is_member = chat_members
                .iter()
                .any(|m| m.username.eq(&self.client_info.username));

            let username;
            match is_member {
                true => username = self.client_info.gen_rand_name(),
                false => username = self.client_info.username.clone(),
            }

            let terminal_handle = TerminalHandle {
                handle: session.handle(),
                sink: Vec::new(),
                channel_id: channel.id(),
            };

            let mut app = ChatApp::new(
                client_id,
                username,
                String::from_utf8_lossy(session.remote_sshid()).to_string(),
                self.client_info.fingerprint.clone(),
            );

            app.history_start_idx = messages.len().saturating_sub(HISTORY_MAX_LEN);

            let user = app.user.clone();
            chat_members.push(user.clone());
            clients.insert(client_id, (terminal_handle, app));

            messages.push(Message::Announce(message::AnnounceMessage {
                from: user,
                body: format!("joined. (Connected: {})", clients.len()),
            }));
        }

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
        self.client_info.username = String::from(user);
        self.client_info.fingerprint = format!("SHA256:{}", pk.fingerprint());
        Ok(Auth::Accept)
    }

    async fn auth_none(&mut self, user: &str) -> Result<Auth, Self::Error> {
        info!("auth_none: user: {user}");
        self.client_info.username = String::from(user);
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
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let mut clients = self.clients.lock().await;
        let mut chat_members = self.chat_members.lock().await;

        let (_, app) = clients
            .get_mut(&self.client_info.id)
            .expect("Connected client must be registered in the clients list");

        let mut input_handler = InputHandler::new(app, &chat_members, &self.messages);

        match input_handler.handle_data(data).await {
            InputCallbackAction::CloseClientSession => {
                chat_members.retain(|m| !m.id.eq(&self.client_info.id));
                clients.remove(&self.client_info.id);
                session.close(channel);
            }
            InputCallbackAction::NoAction => {}
        }

        Ok(())
    }
}
