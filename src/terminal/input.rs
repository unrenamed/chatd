use std::fmt::Display;
use unicode_segmentation::UnicodeSegmentation;

use super::{input_history::InputHistory, unicode};

const MAX_HISTORY_SIZE: usize = 20;

// Struct representing user input state with cursor position
#[derive(Clone, Debug, Default)]
struct InputState {
    text: String,           // String representing user input
    char_count: usize,      // Number of characters in the text
    display_width: usize,   //
    cursor_char_pos: usize, // Cursor position in terms of characters
    cursor_byte_pos: usize,
}

// Struct representing user input with snapshot capability and input history
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

            self.state.cursor_char_pos = word_start;
            self.calc_new_cursor_byte_pos();
        }

        // Update cursor position
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
    // If there is no current navigation index in the history, it first takes a snapshot of the current state
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

    fn calc_new_cursor_byte_pos(&mut self) {
        let graphemes: Vec<&str> = self.state.text.graphemes(true).collect();
        let new_cursor_byte_pos = char_to_byte_pos(&graphemes, self.state.cursor_char_pos);
        self.state.cursor_byte_pos = new_cursor_byte_pos;
    }

    fn calc_new_cursor_char_pos(&mut self) {
        let graphemes: Vec<&str> = self.state.text.graphemes(true).collect();
        let new_cursor_char_pos = byte_to_char_pos(&graphemes, self.state.cursor_byte_pos);
        self.state.cursor_char_pos = new_cursor_char_pos;
    }
}

fn char_to_byte_pos(graphemes: &Vec<&str>, char_pos: usize) -> usize {
    graphemes.iter().take(char_pos).map(|g| g.len()).sum()
}

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
