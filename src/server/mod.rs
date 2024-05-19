use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use ansi_to_tui::IntoText;
use async_trait::async_trait;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::layout::{Direction, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Clear, Paragraph, Wrap};
use ratatui::Terminal;
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

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

static MOTD_FILEPATH: &'static str = "./motd.ans";
static WHITELIST_FILEPATH: &'static str = "./whitelist";

#[derive(Clone)]
pub struct AppServer {
    // per-client connection data
    connection: ServerConnection,
    // shared server state
    clients: Arc<Mutex<HashMap<usize, (SshTerminal, ChatApp)>>>,
    events: Arc<Mutex<Vec<ClientEvent>>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            connection: ServerConnection::new(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            whitelist: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let clients = self.clients.clone();
        let events = self.events.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let events_iter = events.lock().await;
                for (_, (terminal, app)) in clients.lock().await.iter_mut() {
                    terminal
                        .draw(|f| {
                            let size = f.size();
                            f.render_widget(Clear, size);

                            let chunks = Layout::default()
                                .direction(Direction::Vertical)
                                .constraints(
                                    [
                                        Constraint::Percentage(20),
                                        Constraint::Fill(1),
                                    ]
                                    .as_ref(),
                                )
                                .split(size);

                            let motd_buff = std::fs::read(Path::new(MOTD_FILEPATH))
                                .expect("Should have been able to read the file");
                            let motd_out = motd_buff.into_text()
                                .expect("Should have been able to convert text with ANSI color codes to colored text");
                            f.render_widget(motd_out, chunks[0]);

                            let mut lines = vec![];
                            for event in events_iter.iter() {
                                lines.push(event.format_line(&app.user.username));
                            }

                            let user_input = match std::str::from_utf8(app.input.bytes.as_slice()) {
                                Ok(v) => v,
                                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                            };
                            let (r, g, b) = utils::rgb::gen_rgb(&app.user.username);
                            let username_span = Span::styled(format!("[{}]: ", app.user.username),Style::default().fg(Color::Rgb(r, g, b)));
                            let user_input_span = Span::styled(user_input, Style::default());
                            let line = Line::from(vec![username_span, user_input_span]);
                            lines.push(line);

                            let text = Text::from(lines);
                            let p = Paragraph::new(text).wrap(Wrap { trim: true });
                            f.render_widget(p, chunks[1]);
                        })
                        .unwrap();
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

        self.init_whitelist();
        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }

    fn init_whitelist(&mut self) {
        let raw_whitelist = utils::fs::read_lines(WHITELIST_FILEPATH)
            .expect("Should have been able to read the file");

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

            // Create an individual terminal for a new client.
            let backend = CrosstermBackend::new(terminal_handle.clone());
            let mut terminal = Terminal::new(backend)?;
            terminal.clear().unwrap();

            let mut clients = self.clients.lock().await;
            clients.insert(client_id, (terminal, app));

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

    /// The client's window size has changed.
    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            let (terminal, _) = clients.get_mut(&self.connection.id).unwrap();
            let rect = Rect {
                x: 0,
                y: 0,
                width: col_width as u16,
                height: row_height as u16,
            };
            terminal.resize(rect)?;
        }

        Ok(())
    }
}
