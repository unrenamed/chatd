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
    prompt_display_width: u16,
    handle: TerminalHandle,
    outbuff: Vec<u8>,
    term_width: u16,
    term_height: u16,
    cursor_x: u16,
    cursor_y: u16,
    input_end_x: u16,
    input_end_y: u16,
}

impl Terminal {
    pub fn new(handle: TerminalHandle) -> Self {
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

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.term_width = width;
        self.term_height = height;
        self.refresh_cursor_coords();
        self.refresh_input_end_coords();
    }

    pub fn get_prompt(&self, user: &User) -> String {
        format!("[{}] ", user.theme.style_username(&user.username))
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
        self.prompt_display_width = display_width(&self.prompt) as u16;
    }

    pub fn clear_input(&mut self) -> Result<(), anyhow::Error> {
        self.input.clear();
        self.print_input_line()?;
        Ok(())
    }

    pub fn exit(&mut self) {
        self.handle.close();
    }

    pub fn print_input_line(&mut self) -> Result<(), anyhow::Error> {
        self.queue_prompt_cleanup()?;
        self.queue_write_prompt()?;
        self.queue_write_input()?;
        self.queue_write_outbuff()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    pub fn print_message(&mut self, msg: &str) -> Result<(), anyhow::Error> {
        self.queue_prompt_cleanup()?;
        self.queue_write_message(msg)?;
        self.queue_write_prompt()?;
        self.queue_write_input()?;
        self.queue_write_outbuff()?;
        self.queue_move_cursor()?;
        self.handle.flush()?;
        Ok(())
    }

    fn queue_prompt_cleanup(&mut self) -> Result<(), anyhow::Error> {
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

    fn queue_write_message(&mut self, msg: &str) -> Result<(), anyhow::Error> {
        queue!(self.handle, style::Print(msg), style::Print(utils::NEWLINE))?;
        Ok(())
    }

    fn queue_write_prompt(&mut self) -> Result<(), anyhow::Error> {
        queue!(self.handle, style::Print(&self.prompt))?;
        self.advance_cursor_pos(self.prompt_display_width);
        Ok(())
    }

    fn queue_write_input(&mut self) -> Result<(), anyhow::Error> {
        queue!(self.handle, style::Print(&self.input))?;
        self.advance_cursor_pos(self.input.display_width() as u16);
        Ok(())
    }

    fn queue_write_outbuff(&mut self) -> Result<(), anyhow::Error> {
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
            .map(|g| utils::display_width(g) as u16)
            .sum::<u16>()
    }
}
