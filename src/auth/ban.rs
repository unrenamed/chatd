use humantime;
use std::{str::FromStr, time::Duration};

#[derive(Debug, PartialEq)]
struct BanDuration(Duration);

impl FromStr for BanDuration {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let duration = humantime::parse_duration(s);
        match duration {
            Ok(d) => Ok(BanDuration(d)),
            Err(_) => Err("invalid duration string"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Attribute {
    Name(String),
    Fingerprint(String),
    Ip(String),
}

impl FromStr for Attribute {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((key, value)) = s.split_once('=') {
            match key {
                "name" => Ok(Attribute::Name(value.to_string())),
                "fingerprint" => Ok(Attribute::Fingerprint(value.to_string())),
                "ip" => Ok(Attribute::Ip(value.to_string())),
                _ => Err("unknown attribute"),
            }
        } else {
            Err("invalid attribute format")
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BanItem {
    pub attribute: Attribute,
    pub duration: Duration,
}

#[derive(Debug, PartialEq)]
pub enum BanQuery {
    Single { name: String, duration: Duration },
    Multiple(Vec<BanItem>),
}

impl FromStr for BanQuery {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        // Peek the next part to decide the type of command
        let next_part = parts.clone().next().ok_or("missing arguments")?;

        // Single ban command
        if !next_part.contains('=') {
            let name = next_part.to_string();
            let duration_str = parts.nth(1).ok_or("missing duration")?;
            let duration = duration_str.parse::<BanDuration>()?;
            return Ok(BanQuery::Single {
                name,
                duration: duration.0,
            });
        }

        // Multiple ban command
        let mut ban_items = Vec::new();
        while let Some(part) = parts.next() {
            let attribute = part.parse::<Attribute>()?;
            let duration_str = parts.next().ok_or("missing duration for attribute")?;
            let duration = duration_str.parse::<BanDuration>()?;
            ban_items.push(BanItem {
                attribute,
                duration: duration.0,
            });
        }

        Ok(BanQuery::Multiple(ban_items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_ban_duration_valid_duration_seconds() {
        let input = "30s";
        let expected = BanDuration(Duration::new(30, 0));
        let parsed = BanDuration::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_ban_duration_valid_duration_minutes() {
        let input = "5m";
        let expected = BanDuration(Duration::new(300, 0)); // 5 minutes = 300 seconds
        let parsed = BanDuration::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_ban_duration_valid_duration_hours() {
        let input = "2h";
        let expected = BanDuration(Duration::new(7200, 0)); // 2 hours = 7200 seconds
        let parsed = BanDuration::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_ban_duration_invalid_duration_string() {
        let input = "not_a_duration";
        let parsed = BanDuration::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "invalid duration string");
    }

    #[test]
    fn test_ban_duration_empty_duration_string() {
        let input = "";
        let parsed = BanDuration::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "invalid duration string");
    }

    #[test]
    fn test_attribute_valid_name_attribute() {
        let input = "name=John Doe";
        let expected = Attribute::Name("John Doe".to_string());
        let parsed = Attribute::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_attribute_valid_fingerprint_attribute() {
        let input = "fingerprint=abc123";
        let expected = Attribute::Fingerprint("abc123".to_string());
        let parsed = Attribute::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_attribute_valid_ip_attribute() {
        let input = "ip=192.168.1.1";
        let expected = Attribute::Ip("192.168.1.1".to_string());
        let parsed = Attribute::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_attribute_unknown_attribute() {
        let input = "unknown=value";
        let parsed = Attribute::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "unknown attribute");
    }

    #[test]
    fn test_attribute_invalid_attribute_format() {
        let input = "invalid_format";
        let parsed = Attribute::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "invalid attribute format");
    }

    #[test]
    fn test_ban_query_single() {
        let input = "alice 30s";
        let expected = BanQuery::Single {
            name: "alice".to_string(),
            duration: Duration::new(30, 0),
        };
        let parsed = BanQuery::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_ban_query_multiple() {
        let input = "name=alice 30s ip=192.168.1.1 1h";
        let expected = BanQuery::Multiple(vec![
            BanItem {
                attribute: Attribute::Name("alice".to_string()),
                duration: Duration::new(30, 0),
            },
            BanItem {
                attribute: Attribute::Ip("192.168.1.1".to_string()),
                duration: Duration::new(3600, 0), // 1 hour = 3600 seconds
            },
        ]);
        let parsed = BanQuery::from_str(input).unwrap();
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_ban_query_missing_duration_for_single() {
        let input = "alice";
        let parsed = BanQuery::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "missing duration");
    }

    #[test]
    fn test_ban_query_missing_attribute_duration_for_multiple() {
        let input = "name=alice 30s ip=192.168.1.1";
        let parsed = BanQuery::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "missing duration for attribute");
    }

    #[test]
    fn test_ban_query_invalid_duration_format() {
        let input = "alice 30";
        let parsed = BanQuery::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "invalid duration string");
    }

    #[test]
    fn test_ban_query_unknown_attribute_format() {
        let input = "unknown=30s";
        let parsed = BanQuery::from_str(input);
        assert!(parsed.is_err());
        assert_eq!(parsed.unwrap_err(), "unknown attribute");
    }
}
