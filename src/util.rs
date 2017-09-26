use std::io;
use std::io::Write;
use std::fs::OpenOptions;
use std::path::Path;

pub fn append_to_file(data: &str, filename: &str) -> io::Result<()> {
    let path = Path::new(filename);

    let mut file = OpenOptions::new().append(true).write(true).open(&path)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}
