use clap::Parser;
use cli::Cli;
use log::LevelFilter;
use russh_keys::key::{KeyPair, PublicKey};

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
    let oplist = match cli.oplist {
        Some(path) => Some(
            utils::fs::read_file_lines(&path)
                .expect("Failed to read the oplist file")
                .iter()
                .filter_map(|line| utils::ssh::split_ssh_key(line))
                .filter_map(|(_, key, _)| russh_keys::parse_public_key_base64(&key).ok())
                .collect::<Vec<PublicKey>>(),
        ),
        None => None,
    };

    // Initiate server whitelist
    let whitelist = match cli.whitelist {
        Some(path) => Some(
            utils::fs::read_file_lines(&path)
                .expect("Failed to read the whitelist file")
                .iter()
                .filter_map(|line| utils::ssh::split_ssh_key(line))
                .filter_map(|(_, key, _)| russh_keys::parse_public_key_base64(&key).ok())
                .collect::<Vec<PublicKey>>(),
        ),
        None => None,
    };

    // Initiate motd
    let motd = match cli.motd {
        Some(path) => utils::fs::read_file_to_string(&path).expect("Failed to read the MOTD file"),
        None => include_str!("../motd.ans").to_string(),
    }
    .replace("\n", "\n\r"); // normalize line endings into \r

    // Initiate server <-> session repository message channel
    let (tx, rx) = tokio::sync::mpsc::channel(1000);

    // Initate server and session repository
    let mut server = server::AppServer::new(
        cli.port,
        &server_keys,
        oplist.map(|o| o.to_vec()),
        whitelist.map(|w| w.to_vec()),
        &motd,
        tx,
    );
    let repository = server::SessionRepository::new(rx);

    // Run the server
    server.run(repository).await.expect("Failed running server");
}
