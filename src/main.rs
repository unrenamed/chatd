use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;

use ansi_to_tui::IntoText;
use async_trait::async_trait;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::layout::{Direction, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Terminal;
use russh::{server::*, MethodSet};
use russh::{Channel, ChannelId};
use russh_keys::key::PublicKey;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;
use tui_textarea::TextArea;

mod kmp;
mod utils;
use kmp::KMP;

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

static MOTD_FILEPATH: &'static str = "/Users/nazarposhtarenko/Developer/ssh-chat/motd.ans";
static WHITELIST_FILEPATH: &'static str = "/Users/nazarposhtarenko/Developer/ssh-chat/whitelist";

enum ChatEvent {
    ClientConnected(ClientConnectedEvent),
    ClientLeft(ClientLeftEvent),
    Message(MessageEvent),
}

impl ChatEvent {
    fn format_line(&self, current_username: &str) -> Line {
        match self {
            ChatEvent::ClientConnected(event) => event.format_line(current_username),
            ChatEvent::ClientLeft(event) => event.format_line(current_username),
            ChatEvent::Message(event) => event.format_line(current_username),
        }
    }
}

struct ClientConnectedEvent {
    username: String,
    total_connected: usize,
}

struct ClientLeftEvent {
    username: String,
    session_duration: i64,
}

struct MessageEvent {
    username: String,
    message: String,
}

trait Displayable {
    fn format_line(&self, current_username: &str) -> Line;
}

impl Displayable for ClientConnectedEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!(
                " * {} joined. (Connected: {})",
                self.username, self.total_connected
            ),
            Style::default().fg(Color::DarkGray),
        )])
    }
}

impl Displayable for ClientLeftEvent {
    fn format_line(&self, _: &str) -> Line {
        Line::from(vec![Span::styled(
            format!(
                " * {} left. ({})",
                self.username,
                utils::datetime::format_distance_to_now(self.session_duration)
            ),
            Style::default().fg(Color::DarkGray),
        )])
    }
}

impl Displayable for MessageEvent {
    fn format_line(&self, current_username: &str) -> Line {
        let (r, g, b) = utils::rgb::to_rgb(&self.username);
        let username_span = Span::styled(
            format!("{}: ", self.username),
            Style::default().fg(Color::Rgb(r, g, b)),
        );

        let pattern = format!("@{}", current_username);
        let matches = KMP::new(&pattern).search(&self.message);
        let mut message_spans =
            utils::message::split_by_indices(&self.message, &matches, pattern.len());

        let mut spans = vec![username_span];
        spans.append(&mut message_spans);

        Line::from(spans)
    }
}

struct App {
    pub username: String,
    pub created_at: i64,
    pub input: Vec<u8>,
}

impl App {
    pub fn new() -> App {
        Self {
            username: String::new(),
            created_at: chrono::offset::Utc::now().timestamp(),
            input: vec![],
        }
    }
}

#[derive(Clone)]
struct TerminalHandle {
    handle: Handle,
    // The sink collects the data which is finally flushed to the handle.
    sink: Vec<u8>,
    channel_id: ChannelId,
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let handle = self.handle.clone();
        let channel_id = self.channel_id;
        let data = self.sink.clone().into();
        futures::executor::block_on(async move {
            let result = handle.data(channel_id, data).await;
            if result.is_err() {
                eprintln!("Failed to send data: {:?}", result);
            }
        });

        self.sink.clear();
        Ok(())
    }
}

#[derive(Clone)]
struct AppServer {
    id: usize,
    username: String,
    clients: Arc<Mutex<HashMap<usize, (SshTerminal, App)>>>,
    events: Arc<Mutex<Vec<ChatEvent>>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            id: 0,
            username: String::new(),
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
                                        Constraint::Percentage(10),
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
                                lines.push(event.format_line(&app.username));
                            }

                            let user_input = match std::str::from_utf8(app.input.as_slice()) {
                                Ok(v) => v,
                                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                            };
                            let (r, g, b) = utils::rgb::to_rgb(&app.username);
                            let username_span = Span::styled(format!("[{}]: ", app.username),Style::default().fg(Color::Rgb(r, g, b)));
                            let user_input_span = Span::styled(user_input, Style::default());
                            let line = Line::from(vec![username_span, user_input_span]);
                            lines.push(line);

                            let text = Text::from(lines);
                            let p = Paragraph::new(text);
                            f.render_widget(p, chunks[1]);

                            // let mut textarea = TextArea::from([s]);
                            // textarea.set_cursor_line_style(Style::default());
                            // textarea.set_placeholder_text("Enter your message...");
                            // textarea.set_style(Style::default().fg(Color::LightGreen));
                            // textarea.set_block(Block::default().borders(Borders::ALL));
                            // let widget = textarea.widget();
                            // f.render_widget(widget, chunks[2]);
                        })
                        .unwrap();

                    // WORKAROUND: move cursor to the next line after the textarea
                    // let (x, y) = terminal.get_cursor().unwrap();
                    // terminal.set_cursor(x, y + 1).unwrap();
                    // terminal.show_cursor().unwrap();
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

        let raw_whitelist = parse_file(WHITELIST_FILEPATH).unwrap();
        let whitelist = raw_whitelist
            .iter()
            .map(|line| split_ssh_key(line))
            .filter(|key| key.is_some())
            .map(|key| key.unwrap())
            .map(|(_, key, _)| russh_keys::parse_public_key_base64(&key))
            .filter(|key| key.is_ok())
            .map(|key| key.unwrap())
            .collect::<Vec<PublicKey>>();
        self.whitelist = Arc::new(Mutex::new(whitelist));

        self.run_on_address(Arc::new(config), ("0.0.0.0", 2222))
            .await?;
        Ok(())
    }
}

impl Server for AppServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

#[async_trait]
impl Handler for AppServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        {
            let mut clients = self.clients.lock().await;
            let mut events = self.events.lock().await;

            let terminal_handle = TerminalHandle {
                handle: session.handle(),
                sink: Vec::new(),
                channel_id: channel.id(),
            };

            let mut app = App::new();
            app.username = self.username.clone();

            let backend = CrosstermBackend::new(terminal_handle.clone());
            let mut terminal = Terminal::new(backend)?;
            terminal.clear().unwrap();
            terminal.show_cursor().unwrap();

            clients.insert(self.id, (terminal, app));

            let client_connected_event = ClientConnectedEvent {
                username: format!("{}", self.username),
                total_connected: clients.len(),
            };
            events.push(ChatEvent::ClientConnected(client_connected_event));
        }

        Ok(true)
    }

    async fn auth_publickey(&mut self, user: &str, pk: &PublicKey) -> Result<Auth, Self::Error> {
        self.username = String::from(user);

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
        let mut decoder = Decoder::new();

        for keycode in decoder.write(data[0]) {
            print![
                "code={:?} bytes={:?} printable={:?}\r\n",
                keycode,
                keycode.bytes(),
                keycode.printable()
            ];
            match keycode {
                KeyCode::Enter => {
                    let mut clients = self.clients.lock().await;
                    let mut events = self.events.lock().await;
                    let (_, app) = clients.get_mut(&self.id).unwrap();

                    if app.input.starts_with(&[0x2f]) {
                        let exit_command_bytes = [0x2f, 0x65, 0x78, 0x69, 0x74];
                        let matching = app
                            .input
                            .iter()
                            .zip(&exit_command_bytes)
                            .filter(|&(a, b)| a == b)
                            .count();

                        if matching == exit_command_bytes.len() {
                            let curr_timestamp = chrono::offset::Utc::now().timestamp();
                            let session_duration = curr_timestamp - &app.created_at;

                            let client_left_event = ClientLeftEvent {
                                username: self.username.clone(),
                                session_duration,
                            };
                            
                            app.input.clear();
                            events.push(ChatEvent::ClientLeft(client_left_event));
                            clients.remove(&self.id);
                            session.close(channel);
                            return Ok(());
                        }
                    }

                    let message = match std::str::from_utf8(app.input.as_slice()) {
                        Ok(v) => String::from(v),
                        Err(_) => String::new(),
                    };
                    let message_event = MessageEvent {
                        username: self.username.clone(),
                        message,
                    };
                    events.push(ChatEvent::Message(message_event));

                    app.input.clear();
                }
                KeyCode::Backspace => {
                    let mut clients = self.clients.lock().await;
                    let (_, app) = clients.get_mut(&self.id).unwrap();
                    app.input.pop();
                }
                KeyCode::Char(_) | KeyCode::Tab | KeyCode::Space => {
                    let mut clients = self.clients.lock().await;
                    let (_, app) = clients.get_mut(&self.id).unwrap();
                    app.input.extend_from_slice(data);
                }
                _ => {}
            }
        }

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
            let (terminal, _) = clients.get_mut(&self.id).unwrap();
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

#[tokio::main]
async fn main() {
    let mut server = AppServer::new();
    server.run().await.expect("Failed running server");
}

fn parse_file(file_path: &str) -> Result<Vec<Vec<u8>>, std::io::Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut result = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let bytes = line.into_bytes();
        result.push(bytes);
    }

    Ok(result)
}

fn split_ssh_key(ssh_key_bytes: &[u8]) -> Option<(String, String, String)> {
    // Convert the vector of bytes into a string for easier manipulation
    let ssh_key_string = match String::from_utf8(ssh_key_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return None, // Invalid UTF-8 bytes
    };

    // Split the SSH key string by whitespace
    let parts: Vec<&str> = ssh_key_string.split_whitespace().collect();

    // Ensure that there are at least 3 parts
    if parts.len() < 3 {
        return None; // Invalid SSH key format
    }

    // Convert each part back to a vector of bytes
    let algo = parts[0].to_string();
    let key = parts[1].to_string();
    let name = parts[2..].join(" ");

    Some((algo, key, name))
}
