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

#[cfg(test)]
mod should {
    use super::*;
    use terminal_keycode::KeyCode;

    #[test]
    fn decode_single_byte() {
        let bytes = vec![0x41]; // ASCII for 'A'
        let expected = vec![KeyCode::Char('A')];
        let result = decode_bytes_to_codes(&bytes);
        assert_eq!(result, expected);
    }

    #[test]
    fn decode_multiple_bytes() {
        let bytes = vec![0x41, 0x42, 0x43]; // ASCII for 'A', 'B', 'C'
        let expected = vec![KeyCode::Char('A'), KeyCode::Char('B'), KeyCode::Char('C')];
        let result = decode_bytes_to_codes(&bytes);
        assert_eq!(result, expected);
    }

    #[test]
    fn decode_mixed_input() {
        let bytes = vec![0x41, 0x1B, 0x5B, 0x42]; // 'A' followed by Arrow Down
        let expected = vec![KeyCode::Char('A'), KeyCode::ArrowDown];
        let result = decode_bytes_to_codes(&bytes);
        assert_eq!(result, expected);
    }
}
