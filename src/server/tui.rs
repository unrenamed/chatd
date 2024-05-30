use std::{sync::Arc, time::Duration};

use log::error;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::room::ServerRoom;

pub async fn render(room: Arc<Mutex<ServerRoom>>) {
    loop {
        sleep(Duration::from_millis(50)).await;

        let motd = room.lock().await.motd().clone();
        for (_, member) in room.lock().await.apps_mut().iter_mut() {
            member
                .render_motd(&motd)
                .await
                .unwrap_or_else(|error| error!("{}", error));
            member
                .render()
                .await
                .unwrap_or_else(|error| error!("{}", error));
        }
    }
}
