use std::sync::Arc;
use std::time::Duration;

use handler::ThinHandler;
use log::info;
use repository::SessionRepositoryEvent;
use room::ServerRoom;
use russh::server::*;
use russh_keys::key::{KeyPair, PublicKey};
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

pub use repository::SessionRepository;

mod app;
mod command;
mod handler;
mod input;
mod input_history;
mod message;
mod message_history;
mod repository;
mod room;
mod state;
mod terminal;
mod theme;
mod tui;
mod user;

#[derive(Clone)]
pub struct AppServer {
    id_increment: usize,
    port: u16,
    server_keys: Vec<KeyPair>,
    whitelist: Arc<Mutex<Option<Vec<PublicKey>>>>,
    room: Arc<Mutex<ServerRoom>>,
    repo_event_sender: Sender<SessionRepositoryEvent>,
}

impl AppServer {
    pub fn new(
        port: u16,
        server_keys: &[KeyPair],
        whitelist: Option<Vec<PublicKey>>,
        motd: &str,
        repo_event_sender: Sender<SessionRepositoryEvent>,
    ) -> Self {
        Self {
            port,
            id_increment: 0,
            server_keys: server_keys.to_vec(),
            whitelist: Arc::new(Mutex::new(whitelist.map(|w| w.to_vec()))),
            room: Arc::new(Mutex::new(ServerRoom::new(motd))),
            repo_event_sender,
        }
    }

    pub async fn run(&mut self, mut repository: SessionRepository) -> Result<(), anyhow::Error> {
        let room = self.room.clone();

        info!("Spawning a thread to wait for incoming sessions");
        spawn(async move {
            repository.wait_for_sessions(room).await;
        });

        info!("Spawning a thread to render UI to clients terminal handles");
        let room = self.room.clone();
        spawn(async move { tui::render(room).await });

        let config = Config {
            inactivity_timeout: Some(Duration::from_secs(3600)),
            auth_rejection_time: Duration::from_secs(3),
            auth_rejection_time_initial: Some(Duration::from_secs(0)),
            keys: self.server_keys.clone(),
            ..Default::default()
        };

        info!("Server is running on {} port!", self.port);
        self.run_on_address(Arc::new(config), ("0.0.0.0", self.port))
            .await?;

        Ok(())
    }
}

/// Trait used to create new handlers when clients connect
impl Server for AppServer {
    type Handler = ThinHandler;

    fn new_client(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        info!("New client created for peer {:?}", peer_addr);
        self.id_increment += 1;
        Self::Handler::new(
            self.id_increment,
            self.repo_event_sender.clone(),
            self.whitelist.clone(),
        )
    }
}
