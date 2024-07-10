use regex::Regex;

const MAX_LENGTH: usize = 16;

lazy_static::lazy_static! {
    static ref RE_STRIP_NAME: Regex = Regex::new(r"[^\w.-]").unwrap();
}

pub fn name(s: &str) -> String {
    let s = RE_STRIP_NAME.replace_all(s, "").to_string();
    let name_length = if s.len() <= MAX_LENGTH {
        s.len()
    } else {
        MAX_LENGTH
    };
    s[..name_length].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_sanitization() {
        assert_eq!(name("Blaze_Runner123"), "Blaze_Runner123");
        assert_eq!(name("Byte-Bender."), "Byte-Bender.");
        assert_eq!(name("Bob#$%"), "Bob");
        assert_eq!(name("f!r€w@ll"), "frwll");
    }

    #[test]
    fn test_name_truncation() {
        assert_eq!(name("This-Is-VeryLongName12345"), "This-Is-VeryLong");
        assert_eq!(name("ShortName"), "ShortName");
        assert_eq!(name("ExactlySixteen.."), "ExactlySixteen..");
    }

    #[test]
    fn test_name_empty_string() {
        assert_eq!(name(""), "");
    }

    #[test]
    fn test_name_whitespace_only() {
        assert_eq!(name("   "), "");
    }
}
