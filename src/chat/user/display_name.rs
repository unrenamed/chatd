use std::hash::Hash;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct DisplayName(String);

impl PartialEq<str> for DisplayName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl std::ops::Deref for DisplayName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for DisplayName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Hash for DisplayName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<String> for DisplayName {
    fn into(self) -> String {
        self.0
    }
}

impl From<String> for DisplayName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&String> for DisplayName {
    fn from(value: &String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for DisplayName {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for DisplayName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
