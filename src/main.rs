mod server;
mod utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    let mut server = server::AppServer::new(tx);
    let repository = server::SessionRepository::new(rx);
    server.run(repository).await.expect("Failed running server");
}
