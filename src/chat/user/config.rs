use super::{DisplayName, TimestampMode, UserTheme};

#[derive(Debug, Clone)]
pub struct HighlightRegex(regex::Regex);

impl HighlightRegex {
    // Method to find the first match in the text
    pub fn find<'a>(&'a self, text: &'a str) -> Option<&'a str> {
        self.0.find(text).map(|m| m.as_str())
    }

    // Method to replace all matches in the text with the replacement string
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        self.0.replace_all(text, replacement).into_owned()
    }
}

impl PartialEq for HighlightRegex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl From<regex::Regex> for HighlightRegex {
    fn from(value: regex::Regex) -> Self {
        Self(value)
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct UserConfig {
    display_name: DisplayName,
    highlight: Option<HighlightRegex>,
    theme: UserTheme,
    timestamp_mode: TimestampMode,
    quiet: bool,
}

impl UserConfig {
    pub fn highlight(&self) -> Option<&HighlightRegex> {
        self.highlight.as_ref()
    }

    pub fn theme(&self) -> &UserTheme {
        &self.theme
    }

    pub fn quiet(&self) -> bool {
        self.quiet
    }

    pub fn timestamp_mode(&self) -> &TimestampMode {
        &self.timestamp_mode
    }

    pub fn display_name(&self) -> &DisplayName {
        &self.display_name
    }

    pub fn switch_quiet_mode(&mut self) {
        self.quiet = !self.quiet;
    }

    pub fn set_timestamp_mode(&mut self, mode: TimestampMode) {
        self.timestamp_mode = mode;
    }

    pub(in crate::chat::user) fn set_highlight(&mut self, text: &str) {
        let pattern = regex::escape(&text);
        let regex = regex::Regex::new(&pattern);
        self.highlight = regex.ok().map(|r| r.into());
    }

    pub(in crate::chat::user) fn set_display_name(&mut self, username: &str) {
        self.display_name = self.theme.style_username(username).to_string().into();
    }

    pub(in crate::chat::user) fn set_theme(&mut self, theme: UserTheme) {
        self.theme = theme;
    }
}
