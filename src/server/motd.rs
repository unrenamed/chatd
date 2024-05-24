use std::path::Path;

static MOTD_FILEPATH: &'static str = "./motd.ans";

#[derive(Debug, Clone)]
pub struct Motd(String);

impl Motd {
    pub fn get(&self) -> &String {
        &self.0
    }
}

impl Default for Motd {
    fn default() -> Self {
        let bytes = std::fs::read(Path::new(MOTD_FILEPATH))
            .expect("Should have been able to read the motd file");

        // normalize line endings into \r
        let utf8_normalized = String::from_utf8_lossy(&bytes).replace("\n", "\n\r");

        Self(utf8_normalized)
    }
}
