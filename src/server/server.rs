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

/// Maximum size of the internal server event buffer.
///
/// Defines the number of session events a `russh` server can buffer
/// up before blocking the thread. The default size usually handles
/// all events but can be quickly filled by frequent terminal flushes,
/// such as when a new user receives room history messages. To avoid
/// blocking, ensure the buffer size exceeds the room history size.
const SERVER_EVENT_BUFFER_SIZE: usize = 30;

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
            event_buffer_size: SERVER_EVENT_BUFFER_SIZE,
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

#[cfg(test)]
mod should {
    use super::*;
    use crate::auth::Auth;
    use crate::chat::ChatRoom;
    use russh_keys::key::KeyPair;
    use tokio::sync::mpsc;
    use tokio::task::JoinHandle;
    use tokio::time::sleep;

    async fn run_server_in_background(
        chat_server: &mut ChatServer,
        repository: SessionRepository,
    ) -> JoinHandle<anyhow::Result<()>> {
        let mut server = chat_server.clone();
        tokio::spawn(async move { server.run(repository).await })
    }

    #[tokio::test]
    async fn check_if_server_runs_and_port_is_in_use() {
        let port = 22;
        let server_keys = vec![KeyPair::generate_ed25519().unwrap()];
        let (tx, _rx) = mpsc::channel(100);
        let auth = Auth::default();
        let room = ChatRoom::new("Welcome!");

        let mut chat_server = ChatServer::new(port, &server_keys, tx, auth, room);
        let (_, rx) = tokio::sync::mpsc::channel(1);
        let repository = SessionRepository::new(rx);

        let server_handle = run_server_in_background(&mut chat_server, repository).await;

        // Give the server some time to start
        sleep(Duration::from_millis(100)).await;

        // Try binding to the port to ensure it's in use
        let bind_result = tokio::net::TcpListener::bind(("0.0.0.0", port)).await;
        assert!(bind_result.is_err(), "Port should be in use by the server");

        // Cleanup: stop the server
        drop(server_handle);
    }
}
