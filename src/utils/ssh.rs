pub fn split_ssh_key(key_bytes: &[u8]) -> Option<(String, String)> {
    let ssh_key_string = match String::from_utf8(key_bytes.to_vec()) {
        Ok(s) => s,
        Err(_) => return None, // Invalid UTF-8 bytes
    };

    let parts: Vec<&str> = ssh_key_string.split_whitespace().collect();
    if parts.len() < 2 {
        return None; // Invalid SSH key format
    }

    let algo = parts[0].to_string();
    let key = parts[1].to_string();

    Some((algo, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_ssh_key_valid() {
        let key_bytes = b"ssh-rsa AAAAB3NzaC1yc2EAAAABIwAAAQEArO4k9vQ1+...";
        let (algo, key) = split_ssh_key(key_bytes).unwrap();
        assert_eq!(algo, "ssh-rsa");
        assert_eq!(key, "AAAAB3NzaC1yc2EAAAABIwAAAQEArO4k9vQ1+...");
    }

    #[test]
    fn test_split_ssh_key_invalid_utf8() {
        let key_bytes = b"\xff\xfe\xfd"; // Invalid UTF-8 bytes
        assert!(split_ssh_key(key_bytes).is_none());
    }

    #[test]
    fn test_split_ssh_key_invalid_format() {
        let key_bytes = b"ssh-rsa"; // Incomplete SSH key format
        assert!(split_ssh_key(key_bytes).is_none());
    }
}
