use std::sync::Arc;
use std::time::Duration;

use log::info;
use russh::server::{Config, Server};
use russh_keys::key::KeyPair;
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

use crate::auth::Auth;
use crate::chat::ChatRoom;

use super::session::{SessionRepositoryEvent, ThinHandler};
use super::SessionRepository;

#[derive(Clone)]
pub struct ChatServer {
    id_increment: usize,
    port: u16,
    server_keys: Vec<KeyPair>,
    auth: Arc<Mutex<Auth>>,
    room: Arc<Mutex<ChatRoom>>,
    repo_event_sender: Sender<SessionRepositoryEvent>,
}

impl ChatServer {
    pub fn new(
        port: u16,
        server_keys: &[KeyPair],
        repo_event_sender: Sender<SessionRepositoryEvent>,
        auth: Auth,
        room: ChatRoom,
    ) -> Self {
        Self {
            port,
            repo_event_sender,
            id_increment: 0,
            server_keys: server_keys.to_vec(),
            auth: Arc::new(Mutex::new(auth)),
            room: Arc::new(Mutex::new(room)),
        }
    }

    pub async fn run(&mut self, mut repository: SessionRepository) -> anyhow::Result<()> {
        let room = self.room.clone();
        let auth = self.auth.clone();

        info!("Spawning a thread to wait for incoming sessions");
        spawn(async move {
            repository.wait_for_sessions(room, auth).await;
        });

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
impl Server for ChatServer {
    type Handler = ThinHandler;

    fn new_client(&mut self, peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        info!("New client created for peer {:?}", peer_addr);
        self.id_increment += 1;
        Self::Handler::new(
            self.id_increment,
            self.auth.clone(),
            self.repo_event_sender.clone(),
        )
    }
}
