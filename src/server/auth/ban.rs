use humantime;
use log::info;
use std::{str::FromStr, time::Duration};

#[derive(Debug)]
struct BanDuration(Duration);

impl FromStr for BanDuration {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let duration = humantime::parse_duration(s);
        info!("{}", s);
        info!("{:?}", duration);
        match duration {
            Ok(d) => Ok(BanDuration(d)),
            Err(_) => Err("invalid duration string"),
        }
    }
}

#[derive(Debug)]
pub enum Attribute {
    Name(String),
    Fingerprint(String),
    Ip(String),
}

impl FromStr for Attribute {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        info!("{}", s);
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

#[derive(Debug)]
pub struct BanItem {
    pub attribute: Attribute,
    pub duration: Duration,
}

#[derive(Debug)]
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
            let duration_str = parts.next().ok_or("missing duration")?;
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
