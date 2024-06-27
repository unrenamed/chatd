use terminal_keycode::{Decoder, KeyCode};

pub fn decode_bytes_to_codes(bytes: &[u8]) -> Vec<KeyCode> {
    let mut decoder = Decoder::new();
    let mut codes = vec![];
    for byte in bytes {
        for keycode in decoder.write(*byte) {
            codes.push(keycode);
        }
    }
    codes
}
