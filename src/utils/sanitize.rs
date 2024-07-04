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
