use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

use ansi_to_tui::IntoText;
use async_trait::async_trait;
use fnv::FnvHasher;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::layout::{Direction, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Terminal;
use russh::server::*;
use russh::{Channel, ChannelId};
use russh_keys::key::PublicKey;
use terminal_keycode::{Decoder, KeyCode};
use tokio::sync::Mutex;
use tui_textarea::TextArea;

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

static MOTD_FILEPATH: &'static str = "/Users/nazarposhtarenko/Developer/ssh-chat/motd.ans";

struct App {
    pub input: Vec<u8>,
}

impl App {
    pub fn new() -> App {
        Self { input: vec![] }
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
    clients: Arc<Mutex<HashMap<usize, (SshTerminal, App)>>>,
    id: usize,
}

impl AppServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let clients = self.clients.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let clients_len = clients.lock().await.len();

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
                                        Constraint::Percentage(10),
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
                            let mut messages = vec![];
                            messages.push(Message::new("unrenamed", "hi all!"));
                            messages.push(Message::new("nazar", "howdy"));
                            messages.push(Message::new("nazar", "where r u from?"));
                            messages.push(Message::new("unrenamed", "UA"));
                            messages.push(Message::new("bohdan", "hello guys!"));
                            messages.push(Message::new("nazar", "hi bohdan. unrenamed, I'm from USA"));

                            lines.push(Line::from(vec![
                                Span::styled(format!(" * unrenamed joined. (Connected: {})", clients_len), Style::default().fg(Color::DarkGray)),
                            ]));
                            lines.push(Line::from(vec![
                                Span::styled(format!(" * nazar joined. (Connected: {})", clients_len), Style::default().fg(Color::DarkGray)),
                            ]));
                            lines.push(Line::from(vec![
                                Span::styled(format!(" * bohdan joined. (Connected: {})", clients_len), Style::default().fg(Color::DarkGray)),
                            ]));
                            lines.push(Line::from(vec![
                                Span::styled(format!(" * oleg joined. (Connected: {})", clients_len), Style::default().fg(Color::DarkGray)),
                            ]));

                            for message in messages {
                                let (r, g, b) = to_rgb(&message.username);
                                lines.push(Line::from(vec![
                                    Span::styled(format!("{}: ", message.username), Style::default().fg(Color::Rgb(r, g, b))),
                                    Span::styled(message.text, Style::default()),
                                ]));
                            }
                            lines.push(Line::from(vec![
                                Span::styled(format!(" * oleg left. (After: 10 seconds)"), 
                                Style::default().fg(Color::DarkGray)),
                            ]));

                            let text = Text::from(lines);
                            let p = Paragraph::new(text);
                            f.render_widget(p, chunks[1]);

                            let s = match std::str::from_utf8(app.input.as_slice()) {
                                Ok(v) => v,
                                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                            };
                            let mut textarea = TextArea::from([s]);
                            textarea.set_cursor_line_style(Style::default());
                            textarea.set_placeholder_text("Enter your message...");
                            textarea.set_style(Style::default().fg(Color::LightGreen));
                            textarea.set_block(Block::default().borders(Borders::ALL));
                            let widget = textarea.widget();
                            f.render_widget(widget, chunks[2]);
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
            let terminal_handle = TerminalHandle {
                handle: session.handle(),
                sink: Vec::new(),
                channel_id: channel.id(),
            };

            let app = App::new();
            let backend = CrosstermBackend::new(terminal_handle.clone());
            let mut terminal = Terminal::new(backend)?;
            terminal.clear().unwrap();

            clients.insert(self.id, (terminal, app));
        }

        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let mut decoder = Decoder::new();

        println!("{:?}", data);

        for keycode in decoder.write(data[0]) {
            print![
                "code={:?} bytes={:?} printable={:?}\r\n",
                keycode,
                keycode.bytes(),
                keycode.printable()
            ];
            match keycode {
                KeyCode::Char('q') => {
                    // Pressing 'Esc' or 'q' closes the connection.
                    self.clients.lock().await.remove(&self.id);
                    session.close(channel);
                }
                KeyCode::Enter => {
                    let mut clients = self.clients.lock().await;
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
                            self.clients.lock().await.remove(&self.id);
                            session.close(channel);
                            println!("EXIT!!!!");
                        }
                    }
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

fn to_rgb(s: &str) -> (u8, u8, u8) {
    let mut hasher = FnvHasher::default();
    s.hash(&mut hasher);
    let hash = hasher.finish();

    let r = (hash & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = ((hash >> 16) & 0xFF) as u8;
    (r, g, b)
}

struct Message {
    pub text: String,
    pub username: String,
}

impl Message {
    pub fn new(username: &str, text: &str) -> Self {
        Self {
            text: String::from(text),
            username: String::from(username),
        }
    }
}
