#[derive(Clone, Debug, Default)]
pub struct UserInput {
    pub bytes: Vec<u8>,
}

impl UserInput {
    pub fn to_str(&self) -> String {
        match std::str::from_utf8(self.bytes.as_slice()) {
            Ok(v) => String::from(v),
            Err(_) => String::new(),
        }
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub fn pop(&mut self) {
        self.bytes.pop();
    }

    pub fn extend(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }
}
