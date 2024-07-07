pub fn split_ssh_key(key_bytes: &[u8]) -> Option<(String, String)> {
    // Convert the vector of bytes into a string for easier manipulation
    let ssh_key_string = match String::from_utf8(key_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return None, // Invalid UTF-8 bytes
    };

    // Split the SSH key string by whitespace
    let parts: Vec<&str> = ssh_key_string.split_whitespace().collect();

    // Ensure that there are at least 2 parts
    if parts.len() < 2 {
        return None; // Invalid SSH key format
    }

    // Convert each part back to a vector of bytes
    let algo = parts[0].to_string();
    let key = parts[1].to_string();

    Some((algo, key))
}
