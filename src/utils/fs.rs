use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub fn read_lines(file_path: &str) -> Result<Vec<Vec<u8>>, std::io::Error> {
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
