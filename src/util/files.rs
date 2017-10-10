/*
    Precached - A Linux process monitor and pre-caching daemon
    Copyright (C) 2017 the precached developers

    This file is part of precached.

    Precached is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Precached is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Precached.  If not, see <http://www.gnu.org/licenses/>.
*/

extern crate globset;
extern crate zstd;

use self::globset::{Glob, GlobSetBuilder};
use constants;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::BufReader;
use std::io::prelude::*;
use std::path;
use std::path::Path;

pub fn get_lines_from_file(filename: &str) -> io::Result<Vec<String>> {
    let path = Path::new(filename);
    let file = try!(OpenOptions::new().read(true).open(&path));

    let reader = BufReader::new(file);
    Ok(
        reader
            .lines()
            .filter_map(|l| {
                match l {
                    Ok(s) => Some(s),
                    Err(_) => {
                        // error!("Error while reading file!");
                        return None;
                    }
                }
            })
            .collect(),
    )
}

pub fn read_text_file(filename: &str) -> io::Result<String> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().read(true).open(&path));

    let decompressed = zstd::decode_all(BufReader::new(file)).unwrap();
    let result = String::from_utf8(decompressed).unwrap();

    Ok(result)
}

pub fn write_text_file(filename: &str, text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
    );


    let compressed = zstd::encode_all(
        BufReader::new(text.as_bytes()),
        constants::ZSTD_COMPRESSION_RATIO,
    ).unwrap();
    file.write_all(&compressed)?;
    // file.sync_data()?;

    Ok(())
}

pub fn echo(filename: &str, mut text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().write(true).append(false).open(&path));

    text.push_str("\n");

    trace!("echo: write: '{}' -> {}", &text, &filename);
    file.write_all(text.into_bytes().as_slice())?;

    Ok(())
}

pub fn append(filename: &str, mut text: String) -> io::Result<()> {
    let path = Path::new(filename);
    let mut file = try!(OpenOptions::new().write(true).append(true).open(&path));

    text.push_str("\n");

    trace!("append: write: '{}' -> {}", &text, &filename);
    file.write_all(text.into_bytes().as_slice())?;

    Ok(())
}

pub fn remove_file(filename: &str, dry_run: bool) -> io::Result<()> {
    let path = Path::new(filename);

    trace!("deleting file: '{}'", &filename);

    if !dry_run {
        fs::remove_file(filename)
    } else {
        if let Err(result) = fs::metadata(Path::new(&filename)) {
            Err(result)
        } else {
            Ok(())
        }
    }
}

pub fn is_filename_valid(filename: &String) -> bool {
    let f = filename.trim();

    if f.len() == 0 {
        return false;
    }

    // blacklist linux special mappings
    let blacklist = vec![
        String::from("[mpx]"),
        String::from("[vvar]"),
        String::from("[vdso]"),
        String::from("[heap]"),
        String::from("[stack]"),
        String::from("[vsyscall]"),
        String::from("/memfd:"),
        String::from("(deleted)"),
    ];

    if blacklist.iter().any(|bi| f.contains(bi)) {
        return false;
    }

    return true;
}

pub fn is_file_blacklisted(filename: &String, pattern: &Vec<String>) -> bool {
    let mut builder = GlobSetBuilder::new();

    for p in pattern.iter() {
        builder.add(Glob::new(p).unwrap());
    }

    let set = builder.build().unwrap();
    set.matches(&filename).len() > 0
}

pub fn is_path_a_directory(path: &String) -> bool {
    match fs::metadata(Path::new(&path)) {
        Err(_) => false,
        Ok(metadata) => metadata.is_dir(),
    }
}

pub fn is_file_accessible(filename: &String) -> bool {
    fs::metadata(Path::new(&filename)).is_ok()
}

pub fn visit_dirs(dir: &Path, cb: &mut FnMut(&Path)) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                // trace!("{:#?}", &entry);
                cb(&entry.path());
            }
        }
    }

    Ok(())
}

pub fn walk_directories<F>(entries: &Vec<String>, cb: &mut F) -> io::Result<()>
where
    F: FnMut(&Path),
{
    for e in entries.iter() {
        let path = Path::new(e);
        if path.is_file() {
            cb(path);
        } else {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dirs(&path, cb)?;
                } else {
                    // trace!("{:#?}", &entry);
                    cb(&entry.path());
                }
            }
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use util::files::*;

    #[test]
    fn test_validity_of_filenames() {
        assert_eq!(is_filename_valid(&String::from("filename")), true);

        assert_eq!(is_filename_valid(&String::from("")), false);
        assert_eq!(is_filename_valid(&String::from(" ")), false);
        assert_eq!(is_filename_valid(&String::from("  ")), false);

        assert_eq!(is_filename_valid(&String::from("(deleted)")), false);
        assert_eq!(is_filename_valid(&String::from("123.txt (deleted)")), false);

        assert_eq!(is_filename_valid(&String::from("[stack]")), false);
        assert_eq!(is_filename_valid(&String::from("[heap]")), false);
    }

    #[test]
    fn test_blacklist() {
        let mut blacklist = Vec::new();

        blacklist.push(String::from("123.txt"));

        assert_eq!(
            is_file_blacklisted(&String::from("123.txt"), &blacklist),
            true
        );
        assert_eq!(
            is_file_blacklisted(&String::from("456.txt"), &blacklist),
            false
        );
    }
}
