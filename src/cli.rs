use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Port to listen on
    #[arg(long, default_value_t = 2222)]
    pub port: u16,

    /// Private key to identify server with. Defaults to a temporary ed25519 key
    #[arg(short = 'i', long, value_name = "KEY")]
    pub identity: Option<String>,

    /// Optional file of public keys who are allowed to connect
    #[arg(long, value_name = "FILE")]
    pub whitelist: Option<String>,

    /// Optional file with a message of the day or welcome message
    #[arg(long, value_name = "FILE")]
    pub motd: Option<String>,

    /// Write chat log to this file
    #[arg(long, value_name = "FILE")]
    pub log: Option<String>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,
}
