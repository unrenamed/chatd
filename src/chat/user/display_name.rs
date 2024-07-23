use std::hash::Hash;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct DisplayName(String);

impl PartialEq<str> for DisplayName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for DisplayName {
    fn eq(&self, other: &&str) -> bool {
        &self.0 == other
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

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn compare_display_name_with_str() {
        let display_name = DisplayName("test".to_string());
        assert_eq!(display_name, "test");
    }

    #[test]
    fn dereference_display_name() {
        let display_name = DisplayName("test".to_string());
        assert_eq!(*display_name, "test".to_string());
    }

    #[test]
    fn convert_display_name_to_str_ref() {
        let display_name = DisplayName("test".to_string());
        assert_eq!(display_name.as_ref(), "test");
    }

    #[test]
    fn hash_display_name_correctly() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let display_name = DisplayName("test".to_string());
        let mut hasher = DefaultHasher::new();
        display_name.hash(&mut hasher);
        let hash1 = hasher.finish();

        let display_name2 = DisplayName("test".to_string());
        let mut hasher2 = DefaultHasher::new();
        display_name2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn convert_display_name_into_string() {
        let display_name = DisplayName("test".to_string());
        let s: String = display_name.into();
        assert_eq!(s, "test");
    }

    #[test]
    fn convert_string_to_display_name() {
        let s = "test".to_string();
        let display_name: DisplayName = s.clone().into();
        assert_eq!(display_name, "test");
    }

    #[test]
    fn convert_str_to_display_name() {
        let s = "test";
        let display_name: DisplayName = s.into();
        assert_eq!(display_name, "test");
    }

    #[test]
    fn display_display_name() {
        let display_name = DisplayName("test".to_string());
        assert_eq!(format!("{}", display_name), "test");
    }
}
