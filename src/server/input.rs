#[derive(Clone, Debug, Default)]
pub struct UserInput {
    bytes: Vec<u8>,
}

impl UserInput {
    pub fn to_str(&self) -> String {
        match std::str::from_utf8(self.bytes.as_slice()) {
            Ok(v) => String::from(v),
            Err(_) => String::new(),
        }
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    pub fn pop(&mut self) {
        self.bytes.pop();
    }

    pub fn remove_last_word(&mut self) {
        let is_word_char = |c: u8| c.is_ascii_alphanumeric() || c == b'_';

        // First, skip any trailing non-word characters (e.g., spaces, punctuation).
        while let Some(&byte) = self.bytes.last() {
            if is_word_char(byte) {
                break;
            }
            self.bytes.pop();
        }

        // Then, remove word characters until we hit a non-word character.
        while let Some(&byte) = self.bytes.last() {
            if !is_word_char(byte) {
                break;
            }
            self.bytes.pop();
        }
    }

    pub fn extend(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }
}
