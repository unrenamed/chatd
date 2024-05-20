use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: usize,
    pub username: String,
    pub fingerprint: String,
}

impl ClientInfo {
    pub fn new() -> Self {
        Self {
            id: 0,
            username: String::new(),
            fingerprint: String::new(),
        }
    }

    pub fn gen_rand_name(&mut self) -> String {
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

        let mut rng = rand::thread_rng();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let noun = nouns.choose(&mut rng).unwrap();
        let number: u16 = rng.gen_range(1..=9999);

        format!("{}{}{}", adjective, noun, number)
    }
}
