use std::sync::Arc;

use clap::Parser;
use cli::Cli;
use log::LevelFilter;
use russh_keys::key::KeyPair;
use server::PublicKeyLoader;
use tokio::sync::Mutex;

mod cli;
mod logger;
mod server;
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

    // Initiate server oplist
    let mut oplist_loader = None;
    if let Some(path) = cli.oplist {
        oplist_loader = Some(PublicKeyLoader::new(&path));
    };

    // Initiate server whitelist loader
    let mut whitelist_loader = None;
    if let Some(path) = cli.whitelist {
        whitelist_loader = Some(PublicKeyLoader::new(&path));
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
    let auth = Arc::new(Mutex::new(server::Auth::new(
        oplist_loader.clone(),
        whitelist_loader.clone(),
    )));
    if whitelist_loader.is_some() {
        auth.lock()
            .await
            .load_trusted_keys()
            .expect("Failed to load public keys from whitelist");
    }
    if oplist_loader.is_some() {
        auth.lock()
            .await
            .load_operators()
            .expect("Failed to load public keys from oplist");
    }

    // Initate server and session repository
    let room = server::ServerRoom::new(&motd, auth.clone());
    let repository = server::SessionRepository::new(rx);
    let mut server = server::AppServer::new(cli.port, auth.clone(), room, &server_keys, tx);

    // Run the server
    server.run(repository).await.expect("Failed running server");
}
