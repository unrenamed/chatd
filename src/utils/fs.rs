use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

/// Reads the contents of a file into a string.
pub fn read_file_to_string(file_path: &str) -> Result<String, std::io::Error> {
    let expanded_path = expand_tilde(file_path);
    let mut file = File::open(&expanded_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

/// Reads the lines of a file into a vector of byte vectors.
pub fn read_file_lines(file_path: &str) -> Result<Vec<Vec<u8>>, std::io::Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut result = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let bytes = line.into_bytes();
        result.push(bytes);
    }

    Ok(result)
}

/// Expands a tilde in a file path to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home_dir) = dirs::home_dir() {
            let mut expanded_path = PathBuf::from(home_dir);
            expanded_path.push(&path[2..]); // Exclude the '~/'
            return expanded_path;
        }
    }
    PathBuf::from(path)
}
