use rand::distributions::{Distribution, Standard};
use rand::seq::SliceRandom;
use rand::Rng;
use std::hash::Hash;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct UserName(String);

impl Distribution<UserName> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> UserName {
        let adjectives = [
            "Cool", "Mighty", "Brave", "Clever", "Happy", "Calm", "Eager", "Gentle", "Kind",
            "Jolly", "Swift", "Bold", "Fierce", "Wise", "Valiant", "Bright", "Noble", "Zany",
            "Epic",
        ];
        let nouns = [
            "Tiger", "Eagle", "Panda", "Shark", "Lion", "Wolf", "Dragon", "Phoenix", "Hawk",
            "Bear", "Falcon", "Panther", "Griffin", "Lynx", "Orca", "Cobra", "Jaguar", "Kraken",
            "Pegasus", "Stallion",
        ];
        let adjective = adjectives.choose(rng).unwrap_or(&"Guest");
        let noun = nouns.choose(rng).unwrap_or(&"User");
        let number: u16 = rng.gen_range(1..=9999);

        let username = format!("{}{}{}", adjective, noun, number);
        UserName(username)
    }
}

impl PartialEq<str> for UserName {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl std::ops::Deref for UserName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Hash for UserName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<String> for UserName {
    fn into(self) -> String {
        self.0
    }
}

impl From<String> for UserName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&String> for UserName {
    fn from(value: &String) -> Self {
        Self(value.into())
    }
}

impl From<&str> for UserName {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for UserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod should {
    use rand::{rngs::StdRng, SeedableRng};

    use super::*;

    #[test]
    fn generate_username_with_correct_format() {
        let seed = [0u8; 32]; // A fixed seed
        let mut rng = StdRng::from_seed(seed);
        let username: UserName = rng.gen();
        // Since the seed is fixed, the generated username should be consistent across runs
        assert_eq!(username.0, "EagerEagle8340");
    }

    #[test]
    fn generate_unique_usernames() {
        let mut rng = rand::thread_rng();
        let mut usernames = std::collections::HashSet::new();
        let iterations = 1000;

        for _ in 0..iterations {
            let username: UserName = rng.gen();
            assert!(
                usernames.insert(username.0.clone()),
                "Generated duplicate username"
            );
        }

        assert_eq!(usernames.len(), iterations, "Expected unique usernames");
    }
}
