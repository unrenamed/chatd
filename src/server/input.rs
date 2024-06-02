use std::fmt::Display;

use super::input_history::InputHistory;

const MAX_HISTORY_SIZE: usize = 20;

// Struct representing user input state with cursor position
#[derive(Clone, Debug, Default)]
struct InputState {
    bytes: Vec<u8>,         // Bytes representing user input
    char_cursor_pos: usize, // Cursor position in terms of characters
}

// Struct representing user input with snapshot capability and input history 
#[derive(Clone, Debug, Default)]
pub struct UserInput {
    state: InputState,                                   // Current input state
    snapshot: Option<InputState>,                        // Snapshot of previous state
    history: InputHistory<InputState, MAX_HISTORY_SIZE>, // Records the history of inputs made by the user
}

impl Display for UserInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Convert bytes to string and write to formatter
        write!(
            f,
            "{}",
            std::str::from_utf8(self.state.bytes.as_slice()).unwrap()
        )
    }
}

impl UserInput {
    // Get a reference to the bytes representing user input
    pub fn bytes(&self) -> &Vec<u8> {
        &self.state.bytes
    }

    // Get a reference to the cursor position in terms of characters
    pub fn char_cursor_pos(&self) -> &usize {
        &self.state.char_cursor_pos
    }

    // Restore previous state from snapshot
    pub fn restore(&mut self) {
        if self.snapshot.is_some() {
            // Clone bytes and cursor position from snapshot
            self.state.bytes = self.snapshot.as_ref().unwrap().bytes.clone();
            self.state.char_cursor_pos = self.snapshot.as_ref().unwrap().char_cursor_pos;
            self.clear_snapshot();
        }
    }

    // Clear user input
    pub fn clear(&mut self) {
        if !self.state.bytes.is_empty() {
            // Create snapshot before clearing
            self.make_snapshot();
            // Clear bytes and reset cursor position
            self.state.bytes.clear();
            self.state.char_cursor_pos = 0;
        }
    }

    // Move cursor to the next character
    pub fn move_cursor_next(&mut self) {
        let s = std::str::from_utf8(&self.state.bytes).unwrap();
        if (self.state.char_cursor_pos as usize) < s.chars().count() {
            self.state.char_cursor_pos += 1;
        }
    }

    // Move cursor to the previous character
    pub fn move_cursor_prev(&mut self) {
        if self.state.char_cursor_pos > 0 {
            self.state.char_cursor_pos -= 1;
        }
    }

    // Move cursor to the start of line
    pub fn move_cursor_start(&mut self) {
        self.state.char_cursor_pos = 0;
    }

    // Move cursor to the end of line
    pub fn move_cursor_end(&mut self) {
        let s = std::str::from_utf8(&self.state.bytes).unwrap();
        self.state.char_cursor_pos = s.len();
    }

    // Get the byte index of the cursor
    pub fn byte_cursor_pos(&self) -> usize {
        let s = std::str::from_utf8(&self.state.bytes).unwrap();
        s.char_indices()
            .nth(self.state.char_cursor_pos as usize)
            .map(|(i, _)| i)
            .unwrap_or(self.state.bytes.len()) // Return length if cursor at end
    }

    // Insert bytes before cursor position and update cursor
    pub fn insert_before_cursor(&mut self, insert_bytes: &[u8]) {
        let byte_pos = self.byte_cursor_pos();
        self.state
            .bytes
            .splice(byte_pos..byte_pos, insert_bytes.iter().cloned());

        // Update cursor position after insertion
        self.state.char_cursor_pos += std::str::from_utf8(insert_bytes).unwrap().chars().count();
    }

    // Remove character before cursor position
    pub fn remove_before_cursor(&mut self) {
        if self.state.char_cursor_pos == 0 {
            return; // Nothing to remove if cursor is at start
        }

        self.move_cursor_prev();
        let s = std::str::from_utf8(&self.state.bytes).unwrap();
        if let Some((char_start, c)) = s.char_indices().nth(self.state.char_cursor_pos as usize) {
            let char_end = char_start + c.len_utf8();
            self.state.bytes.drain(char_start..char_end);
        }
    }

    // Remove last word before cursor position
    pub fn remove_last_word_before_cursor(&mut self) {
        let prev = self.state.clone();

        let is_word_char = |c: u8| c != b' ';

        // Get byte position of cursor
        let byte_pos = self.byte_cursor_pos();

        // Find closest word character before cursor
        let mut word_end = byte_pos;
        while word_end > 0 {
            if is_word_char(self.state.bytes[word_end - 1]) {
                break;
            }
            word_end -= 1;
        }

        // Find start of last word before cursor
        let mut word_start = word_end;
        while word_start > 0 {
            if is_word_char(self.state.bytes[word_start - 1]) {
                word_start -= 1;
            } else {
                break;
            }
        }

        // Remove last word from start to end
        let drained_count = { self.state.bytes.drain(word_start..byte_pos).len() };

        if drained_count > 0 {
            self.make_snapshot_from(prev);
        }

        // Update cursor position
        self.state.char_cursor_pos = std::str::from_utf8(&self.state.bytes[..word_start])
            .unwrap()
            .chars()
            .count();
    }

    // Remove everything after cursor position
    pub fn remove_after_cursor(&mut self) {
        let prev = self.state.clone();
        let byte_pos = self.byte_cursor_pos();
        let drained_count = { self.state.bytes.drain(byte_pos..).len() };
        if drained_count > 0 {
            self.make_snapshot_from(prev);
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
            self.state.bytes = prev.bytes.clone();
            self.state.char_cursor_pos = prev.char_cursor_pos.clone();
        }
    }

    // Sets the current state to the next state in the input history.
    // If there is no next state, it restores the state from the snapshot
    pub fn set_history_next(&mut self) {
        self.history.insert_at_current(self.state.clone());
        if let Some(next) = self.history.next() {
            self.state.bytes = next.bytes.clone();
            self.state.char_cursor_pos = next.char_cursor_pos.clone();
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
        if self.snapshot.is_some() {
            self.snapshot = None;
        }
    }
}
