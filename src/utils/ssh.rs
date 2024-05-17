pub fn split_ssh_key(ssh_key_bytes: &[u8]) -> Option<(String, String, String)> {
    // Convert the vector of bytes into a string for easier manipulation
    let ssh_key_string = match String::from_utf8(ssh_key_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return None, // Invalid UTF-8 bytes
    };

    // Split the SSH key string by whitespace
    let parts: Vec<&str> = ssh_key_string.split_whitespace().collect();

    // Ensure that there are at least 3 parts
    if parts.len() < 3 {
        return None; // Invalid SSH key format
    }

    // Convert each part back to a vector of bytes
    let algo = parts[0].to_string();
    let key = parts[1].to_string();
    let name = parts[2..].join(" ");

    Some((algo, key, name))
}
