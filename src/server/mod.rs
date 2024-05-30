use std::sync::Arc;

use handler::ThinHandler;
use log::info;
use repository::SessionRepositoryEvent;
use room::ServerRoom;
use russh::server::*;
use russh_keys::key::PublicKey;
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::utils;

pub use repository::SessionRepository;

mod app;
mod command;
mod handler;
mod history;
mod input;
mod message;
mod motd;
mod repository;
mod room;
mod state;
mod terminal;
mod theme;
mod tui;
mod user;

static WHITELIST_FILEPATH: &'static str = "./whitelist";

#[derive(Clone)]
pub struct AppServer {
    id_increment: usize,
    room: Arc<Mutex<ServerRoom>>,
    whitelist: Arc<Mutex<Vec<PublicKey>>>,
    repo_event_sender: Sender<SessionRepositoryEvent>,
}

impl AppServer {
    pub fn new(repo_event_sender: Sender<SessionRepositoryEvent>) -> Self {
        Self {
            id_increment: 0,
            room: Arc::new(Mutex::new(ServerRoom::new())),
            whitelist: Arc::new(Mutex::new(Vec::new())),
            repo_event_sender,
        }
    }

    pub async fn run(&mut self, mut repository: SessionRepository) -> Result<(), anyhow::Error> {
        self.init_whitelist();

        let room = self.room.clone();
        spawn(async move {
            repository.wait_for_sessions(room).await;
        });

        let room = self.room.clone();
        spawn(async move { tui::render(room).await });

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(10)),
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
