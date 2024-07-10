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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::fixture::{FileWriteBin, PathChild};
    use assert_fs::TempDir;
    use std::path::Path;

    fn setup_test_file(content: &str) -> TempDir {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        temp_dir
            .child("test_file.txt")
            .write_binary(content.as_bytes())
            .expect("failed to write to temp file");
        temp_dir
    }

    fn setup_unreadable_file() -> TempDir {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        temp_dir
            .child("unreadable_test_file.txt")
            .write_binary(&[0u8, 128u8, 255u8]) // non-UTF-8 bytes
            .expect("failed to write to temp file");
        temp_dir
    }

    #[test]
    fn test_read_file_to_string_existing_file() {
        let temp_dir = setup_test_file("Hello, world!");
        let file_path = temp_dir.path().join("test_file.txt");
        let result = read_file_to_string(file_path.to_str().unwrap());

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");
    }

    #[test]
    fn test_read_file_to_string_nonexistent_file() {
        let file_path = "/nonexistent/file/path.txt";
        let result = read_file_to_string(file_path);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_read_file_to_string_unreadable_file() {
        let temp_dir = setup_unreadable_file();
        let file_path = temp_dir.path().join("unreadable_test_file.txt");
        let result = read_file_to_string(file_path.to_str().unwrap());

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_read_file_lines_existing_file() {
        let temp_dir = setup_test_file("Line 1\nLine 2\nLine 3");
        let file_path = temp_dir.path().join("test_file.txt");
        let result = read_file_lines(file_path.to_str().unwrap());

        assert!(result.is_ok());
        let lines = result.unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], b"Line 1");
        assert_eq!(lines[1], b"Line 2");
        assert_eq!(lines[2], b"Line 3");
    }

    #[test]
    fn test_read_file_lines_nonexistent_file() {
        let file_path = "/nonexistent/file/path.txt";
        let result = read_file_lines(file_path);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn test_expand_tilde_with_home_dir() {
        let home_dir = dirs::home_dir().expect("failed to get home directory");
        let path_with_tilde = "~/test_file.txt";
        let expanded_path = expand_tilde(path_with_tilde);
        let expected_path = home_dir.join("test_file.txt");

        assert_eq!(expanded_path, expected_path);
    }

    #[test]
    fn test_expand_tilde_no_tilde_in_path() {
        let path_with_tilde = "/test_file.txt";
        let expanded_path = expand_tilde(path_with_tilde);

        assert_eq!(expanded_path, Path::new("/test_file.txt"));
    }
}
