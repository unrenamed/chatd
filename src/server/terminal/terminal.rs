use crossterm::cursor;
use crossterm::queue;
use crossterm::style;
use crossterm::terminal::{Clear, ClearType};
use std::io::Write;
use unicode_segmentation::UnicodeSegmentation;

use crate::server::room::User;
use crate::utils;
use crate::utils::display_width;

use super::handle::TerminalHandle;
use super::input::TerminalInput;

#[derive(Clone)]
pub struct Terminal {
    pub input: TerminalInput,
    prompt: String,
    prompt_visual_len: u16,
    handle: TerminalHandle,
    term_width: u16,
    term_height: u16,
    cursor_x: u16,
    cursor_y: u16,
    prompt_x: u16,
    prompt_y: u16,
    outbuff: Vec<u8>,
}

impl Terminal {
    pub fn new(handle: TerminalHandle) -> Self {
        Self {
            handle,
            prompt: String::new(),
            prompt_visual_len: 0,
            input: Default::default(),
            term_width: 0,
            term_height: 0,
            cursor_x: 0,
            cursor_y: 0,
            prompt_x: 0,
            prompt_y: 0,
            outbuff: vec![],
        }
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
        self.update_prompt_cursor();
        self.update_cursor();
    }

    pub fn get_prompt(&self, user: &User) -> String {
        format!("[{}] ", user.theme.style_username(&user.username))
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
        self.prompt_visual_len = display_width(&self.prompt) as u16;
    }

    pub fn clear_input(&mut self) -> Result<(), anyhow::Error> {
        self.input.clear();
        self.write_prompt()?;
        Ok(())
    }

    pub fn exit(&mut self) {
        self.handle.close();
    }

    pub fn write_prompt(&mut self) -> Result<(), anyhow::Error> {
        self.queue_prompt_cleanup()?;
        self.queue_write_line(&self.prompt.to_string())?;
        self.queue_write_line(&self.input.to_string())?;
        self.queue_outbuff_write()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    pub fn write_message(&mut self, msg: &str) -> Result<(), anyhow::Error> {
        self.queue_prompt_cleanup()?;
        self.queue_write_with_crlf(msg)?;
        self.queue_write_line(&self.prompt.to_string())?;
        self.queue_write_line(&self.input.to_string())?;
        self.queue_outbuff_write()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    fn queue_prompt_cleanup(&mut self) -> Result<(), anyhow::Error> {
        if self.cursor_y < self.prompt_y {
            queue!(self.handle, cursor::MoveDown(self.prompt_y - self.cursor_y))?;
        }

        self.prompt_x = 0;
        queue!(
            self.handle,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine)
        )?;

        while self.prompt_y > 0 {
            queue!(
                self.handle,
                cursor::MoveUp(1),
                Clear(ClearType::CurrentLine)
            )?;
            self.prompt_y -= 1;
        }

        self.cursor_x = self.prompt_x;
        self.cursor_y = self.prompt_y;

        Ok(())
    }

    fn queue_write_with_crlf(&mut self, line: &str) -> Result<(), anyhow::Error> {
        queue!(
            self.handle,
            style::Print(line),
            style::Print(utils::NEWLINE)
        )?;
        Ok(())
    }

    fn queue_write_line(&mut self, line: &str) -> Result<(), anyhow::Error> {
        queue!(self.handle, style::Print(line))?;
        self.advance_cursor(utils::display_width(line) as u16);
        Ok(())
    }

    fn queue_outbuff_write(&mut self) -> Result<(), anyhow::Error> {
        queue!(
            self.handle,
            style::Print(String::from_utf8_lossy(&self.outbuff))
        )?;
        self.outbuff = vec![];
        Ok(())
    }

    fn queue_move_cursor(&mut self) -> Result<(), anyhow::Error> {
        if self.term_width == 0 {
            return Ok(());
        }

        let input = self.input.text().as_str();
        let pos = self.input.cursor_char_pos().min(self.input.char_count());
        let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(input, true).collect();
        let visual_length_up_to_pos = graphemes
            .iter()
            .take(pos)
            .map(|g| utils::display_width(g) as u16)
            .sum::<u16>();
        let total_visual_length = self.prompt_visual_len + visual_length_up_to_pos;
        let y = total_visual_length / self.term_width;
        let x = total_visual_length % self.term_width;

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

    fn advance_cursor(&mut self, places: u16) {
        if self.term_width == 0 {
            return;
        }

        self.cursor_x += places;
        self.cursor_y += self.cursor_x / self.term_width;
        self.cursor_x = self.cursor_x % self.term_width;

        self.prompt_x += places;
        self.prompt_y += self.prompt_x / self.term_width;
        self.prompt_x = self.prompt_x % self.term_width;

        if places > 0 && self.cursor_x == 0 {
            self.outbuff.push(b'\r');
            self.outbuff.push(b'\n');
        }
    }

    fn update_prompt_cursor(&mut self) {
        if self.term_width == 0 {
            return;
        }

        let input = self.input.text().as_str();
        let pos = self.input.cursor_char_pos().min(self.input.char_count());
        let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(input, true).collect();
        let visual_length_up_to_pos = graphemes
            .iter()
            .take(pos)
            .map(|g| utils::display_width(g) as u16)
            .sum::<u16>();
        let total_visual_length = self.prompt_visual_len + visual_length_up_to_pos;
        self.prompt_y = total_visual_length / self.term_width;
        self.prompt_x = total_visual_length % self.term_width;
    }

    fn update_cursor(&mut self) {
        if self.term_width == 0 {
            return;
        }
        let total_visual_length = self.prompt_visual_len + self.input.display_width() as u16;
        self.cursor_y = total_visual_length / self.term_width;
        self.cursor_x = total_visual_length % self.term_width;
    }
}
