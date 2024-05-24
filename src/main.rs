mod server;
mod utils;

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut server = server::AppServer::new();
    server.run().await.expect("Failed running server");
}
