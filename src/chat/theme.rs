use crossterm::style::{Color, StyledContent, Stylize};
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Default)]
pub struct Theme {}

impl Theme {
    pub fn style_text<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.with(Color::White)
    }

    pub fn style_system_text<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.with(Color::DarkGrey)
    }

    pub fn style_username<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        let (r, g, b) = Theme::gen_rgb(&s);
        s.with(Color::Rgb { r, g, b })
    }

    pub fn style_tagged_username<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.on(Color::DarkYellow).with(Color::Black)
    }

    fn gen_rgb(s: &str) -> (u8, u8, u8) {
        let mut hasher = FnvHasher::default();
        s.hash(&mut hasher);
        let hash = hasher.finish();

        let r = (hash & 0xFF) as u8;
        let g = ((hash >> 8) & 0xFF) as u8;
        let b = ((hash >> 16) & 0xFF) as u8;
        (r, g, b)
    }
}
