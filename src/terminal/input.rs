use std::fmt::Display;
use unicode_segmentation::UnicodeSegmentation;

use super::input_history::InputHistory;
use super::unicode;

const MAX_HISTORY_SIZE: usize = 20;

// Struct representing user input state with cursor position
#[derive(Clone, Debug, Default)]
struct InputState {
    text: String,           // String representing user input
    char_count: usize,      // Number of characters in the text
    display_width: usize,   // Display width of the text (accounting for wide characters, etc.)
    cursor_char_pos: usize, // Cursor position in terms of characters
    cursor_byte_pos: usize, // Cursor position in terms of bytes
}

// Struct representing user input with snapshot capability and input
// history
#[derive(Clone, Debug, Default)]
pub struct TerminalInput {
    state: InputState,                                   // Current input state
    snapshot: Option<InputState>,                        // Snapshot of previous state
    history: InputHistory<InputState, MAX_HISTORY_SIZE>, // Records the history of inputs made by the user
}

impl Display for TerminalInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.state.text)
    }
}

impl TerminalInput {
    // Get a reference to the text representing user input
    pub fn text(&self) -> &String {
        &self.state.text
    }

    // Get a reference to the bytes representing user input
    pub fn bytes(&self) -> &[u8] {
        &self.state.text.as_bytes()
    }

    // Get a cursor position in terms of characters
    pub fn cursor_char_pos(&self) -> usize {
        self.state.cursor_char_pos
    }

    // Get a cursor position in terms of bytes
    pub fn cursor_byte_pos(&self) -> usize {
        self.state.cursor_byte_pos
    }

    // Get an input characters count
    pub fn char_count(&self) -> usize {
        self.state.char_count
    }

    // Get an input visual length
    pub fn display_width(&self) -> usize {
        self.state.display_width
    }

    // Restore previous state from snapshot
    pub fn restore(&mut self) {
        if let Some(snapshot) = &self.snapshot {
            self.state = snapshot.clone();
            self.clear_snapshot();
        }
    }

    // Clear user input
    pub fn clear(&mut self) {
        if !self.state.text.is_empty() {
            self.make_snapshot();
            self.state.text.clear();
            self.state.cursor_char_pos = 0;
            self.state.cursor_byte_pos = 0;
            self.state.char_count = 0;
            self.state.display_width = 0;
        }
    }

    // Move cursor to the next character
    pub fn move_cursor_next(&mut self) {
        if self.state.cursor_char_pos < self.state.char_count {
            self.state.cursor_char_pos += 1;
            self.calc_new_cursor_byte_pos();
        }
    }

    // Move cursor to the previous character
    pub fn move_cursor_prev(&mut self) {
        if self.state.cursor_char_pos > 0 {
            self.state.cursor_char_pos -= 1;
            self.calc_new_cursor_byte_pos();
        }
    }

    // Move cursor to the start of line
    pub fn move_cursor_start(&mut self) {
        self.state.cursor_char_pos = 0;
        self.state.cursor_byte_pos = 0;
    }

    // Move cursor to the end of line
    pub fn move_cursor_end(&mut self) {
        self.state.cursor_char_pos = self.state.char_count;
        self.state.cursor_byte_pos = self.state.text.len();
    }

    // Move cursor to the given byte position
    pub fn move_cursor_to(&mut self, pos: usize) {
        if pos <= self.bytes().len() {
            self.state.cursor_byte_pos = pos;
            self.calc_new_cursor_char_pos();
        }
    }

    // Insert text before cursor position and update cursor
    pub fn insert_before_cursor(&mut self, bytes: &[u8]) {
        let insert_text = &String::from_utf8_lossy(bytes);
        self.state
            .text
            .insert_str(self.state.cursor_byte_pos, insert_text);

        let graphemes = self.state.text.graphemes(true).collect::<Vec<&str>>();
        let new_cursor_byte_pos = self.state.cursor_byte_pos + bytes.len();

        self.state.char_count = graphemes.len();
        self.state.cursor_byte_pos = new_cursor_byte_pos;
        self.calc_new_cursor_char_pos();
        self.state.display_width = unicode::display_width(&self.state.text);
    }

    // Remove character before cursor position
    pub fn remove_before_cursor(&mut self) {
        if self.state.cursor_char_pos == 0 {
            return; // Nothing to remove if cursor is at start
        }

        self.move_cursor_prev();

        let graphemes: Vec<&str> = self.state.text.graphemes(true).collect();
        let remove_len = graphemes[self.state.cursor_char_pos].len();
        let start = self.state.cursor_byte_pos;

        self.state.text.drain(start..start + remove_len);
        self.state.char_count -= 1;
        self.state.display_width = unicode::display_width(&self.state.text);
    }

    // Remove last word before cursor position
    pub fn remove_last_word_before_cursor(&mut self) {
        let prev = self.state.clone();
        let is_word_char = |c: u8| c != b' ';

        // Get byte position of cursor
        let bytes = self.bytes();
        let byte_pos = self.state.cursor_byte_pos;

        // Find closest word character before cursor
        let mut word_end = byte_pos;
        while word_end > 0 {
            if is_word_char(bytes[word_end - 1]) {
                break;
            }
            word_end -= 1;
        }

        // Find start of last word before cursor
        let mut word_start = word_end;
        while word_start > 0 {
            if is_word_char(bytes[word_start - 1]) {
                word_start -= 1;
            } else {
                break;
            }
        }

        // Remove last word from start to end
        let drained = self.state.text.drain(word_start..byte_pos).count();
        if drained > 0 {
            self.make_snapshot_from(prev);

            let total_char_count = self.state.text.graphemes(true).count();
            self.state.char_count = total_char_count;
            self.state.display_width = unicode::display_width(&self.text());

            // Update cursor position
            self.state.cursor_byte_pos = word_start;
            self.calc_new_cursor_char_pos();
        }
    }

    // Remove everything after cursor position
    pub fn remove_after_cursor(&mut self) {
        let prev = self.state.clone();
        let drained = self.state.text.drain(self.state.cursor_byte_pos..).count();
        if drained > 0 {
            self.make_snapshot_from(prev);
            let total_char_count = self.state.text.graphemes(true).count();
            self.state.char_count = total_char_count;
            self.state.display_width = unicode::display_width(&self.state.text);
        }
    }

    // Pushes the current state to the input history
    pub fn push_to_history(&mut self) {
        self.history.push(self.state.clone());
    }

    // Sets the current state to the previous state in the input history.
    // If there is no current navigation index in the history, it first
    // takes a snapshot of the current state
    pub fn set_history_prev(&mut self) {
        if self.history.nav_index().is_none() {
            self.make_snapshot();
        }
        self.history.insert_at_current(self.state.clone());
        if let Some(prev) = self.history.prev() {
            self.state = prev.clone();
        }
    }

    // Sets the current state to the next state in the input history.
    // If there is no next state, it restores the state from the snapshot
    pub fn set_history_next(&mut self) {
        self.history.insert_at_current(self.state.clone());
        if let Some(next) = self.history.next() {
            self.state = next.clone();
        } else {
            self.restore();
        }
    }

    // Create a snapshot of current state
    fn make_snapshot(&mut self) {
        self.snapshot = Some(self.state.clone());
    }

    // Create a snapshot from another InputState instance
    fn make_snapshot_from(&mut self, other: InputState) {
        self.snapshot = Some(other);
    }

    // Clear current snapshot
    fn clear_snapshot(&mut self) {
        self.snapshot = None;
    }

    // Calculate the new byte cursor position based on the current character position
    fn calc_new_cursor_byte_pos(&mut self) {
        let graphemes: Vec<&str> = self.state.text.graphemes(true).collect();
        let new_cursor_byte_pos = char_to_byte_pos(&graphemes, self.state.cursor_char_pos);
        self.state.cursor_byte_pos = new_cursor_byte_pos;
    }

    // Calculate the new character cursor position based on the current byte position
    fn calc_new_cursor_char_pos(&mut self) {
        let graphemes: Vec<&str> = self.state.text.graphemes(true).collect();
        let new_cursor_char_pos = byte_to_char_pos(&graphemes, self.state.cursor_byte_pos);
        self.state.cursor_char_pos = new_cursor_char_pos;
    }
}

// Utility to convert cursor char position to byte position
fn char_to_byte_pos(graphemes: &Vec<&str>, char_pos: usize) -> usize {
    graphemes.iter().take(char_pos).map(|g| g.len()).sum()
}

// Utility to convert cursor byte position to char position
fn byte_to_char_pos(graphemes: &Vec<&str>, byte_pos: usize) -> usize {
    let mut byte_count = 0;
    let mut cursor_pos = 0;
    let mut found = false;

    for (i, grapheme) in graphemes.iter().enumerate() {
        byte_count += grapheme.len();
        if byte_count > byte_pos {
            cursor_pos = i;
            found = true;
            break;
        }
    }

    if found {
        cursor_pos
    } else {
        graphemes.len()
    }
}

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn have_initial_state() {
        let input = TerminalInput::default();
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_char_pos(), 0);
        assert_eq!(input.cursor_byte_pos(), 0);
        assert_eq!(input.char_count(), 0);
        assert_eq!(input.display_width(), 0);
    }

    #[test]
    fn insert_text_before_cursor() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        assert_eq!(input.text(), test_str);
        assert_eq!(input.cursor_char_pos(), 12); // "hello 你好 🌍 👨‍👩‍👧‍👦" has 12 graphemes
        assert_eq!(input.cursor_byte_pos(), test_str.len());
        assert_eq!(input.char_count(), 12);
        assert_eq!(input.display_width(), 16); // display width of text
    }

    #[test]
    fn clear_input() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.clear();
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_char_pos(), 0);
        assert_eq!(input.cursor_byte_pos(), 0);
        assert_eq!(input.char_count(), 0);
        assert_eq!(input.display_width(), 0);
    }

    #[test]
    fn move_cursor_next() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_start();
        input.move_cursor_next();
        assert_eq!(input.cursor_char_pos(), 1);
        assert_eq!(input.cursor_byte_pos(), 1);
    }

    #[test]
    fn move_cursor_prev() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_end();
        input.move_cursor_prev();
        assert_eq!(input.cursor_char_pos(), 11);
        assert_eq!(input.cursor_byte_pos(), 18);
    }

    #[test]
    fn move_cursor_to_start() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_start();
        assert_eq!(input.cursor_char_pos(), 0);
        assert_eq!(input.cursor_byte_pos(), 0);
    }

    #[test]
    fn move_cursor_to_end() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_start();
        input.move_cursor_end();
        assert_eq!(input.cursor_char_pos(), 12);
        assert_eq!(input.cursor_byte_pos(), test_str.len());
    }

    #[test]
    fn move_cursor_to_given_position() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_to(7); // Position after "hello 你"
        assert_eq!(input.cursor_byte_pos(), 7);
        assert_eq!(input.cursor_char_pos(), 6);
    }

    #[test]
    fn remove_character_before_cursor() {
        let mut input = TerminalInput::default();
        let test_str = "hello world";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_end();
        input.remove_before_cursor();
        let expected_str = "hello worl";
        assert_eq!(input.text(), expected_str);
        assert_eq!(input.cursor_char_pos(), 10);
        assert_eq!(input.cursor_byte_pos(), 10);
        assert_eq!(input.char_count(), 10);
        assert_eq!(input.display_width(), 10);
    }

    #[test]
    fn remove_emoji_before_cursor() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_end();
        input.remove_before_cursor();
        let expected_str = "hello 你好 🌍 ";
        assert_eq!(input.text(), expected_str);
        assert_eq!(input.cursor_char_pos(), 11);
        assert_eq!(input.cursor_byte_pos(), 18);
        assert_eq!(input.char_count(), 11);
        assert_eq!(input.display_width(), 14);
    }

    #[test]
    fn remove_last_word_before_cursor() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 wor👨‍👨‍👧‍👧ld";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_end();
        input.remove_last_word_before_cursor();
        let expected_str = "hello 你好 🌍 ";
        assert_eq!(input.text(), expected_str);
        assert_eq!(input.cursor_char_pos(), 11);
        assert_eq!(input.cursor_byte_pos(), 18);
        assert_eq!(input.char_count(), 11);
        assert_eq!(input.display_width(), 14);
    }

    #[test]
    fn remove_everything_after_cursor() {
        let mut input = TerminalInput::default();
        let test_str = "hello 你好 🌍 👨‍👩‍👧‍👦";
        input.insert_before_cursor(test_str.as_bytes());
        input.move_cursor_to(5);
        input.remove_after_cursor();
        let expected_str = "hello";
        assert_eq!(input.text(), expected_str);
        assert_eq!(input.cursor_char_pos(), 5);
        assert_eq!(input.cursor_byte_pos(), 5);
        assert_eq!(input.char_count(), 5);
        assert_eq!(input.display_width(), 5);
    }

    #[test]
    fn push_to_history() {
        let mut input = TerminalInput::default();
        input.insert_before_cursor("hello".as_bytes());
        input.push_to_history();
        assert_eq!(input.history.prev().unwrap().text, "hello");
    }

    #[test]
    fn set_history_prev() {
        let mut input = TerminalInput::default();
        input.insert_before_cursor("1st pushed text".as_bytes());
        input.push_to_history();
        input.insert_before_cursor("2nd pushed text".as_bytes());
        input.set_history_prev();
        assert_eq!(input.text(), "1st pushed text");
    }

    #[test]
    fn set_history_next() {
        let mut input = TerminalInput::default();
        input.insert_before_cursor("1st pushed text".as_bytes());
        input.push_to_history();
        input.clear();
        input.insert_before_cursor("2nd pushed text".as_bytes());
        input.push_to_history();
        input.clear();
        input.insert_before_cursor("not pushed text".as_bytes());
        input.set_history_prev();
        input.set_history_prev();

        assert_eq!(input.text(), "1st pushed text");
        input.set_history_next();
        assert_eq!(input.text(), "2nd pushed text");
        input.set_history_next();
        assert_eq!(
            input.text(),
            "not pushed text",
            "Should restore from snapshot"
        );
    }
}
