mod chat;
mod models;
mod server;
mod tui;
mod utils;

#[tokio::main]
async fn main() {
    let mut server = server::AppServer::new();
    server.run().await.expect("Failed running server");
}
