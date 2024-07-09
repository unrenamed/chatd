use auth::{Auth, PubKeyFileManager};
use chat::ChatRoom;
use clap::Parser;
use cli::Cli;
use log::LevelFilter;
use russh_keys::key::KeyPair;
use server::{ChatServer, SessionRepository};

mod auth;
mod chat;
mod cli;
mod logger;
mod server;
mod terminal;
mod utils;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initiate logger
    let level = match cli.debug {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::max(),
    };
    if let Err(err) = logger::setup(cli.log, level) {
        panic!("Failed to setup logger: {}", err);
    }

    // Initiate server keys
    let key_pair = match cli.identity {
        None => KeyPair::generate_ed25519().expect("Failed to generate a new ed25519 key pair"),
        Some(path) => {
            let key =
                utils::fs::read_file_to_string(&path).expect("Failed to read the identity file");
            russh_keys::decode_secret_key(&key, None)
                .expect("Failed to decode the secret key from the identity file")
        }
    };
    let server_keys = vec![key_pair];

    // Initiate server oplist file manager
    let mut oplist_manager = None;
    if let Some(path) = cli.oplist {
        oplist_manager = Some(PubKeyFileManager::new(&path));
    };

    // Initiate server whitelist file manager
    let mut whitelist_manager = None;
    if let Some(path) = cli.whitelist {
        whitelist_manager = Some(PubKeyFileManager::new(&path));
    };

    // Initiate motd
    let motd = match cli.motd {
        Some(path) => utils::fs::read_file_to_string(&path).expect("Failed to read the MOTD file"),
        None => include_str!("../motd.ans").to_string(),
    }
    .replace("\n", "\n\r"); // normalize line endings into \r

    // Initiate server <-> session repository message channel
    let (tx, rx) = tokio::sync::mpsc::channel(1000);

    // Initate authorization service
    let mut auth = Auth::default();
    if let Some(whitelist) = whitelist_manager {
        auth.set_whitelist(whitelist);
        auth.enable_whitelist_mode();
        auth.load_trusted_keys()
            .expect("Failed to load public keys from whitelist");
    }
    if let Some(oplist) = oplist_manager {
        auth.set_oplist(oplist);
        auth.load_operators()
            .expect("Failed to load public keys from oplist");
    }

    // Initate server and session repository
    let room = ChatRoom::new(&motd);
    let repository = SessionRepository::new(rx);
    let mut server = ChatServer::new(cli.port, &server_keys, tx, auth, room);

    // Run the server
    server.run(repository).await.expect("Failed running server");
}
