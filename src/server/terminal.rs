use log::error;
use russh::{server::Handle, ChannelId};

#[derive(Clone)]
pub struct TerminalHandle {
    pub handle: Handle,
    // The sink collects the data which is finally flushed to the handle.
    pub sink: Vec<u8>,
    pub channel_id: ChannelId,
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let handle = self.handle.clone();
        let channel_id = self.channel_id;
        let data = self.sink.clone().into();
        futures::executor::block_on(async move {
            let result = handle.data(channel_id, data).await;
            if result.is_err() {
                error!("Failed to send data: {:?}", result);
            }
        });

        self.sink.clear();
        Ok(())
    }
}

impl Drop for TerminalHandle {
    fn drop(&mut self) {
        futures::executor::block_on(async move {
            let result = self.handle.close(self.channel_id).await;
            if result.is_err() {
                error!("Failed to close session: {:?}", result);
            }
        });
    }
}
