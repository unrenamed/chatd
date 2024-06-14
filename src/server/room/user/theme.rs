use crossterm::style::{Attribute, Color, StyledContent, Stylize};
use fnv::FnvHasher;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use strum::{EnumIter, EnumString, IntoEnumIterator};

#[derive(Debug, Clone)]
enum ThemeColor {
    // Include predefined crossterm colors
    Black,
    DarkGrey,
    Green,
    DarkGreen,
    DarkYellow,
    White,

    // Custom colors
    FromString(String),
}

impl From<ThemeColor> for Color {
    fn from(my_color: ThemeColor) -> Self {
        match my_color {
            ThemeColor::Black => Color::Black,
            ThemeColor::DarkGrey => Color::DarkGrey,
            ThemeColor::Green => Color::Green,
            ThemeColor::DarkGreen => Color::DarkGreen,
            ThemeColor::DarkYellow => Color::DarkYellow,
            ThemeColor::White => Color::White,
            ThemeColor::FromString(s) => {
                let mut hasher = FnvHasher::default();
                s.hash(&mut hasher);
                let hash = hasher.finish();

                let r = (hash & 0xFF) as u8;
                let g = ((hash >> 8) & 0xFF) as u8;
                let b = ((hash >> 16) & 0xFF) as u8;

                Color::Rgb { r, g, b }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, EnumIter, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum Theme {
    Colors,
    Mono,
    Hacker,
}

impl Theme {
    pub fn all() -> Vec<String> {
        Theme::iter()
            .map(|t| t.to_string())
            .collect::<Vec<String>>()
    }
}

impl Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Theme::Colors => "colors",
                Theme::Mono => "mono",
                Theme::Hacker => "hacker",
            }
        )
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::Colors
    }
}

impl Into<UserTheme> for Theme {
    fn into(self) -> UserTheme {
        match self {
            Theme::Colors => UserTheme {
                text_fg: ThemeColor::White.into(),
                system_text_fg: ThemeColor::DarkGrey.into(),
                tagged_username_fg: ThemeColor::Black.into(),
                tagged_username_bg: ThemeColor::DarkYellow.into(),
                username_fg: |s| ThemeColor::FromString(s).into(),
            },
            Theme::Mono => UserTheme {
                text_fg: ThemeColor::White.into(),
                system_text_fg: ThemeColor::White.into(),
                tagged_username_fg: ThemeColor::White.into(),
                tagged_username_bg: ThemeColor::DarkGrey.into(),
                username_fg: |_| ThemeColor::White.into(),
            },
            Theme::Hacker => UserTheme {
                text_fg: ThemeColor::Green.into(),
                system_text_fg: ThemeColor::DarkGreen.into(),
                tagged_username_fg: ThemeColor::DarkGreen.into(),
                tagged_username_bg: ThemeColor::Green.into(),
                username_fg: |_| ThemeColor::Green.into(),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserTheme {
    text_fg: Color,
    system_text_fg: Color,
    username_fg: fn(String) -> Color,
    tagged_username_fg: Color,
    tagged_username_bg: Color,
}

impl Default for UserTheme {
    fn default() -> Self {
        Theme::default().into()
    }
}

impl UserTheme {
    pub fn style_text<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.with(self.text_fg)
    }

    pub fn style_system_text<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.with(self.system_text_fg)
    }

    pub fn style_username<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.with(self.get_username_fg(s))
    }

    pub fn style_tagged_username<'a>(&self, s: &'a str) -> StyledContent<&'a str> {
        s.on(self.tagged_username_bg)
            .with(self.tagged_username_fg)
            .attribute(Attribute::Bold)
    }

    fn get_username_fg(&self, arg: &str) -> Color {
        (self.username_fg)(arg.to_string())
    }
}
