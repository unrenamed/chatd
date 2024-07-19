use std::io::Write;

use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue, style};
use unicode_segmentation::UnicodeSegmentation;

use super::input::TerminalInput;
use super::{unicode, CloseHandle};
use crate::utils;

#[derive(Clone)]
pub struct Terminal<H>
where
    H: Clone + Write + CloseHandle,
{
    pub input: TerminalInput,
    prompt: String,
    prompt_display_width: u16,
    handle: H,
    outbuff: Vec<u8>,
    term_width: u16,
    term_height: u16,
    cursor_x: u16,
    cursor_y: u16,
    input_end_x: u16,
    input_end_y: u16,
}

impl<H> Terminal<H>
where
    H: Clone + Write + CloseHandle,
{
    pub fn new(handle: H) -> Self {
        Self {
            handle,
            prompt: String::new(),
            prompt_display_width: 0,
            input: Default::default(),
            outbuff: vec![],
            term_width: 0,
            term_height: 0,
            cursor_x: 0,
            cursor_y: 0,
            input_end_x: 0,
            input_end_y: 0,
        }
    }

    #[cfg(test)]
    pub fn handle(&mut self) -> &mut H {
        &mut self.handle
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
        self.refresh_cursor_coords();
        self.refresh_input_end_coords();
    }

    pub fn set_prompt(&mut self, username: &str) {
        self.prompt = format!("[{}] ", username.to_string());
        self.prompt_display_width = unicode::display_width(&self.prompt) as u16;
    }

    pub fn clear_input(&mut self) -> anyhow::Result<()> {
        self.input.clear();
        self.print_input_line()?;
        Ok(())
    }

    pub fn exit(&mut self) {
        self.handle.close();
    }

    pub fn print_input_line(&mut self) -> anyhow::Result<()> {
        self.queue_prompt_cleanup()?;
        self.queue_write_prompt()?;
        self.queue_write_input()?;
        self.queue_write_outbuff()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    pub fn print_message(&mut self, msg: &str) -> anyhow::Result<()> {
        self.queue_prompt_cleanup()?;
        self.queue_write_message(msg)?;
        self.queue_write_prompt()?;
        self.queue_write_input()?;
        self.queue_write_outbuff()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    fn queue_prompt_cleanup(&mut self) -> anyhow::Result<()> {
        if self.cursor_y < self.input_end_y {
            queue!(
                self.handle,
                cursor::MoveDown(self.input_end_y - self.cursor_y)
            )?;
        }

        self.input_end_x = 0;
        queue!(
            self.handle,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine)
        )?;

        while self.input_end_y > 0 {
            queue!(
                self.handle,
                cursor::MoveUp(1),
                Clear(ClearType::CurrentLine)
            )?;
            self.input_end_y -= 1;
        }

        self.cursor_x = self.input_end_x;
        self.cursor_y = self.input_end_y;

        Ok(())
    }

    fn queue_write_message(&mut self, msg: &str) -> anyhow::Result<()> {
        queue!(self.handle, style::Print(msg), style::Print(utils::NEWLINE))?;
        Ok(())
    }

    fn queue_write_prompt(&mut self) -> anyhow::Result<()> {
        queue!(self.handle, style::Print(&self.prompt))?;
        self.advance_cursor_pos(self.prompt_display_width);
        Ok(())
    }

    fn queue_write_input(&mut self) -> anyhow::Result<()> {
        queue!(self.handle, style::Print(&self.input))?;
        self.advance_cursor_pos(self.input.display_width() as u16);
        Ok(())
    }

    fn queue_write_outbuff(&mut self) -> anyhow::Result<()> {
        queue!(
            self.handle,
            style::Print(String::from_utf8_lossy(&self.outbuff))
        )?;
        self.outbuff = vec![];
        Ok(())
    }

    fn queue_move_cursor(&mut self) -> anyhow::Result<()> {
        if self.term_width == 0 {
            return Ok(());
        }

        let total_width = self.prompt_display_width + self.get_display_width_up_to_cursor_pos();
        let y = total_width / self.term_width;
        let x = total_width % self.term_width;

        let up = if y < self.cursor_y {
            self.cursor_y - y
        } else {
            0
        };
        let down = if y > self.cursor_y {
            y - self.cursor_y
        } else {
            0
        };
        let left = if x < self.cursor_x {
            self.cursor_x - x
        } else {
            0
        };
        let right = if x > self.cursor_x {
            x - self.cursor_x
        } else {
            0
        };

        self.cursor_x = x;
        self.cursor_y = y;

        if up > 0 {
            queue!(self.handle, cursor::MoveUp(up))?;
        }
        if down > 0 {
            queue!(self.handle, cursor::MoveDown(down))?;
        }
        if right > 0 {
            queue!(self.handle, cursor::MoveRight(right))?;
        }
        if left > 0 {
            queue!(self.handle, cursor::MoveLeft(left))?;
        }

        Ok(())
    }

    fn advance_cursor_pos(&mut self, places: u16) {
        if self.term_width == 0 {
            return;
        }

        self.cursor_x += places;
        self.cursor_y += self.cursor_x / self.term_width;
        self.cursor_x = self.cursor_x % self.term_width;

        self.input_end_x += places;
        self.input_end_y += self.input_end_x / self.term_width;
        self.input_end_x = self.input_end_x % self.term_width;

        if places > 0 && self.cursor_x == 0 {
            self.outbuff.push(b'\r');
            self.outbuff.push(b'\n');
        }
    }

    fn refresh_cursor_coords(&mut self) {
        if self.term_width == 0 {
            return;
        }
        let total_width = self.prompt_display_width + self.get_display_width_up_to_cursor_pos();
        self.cursor_y = total_width / self.term_width;
        self.cursor_x = total_width % self.term_width;
    }

    fn refresh_input_end_coords(&mut self) {
        if self.term_width == 0 {
            return;
        }
        let total_visual_length = self.prompt_display_width + self.input.display_width() as u16;
        self.input_end_y = total_visual_length / self.term_width;
        self.input_end_x = total_visual_length % self.term_width;
    }

    fn get_display_width_up_to_cursor_pos(&self) -> u16 {
        let pos = self.input.cursor_char_pos().min(self.input.char_count());
        let input = self.input.text().as_str();
        let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(input, true).collect();
        graphemes
            .iter()
            .take(pos)
            .map(|g| unicode::display_width(g) as u16)
            .sum::<u16>()
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use mockall::mock;

    mock! {
        pub Handle {}

        impl Write for Handle {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize>;
            fn flush(&mut self) -> std::io::Result<()>;
        }

        impl Clone for Handle {
            fn clone(&self) -> Self;
        }

        impl CloseHandle for Handle {
            fn close(&mut self) {}
        }
    }

    #[derive(Clone, Default)]
    pub struct TestHandle {
        pub mock: MockHandle,
        pub written: Vec<u8>,
    }

    impl Write for TestHandle {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written.extend_from_slice(buf);
            self.mock.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.mock.flush()
        }
    }

    impl CloseHandle for TestHandle {
        fn close(&mut self) {
            self.mock.close()
        }
    }

    #[test]
    fn set_prompt() {
        let mut terminal = Terminal::new(TestHandle::default());
        terminal.set_prompt("user");
        assert_eq!(terminal.prompt, "[user] ");
        assert_eq!(terminal.prompt_display_width, 7);
    }

    #[test]
    fn refresh_cursor_coordinates_on_resize() {
        let mut terminal = Terminal::new(TestHandle::default());
        terminal.set_size(80, 24);
        terminal.set_prompt("user");
        let long_input = "a".repeat(240); // takes at least 3 rows
        terminal.input.insert_before_cursor(long_input.as_bytes());
        terminal.input.move_cursor_to(120);

        terminal
            .handle()
            .mock
            .expect_write()
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .mock
            .expect_flush()
            .times(1)
            .returning(|| Ok(()));

        terminal.print_input_line().unwrap();

        // Assert state before resize
        assert_eq!(terminal.term_width, 80);
        assert_eq!(terminal.cursor_x, 47);
        assert_eq!(terminal.cursor_y, 1, "Cursor must be on the 2nd row");
        assert_eq!(terminal.input_end_x, 7);
        assert_eq!(terminal.input_end_y, 3, "Input must end on the 4th row");

        terminal.set_size(40, 24); // reduce term width by half

        // Assert state after 1st resize
        assert_eq!(terminal.term_width, 40);
        assert_eq!(terminal.cursor_x, 7);
        assert_eq!(
            terminal.cursor_y, 3,
            "Cursor must slide down to the 4th row"
        );
        assert_eq!(terminal.input_end_x, 7);
        assert_eq!(
            terminal.input_end_y, 6,
            "Input end must slide down to the 7th row"
        );

        terminal.set_size(120, 24); // tripple the term width

        // Assert state after 2nd resize
        assert_eq!(terminal.term_width, 120);
        assert_eq!(terminal.cursor_x, 7);
        assert_eq!(terminal.cursor_y, 1, "Cursor must slide up to the 2nd row");
        assert_eq!(terminal.input_end_x, 7);
        assert_eq!(
            terminal.input_end_y, 2,
            "Input end must slide up to the 3rd row"
        );
    }

    #[test]
    fn close_handle_on_exit() {
        let mut terminal = Terminal::new(TestHandle::default());

        terminal
            .handle()
            .mock
            .expect_close()
            .times(1)
            .returning(|| ());

        terminal.exit();
    }

    #[test]
    fn clear_input() {
        let mut terminal = Terminal::new(TestHandle::default());
        terminal.input.insert_before_cursor(b"some input");

        terminal
            .handle()
            .mock
            .expect_write()
            .times(..)
            .returning(|buf| Ok(buf.len()));

        terminal
            .handle()
            .mock
            .expect_flush()
            .times(1)
            .returning(|| Ok(()));

        terminal.clear_input().unwrap();
        assert_eq!(terminal.input.text(), "");
    }

    #[test]
    fn print_input_line() {
        let mut terminal = Terminal::new(TestHandle::default());
        terminal.set_size(240, 24);
        terminal.set_prompt("user");
        let long_input = "a".repeat(240); // takes 2 rows
        terminal.input.insert_before_cursor(long_input.as_bytes());
        terminal.set_size(120, 24);

        terminal
            .handle
            .mock
            .expect_write()
            .returning(|buf| Ok(buf.len()));

        terminal.handle.mock.expect_flush().returning(|| Ok(()));

        terminal.print_input_line().unwrap();

        let mut expected_output = String::new();
        expected_output.push_str("\x1B[1G"); // moves the cursor to the beginning of the current line
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("\x1B[1A"); // moves the cursor 1 row up
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("\x1B[1A"); // moves the cursor 1 row up
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("[user] "); // prints prompt
        expected_output.push_str(&long_input); // prints input
        let written = String::from_utf8(terminal.handle().written.clone()).unwrap();
        assert_eq!(written, expected_output);
    }

    #[test]
    fn print_message_and_then_input_line() {
        let mut terminal = Terminal::new(TestHandle::default());
        terminal.set_size(240, 24);
        terminal.set_prompt("user");
        let long_input = "a".repeat(240); // takes 2 rows
        terminal.input.insert_before_cursor(long_input.as_bytes());
        terminal.set_size(120, 24);

        terminal
            .handle
            .mock
            .expect_write()
            .returning(|buf| Ok(buf.len()));

        terminal.handle.mock.expect_flush().returning(|| Ok(()));

        terminal.print_message("[bob] hello @user").unwrap();

        let mut expected_output = String::new();
        expected_output.push_str("\x1B[1G"); // moves the cursor to the beginning of the current line
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("\x1B[1A"); // moves the cursor 1 row up
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("\x1B[1A"); // moves the cursor 1 row up
        expected_output.push_str("\x1B[2K"); // clears the current line
        expected_output.push_str("[bob] hello @user"); // prints message
        expected_output.push_str("\n\r"); // prints newline
        expected_output.push_str("[user] "); // prints prompt
        expected_output.push_str(&long_input); // prints input
        let written = String::from_utf8(terminal.handle().written.clone()).unwrap();
        assert_eq!(written, expected_output);
    }
}
