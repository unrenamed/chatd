use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use crossterm::cursor::{self};
use crossterm::queue;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use russh::{server::*, MethodSet};
use russh::{Channel, ChannelId};
use russh_keys::key::PublicKey;
use tokio::sync::Mutex;

use crate::chat::app::ChatApp;
use crate::utils;

use self::connection::ServerConnection;
use self::event::*;
use self::input_handler::InputHandler;
use self::terminal::TerminalHandle;

mod command;
mod connection;
mod event;
mod input_handler;
mod terminal;

static MOTD_FILEPATH: &'static str = "./motd.ans";
static WHITELIST_FILEPATH: &'static str = "./whitelist";

#[derive(Clone)]
pub struct AppServer {
    // per-client connection data
    connection: ServerConnection,
    // shared server state (these aren't copied, only the pointers are)
    clients: Arc<Mutex<HashMap<usize, (TerminalHandle, ChatApp)>>>,
    events: Arc<Mutex<Vec<ClientEvent>>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
    motd: Arc<Mutex<String>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            connection: ServerConnection::new(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            whitelist: Arc::new(Mutex::new(Vec::new())),
            motd: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        self.init_motd();
        self.init_whitelist();

        let clients = self.clients.clone();
        let events = self.events.clone();
        let motd = self.motd.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let events_iter = events.lock().await;
                let motd_content = motd.lock().await;

                for (_, (terminal, app)) in clients.lock().await.iter_mut() {
                    queue!(
                        terminal,
                        cursor::SavePosition,
                        cursor::Hide,
                        Clear(ClearType::FromCursorDown)
                    )
                    .unwrap();

                    queue!(
                        terminal,
                        Print("\n\r"),
                        Print(format!("{}\n\r", &motd_content)),
                        Print("\n\r")
                    )
                    .unwrap();

                    for event in events_iter.iter() {
                        let text = event.format(&app.user.username);
                        for part in text.parts.iter() {
                            match part {
                                TextPart::Info(text) => {
                                    queue!(terminal, Print(text)).unwrap();
                                }
                                TextPart::InfoDimmed(text) => {
                                    queue!(
                                        terminal,
                                        SetForegroundColor(Color::DarkGrey),
                                        Print(text),
                                        ResetColor
                                    )
                                    .unwrap();
                                }
                                TextPart::Message(text) => {
                                    queue!(terminal, Print(text),).unwrap();
                                }
                                TextPart::MessageHighlighted(text) => {
                                    queue!(
                                        terminal,
                                        SetBackgroundColor(Color::DarkYellow),
                                        Print(text),
                                        ResetColor
                                    )
                                    .unwrap();
                                }
                                TextPart::Username(text) => {
                                    let (r, g, b) = utils::rgb::gen_rgb(&app.user.username);
                                    queue!(
                                        terminal,
                                        SetForegroundColor(Color::Rgb { r, g, b }),
                                        Print(text),
                                        ResetColor
                                    )
                                    .unwrap();
                                }
                            }
                        }
                        queue!(terminal, Print("\n\r"),).unwrap();
                    }

                    let (r, g, b) = utils::rgb::gen_rgb(&app.user.username);
                    queue!(
                        terminal,
                        SetForegroundColor(Color::Rgb { r, g, b }),
                        Print(format!("[{}]: ", app.user.username)),
                        ResetColor,
                        Print(app.input.to_str())
                    )
                    .unwrap();

                    queue!(terminal, cursor::RestorePosition).unwrap();

                    terminal.flush().unwrap();
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
        self.connection.id += 1;
        s
    }

    fn handle_session_error(&mut self, _error: <Self::Handler as Handler>::Error) {
        eprintln!("{:?}", _error);
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
            let client_id = self.connection.id;
            let username = &self.connection.username;

            // Create a terminal handle for a new client.
            let terminal_handle = TerminalHandle {
                handle: session.handle(),
                sink: Vec::new(),
                channel_id: channel.id(),
            };

            // Create an individual app state for a new client.
            let app = ChatApp::new(username.clone());

            let mut clients = self.clients.lock().await;
            clients.insert(client_id, (terminal_handle, app));

            let mut events = self.events.lock().await;
            events.push(ClientEvent::Connected(ConnectedEvent {
                username: String::from(username.clone()),
                total_connected: clients.len(),
            }));
        }

        Ok(true)
    }

    async fn auth_publickey(&mut self, user: &str, pk: &PublicKey) -> Result<Auth, Self::Error> {
        self.connection.username = String::from(user);

        let whitelist = self.whitelist.lock().await;
        if whitelist.iter().any(|key| key.eq(pk)) {
            return Ok(Auth::Accept);
        }

        Ok(Auth::Reject {
            proceed_with_methods: Some(MethodSet::NONE),
        })
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let input_handler = InputHandler::new(&self.connection.id, &self.clients, &self.events);

        input_handler
            .handle_data(channel, session, data) // TODO: channel and session must be processed by server, not data handler
            .await
            .unwrap();

        Ok(())
    }
}
