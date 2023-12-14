use rpassword::prompt_password;
use ssh_key::PrivateKey;
use std::{env, path::Path};

type Result<T> = core::result::Result<T, String>;

fn read_private_key(path_to_file: &str) -> Result<PrivateKey> {
    let private_key = match PrivateKey::read_openssh_file(Path::new(path_to_file)) {
        Ok(key) => key,
        Err(err) => return Err(format!("Cannot read private key: {}", err.to_string())),
    };

    if !private_key.is_encrypted() {
        return Ok(private_key);
    }

    let password = match prompt_password("Enter the passphrase: ") {
        Ok(pass) => pass,
        Err(err) => return Err(format!("Cannot read password: {}", err.to_string())),
    };

    let decrypted_key = private_key.decrypt(password);
    match decrypted_key {
        Ok(key) => Ok(key),
        Err(_) => Err(String::from("Cryptographic error. Try another passphrase")),
    }
}

fn main() {
    let args = env::args().collect::<Vec<String>>();
    let default_path = String::from("/Users/nazarposhtarenko/Developer/ssh-chat/keys/test");
    let path_to_private_key = args.get(1).unwrap_or(&default_path);
    let pk = read_private_key(path_to_private_key);

    if pk.is_err() {
        eprintln!("{}", pk.unwrap_err());
        return;
    }

    let pk = pk.unwrap();
    println!("Algo: {}", pk.algorithm());
    println!("Comment: {}", pk.comment());
    println!("Pub: {}", pk.public_key().to_string());
}
